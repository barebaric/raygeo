//! Fold output: the immutable per-stock snapshot.

use crate::compressed_array::CompressedArray;
use crate::geo::types::Polygon;

use crate::ops::material::spec::GridSpec;
use crate::ops::material::{Escalation, FoldProfile};

/// The folded state of one stock: everything an operation removed,
/// aggregated into an immutable snapshot.
///
/// All geometry is world mm, clipped to the stock. The profile tag
/// tells consumers which representation the fields carry; consumers
/// should go through the state's projections rather than assuming a
/// profile.
#[derive(Clone, Debug)]
pub struct MaterialState {
    /// Which profile produced this state.
    pub profile: FoldProfile,
    /// Regions removed through the full stock thickness.
    pub void_polygons: Vec<Polygon>,
    /// Removal-depth heightmap in mm on the stock grid (f32,
    /// negative-down). `None` until depth folding lands.
    pub depth_field: Option<CompressedArray>,
    /// Per-pixel maximum laser power on the stock grid (R8), the
    /// burn-in input. `None` when no raster effects contributed.
    pub surface_map: Option<CompressedArray>,
    /// Grid shared by `depth_field` and `surface_map`.
    pub grid: Option<GridSpec>,
    /// Sorted unique source keys whose effects were applied.
    pub provenance: Vec<String>,
    /// First invariant violation encountered, if any.
    pub escalation: Option<Escalation>,
}
