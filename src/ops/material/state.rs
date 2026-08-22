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
    /// Per-pixel maximum laser fluence on the stock grid (F32,
    /// J/cm²), the burn-in input. `None` when no raster effects
    /// contributed.
    pub surface_map: Option<CompressedArray>,
    /// Grid shared by `depth_field` and `surface_map`.
    pub grid: Option<GridSpec>,
    /// Sorted unique source keys whose effects were applied.
    pub provenance: Vec<String>,
    /// First invariant violation encountered, if any.
    pub escalation: Option<Escalation>,
    /// Emission wavelength in nm of the laser that produced the
    /// surface-map fluence. The renderer looks up the material's
    /// absorption coefficient for this wavelength's band. 0 means
    /// "unconfigured"; the renderer falls back to full absorption.
    pub wavelength_nm: f64,
    /// Optical output power in watts at full power of the laser that
    /// produced the surface-map fluence. Carried for provenance and
    /// future depth modeling; the renderer does not use it directly.
    pub max_power_watts: f64,
}
