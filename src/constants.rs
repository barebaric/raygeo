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

/// Clipper integer coordinate scale factor (10^7).
/// Converts float geometry to Clipper's integer grid for boolean-accuracy ops.
pub const CLIPPER_SCALE: f64 = 10_000_000.0;

/// Tolerance for collision/overlap detection in nesting.
pub const EPSILON_NEST: f64 = 1e-4;

/// Tolerance for geometric comparisons (mid-precision).
pub const EPSILON_MEDIUM: f64 = 1e-5;

/// Tolerance used when comparing against zero in line merging.
pub const EPSILON_MERGE: f64 = 1e-5;

/// Tolerance for boundary/island extraction from geometry (mm).
///
/// This controls how finely bezier curves are linearised when
/// extracting outer/inner contour topology for stock region
/// computation.  A coarse tolerance is intentional — the result
/// only needs to distinguish solid contours from holes, which
/// does not require millimeter precision.
///
/// Using a tight tolerance (e.g. `EPSILON_MERGE`) on large
/// geometry with many bezier curves (e.g. text glyphs scaled
/// to a 600 mm workpiece) would generate millions of line
/// segments per curve and cause OOM.
pub const EPSILON_BOUNDARY: f64 = 0.1;
