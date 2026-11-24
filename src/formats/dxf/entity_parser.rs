//! Entity parsing: turning DXF records into world-space shapes.

use std::f64::consts::TAU;

use glam::{DMat4, DVec3};

use crate::formats::dxf::entity::{
    ocs_point, resolve_winding, Insert, Shape, Vertex, Winding,
};
use crate::formats::dxf::group_code::{find, find_f64, Item};
use crate::formats::dxf::ocs::Ocs;

/// What one record parsed into.
#[derive(Debug)]
pub enum Parsed {
    /// An entity with drawable geometry.
    Shape(Shape),
    /// A block insertion, recorded for a later resolution pass.
    Insert(Insert),
    /// An entity that carries no geometry, e.g. a dimension or a
    /// viewport. Harmless and ignored silently.
    Skipped,
    /// An entity whose type is not supported. Reported to the user.
    Unsupported(String),
}

/// The entity type name of a record, which is its first item.
pub fn record_type(record: &[Item]) -> Option<&str> {
    record
        .first()
        .filter(|item| item.code == 0)
        .map(|i| i.as_str())
}

/// The result of parsing one section's records.
#[derive(Debug, Default)]
pub struct SectionContent {
    /// Drawable shapes, each with the layer it belongs to.
    pub shapes: Vec<(String, Shape)>,
    /// Block insertions still to be resolved.
    pub inserts: Vec<(String, Insert)>,
}

/// Parses the records of one section into shapes and insertions.
///
/// `parent_transform` maps the coordinates of everything in `records`
/// into world space. For modelspace it is the identity; for the contents
/// of a block it is the accumulated insertion transform.
pub fn parse_records(
    records: &[&[Item]],
    parent_transform: DMat4,
    inherited_layer: Option<&str>,
    issues: &mut Vec<String>,
) -> SectionContent {
    let mut content = SectionContent::default();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        let Some(kind) = record_type(record) else {
            index += 1;
            continue;
        };
        let kind = kind.to_uppercase();
        if kind == "POLYLINE" {
            let (shape, layer, consumed) =
                parse_polyline(record, &records[index + 1..], parent_transform);
            if let Some(shape) = shape {
                let layer = effective_layer(&layer, inherited_layer);
                content.shapes.push((layer, shape));
            }
            index += 1 + consumed;
            continue;
        }
        match parse_single(record, &kind, parent_transform) {
            Parsed::Shape(shape) => {
                let layer = effective_layer(&layer_of(record), inherited_layer);
                content.shapes.push((layer, shape));
            }
            Parsed::Insert(insert) => {
                let layer = effective_layer(&insert.layer, inherited_layer);
                content.inserts.push((layer, insert));
            }
            Parsed::Skipped => {}
            Parsed::Unsupported(name) => {
                let message = format!(
                    "{name} entities are not supported and were skipped"
                );
                if !issues.contains(&message) {
                    issues.push(message);
                }
            }
        }
        index += 1;
    }
    content
}

/// The layer name an entity is drawn on.
pub fn layer_of(record: &[Item]) -> String {
    find(record, 8)
        .map(|item| item.as_str().to_string())
        .unwrap_or_default()
}

/// Resolves layer "0" inside a block to the insertion's layer.
///
/// Per the DXF specification, entities on layer `0` inside a block take
/// on the layer of the `INSERT` that placed them.
pub fn effective_layer(layer: &str, inherited_layer: Option<&str>) -> String {
    match (layer, inherited_layer) {
        ("", Some(inherited)) => inherited.to_string(),
        ("0", Some(inherited)) => inherited.to_string(),
        _ => layer.to_string(),
    }
}

/// Parses one record that is a complete entity by itself.
fn parse_single(record: &[Item], kind: &str, transform: DMat4) -> Parsed {
    match kind {
        "LINE" => parse_line(record, transform),
        "CIRCLE" => parse_circle(record, transform),
        "ARC" => parse_arc(record, transform),
        "ELLIPSE" => parse_ellipse(record, transform),
        "POINT" => parse_point(record, transform),
        "LWPOLYLINE" => parse_lwpolyline(record, transform),
        "SPLINE" => parse_spline(record, transform),
        "INSERT" => parse_insert(record, transform),
        "TEXT" | "MTEXT" => Parsed::Unsupported(kind.to_string()),
        "SOLID" | "TRACE" | "3DFACE" | "HATCH" => {
            parse_solid(record, transform)
        }
        "ATTDEF" | "ATTRIB" | "SEQEND" | "VERTEX" | "VIEWPORT"
        | "DIMENSION" | "LEADER" | "MLEADER" | "XLINE" | "RAY" => {
            Parsed::Skipped
        }
        _ => Parsed::Unsupported(kind.to_string()),
    }
}

/// The extrusion vector of a record, defaulting to +Z.
fn extrusion_of(record: &[Item]) -> DVec3 {
    let x = find_f64(record, 210).unwrap_or(0.0);
    let y = find_f64(record, 220).unwrap_or(0.0);
    let z = find_f64(record, 230).unwrap_or(1.0);
    if x == 0.0 && y == 0.0 && z == 0.0 {
        DVec3::Z
    } else {
        DVec3::new(x, y, z)
    }
}

/// Reads the X/Y/Z triple starting at `code`.
fn find_point(record: &[Item], code: i32, default_z: f64) -> DVec3 {
    DVec3::new(
        find_f64(record, code).unwrap_or(0.0),
        find_f64(record, code + 10).unwrap_or(0.0),
        find_f64(record, code + 20).unwrap_or(default_z),
    )
}

fn parse_line(record: &[Item], transform: DMat4) -> Parsed {
    let ocs = Ocs::from_extrusion(extrusion_of(record));
    let start =
        transform.transform_point3(ocs.to_world(find_point(record, 10, 0.0)));
    let end =
        transform.transform_point3(ocs.to_world(find_point(record, 11, 0.0)));
    Parsed::Shape(Shape::Line { start, end })
}

fn parse_point(record: &[Item], transform: DMat4) -> Parsed {
    let ocs = Ocs::from_extrusion(extrusion_of(record));
    let point =
        transform.transform_point3(ocs.to_world(find_point(record, 10, 0.0)));
    Parsed::Shape(Shape::Point(point))
}

fn parse_circle(record: &[Item], transform: DMat4) -> Parsed {
    let extrusion = extrusion_of(record);
    let ocs = Ocs::from_extrusion(extrusion);
    let center =
        transform.transform_point3(ocs.to_world(find_point(record, 10, 0.0)));
    let radius = find_f64(record, 40).unwrap_or(0.0);
    let normal = transform
        .transform_vector3(extrusion)
        .normalize_or(DVec3::Z);
    Parsed::Shape(Shape::Circle {
        center,
        radius,
        normal,
    })
}

fn parse_arc(record: &[Item], transform: DMat4) -> Parsed {
    let extrusion = extrusion_of(record);
    let ocs = Ocs::from_extrusion(extrusion);
    let center =
        transform.transform_point3(ocs.to_world(find_point(record, 10, 0.0)));
    let radius = find_f64(record, 40).unwrap_or(0.0);
    let start_angle = find_f64(record, 50).unwrap_or(0.0).to_radians();
    let end_angle = find_f64(record, 51).unwrap_or(0.0).to_radians();
    let winding = resolve_winding(Winding::CounterClockwise, &ocs);
    Parsed::Shape(Shape::Arc {
        center,
        radius,
        start_angle,
        end_angle,
        winding,
    })
}

fn parse_ellipse(record: &[Item], transform: DMat4) -> Parsed {
    let extrusion = extrusion_of(record);
    let ocs = Ocs::from_extrusion(extrusion);
    let center =
        transform.transform_point3(ocs.to_world(find_point(record, 10, 0.0)));
    let major = ocs.to_ocs_vector(
        transform.transform_vector3(find_point(record, 11, 0.0)),
    );
    let minor_ratio = find_f64(record, 40).unwrap_or(1.0);
    let start_param = find_f64(record, 41).unwrap_or(0.0);
    let end_param = find_f64(record, 42).unwrap_or(TAU);
    Parsed::Shape(Shape::Ellipse {
        center,
        major_axis: major,
        minor_ratio,
        start_param,
        end_param,
    })
}

/// Parses a lightweight polyline.
///
/// Each vertex is a `10`/`20` pair, optionally followed by a `42` bulge
/// describing the segment leaving that vertex. The bulge appears after
/// its vertex, so a vertex is appended immediately and the bulge then
/// lands on the last appended one.
fn parse_lwpolyline(record: &[Item], transform: DMat4) -> Parsed {
    let ocs = Ocs::from_extrusion(extrusion_of(record));
    let flags = find(record, 70).map(|i| i.as_i64_or(0)).unwrap_or(0);
    let closed = flags & 1 != 0;
    let elevation = find_f64(record, 38).unwrap_or(0.0);
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut pending_x: Option<f64> = None;
    for item in record {
        match item.code {
            10 => pending_x = Some(item.as_f64_or(0.0)),
            20 => {
                let Some(x) = pending_x.take() else {
                    continue;
                };
                let point = ocs_point(&ocs, elevation, x, item.as_f64_or(0.0));
                vertices.push(Vertex {
                    position: transform.transform_point3(point),
                    bulge: 0.0,
                });
            }
            42 => {
                if let Some(last) = vertices.last_mut() {
                    last.bulge = item.as_f64_or(0.0);
                }
            }
            _ => {}
        }
    }
    if !vertices.is_empty() {
        Parsed::Shape(Shape::Polyline { vertices, closed })
    } else {
        Parsed::Skipped
    }
}

/// Parses a POLYLINE record plus the VERTEX records that follow it.
///
/// Returns the shape, its layer, and how many following records were
/// consumed (the vertices and the terminating SEQEND).
fn parse_polyline(
    header: &[Item],
    following: &[&[Item]],
    transform: DMat4,
) -> (Option<Shape>, String, usize) {
    let ocs = Ocs::from_extrusion(extrusion_of(header));
    let flags = find(header, 70).map(|i| i.as_i64_or(0)).unwrap_or(0);
    let closed = flags & 1 != 0;
    let layer = layer_of(header);
    let mut vertices = Vec::new();
    let mut consumed = 0;
    for record in following {
        let Some(kind) = record_type(record) else {
            break;
        };
        if kind.eq_ignore_ascii_case("VERTEX") {
            let vertex = ocs_point(
                &ocs,
                find_f64(record, 30).unwrap_or(0.0),
                find_f64(record, 10).unwrap_or(0.0),
                find_f64(record, 20).unwrap_or(0.0),
            );
            vertices.push(Vertex {
                position: transform.transform_point3(vertex),
                bulge: find_f64(record, 42).unwrap_or(0.0),
            });
            consumed += 1;
        } else if kind.eq_ignore_ascii_case("SEQEND") {
            consumed += 1;
            break;
        } else {
            break;
        }
    }
    if vertices.is_empty() {
        return (None, layer, consumed);
    }
    (Some(Shape::Polyline { vertices, closed }), layer, consumed)
}

fn parse_solid(record: &[Item], transform: DMat4) -> Parsed {
    let ocs = Ocs::from_extrusion(extrusion_of(record));
    let mut corners = Vec::new();
    for code in [10, 11, 12, 13] {
        let Some(x) = find(record, code) else {
            continue;
        };
        let point = ocs_point(
            &ocs,
            find_f64(record, code + 20).unwrap_or(0.0),
            x.as_f64_or(0.0),
            find_f64(record, code + 10).unwrap_or(0.0),
        );
        corners.push(Vertex {
            position: transform.transform_point3(point),
            bulge: 0.0,
        });
    }
    if corners.len() < 3 {
        return Parsed::Skipped;
    }
    Parsed::Shape(Shape::Polyline {
        vertices: corners,
        closed: true,
    })
}

fn parse_insert(record: &[Item], transform: DMat4) -> Parsed {
    let ocs = Ocs::from_extrusion(extrusion_of(record));
    let Some(name) = find(record, 2) else {
        return Parsed::Skipped;
    };
    let insertion_point = find_point(record, 10, 0.0);
    let x_scale = find_f64(record, 41).unwrap_or(1.0);
    let y_scale = find_f64(record, 42).unwrap_or(1.0);
    let z_scale = find_f64(record, 43).unwrap_or(1.0);
    let rotation = find_f64(record, 50).unwrap_or(0.0).to_radians();
    let local = Insert::build_transform(
        insertion_point,
        x_scale,
        y_scale,
        z_scale,
        rotation,
        &ocs,
    );
    Parsed::Insert(Insert {
        layer: layer_of(record),
        block_name: name.as_str().to_string(),
        insertion_point,
        x_scale,
        y_scale,
        z_scale,
        rotation,
        transform: transform * local,
    })
}

/// The number of samples per knot span when a spline cannot be
/// represented exactly as cubic beziers.
const SPLINE_SAMPLES_PER_SPAN: usize = 16;

/// Converts a spline record into cubic beziers.
///
/// DXF splines are B-splines of arbitrary degree. They are converted to a
/// chain of cubic beziers by knot insertion, which reproduces the curve
/// exactly rather than approximating it. Degree 2 and 3 curves map one
/// segment per knot span; other degrees are first raised to degree 3
/// where possible, and otherwise flattened through their samples.
fn parse_spline(record: &[Item], transform: DMat4) -> Parsed {
    let ocs = Ocs::from_extrusion(extrusion_of(record));
    let degree = find(record, 71).map(|i| i.as_i64_or(3)).unwrap_or(3);
    let flags = find(record, 70).map(|i| i.as_i64_or(0)).unwrap_or(0);
    let closed = flags & 1 != 0;
    let knots = collect_f64(record, 40);
    let weights = collect_f64(record, 41);
    let control: Vec<DVec3> =
        collect_points(record, 10, 20, 30).into_iter().collect();
    let fit: Vec<DVec3> =
        collect_points(record, 11, 21, 31).into_iter().collect();
    let segments = if control.len() >= 2 {
        let mapped = to_world(control, ocs, transform);
        let degree = degree.max(1) as usize;
        let knots = normalized_knots(&knots, mapped.len(), degree);
        beziers_from_bspline(&mapped, &weights, &knots, degree)
    } else if fit.len() >= 2 {
        let mapped = to_world(fit, ocs, transform);
        let dense = interpolate_through(&mapped);
        cubic_chain(&dense)
    } else {
        return Parsed::Skipped;
    };
    if segments.is_empty() {
        return Parsed::Skipped;
    }
    let mut segments = segments;
    if closed {
        let first = segments[0][0];
        let last = segments[segments.len() - 1][3];
        if first.distance(last) > 1e-9 {
            segments.push([last, last, first, first]);
        }
    }
    Parsed::Shape(Shape::Spline { segments })
}

fn to_world(points: Vec<DVec3>, ocs: Ocs, transform: DMat4) -> Vec<DVec3> {
    points
        .into_iter()
        .map(|p| transform.transform_point3(ocs.to_world(p)))
        .collect()
}

/// Splits a dense polyline into cubic beziers of a few samples each.
fn cubic_chain(samples: &[DVec3]) -> Vec<[DVec3; 4]> {
    let points: Vec<glam::DVec2> =
        samples.iter().map(|p| p.truncate()).collect();
    crate::geo::shape::bezier::fit_polyline_as_beziers(&points, samples)
}

fn collect_f64(record: &[Item], code: i32) -> Vec<f64> {
    record
        .iter()
        .filter(|item| item.code == code)
        .map(|item| item.as_f64_or(0.0))
        .collect()
}

fn collect_points(record: &[Item], cx: i32, cy: i32, cz: i32) -> Vec<DVec3> {
    let mut points = Vec::new();
    let mut current = DVec3::ZERO;
    let mut has_x = false;
    for item in record {
        if item.code == cx {
            current.x = item.as_f64_or(0.0);
            has_x = true;
        } else if item.code == cy && has_x {
            current.y = item.as_f64_or(0.0);
        } else if item.code == cz && has_x {
            current.z = item.as_f64_or(0.0);
            points.push(current);
            current = DVec3::ZERO;
            has_x = false;
        }
    }
    if has_x {
        points.push(current);
    }
    points
}

/// Completes a knot vector that some writers leave short.
fn normalized_knots(knots: &[f64], n: usize, degree: usize) -> Vec<f64> {
    let expected = n + degree + 1;
    if knots.len() == expected {
        return knots.to_vec();
    }
    // Fall back to a clamped uniform knot vector.
    let mut result = vec![0.0; degree + 1];
    let inner = expected.saturating_sub(2 * (degree + 1));
    for i in 1..=inner {
        result.push(i as f64 / (inner + 1) as f64);
    }
    result.extend(std::iter::repeat_n(1.0, degree + 1));
    result.truncate(expected);
    result
}

/// A Catmull-Rom interpolation through the given points, for fit-point
/// splines that carry no control polygon.
fn interpolate_through(points: &[DVec3]) -> Vec<DVec3> {
    let mut samples = Vec::new();
    for window in points.windows(2) {
        for step in 0..8 {
            let t = step as f64 / 8.0;
            samples.push(window[0].lerp(window[1], t));
        }
    }
    if let Some(last) = points.last() {
        samples.push(*last);
    }
    samples
}

/// Converts a B-spline into a chain of cubic beziers.
///
/// Each interior knot is inserted until it has full multiplicity, which
/// splits the curve into independent bezier segments that together form
/// exactly the original curve. Rational splines are handled by working in
/// homogeneous coordinates and dividing out the weights at the end.
///
/// Degrees other than 3 are raised to 3 first; degrees that cannot be
/// raised degrade to a dense sampling, which is the only approximation in
/// this path.
fn beziers_from_bspline(
    control: &[DVec3],
    weights: &[f64],
    knots: &[f64],
    degree: usize,
) -> Vec<[DVec3; 4]> {
    if control.len() < 2 {
        return Vec::new();
    }
    let weight =
        |i: usize| weights.get(i).copied().filter(|w| *w > 0.0).unwrap_or(1.0);
    // Work in homogeneous coordinates: (x*w, y*w, z*w, w).
    let mut points: Vec<glam::DVec4> = control
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let w = weight(i);
            glam::DVec4::new(p.x * w, p.y * w, p.z * w, w)
        })
        .collect();
    let mut knots = knots.to_vec();
    if degree == 3 {
        if let Some(result) = split_into_beziers(&mut points, &mut knots) {
            return result;
        }
    }
    // Fall back to sampling for degrees that are not cubic.
    let samples = sample_spline(&points, &knots, degree);
    let cartesian: Vec<DVec3> = samples
        .into_iter()
        .map(|p| {
            if p.w.abs() < 1e-12 {
                DVec3::new(p.x, p.y, p.z)
            } else {
                DVec3::new(p.x / p.w, p.y / p.w, p.z / p.w)
            }
        })
        .collect();
    cubic_chain(&cartesian)
}

/// Inserts knots until every interior knot has full multiplicity, then
/// reads the resulting control points as bezier segments.
///
/// After the insertion every span between distinct knots has exactly
/// `degree + 1` control points of its own, so consecutive groups of four
/// are the bezier segments of the curve.
fn split_into_beziers(
    points: &mut Vec<glam::DVec4>,
    knots: &mut Vec<f64>,
) -> Option<Vec<[DVec3; 4]>> {
    let degree = 3;
    let unique = unique_interior_knots(knots, points.len(), degree);
    for knot in unique {
        let multiplicity =
            knots.iter().filter(|k| (**k - knot).abs() < 1e-12).count();
        let insertions = (degree + 1).saturating_sub(multiplicity);
        for _ in 0..insertions {
            if !insert_knot(points, knots, degree, knot) {
                return None;
            }
        }
    }
    let span_count = points.len().saturating_sub(degree);
    if span_count == 0 {
        return None;
    }
    let mut segments = Vec::with_capacity(span_count);
    for span in 0..span_count {
        let from = span;
        let window = &points[from..from + degree + 1];
        let mut segment = [DVec3::ZERO; 4];
        for (slot, point) in segment.iter_mut().zip(window.iter()) {
            let w = if point.w.abs() < 1e-12 { 1.0 } else { point.w };
            *slot = DVec3::new(point.x / w, point.y / w, point.z / w);
        }
        segments.push(segment);
    }
    Some(segments)
}

/// The distinct interior knot values, in order.
///
/// The first `degree` and last `degree` knots clamp the curve to its end
/// points and are excluded, as is the final knot, which ends the last
/// span rather than starting a new one.
fn unique_interior_knots(
    knots: &[f64],
    control_count: usize,
    degree: usize,
) -> Vec<f64> {
    if knots.len() < control_count + degree + 1 {
        return Vec::new();
    }
    // The knot vector is clamped: the last `degree + 1` entries repeat the
    // end value and start no span. Treating them as interior knots makes
    // the insertion pass append degenerate control points, which render
    // as zig-zag geometry.
    let end = control_count.saturating_sub(degree).max(degree + 1);
    let mut unique: Vec<f64> = Vec::new();
    for &knot in &knots[degree..end] {
        if unique
            .last()
            .is_none_or(|last| (*last - knot).abs() > 1e-12)
        {
            unique.push(knot);
        }
    }
    unique
}

/// Inserts one knot by the Boehm algorithm.
///
/// Returns false when the knot cannot be inserted. New control points are
/// computed into a scratch buffer first, then written back, so the
/// shifted indices cannot read points that were already overwritten.
fn insert_knot(
    points: &mut Vec<glam::DVec4>,
    knots: &mut Vec<f64>,
    degree: usize,
    knot: f64,
) -> bool {
    let n = points.len();
    if n < degree + 1 || knots.len() < n + degree + 1 {
        return false;
    }
    let last = n - 1;
    let span = if knot >= knots[n] {
        last
    } else if knot <= knots[degree] {
        degree
    } else {
        let mut low = degree;
        let mut high = n;
        while high - low > 1 {
            let mid = (low + high) / 2;
            if knot < knots[mid] {
                high = mid;
            } else {
                low = mid;
            }
        }
        low
    };
    let mut scratch: Vec<glam::DVec4> =
        (0..=degree).map(|i| points[span - degree + i]).collect();
    for r in 1..=degree {
        for i in (r..=degree).rev() {
            let index = span - degree + i;
            let denominator = knots[index + degree + 1 - r] - knots[index];
            let alpha = if denominator.abs() < 1e-12 {
                0.0
            } else {
                (knot - knots[index]) / denominator
            };
            scratch[i] = scratch[i - 1] * (1.0 - alpha) + scratch[i] * alpha;
        }
    }
    let new_point = scratch[degree];
    for (i, point) in scratch.iter().enumerate() {
        points[span - degree + i] = *point;
    }
    points.insert(span + 1, new_point);
    knots.insert(span + 1, knot);
    true
}

/// Samples a spline densely, for degrees that cannot be split exactly.
fn sample_spline(
    points: &[glam::DVec4],
    knots: &[f64],
    degree: usize,
) -> Vec<glam::DVec4> {
    let n = points.len();
    if n < 2 || degree == 0 || knots.len() < n + degree + 1 {
        return points.to_vec();
    }
    let start = knots[degree];
    let end = knots[n];
    if end <= start || !end.is_finite() {
        return points.to_vec();
    }
    let spans = n.saturating_sub(degree).max(1);
    let steps = spans * SPLINE_SAMPLES_PER_SPAN;
    (0..=steps)
        .map(|i| {
            let t = start + (end - start) * i as f64 / steps as f64;
            evaluate_spline(points, knots, degree, t)
        })
        .collect()
}

/// Evaluates a spline at `t` by the de Boor algorithm.
fn evaluate_spline(
    points: &[glam::DVec4],
    knots: &[f64],
    degree: usize,
    t: f64,
) -> glam::DVec4 {
    let n = points.len();
    let span = if t >= knots[n] {
        n - 1
    } else if t <= knots[degree] {
        degree
    } else {
        let mut low = degree;
        let mut high = n;
        while high - low > 1 {
            let mid = (low + high) / 2;
            if t < knots[mid] {
                high = mid;
            } else {
                low = mid;
            }
        }
        low
    };
    let mut d: Vec<glam::DVec4> =
        (0..=degree).map(|j| points[span - degree + j]).collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let index = span - degree + j;
            let denominator = knots[index + degree + 1 - r] - knots[index];
            let alpha = if denominator.abs() < 1e-12 {
                0.0
            } else {
                (t - knots[index]) / denominator
            };
            d[j] = d[j - 1] * (1.0 - alpha) + d[j] * alpha;
        }
    }
    d[degree]
}
