use std::collections::BTreeMap;

use crate::geo::shape::text::{text_to_geometry, FontConfig};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::tracelet::Tracelet;
use crate::ops::assembly::{AssembleCtx, Assembler};
use crate::ops::container::Ops;
use crate::ops::state::State;
use crate::ops::types::{MoveCmd, OpCategory, ToolPose};
use crate::trace_types::{Meta, MetaValue, MoveKind};
use crate::types::{Point, Point3D};

pub(crate) mod trace_helpers;

/// Spec for the material-test-grid assembler.
///
/// Carries every parameter the grid generator needs (cell count,
/// speed/power/passes/offset ranges, label settings) plus the
/// target ``size_mm`` of the workpiece area to fill.
#[derive(Clone, Debug)]
pub struct MaterialTestGridSpec {
    pub size_mm: (f64, f64),
    pub cols: u32,
    pub rows: u32,
    pub min_speed: f64,
    pub max_speed: f64,
    pub min_power: f64,
    pub max_power: f64,
    pub min_passes: u32,
    pub max_passes: u32,
    pub min_offset: f64,
    pub max_offset: f64,
    pub fixed_speed: f64,
    pub fixed_power: f64,
    pub shape_size: f64,
    pub spacing: f64,
    pub line_interval_mm: f64,
    /// "engrave" or "cut"
    pub mode: String,
    /// "Power vs Speed", "Power vs Passes", "Speed vs Passes", or
    /// "Speed vs Offset"
    pub grid_mode: String,
    /// Whether to generate text labels (column headers, row labels, axis titles).
    pub include_labels: bool,
    /// Power for label engraving (0.0–1.0). Default 0.1 (10%).
    pub label_power: f64,
    /// Feed rate for label engraving in mm/min. Default 1000.
    pub label_speed: i32,
}

impl Assembler for MaterialTestGridSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks
            .report_progress(0.0, "material_test_grid: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = generate_material_test_grid(self, ctx.trace, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks
            .report_progress(1.0, "material_test_grid: done");
        Ok(meta)
    }

    fn name(&self) -> &'static str {
        "material_test_grid"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Generate a material test grid with varying speed and power settings.
///
/// Creates grid cells arranged in rows × cols, each with baked-in power,
/// speed, and pass count.  All motion is written through the provided
/// [`Tracelet`] so that `drain()` produces the full toolpath.
pub fn generate_material_test_grid(
    params: &MaterialTestGridSpec,
    trace: &mut Tracelet,
    base_state: &State,
) -> Result<AssemblyMeta, crate::RaygeoError> {
    let size_mm = params.size_mm;
    let (_target_width, target_height) = size_mm;

    // Calculate proportional base size
    let base_width = get_material_test_proportional_size(params);
    let base_height = get_material_test_proportional_height(params);
    let scale_x = if base_width > 1e-9 {
        size_mm.0 / base_width
    } else {
        1.0
    };
    let scale_y = if base_height > 1e-9 {
        size_mm.1 / base_height
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
            "Speed vs Offset" => (
                ColRange::Linear(params.min_speed, params.max_speed),
                ColRange::Linear(params.min_offset, params.max_offset),
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

    trace.set_attrs(trace_helpers::build_attrs(params));

    let is_engrave = params.mode.eq_ignore_ascii_case("engrave");
    let section_type = if is_engrave {
        crate::ops::enums::SectionType::RasterFill
    } else {
        crate::ops::enums::SectionType::VectorOutline
    };
    let raster_mode = if is_engrave {
        Some(crate::ops::enums::RasterMode::ConstantPower)
    } else {
        None
    };
    trace
        .ops_section_start(section_type, "material_test_grid", raster_mode)
        .expect("valid section params");

    // Wrap labels in a state block
    if params.include_labels {
        trace.state_block_start(Some("labels"));
        let mut label_ops = Ops::new();
        generate_labels(
            &mut label_ops,
            params,
            base_state,
            scale_x,
            scale_y,
            margin_left,
            margin_top,
        );
        if !label_ops.is_empty() {
            label_ops
                .scale(1.0, -1.0, 1.0)
                .translate(0.0, target_height, 0.0);
            let mut prev = Point3D::ZERO;
            for node in label_ops.commands {
                let (end, is_travel) = match &node.category {
                    OpCategory::Moving {
                        end,
                        cmd: MoveCmd::MoveTo,
                        ..
                    } => (*end, true),
                    OpCategory::Moving { end, .. } => (*end, false),
                    _ => {
                        trace.push_raw(node);
                        continue;
                    }
                };
                let tool = trace_helpers::tool_snapshot(end, prev);
                prev = end;
                trace.push_raw(node);
                if is_travel {
                    trace.move_event(MoveKind::Travel, tool, None);
                } else {
                    trace.cut(tool, None);
                }
            }
        }
        trace.state_block_end();
    }

    // Build grid cells sorted by risk: highest speed first, then lowest power
    let mut cells: Vec<GridCell> = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let col_val = col_range.min() + c as f64 * col_step;
            let row_val = row_range.min() + r as f64 * row_step;

            let (speed, power, passes, offset) = match params.grid_mode.as_str()
            {
                "Power vs Passes" => (
                    params.fixed_speed,
                    col_val,
                    (row_val.round() as u32).max(1),
                    0.0,
                ),
                "Speed vs Passes" => (
                    col_val,
                    params.fixed_power,
                    (row_val.round() as u32).max(1),
                    0.0,
                ),
                "Speed vs Offset" => {
                    let speed = col_val;
                    // Scale power with speed (anchored at fixed_power at
                    // min_speed) so darkness stays comparable across the
                    // row - otherwise slow columns overburn and fast
                    // columns look too faint to compare offsets by eye.
                    let power = if params.min_speed > 1e-9 {
                        (params.fixed_power * speed / params.min_speed)
                            .clamp(1.0, 100.0)
                    } else {
                        params.fixed_power
                    };
                    (speed, power, 1, row_val)
                }
                _ => (row_val, col_val, 1, 0.0),
            };

            cells.push(GridCell {
                col: c,
                row: r,
                x: margin_left + c as f64 * (shape_w + spacing_x),
                y: margin_top + r as f64 * (shape_h + spacing_y),
                width: shape_w,
                height: shape_h,
                speed,
                power,
                passes,
                offset,
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

    let line_spacing = params.line_interval_mm;

    let total_cells = cells.len() as u32;
    let init_pos = cells
        .first()
        .map(|c| Point3D::new(c.x, target_height - c.y - c.height, 0.0))
        .unwrap_or(Point3D::ZERO);
    trace.init(
        trace_helpers::tool_snapshot(init_pos, init_pos),
        Some(trace_helpers::init_meta(total_cells)),
    );

    for (cell_idx, cell) in (0_u32..).zip(cells.iter()) {
        let cell_name = format!("cell-r{}-c{}", cell.row, cell.col);
        trace.state_block_start(Some(&cell_name));
        let cell_state = State {
            power: cell.power / 100.0,
            feed_rate: Some(cell.speed as i32),
            ..base_state.clone()
        };
        trace.apply_state(&cell_state);

        let cell_meta = cell_cut_meta(
            cell_idx,
            cell.col,
            cell.row,
            cell.speed,
            cell.power,
            cell.passes,
        );

        // Y-flip: convert from grid-Y-up to display-Y-down.
        // Rectangle at (x, y) with height h → (x, H-y-h) with same h.
        let fy = target_height - cell.y - cell.height;

        for _ in 0..cell.passes {
            if is_engrave {
                draw_filled_box(
                    trace,
                    cell.x,
                    fy,
                    cell.width,
                    cell.height,
                    line_spacing,
                    cell.offset,
                    &cell_meta,
                );
            } else {
                draw_rectangle(
                    trace,
                    cell.x,
                    fy,
                    cell.width,
                    cell.height,
                    &cell_meta,
                );
            }
        }

        trace.state_block_end();

        if first_point.is_none() {
            first_point = Some(Point::new(cell.x, fy));
        }
        last_point = Some(Point::new(cell.x + cell.width, fy + cell.height));
    }

    trace
        .ops_section_end(section_type, raster_mode)
        .expect("valid section params");

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

    trace.exit(
        trace_helpers::tool_snapshot(
            Point3D::new(end_pos.x, end_pos.y, 0.0),
            Point3D::new(start_pos.x, start_pos.y, 0.0),
        ),
        None,
    );

    Ok(meta)
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

    // Use the visual bounding box for centering.
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
    params: &MaterialTestGridSpec,
    base_state: &State,
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

    // Font size in mm proportional to margin,
    // converted to points (1 pt = 25.4/72 mm).
    let pt_to_mm = 25.4 / 72.0;
    let margin_min = margin_left.min(margin_top);
    let axis_font_mm = margin_min * 0.25;
    let grid_font_mm = axis_font_mm * 0.85;
    let label_font = FontConfig::new("sans-serif", grid_font_mm / pt_to_mm);
    let title_font =
        FontConfig::new("sans-serif", axis_font_mm / pt_to_mm).bold(true);

    let label_state = State {
        power: params.label_power,
        feed_rate: Some(params.label_speed),
        ..base_state.clone()
    };
    ops.apply_state(&label_state);

    // Determine column/row range descriptors.
    let (col_title, row_title) = match params.grid_mode.as_str() {
        "Power vs Passes" => ("Power (%)", "Passes"),
        "Speed vs Passes" => ("Speed (mm/min)", "Passes"),
        "Speed vs Offset" => ("Speed (mm/min)", "Offset (mm)"),
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
            "Speed vs Offset" => (
                ColRange::Linear(params.min_speed, params.max_speed),
                ColRange::Linear(params.min_offset, params.max_offset),
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
    // Uses round-then-truncate (like Python int(round())) for row values,
    // except Speed vs Offset which needs decimals - otherwise multiple
    // distinct sub-mm offset rows would collapse to the same label.
    for r in 0..rows {
        let val = row_range.min() + r as f64 * row_step;
        let text = if params.grid_mode == "Speed vs Offset" {
            format!("{val:+.2}")
        } else {
            format_row_label(val)
        };
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

    // Fixed-parameter labels.
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

fn cell_cut_meta(
    cell_idx: u32,
    col: u32,
    row: u32,
    speed: f64,
    power: f64,
    passes: u32,
) -> Meta {
    let mut m: Meta = BTreeMap::new();
    m.insert("cell_idx".into(), MetaValue::U32(cell_idx));
    m.insert("col".into(), MetaValue::U32(col));
    m.insert("row".into(), MetaValue::U32(row));
    m.insert("speed".into(), MetaValue::F64(speed));
    m.insert("power".into(), MetaValue::F64(power));
    m.insert("passes".into(), MetaValue::U32(passes));
    m
}

fn draw_rectangle(
    trace: &mut Tracelet,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    cell_meta: &Meta,
) {
    let prev =
        trace_helpers::tool_snapshot(Point3D::new(x, y, 0.0), Point3D::ZERO);
    trace.move_to(x, y, 0.0, None);
    trace.move_event(MoveKind::Travel, prev, None);
    trace.line_to(x + w, y, 0.0, None);
    trace.cut(
        trace_helpers::tool_snapshot(
            Point3D::new(x + w, y, 0.0),
            Point3D::new(x, y, 0.0),
        ),
        Some(cell_meta.clone()),
    );
    trace.line_to(x + w, y + h, 0.0, None);
    trace.cut(
        trace_helpers::tool_snapshot(
            Point3D::new(x + w, y + h, 0.0),
            Point3D::new(x + w, y, 0.0),
        ),
        Some(cell_meta.clone()),
    );
    trace.line_to(x, y + h, 0.0, None);
    trace.cut(
        trace_helpers::tool_snapshot(
            Point3D::new(x, y + h, 0.0),
            Point3D::new(x + w, y + h, 0.0),
        ),
        Some(cell_meta.clone()),
    );
    trace.line_to(x, y, 0.0, None);
    trace.cut(
        trace_helpers::tool_snapshot(
            Point3D::new(x, y, 0.0),
            Point3D::new(x, y + h, 0.0),
        ),
        Some(cell_meta.clone()),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_filled_box(
    trace: &mut Tracelet,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    line_spacing: f64,
    offset: f64,
    cell_meta: &Meta,
) {
    if h < 1e-6 {
        return;
    }

    let num_lines = (h / line_spacing) as u32;
    if num_lines < 1 {
        trace.move_to(x, y + h / 2.0, 0.0, None);
        trace.move_event(
            MoveKind::Travel,
            trace_helpers::tool_snapshot(
                Point3D::new(x, y + h / 2.0, 0.0),
                Point3D::ZERO,
            ),
            None,
        );
        trace.line_to(x + w, y + h / 2.0, 0.0, None);
        trace.cut(
            trace_helpers::tool_snapshot(
                Point3D::new(x + w, y + h / 2.0, 0.0),
                Point3D::new(x, y + h / 2.0, 0.0),
            ),
            Some(cell_meta.clone()),
        );
        return;
    }

    let y_step = h / num_lines as f64;
    for i in 0..=num_lines {
        let cur_y = y + i as f64 * y_step;
        if i % 2 == 0 {
            trace.move_to(x, cur_y, 0.0, None);
            trace.move_event(
                MoveKind::Travel,
                trace_helpers::tool_snapshot(
                    Point3D::new(x, cur_y, 0.0),
                    Point3D::ZERO,
                ),
                None,
            );
            trace.line_to(x + w, cur_y, 0.0, None);
            trace.cut(
                trace_helpers::tool_snapshot(
                    Point3D::new(x + w, cur_y, 0.0),
                    Point3D::new(x, cur_y, 0.0),
                ),
                Some(cell_meta.clone()),
            );
        } else {
            // Right-to-left pass: shift by the cell's offset, mirroring
            // BidirScanOffsetTransformer correction for real
            // engraves.
            trace.move_to(x + w + offset, cur_y, 0.0, None);
            trace.move_event(
                MoveKind::Travel,
                trace_helpers::tool_snapshot(
                    Point3D::new(x + w + offset, cur_y, 0.0),
                    Point3D::ZERO,
                ),
                None,
            );
            trace.line_to(x + offset, cur_y, 0.0, None);
            trace.cut(
                trace_helpers::tool_snapshot(
                    Point3D::new(x + offset, cur_y, 0.0),
                    Point3D::new(x + w + offset, cur_y, 0.0),
                ),
                Some(cell_meta.clone()),
            );
        }
    }
}

pub fn get_material_test_proportional_size(
    params: &MaterialTestGridSpec,
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
    params: &MaterialTestGridSpec,
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
    #[allow(dead_code)]
    col: u32,
    #[allow(dead_code)]
    row: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    speed: f64,
    power: f64,
    passes: u32,
    offset: f64,
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
