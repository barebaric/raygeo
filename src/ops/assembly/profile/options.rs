use std::path::PathBuf;

use crate::ops::cut::CutDirection;
use crate::types::{Point3D, Polygon};

/// Options for inner-boundary adaptive profiling.
///
/// Walks the **inset** boundary (pocket wall offset inward by tool
/// radius), material-aware around islands.
#[derive(Clone, Debug)]
pub struct ProfileInnerOptions {
    pub boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub radius: f64,
    pub cut_z: f64,
    pub safe_z: f64,
    pub step_length: f64,
    pub wall_margin: f64,
    pub stock_to_leave: f64,
    pub cut_direction: CutDirection,
    pub start_pos: Option<Point3D>,
    pub tolerance: f64,
    pub expansion_batch_size: usize,
    pub cancel_check: Option<fn() -> bool>,
    pub engagement_area_threshold: f64,
    pub engagement_angle_threshold: f64,
    pub trace_path: Option<PathBuf>,
}

impl Default for ProfileInnerOptions {
    fn default() -> Self {
        Self {
            boundary: Vec::new(),
            islands: Vec::new(),
            radius: 3.0,
            cut_z: -5.0,
            safe_z: 2.0,
            step_length: 0.6,
            wall_margin: 0.0,
            stock_to_leave: 0.0,
            cut_direction: CutDirection::Ccw,
            start_pos: None,
            tolerance: 0.1,
            expansion_batch_size: 20,
            cancel_check: None,
            engagement_area_threshold: 0.0,
            engagement_angle_threshold: std::f64::consts::PI,
            trace_path: None,
        }
    }
}

/// Options for outer-boundary adaptive profiling.
///
/// Walks the **grown** boundary (stock outline offset outward by tool
/// radius).  Islands are ignored — they are geometrically on the other
/// side of the wall.
#[derive(Clone, Debug)]
pub struct ProfileOuterOptions {
    pub boundary: Polygon,
    pub radius: f64,
    pub cut_z: f64,
    pub safe_z: f64,
    pub step_length: f64,
    pub wall_margin: f64,
    pub stock_to_leave: f64,
    pub cut_direction: CutDirection,
    pub start_pos: Option<Point3D>,
    pub tolerance: f64,
    pub expansion_batch_size: usize,
    pub cancel_check: Option<fn() -> bool>,
    pub engagement_area_threshold: f64,
    pub engagement_angle_threshold: f64,
    pub trace_path: Option<PathBuf>,
}

impl Default for ProfileOuterOptions {
    fn default() -> Self {
        Self {
            boundary: Vec::new(),
            radius: 3.0,
            cut_z: -5.0,
            safe_z: 2.0,
            step_length: 0.6,
            wall_margin: 0.0,
            stock_to_leave: 0.0,
            cut_direction: CutDirection::Ccw,
            start_pos: None,
            tolerance: 0.1,
            expansion_batch_size: 20,
            cancel_check: None,
            engagement_area_threshold: 0.0,
            engagement_angle_threshold: std::f64::consts::PI,
            trace_path: None,
        }
    }
}
