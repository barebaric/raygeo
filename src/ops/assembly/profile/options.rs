use std::path::PathBuf;

use crate::ops::cut::CutDirection;
use crate::types::Point3D;

/// Options for inner-boundary adaptive profiling.
///
/// Geometry is supplied via a [`Part`](crate::part::Part) — the
/// assembler extracts boundary and islands from it internally.
#[derive(Clone, Debug)]
pub struct ProfileInnerOptions {
    pub tool_radius: f64,
    pub step_over: f64,
    pub step_length: f64,
    pub target_z: f64,
    pub safe_z: f64,
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
            tool_radius: 3.0,
            step_over: 1.5,
            step_length: 0.6,
            target_z: -5.0,
            safe_z: 2.0,
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
/// Geometry is supplied via a [`Part`](crate::part::Part) — the
/// assembler extracts the boundary from it internally.
#[derive(Clone, Debug)]
pub struct ProfileOuterOptions {
    pub tool_radius: f64,
    pub step_over: f64,
    pub step_length: f64,
    pub target_z: f64,
    pub safe_z: f64,
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
            tool_radius: 3.0,
            step_over: 1.5,
            step_length: 0.6,
            target_z: -5.0,
            safe_z: 2.0,
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
