//! Material-effect folding: classifying what operations removed.
//!
//! Assemblers emit [`MaterialEffect`]s alongside their `Ops` — a
//! description of the material they remove, in one unified language
//! for CNC and laser. [`fold`](self::fold)::[`fold_effects`]
//! aggregates the effects of many operations against one stock into
//! an immutable [`MaterialState`] snapshot: through-cut voids, the
//! burn surface map, provenance, and escalation signals for geometry
//! the current profiles cannot represent exactly.
//!
//! Effects are *world-space facts about the stock*: an effect's
//! geometry may extend past the workpiece outline that produced it
//! (frames, overscan, leads, raster bboxes), and folding clips only
//! against the stock — only material that exists can be removed.
//!
//! Z semantics: effect Z values and the stock's Z extent use the
//! toolpath convention — the stock's top surface is `z = 0` and the
//! stock extends downward to `z = -thickness`. `z_from`/`z_to` are
//! absolute Z in that space; `None` means "open to the surface"
//! (`z_from`) or "through the bottom" (`z_to`).

pub mod burn;
pub mod fold;
pub mod grid;
pub mod spec;
pub mod state;

use crate::compressed_array::CompressedArray;
use crate::geo::types::Polygon;
use crate::mesh::solid::SolidMesh;
use crate::ops::material::spec::{GridSpec, MaterialResponse};

/// One operation's material removal, in the unified language.
///
/// All variants describe *removed volume*; how the fold represents
/// the result against a given stock is a per-stock profile choice
/// (see [`FoldProfile`]), not a property of the effect.
#[derive(Clone, Debug)]
pub enum MaterialEffect {
    /// Polygons extruded over a Z interval.
    Vector {
        /// Footprint in workpiece-local mm (placed into world space
        /// by the fold via the entry's placement).
        polygons: Vec<Polygon>,
        /// Top of the removed interval; `None` = open to the stock
        /// surface (`z = 0`).
        z_from: Option<f64>,
        /// Bottom of the removed interval; `None` = through the
        /// stock bottom (`z = -thickness`).
        z_to: Option<f64>,
    },
    /// An F32 fluence map (J/cm²) plus the material response
    /// interpreting it.
    Raster {
        /// Per-pixel laser fluence in J/cm² (float32). The fold
        /// max-reduces this into the surface map; the shader applies
        /// the material's absorption coefficient and char curve.
        fluence: CompressedArray,
        /// Grid placement of `fluence` in world mm.
        grid: GridSpec,
        /// Material response used to interpret the fluence values.
        response: MaterialResponse,
    },
    /// Closed solids (placed into world space by the fold).
    ///
    /// No assembler emits these yet; the variant exists so future
    /// 3D assemblers join the same fold without a wire-format
    /// change. Presence escalates the stock to the solid profile.
    Volume {
        /// Closed-manifold solids in workpiece-local mm.
        solids: Vec<SolidMesh>,
    },
}

impl MaterialEffect {
    /// Approximate heap size in bytes, for cache accounting.
    pub fn heap_size(&self) -> usize {
        match self {
            MaterialEffect::Vector { polygons, .. } => polygons
                .iter()
                .map(|p| {
                    p.len() * std::mem::size_of::<crate::geo::types::Point>()
                })
                .sum(),
            MaterialEffect::Raster { fluence, .. } => {
                fluence.data.len() + std::mem::size_of::<CompressedArray>()
            }
            MaterialEffect::Volume { solids } => solids
                .iter()
                .map(|s| {
                    s.positions.len()
                        * std::mem::size_of::<crate::geo::types::Point3D>()
                        + s.triangles.len() * 3 * std::mem::size_of::<u32>()
                })
                .sum(),
        }
    }
}

/// Which representation a [`MaterialState`] was folded with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldProfile {
    /// 2.5D fast path: voids as polygons, relief as a heightmap.
    /// Valid while cuts are vertical, top-open, and the stock is a
    /// prism.
    Prismatic,
    /// Rotary stock folded in unrolled (axial × circumference)
    /// space; burn surface effects only, no voids or depth field.
    Cylindrical,
    /// General path: closed-solid CSG. Arrives with the solid
    /// profile work; `Prismatic` folds never produce it.
    Solid,
}

/// Why a fold could not represent the stock exactly.
///
/// An escalation is a signal, not an error: the fold still completes
/// with the best prismatic approximation it can produce.
#[derive(Clone, Debug, PartialEq)]
pub enum Escalation {
    /// A removed interval starts below the stock surface — the
    /// prismatic profile's top-open invariant is violated (potential
    /// undercut or interior cavity).
    TopOpenViolation {
        /// Node key of the entry that violated the invariant.
        source_key: String,
    },
    /// A [`MaterialEffect::Volume`] was present; representing it
    /// exactly requires the solid profile.
    SolidProfileRequired {
        /// Node key of the entry carrying the volume effect.
        source_key: String,
    },
}

impl Escalation {
    /// Stable machine-readable kind name.
    pub fn kind(&self) -> &'static str {
        match self {
            Escalation::TopOpenViolation { .. } => "top_open_violation",
            Escalation::SolidProfileRequired { .. } => "solid_profile_required",
        }
    }
}
