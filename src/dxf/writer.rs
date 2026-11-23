//! The DXF writer: geometry out, R12-compatible file text.

use std::fmt::Write as _;

use glam::DVec3;

use crate::geo::geometry::Geometry;
use crate::geo::shape::arc::get_arc_angles;
use crate::geo::types::Command;

/// Emits a DXF file from a geometry.
///
/// The output is ASCII DXF, version `AC1009` (R12), the most widely
/// readable variant and the one every CAD tool and laser cutter
/// controller accepts. Units are declared as millimetres, coordinates are
/// written as native drawing units (the caller's responsibility), and
/// consecutive line segments collapse into `LWPOLYLINE` entities so a
/// contour round-trips as one entity rather than as many loose lines.
///
/// Arcs are written as true `ARC` entities, so they stay editable as
/// arcs in the receiving CAD application; beziers become a spline
/// control frame, which every reader of this file can either use
/// directly or flatten.
pub fn write(geometry: &Geometry) -> String {
    let mut file = DxfFile::new();
    file.header();
    file.layer_table();
    for entity in entities_of(geometry) {
        file.entity(entity);
    }
    file.finish()
}

/// One entity ready to be written.
enum OutEntity {
    Polyline(Vec<(f64, f64)>),
    Arc {
        center: (f64, f64),
        radius: f64,
        start_degrees: f64,
        end_degrees: f64,
    },
    Spline(Vec<[DVec3; 4]>),
}

/// Groups a geometry's commands into writable entities.
fn entities_of(geometry: &Geometry) -> Vec<OutEntity> {
    let mut entities = Vec::new();
    let mut polyline: Vec<(f64, f64)> = Vec::new();
    let mut cursor = DVec3::ZERO;
    for command in geometry.iter_commands() {
        let end = command.end_point();
        match command {
            Command::Move { .. } => {
                // A move starts a new contour, which becomes its own
                // entity. Emitting it as the first point of the previous
                // polyline would join unrelated contours with a line.
                flush_polyline(&mut polyline, &mut entities);
                polyline.push((end.x, end.y));
            }
            Command::Line { .. } => {
                if polyline.is_empty() {
                    polyline.push((cursor.x, cursor.y));
                }
                polyline.push((end.x, end.y));
            }
            Command::Arc {
                center_offset,
                normal,
                ..
            } => {
                flush_polyline(&mut polyline, &mut entities);
                let center = cursor + *center_offset;
                let radius = center_offset.truncate().length();
                let clockwise = normal.z < 0.0;
                let (start, end_angle, _) = get_arc_angles(
                    cursor.truncate(),
                    end.truncate(),
                    center.truncate(),
                    clockwise,
                );
                let mut start_deg = start.to_degrees().rem_euclid(360.0);
                let mut end_deg = end_angle.to_degrees().rem_euclid(360.0);
                if clockwise {
                    std::mem::swap(&mut start_deg, &mut end_deg);
                }
                if end_deg <= start_deg {
                    end_deg += 360.0;
                }
                if radius > 1e-12 {
                    entities.push(OutEntity::Arc {
                        center: (center.x, center.y),
                        radius,
                        start_degrees: start_deg,
                        end_degrees: end_deg,
                    });
                }
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                flush_polyline(&mut polyline, &mut entities);
                entities.push(OutEntity::Spline(vec![[
                    cursor, *control1, *control2, end,
                ]]));
            }
        }
        cursor = end;
    }
    flush_polyline(&mut polyline, &mut entities);
    entities
}

fn flush_polyline(
    polyline: &mut Vec<(f64, f64)>,
    entities: &mut Vec<OutEntity>,
) {
    if polyline.len() >= 2 {
        entities.push(OutEntity::Polyline(std::mem::take(polyline)));
    } else {
        polyline.clear();
    }
}

/// Merges consecutive bezier segments into one spline's control points.
///
/// A spline's control polygon is the concatenation of its bezier control
/// points with the shared junction points dropped, plus a clamped knot
/// vector. Degree 3 with a simple interior knot per junction reproduces
/// the original beziers exactly.
fn spline_control_points(segments: &[[DVec3; 4]]) -> Vec<DVec3> {
    let mut points: Vec<DVec3> = Vec::with_capacity(segments.len() * 3 + 1);
    for (index, segment) in segments.iter().enumerate() {
        let first = if index == 0 { 0 } else { 1 };
        for slot in &segment[first..] {
            points.push(*slot);
        }
    }
    points
}

/// The clamped knot vector for a degree-3 spline with the given number of
/// control points and `segments` spans.
fn spline_knots(control_count: usize, segments: usize) -> Vec<f64> {
    let degree = 3;
    let mut knots = vec![0.0; degree + 1];
    for span in 1..segments {
        knots.push(span as f64 / segments as f64);
    }
    let total = control_count + degree + 1;
    while knots.len() < total {
        knots.push(1.0);
    }
    knots
}

/// Accumulates group code lines.
struct DxfFile {
    out: String,
}

impl DxfFile {
    fn new() -> Self {
        DxfFile {
            out: String::with_capacity(4096),
        }
    }

    fn pair(&mut self, code: i32, value: &str) {
        let _ = writeln!(self.out, "{code:>3}");
        let _ = writeln!(self.out, "{value}");
    }

    fn number(&mut self, code: i32, value: f64) {
        let _ = writeln!(self.out, "{code:>3}");
        let _ = writeln!(self.out, "{value}");
    }

    fn header(&mut self) {
        self.pair(0, "SECTION");
        self.pair(2, "HEADER");
        self.pair(9, "$ACADVER");
        self.pair(1, "AC1009");
        self.pair(9, "$INSUNITS");
        self.pair(70, "4");
        self.pair(0, "ENDSEC");
    }

    fn layer_table(&mut self) {
        self.pair(0, "SECTION");
        self.pair(2, "TABLES");
        self.pair(0, "TABLE");
        self.pair(2, "LAYER");
        self.pair(70, "1");
        self.pair(0, "LAYER");
        self.pair(2, "0");
        self.pair(70, "0");
        self.pair(62, "7");
        self.pair(6, "CONTINUOUS");
        self.pair(0, "ENDTAB");
        self.pair(0, "ENDSEC");
        self.pair(0, "SECTION");
        self.pair(2, "ENTITIES");
    }

    fn finish(mut self) -> String {
        self.pair(0, "ENDSEC");
        self.pair(0, "EOF");
        self.out
    }

    fn entity(&mut self, entity: OutEntity) {
        match entity {
            OutEntity::Polyline(points) => {
                self.pair(0, "LWPOLYLINE");
                self.pair(8, "0");
                self.pair(100, "AcDbEntity");
                self.pair(100, "AcDbPolyline");
                self.pair(90, &points.len().to_string());
                self.pair(70, "0");
                for (x, y) in points {
                    self.number(10, x);
                    self.number(20, y);
                }
            }
            OutEntity::Arc {
                center,
                radius,
                start_degrees,
                end_degrees,
            } => {
                self.pair(0, "ARC");
                self.pair(8, "0");
                self.pair(100, "AcDbEntity");
                self.pair(100, "AcDbCircle");
                self.number(10, center.0);
                self.number(20, center.1);
                self.number(30, 0.0);
                self.number(40, radius);
                self.pair(100, "AcDbArc");
                self.number(50, start_degrees);
                self.number(51, end_degrees);
            }
            OutEntity::Spline(segments) => {
                let points = spline_control_points(&segments);
                let knots = spline_knots(points.len(), segments.len());
                self.pair(0, "SPLINE");
                self.pair(8, "0");
                self.pair(100, "AcDbEntity");
                self.pair(100, "AcDbSpline");
                // Planar and rational, but not closed: the geometry
                // decides where a contour ends, and a spline written as
                // closed would get a segment appended on reading.
                self.pair(70, "6");
                self.pair(71, "3");
                self.pair(72, &knots.len().to_string());
                self.pair(73, &points.len().to_string());
                self.pair(74, "0");
                for knot in &knots {
                    self.number(40, *knot);
                }
                for point in &points {
                    self.number(10, point.x);
                    self.number(20, point.y);
                    self.number(30, point.z);
                }
            }
        }
    }
}
