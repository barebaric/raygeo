use crate::geo::shape::text::{text_to_geometry, FontConfig};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::container::Ops;
use crate::ops::types::ToolPose;
use crate::types::{Point, Point3D};

/// Parameters for the material test grid assembler.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MaterialTestGridParams {
    pub cols: u32,
    pub rows: u32,
    pub min_speed: f64,
    pub max_speed: f64,
    pub min_power: f64,
    pub max_power: f64,
    pub min_passes: u32,
    pub max_passes: u32,
    pub fixed_speed: f64,
    pub fixed_power: f64,
    pub shape_size: f64,
    pub spacing: f64,
    pub line_interval_mm: f64,
    /// "engrave" or "cut"
    pub mode: String,
    /// "Power vs Speed", "Power vs Passes", or "Speed vs Passes"
    pub grid_mode: String,
    /// Whether to generate text labels (column headers, row labels, axis titles).
    pub include_labels: bool,
}

/// Generate a material test grid with varying speed and power settings.
///
/// Creates grid cells arranged in rows × cols, each with baked-in power,
/// speed, and pass count. Returns the motion `Ops` for the grid cells.
/// Labels are **not** generated here — the caller should handle text
/// vectorization via the `post_fn` hook.
pub fn generate_material_test_grid(
    params: &MaterialTestGridParams,
    size_mm: (f64, f64),
) -> Result<(Ops, AssemblyMeta), crate::RaygeoError> {
    let (target_width, target_height) = size_mm;

    // Calculate proportional base size
    let base_width = get_material_test_proportional_size(params);
    let base_height = get_material_test_proportional_height(params);
    let scale_x = if base_width > 1e-9 {
        target_width / base_width
    } else {
        1.0
    };
    let scale_y = if base_height > 1e-9 {
        target_height / base_height
    } else {
        1.0
    };

    let cols = params.cols;
    let rows = params.rows;
    let shape_size = params.shape_size;
    let spacing = params.spacing;

    // Calculate column/row ranges based on grid_mode
    let (col_range, row_range): (ColRange, ColRange) =
        match params.grid_mode.as_str() {
            "Power vs Passes" => (
                ColRange::Linear(params.min_power, params.max_power),
                ColRange::Linear(
                    params.min_passes as f64,
                    params.max_passes as f64,
                ),
            ),
            "Speed vs Passes" => (
                ColRange::Linear(params.min_speed, params.max_speed),
                ColRange::Linear(
                    params.min_passes as f64,
                    params.max_passes as f64,
                ),
            ),
            _ => (
                ColRange::Linear(params.min_power, params.max_power),
                ColRange::Linear(params.min_speed, params.max_speed),
            ),
        };

    let col_step = if cols > 1 {
        (col_range.max() - col_range.min()) / (cols - 1) as f64
    } else {
        0.0
    };
    let row_step = if rows > 1 {
        (row_range.max() - row_range.min()) / (rows - 1) as f64
    } else {
        0.0
    };

    let base_margin = (shape_size * 1.5).min(15.0);
    let (margin_left, margin_top) =
        (base_margin * scale_x, base_margin * scale_y);

    let shape_w = shape_size * scale_x;
    let shape_h = shape_size * scale_y;
    let spacing_x = spacing * scale_x;
    let spacing_y = spacing * scale_y;

    let mut first_point: Option<Point> = None;
    let mut last_point: Option<Point> = None;

    let mut ops = Ops::new();

    // Build grid cells sorted by risk: highest speed first, then lowest power
    let mut cells: Vec<GridCell> = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let col_val = col_range.min() + c as f64 * col_step;
            let row_val = row_range.min() + r as f64 * row_step;

            let (speed, power, passes) = match params.grid_mode.as_str() {
                "Power vs Passes" => (
                    params.fixed_speed,
                    col_val,
                    (row_val.round() as u32).max(1),
                ),
                "Speed vs Passes" => (
                    col_val,
                    params.fixed_power,
                    (row_val.round() as u32).max(1),
                ),
                _ => (row_val, col_val, 1),
            };

            cells.push(GridCell {
                x: margin_left + c as f64 * (shape_w + spacing_x),
                y: margin_top + r as f64 * (shape_h + spacing_y),
                width: shape_w,
                height: shape_h,
                speed,
                power,
                passes,
            });
        }
    }

    // Sort by risk: highest speed first, then lowest power, then fewest passes
    cells.sort_by(|a, b| {
        b.speed
            .partial_cmp(&a.speed)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.power
                    .partial_cmp(&b.power)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.passes.cmp(&b.passes).reverse())
    });

    let is_engrave = params.mode.eq_ignore_ascii_case("engrave");
    let line_spacing = params.line_interval_mm;

    for cell in &cells {
        ops.set_power(0.0);
        ops.set_power(cell.power / 100.0);
        ops.set_feed_rate(cell.speed as i32);

        for _ in 0..cell.passes {
            if is_engrave {
                draw_filled_box(
                    &mut ops,
                    cell.x,
                    cell.y,
                    cell.width,
                    cell.height,
                    line_spacing,
                );
            } else {
                draw_rectangle(
                    &mut ops,
                    cell.x,
                    cell.y,
                    cell.width,
                    cell.height,
                );
            }
        }

        if first_point.is_none() {
            first_point = Some(Point::new(cell.x, cell.y));
        }
        last_point =
            Some(Point::new(cell.x + cell.width, cell.y + cell.height));
    }

    // Generate text labels before the grid (machined first, lower power)
    if params.include_labels {
        generate_labels(
            &mut ops,
            params,
            scale_x,
            scale_y,
            margin_left,
            margin_top,
        );
    }

    if !ops.is_empty() {
        ops.scale(1.0, -1.0, 1.0).translate(0.0, target_height, 0.0);
    }

    let start_pos = first_point.unwrap_or(Point::ZERO);
    let end_pos = last_point.unwrap_or(Point::ZERO);

    let meta = AssemblyMeta {
        start: ToolPose {
            pos: Point3D::new(start_pos.x, start_pos.y, 0.0),
            heading: 0.0,
        },
        end: ToolPose {
            pos: Point3D::new(end_pos.x, end_pos.y, 0.0),
            heading: 0.0,
        },
    };

    Ok((ops, meta))
}

/// Format a numeric column label: truncate toward zero (like Python's int()).
fn format_col_label(value: f64) -> String {
    format!("{}", value as i64)
}

/// Format a numeric row label: round then truncate (like Python's int(round())).
fn format_row_label(value: f64) -> String {
    format!("{}", (value + 0.5).floor() as i64)
}

/// Position a text label at `(cx, cy)` with the given alignment and add it to `ops`.
///
/// The label coordinate system matches the grid (Y‑up before the final flip).
/// `text_to_geometry` returns Y‑up geometry.  We Y‑flip it here so it reads
/// correctly after the caller applies the final Y‑flip to the combined ops.
#[allow(clippy::too_many_arguments)]
fn add_text_label(
    ops: &mut Ops,
    text: &str,
    font: &FontConfig,
    cx: f64,
    cy: f64,
    h_align: HAlign,
    angle_deg: f64,
) {
    let Some(mut geo) = text_to_geometry(text, font) else {
        return;
    };

    // Use the visual bounding box for centering, matching the original
    // rayforge producer which calls geo.rect() and uses the total width.
    let rect = geo.rect();
    let width = rect.max.x - rect.min.x;

    // Y-flip: Y-UP → Y-DOWN so the final collective flip makes it Y-UP again.
    geo.transform_2d(1.0, 0.0, 0.0, -1.0, 0.0, 0.0);

    // Alignment offset — matches the original _vectorize_text_to_ops
    // which uses geo.rect() for width and applies x_offset directly
    // to the el_x translation (no left-bearing adjustment).
    let x_offset = match h_align {
        HAlign::Left => 0.0,
        HAlign::Center => -width / 2.0,
        HAlign::Right => -width,
    };

    // 1. Alignment offset (pre-rotation)
    geo.transform_2d(1.0, 0.0, 0.0, 1.0, x_offset, 0.0);

    // 2. Rotation around origin (if needed).
    //    Uses the same convention as DMat3::from_rotation_z (applied
    //    via Matrix.rotation() in the Python bindings):
    //    x' = cos*x - sin*y,   y' = sin*x + cos*y
    if angle_deg.abs() > 0.5 {
        let rad = angle_deg.to_radians();
        let cos = rad.cos();
        let sin = rad.sin();
        geo.transform_2d(cos, -sin, sin, cos, 0.0, 0.0);
    }

    // 3. Translate to final position
    geo.transform_2d(1.0, 0.0, 0.0, 1.0, cx, cy);

    if let Ok(text_ops) = Ops::from_geometry(&geo) {
        ops.extend(&text_ops);
    }
}

enum HAlign {
    Left,
    Center,
    Right,
}

/// Generate text labels for the material test grid.
#[allow(clippy::too_many_arguments)]
fn generate_labels(
    ops: &mut Ops,
    params: &MaterialTestGridParams,
    scale_x: f64,
    scale_y: f64,
    margin_left: f64,
    margin_top: f64,
) {
    let cols = params.cols;
    let rows = params.rows;
    let shape_w = params.shape_size * scale_x;
    let shape_h = params.shape_size * scale_y;
    let spacing_x = params.spacing * scale_x;
    let spacing_y = params.spacing * scale_y;

    // Match original producer: font size in mm proportional to margin,
    // then converted to points (1 pt = 25.4/72 mm).
    let pt_to_mm = 25.4 / 72.0;
    let margin_min = margin_left.min(margin_top);
    let axis_font_mm = margin_min * 0.25;
    let grid_font_mm = axis_font_mm * 0.85;
    let label_font = FontConfig::new("sans-serif", grid_font_mm / pt_to_mm);
    let title_font =
        FontConfig::new("sans-serif", axis_font_mm / pt_to_mm).bold(true);

    ops.set_power(0.3);
    ops.set_feed_rate(1000);

    // Determine column/row range descriptors (axis titles match the
    // original rayforge producer exactly).
    let (col_title, row_title) = match params.grid_mode.as_str() {
        "Power vs Passes" => ("Power (%)", "Passes"),
        "Speed vs Passes" => ("Speed (mm/min)", "Passes"),
        _ => ("Power (%)", "Speed (mm/min)"),
    };

    let (col_range, row_range): (ColRange, ColRange) =
        match params.grid_mode.as_str() {
            "Power vs Passes" => (
                ColRange::Linear(params.min_power, params.max_power),
                ColRange::Linear(
                    params.min_passes as f64,
                    params.max_passes as f64,
                ),
            ),
            "Speed vs Passes" => (
                ColRange::Linear(params.min_speed, params.max_speed),
                ColRange::Linear(
                    params.min_passes as f64,
                    params.max_passes as f64,
                ),
            ),
            _ => (
                ColRange::Linear(params.min_power, params.max_power),
                ColRange::Linear(params.min_speed, params.max_speed),
            ),
        };

    let col_step = if cols > 1 {
        (col_range.max() - col_range.min()) / (cols - 1) as f64
    } else {
        0.0
    };
    let row_step = if rows > 1 {
        (row_range.max() - row_range.min()) / (rows - 1) as f64
    } else {
        0.0
    };

    // Column headers: centered above each column (y = 75% of margin_top).
    // Uses truncation (like Python int()) for column values.
    for c in 0..cols {
        let val = col_range.min() + c as f64 * col_step;
        let text = format_col_label(val);
        let cx = margin_left + c as f64 * (shape_w + spacing_x) + shape_w / 2.0;
        let cy = margin_top * 0.75;
        add_text_label(ops, &text, &label_font, cx, cy, HAlign::Center, 0.0);
    }

    // Row labels: right-aligned at 90% of margin_left.
    // Uses round-then-truncate (like Python int(round())) for row values.
    for r in 0..rows {
        let val = row_range.min() + r as f64 * row_step;
        let text = format_row_label(val);
        let rx = margin_left * 0.9;
        let ry = margin_top + r as f64 * (shape_h + spacing_y) + shape_h / 2.0;
        add_text_label(ops, &text, &label_font, rx, ry, HAlign::Right, 0.0);
    }

    // Column axis title: centered above grid at 30% of margin_top
    let col_title_cx = margin_left + cols as f64 * (shape_w + spacing_x) / 2.0
        - spacing_x / 2.0;
    let col_title_cy = margin_top * 0.3;
    add_text_label(
        ops,
        col_title,
        &title_font,
        col_title_cx,
        col_title_cy,
        HAlign::Center,
        0.0,
    );

    // Row axis title: rotated -90°, aligned at 30% of margin_left
    let row_title_x = margin_left * 0.3;
    let row_title_y = margin_top + rows as f64 * (shape_h + spacing_y) / 2.0
        - spacing_y / 2.0;
    add_text_label(
        ops,
        row_title,
        &title_font,
        row_title_x,
        row_title_y,
        HAlign::Center,
        -90.0,
    );

    // Fixed-parameter labels (matching original producer)
    let fixed_label_offset = margin_min * 0.15;
    let fixed_font =
        FontConfig::new("sans-serif", grid_font_mm * 0.8 / pt_to_mm);
    match params.grid_mode.as_str() {
        "Power vs Passes" => {
            let text = format!("Speed: {:.0} mm/min", params.fixed_speed);
            add_text_label(
                ops,
                &text,
                &fixed_font,
                fixed_label_offset,
                fixed_label_offset,
                HAlign::Left,
                0.0,
            );
        }
        "Speed vs Passes" => {
            let text = format!("Power: {:.0}%", params.fixed_power);
            add_text_label(
                ops,
                &text,
                &fixed_font,
                fixed_label_offset,
                fixed_label_offset,
                HAlign::Left,
                0.0,
            );
        }
        _ => {}
    }
}

fn draw_rectangle(ops: &mut Ops, x: f64, y: f64, w: f64, h: f64) {
    ops.move_to(x, y, 0.0, None);
    ops.line_to(x + w, y, 0.0, None);
    ops.line_to(x + w, y + h, 0.0, None);
    ops.line_to(x, y + h, 0.0, None);
    ops.line_to(x, y, 0.0, None);
}

fn draw_filled_box(
    ops: &mut Ops,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    line_spacing: f64,
) {
    if h < 1e-6 {
        return;
    }

    let num_lines = (h / line_spacing) as u32;
    if num_lines < 1 {
        ops.move_to(x, y + h / 2.0, 0.0, None);
        ops.line_to(x + w, y + h / 2.0, 0.0, None);
        return;
    }

    let y_step = h / num_lines as f64;
    for i in 0..=num_lines {
        let cur_y = y + i as f64 * y_step;
        if i % 2 == 0 {
            ops.move_to(x, cur_y, 0.0, None);
            ops.line_to(x + w, cur_y, 0.0, None);
        } else {
            ops.move_to(x + w, cur_y, 0.0, None);
            ops.line_to(x, cur_y, 0.0, None);
        }
    }
}

pub fn get_material_test_proportional_size(
    params: &MaterialTestGridParams,
) -> f64 {
    let cols = params.cols;
    let shape_size = params.shape_size;
    let spacing = params.spacing;
    let base_margin_left = (shape_size * 1.5).min(15.0);
    (cols * shape_size as u32) as f64
        + ((cols - 1) as f64 * spacing)
        + base_margin_left
}

pub fn get_material_test_proportional_height(
    params: &MaterialTestGridParams,
) -> f64 {
    let rows = params.rows;
    let shape_size = params.shape_size;
    let spacing = params.spacing;
    let base_margin_top = (shape_size * 1.5).min(15.0);
    (rows * shape_size as u32) as f64
        + ((rows - 1) as f64 * spacing)
        + base_margin_top
}

#[derive(Clone, Debug)]
struct GridCell {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    speed: f64,
    power: f64,
    passes: u32,
}

#[derive(Clone, Copy, Debug)]
enum ColRange {
    Linear(f64, f64),
}

impl ColRange {
    fn min(&self) -> f64 {
        match self {
            ColRange::Linear(a, _) => *a,
        }
    }

    fn max(&self) -> f64 {
        match self {
            ColRange::Linear(_, b) => *b,
        }
    }
}
