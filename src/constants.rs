/// Tolerance for detecting when two 2D points are close enough to skip
/// a move-to command (gap closing). Used in clipping and path continuity.
pub const EPSILON_GAP_CLOSE: f64 = 1e-6;

/// Tolerance for treating a segment as degenerate (zero length).
/// Used for length-squared comparisons and collinearity checks.
pub const EPSILON_COLLINEAR: f64 = 1e-9;

/// Tolerance for intersection endpoint comparisons.
/// Used when determining if an intersection point coincides with
/// a segment endpoint (T-junction detection).
pub const EPSILON_INTERSECT: f64 = 1e-12;
