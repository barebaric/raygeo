//! Bed-mesh correction: warp toolpath Z onto a probed height map.
//!
//! Lasers and CNC machines assume the work surface is flat. On large
//! beds it often is not; a height map probed on a grid lets the focal
//! point (or cut depth) follow the surface. [`mesh_correction`] adds
//! the bilinearly interpolated grid height to every moving command's
//! Z, in place.

use crate::error::RaygeoError;
use crate::geo::types::Point3D;
use crate::ops::callbacks::Callbacks;
use crate::ops::container::Ops;
use crate::ops::transform::{Phase, Transformer};
use crate::ops::types::{MoveCmd, OpCategory, OpNode};

use std::sync::Arc;

/// A probed bed height map and the parameters needed to sample it.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshCorrectionSpec {
    /// X coordinate of the first grid column.
    pub x0: f64,
    /// Y coordinate of the first grid row.
    pub y0: f64,
    /// Spacing between grid columns along X. Must be positive when
    /// ``nx > 1``.
    pub dx: f64,
    /// Spacing between grid rows along Y. Must be positive when
    /// ``ny > 1``.
    pub dy: f64,
    /// Surface heights in row-major order: ``ny`` rows of ``nx``
    /// values, ``heights[j * nx + i]`` being the height at
    /// ``(x0 + i * dx, y0 + j * dy)``.
    pub heights: Vec<f64>,
    /// Number of grid columns (X direction), at least 1.
    pub nx: usize,
    /// Number of grid rows (Y direction), at least 1.
    pub ny: usize,
    /// Constant Z added on top of every sampled height, e.g. a focus
    /// offset above the surface or the distance between the original
    /// Z reference plane and the height map datum.
    pub z_offset: f64,
    /// Split straight cutting moves (``LineTo`` and scanlines) where
    /// they cross a grid line, so Z follows the height map between
    /// grid points instead of only at each move's ends. Travel moves,
    /// arcs and beziers are not split, nor are moves that carry extra
    /// axes.
    pub subdivide: bool,
}

impl MeshCorrectionSpec {
    /// Validate *spec*'s grid and return it unchanged.
    ///
    /// The Python binding validates at construction so the
    /// [`Transformer::apply`] path is infallible; direct Rust
    /// construction skips validation and [`mesh_correction`] rejects
    /// the result instead.
    pub fn validated(spec: MeshCorrectionSpec) -> Result<Self, RaygeoError> {
        spec.validate()?;
        Ok(spec)
    }

    /// Bilinearly interpolated height at ``(x, y)``. Coordinates
    /// outside the grid are clamped to the nearest edge sample.
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let last_col = (self.nx - 1) as f64;
        let last_row = (self.ny - 1) as f64;
        let fx = if self.nx > 1 {
            ((x - self.x0) / self.dx).clamp(0.0, last_col)
        } else {
            0.0
        };
        let fy = if self.ny > 1 {
            ((y - self.y0) / self.dy).clamp(0.0, last_row)
        } else {
            0.0
        };
        let i = (fx as usize).min(self.nx - 1);
        let j = (fy as usize).min(self.ny - 1);
        let i1 = (i + 1).min(self.nx - 1);
        let j1 = (j + 1).min(self.ny - 1);
        let tx = fx - i as f64;
        let ty = fy - j as f64;
        let h = &self.heights;
        let lo = h[j * self.nx + i] * (1.0 - tx) + h[j * self.nx + i1] * tx;
        let hi = h[j1 * self.nx + i] * (1.0 - tx) + h[j1 * self.nx + i1] * tx;
        lo * (1.0 - ty) + hi * ty
    }

    fn validate(&self) -> Result<(), RaygeoError> {
        if self.nx == 0 || self.ny == 0 {
            return Err(RaygeoError::InvalidCommand(
                "mesh grid must have at least one row and one column"
                    .to_string(),
            ));
        }
        if self.heights.len() != self.nx * self.ny {
            return Err(RaygeoError::InvalidCommand(format!(
                "heights length {} does not match a {} x {} grid",
                self.heights.len(),
                self.ny,
                self.nx
            )));
        }
        if self.nx > 1 && (!self.dx.is_finite() || self.dx <= 0.0) {
            return Err(RaygeoError::InvalidCommand(
                "dx must be positive and finite".to_string(),
            ));
        }
        if self.ny > 1 && (!self.dy.is_finite() || self.dy <= 0.0) {
            return Err(RaygeoError::InvalidCommand(
                "dy must be positive and finite".to_string(),
            ));
        }
        if !self.x0.is_finite()
            || !self.y0.is_finite()
            || !self.z_offset.is_finite()
        {
            return Err(RaygeoError::InvalidCommand(
                "x0, y0 and z_offset must be finite".to_string(),
            ));
        }
        if let Some(idx) = self.heights.iter().position(|h| !h.is_finite()) {
            return Err(RaygeoError::InvalidCommand(format!(
                "height at flat index {idx} is not finite"
            )));
        }
        Ok(())
    }
}

impl Transformer for MeshCorrectionSpec {
    fn phase(&self) -> Phase {
        Phase::PostProcessing
    }

    fn apply(&self, ctx: &mut crate::ops::transform::TransformCtx<'_>) {
        // Specs built through `try_new` (the Python binding) are
        // pre-validated; a directly-constructed invalid spec is a
        // no-op rather than a panic.
        let _ = mesh_correction(ctx.ops, self, ctx.callbacks);
    }

    fn name(&self) -> &str {
        "mesh_correction"
    }

    fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name().hash(&mut h);
        self.x0.to_bits().hash(&mut h);
        self.y0.to_bits().hash(&mut h);
        self.dx.to_bits().hash(&mut h);
        self.dy.to_bits().hash(&mut h);
        self.heights.len().hash(&mut h);
        for height in &self.heights {
            height.to_bits().hash(&mut h);
        }
        self.nx.hash(&mut h);
        self.ny.hash(&mut h);
        self.z_offset.to_bits().hash(&mut h);
        self.subdivide.hash(&mut h);
        h.finish()
    }
}

/// Warp every moving command's Z onto the height map in *spec*.
///
/// With [`MeshCorrectionSpec::subdivide`] set, straight cutting moves
/// are first split where they cross a grid line, so a long line follows
/// the map between grid points instead of running straight from one
/// corrected end to the other.
///
/// The height at each command's XY position is sampled bilinearly from
/// the grid and added to the command's Z together with
/// [`MeshCorrectionSpec::z_offset`]. This covers travel moves as well
/// as cutting moves, so travel at cut height clears the wavy surface
/// too. Arcs keep their shape: arc centres are relative I/J offsets
/// in the XY plane, so a corrected arc becomes a helical arc, and
/// bezier control points are corrected at their own XY positions.
/// Commands with non-finite XY are left unchanged.
///
/// Operates in place; the command buffer is only re-allocated (CoW)
/// when it is still shared with another `Ops` clone.
pub fn mesh_correction(
    ops: &mut Ops,
    spec: &MeshCorrectionSpec,
    callbacks: &dyn Callbacks,
) -> Result<(), RaygeoError> {
    spec.validate()?;
    if callbacks.is_cancelled() || ops.is_empty() {
        return Ok(());
    }

    if spec.subdivide {
        subdivide_at_grid_lines(ops, spec);
    }

    let total = ops.len();
    let stride = (total / 8).max(1);
    let z_offset = spec.z_offset;
    for (i, node) in ops.cmds_mut().iter_mut().enumerate() {
        if i % stride == 0 {
            callbacks
                .report_progress(i as f64 / total as f64, "mesh_correction");
        }
        let OpCategory::Moving { end, cmd } = &mut node.category else {
            continue;
        };
        if !end.x.is_finite() || !end.y.is_finite() {
            continue;
        }
        end.z += spec.sample(end.x, end.y) + z_offset;
        match cmd {
            MoveCmd::BezierTo(data) => {
                data.control1.z +=
                    spec.sample(data.control1.x, data.control1.y) + z_offset;
                data.control2.z +=
                    spec.sample(data.control2.x, data.control2.y) + z_offset;
            }
            MoveCmd::QuadraticBezierTo { control } => {
                control.z += spec.sample(control.x, control.y) + z_offset;
            }
            _ => {}
        }
    }
    ops.last_move_to.z +=
        spec.sample(ops.last_move_to.x, ops.last_move_to.y) + z_offset;
    callbacks.report_progress(1.0, "mesh_correction");
    ops.invalidate_time_cache();
    Ok(())
}

/// Parameters ``t`` in ``(0, 1)`` where the segment from *start* to
/// *end* crosses a grid column or row line, sorted and without
/// duplicates.
fn grid_crossings(
    spec: &MeshCorrectionSpec,
    start: Point3D,
    end: Point3D,
) -> Vec<f64> {
    const EPS: f64 = 1e-9;
    let mut ts = Vec::new();
    let mut axis = |a0: f64, a1: f64, origin: f64, step: f64, n: usize| {
        if n < 2 || (a1 - a0).abs() < EPS {
            return;
        }
        let (lo, hi) = if a0 < a1 { (a0, a1) } else { (a1, a0) };
        let first = (((lo - origin) / step).ceil().max(0.0)) as usize;
        let last = (((hi - origin) / step).floor()).min((n - 1) as f64);
        if last < 0.0 {
            return;
        }
        for k in first..=(last as usize) {
            let line = origin + k as f64 * step;
            let t = (line - a0) / (a1 - a0);
            if t > EPS && t < 1.0 - EPS {
                ts.push(t);
            }
        }
    };
    axis(start.x, end.x, spec.x0, spec.dx, spec.nx);
    axis(start.y, end.y, spec.y0, spec.dy, spec.ny);
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ts.dedup_by(|a, b| (*a - *b).abs() < EPS);
    ts
}

fn with_end(node: &OpNode, end: Point3D, cmd: MoveCmd) -> OpNode {
    OpNode {
        category: OpCategory::Moving { end, cmd },
        state: node.state.clone(),
        extra_axes: None,
    }
}

/// Replace each straight cutting move that crosses a grid line with
/// pieces that end on the grid lines (see
/// [`MeshCorrectionSpec::subdivide`]). Runs before Z is corrected, on
/// the original coordinates.
fn subdivide_at_grid_lines(ops: &mut Ops, spec: &MeshCorrectionSpec) {
    let source = std::mem::take(ops.cmds_mut());
    let mut out: Vec<OpNode> = Vec::with_capacity(source.len());
    let mut prev: Option<Point3D> = None;
    for node in source {
        let OpCategory::Moving { end, cmd } = &node.category else {
            out.push(node);
            continue;
        };
        let end = *end;
        let start = prev;
        prev = if end.x.is_finite() && end.y.is_finite() {
            Some(end)
        } else {
            None
        };
        let Some(start) = start else {
            out.push(node);
            continue;
        };
        if node.extra_axes.is_some() {
            out.push(node);
            continue;
        }
        match cmd {
            MoveCmd::LineTo => {
                for t in grid_crossings(spec, start, end) {
                    out.push(with_end(
                        &node,
                        start.lerp(end, t),
                        MoveCmd::LineTo,
                    ));
                }
                out.push(node);
            }
            MoveCmd::ScanLine { power_values } => {
                let n = power_values.len();
                let mut cuts: Vec<usize> = grid_crossings(spec, start, end)
                    .into_iter()
                    .map(|t| (t * n as f64).round() as usize)
                    .filter(|&k| k > 0 && k < n)
                    .collect();
                cuts.dedup();
                if cuts.is_empty() {
                    out.push(node);
                    continue;
                }
                let mut from = 0;
                for &k in &cuts {
                    let piece = Arc::new(power_values[from..k].to_vec());
                    let point = start.lerp(end, k as f64 / n as f64);
                    out.push(with_end(
                        &node,
                        point,
                        MoveCmd::ScanLine {
                            power_values: piece,
                        },
                    ));
                    from = k;
                }
                let rest = Arc::new(power_values[from..].to_vec());
                out.push(with_end(
                    &node,
                    end,
                    MoveCmd::ScanLine { power_values: rest },
                ));
            }
            _ => out.push(node),
        }
    }
    *ops.cmds_mut() = out;
}
