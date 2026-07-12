use crate::geo::shape::text::{
    get_font_metrics, get_text_width, text_to_geometry, FontConfig,
};
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

/// Format a numeric column/row value with an optional unit suffix.
fn format_label(value: f64, unit: &str) -> String {
    if unit == "%" {
        format!("{:.0}%", value)
    } else if unit == " pass" && value == 1.0 {
        "1 pass".to_string()
    } else if unit == " pass" {
        format!("{:.0} passes", value)
    } else {
        format!("{:.0}", value)
    }
}

/// Position a text label at `(cx, cy)` with the given alignment and add it to `ops`.
///
/// If `angle_deg` is non-zero the text is rotated around `(cx, cy)`.
#[allow(clippy::too_many_arguments)]
fn add_text_label(
    ops: &mut Ops,
    text: &str,
    font: &FontConfig,
    cx: f64,
    cy: f64,
    h_align: HAlign,
    v_align: VAlign,
    angle_deg: f64,
) {
    let Some(metrics) = get_font_metrics(font) else {
        return;
    };
    let (ascent, _descent, _height) = metrics;
    let width = get_text_width(text, font).unwrap_or(0.0);

    // Compute the baseline origin (text_to_geometry origin is baseline start)
    let origin_x = match h_align {
        HAlign::Center => cx - width / 2.0,
        HAlign::Right => cx - width,
    };
    let origin_y = match v_align {
        VAlign::Bottom => cy + ascent, // bottom of text = baseline + ascent
        VAlign::Center => cy + ascent / 2.0, // center ≈ baseline + ascent/2
    };

    let Some(mut geo) = text_to_geometry(text, font) else {
        return;
    };

    // Rotate around the origin if needed
    if angle_deg.abs() > 0.5 {
        let rad = angle_deg.to_radians();
        let cos = rad.cos();
        let sin = rad.sin();
        geo.transform_2d(cos, sin, -sin, cos, origin_x, origin_y);
    } else {
        geo.transform_2d(1.0, 0.0, 0.0, 1.0, origin_x, origin_y);
    }

    if let Ok(text_ops) = Ops::from_geometry(&geo) {
        ops.extend(&text_ops);
    }
}

enum HAlign {
    Center,
    Right,
}

enum VAlign {
    Center,
    Bottom,
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

    let label_font = FontConfig::new("sans-serif", 2.5);
    let title_font = FontConfig::new("sans-serif", 3.5).bold(true);

    let label_gap = 2.5;

    ops.set_power(0.3);
    ops.set_feed_rate(1000);

    // Determine column/row range descriptors
    let (col_unit, row_unit, col_title, row_title) =
        match params.grid_mode.as_str() {
            "Power vs Passes" => ("%", " pass", "Power", "Passes"),
            "Speed vs Passes" => ("", " pass", "Speed", "Passes"),
            _ => ("%", "", "Power", "Speed"),
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

    // Column headers: centered above each column
    for c in 0..cols {
        let val = col_range.min() + c as f64 * col_step;
        let text = format_label(val, col_unit);
        let cx = margin_left + c as f64 * (shape_w + spacing_x) + shape_w / 2.0;
        let cy = margin_top - label_gap;
        add_text_label(
            ops,
            &text,
            &label_font,
            cx,
            cy,
            HAlign::Center,
            VAlign::Bottom,
            0.0,
        );
    }

    // Row labels: right-aligned to the left of each row
    for r in 0..rows {
        let val = row_range.min() + r as f64 * row_step;
        let text = format_label(val, row_unit);
        let rx = margin_left - label_gap;
        let ry = margin_top + r as f64 * (shape_h + spacing_y) + shape_h / 2.0;
        add_text_label(
            ops,
            &text,
            &label_font,
            rx,
            ry,
            HAlign::Right,
            VAlign::Center,
            0.0,
        );
    }

    // Column axis title: centered above column headers
    let col_title_cx = margin_left + cols as f64 * (shape_w + spacing_x) / 2.0
        - spacing_x / 2.0;
    let col_title_cy = margin_top - label_gap - 5.0;
    add_text_label(
        ops,
        col_title,
        &title_font,
        col_title_cx,
        col_title_cy,
        HAlign::Center,
        VAlign::Bottom,
        0.0,
    );

    // Row axis title: rotated 90°, left of row labels
    let row_title_x = margin_left - label_gap - 10.0;
    let row_title_y = margin_top + rows as f64 * (shape_h + spacing_y) / 2.0
        - spacing_y / 2.0;
    add_text_label(
        ops,
        row_title,
        &title_font,
        row_title_x,
        row_title_y,
        HAlign::Center,
        VAlign::Center,
        -90.0,
    );
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

fn get_material_test_proportional_size(params: &MaterialTestGridParams) -> f64 {
    let cols = params.cols;
    let shape_size = params.shape_size;
    let spacing = params.spacing;
    let base_margin_left = (shape_size * 1.5).min(15.0);
    (cols * shape_size as u32) as f64
        + ((cols - 1) as f64 * spacing)
        + base_margin_left
}

fn get_material_test_proportional_height(
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
