//! The fold kernel: aggregate material effects against one stock.
//!
//! Pure Rust, deterministic, and order-independent — union and
//! max-reduce are commutative and associative, and provenance is
//! sorted — so cache tokens and snapshot equality are stable
//! regardless of entry order.

use rayon::prelude::*;

use super::grid::{
    polygons_bounds, resolve_grid, transform_polygons, RasterView,
};
use crate::geo::shape::polygon::{
    get_polygons_group_intersection, get_polygons_union,
};
use crate::geo::types::Polygon;
use crate::ops::material::spec::{MaterialFoldSpec, StockShape};
use crate::ops::material::state::MaterialState;
use crate::ops::material::{Escalation, FoldProfile, MaterialEffect};

/// Epsilon for Z comparisons against the stock surfaces (mm).
const Z_EPS: f64 = 1e-9;

/// Why a fold was rejected before producing a state.
#[derive(Clone, Debug, PartialEq)]
pub enum FoldError {
    /// A validation rule was violated (empty stock, non-positive
    /// thickness, non-positive grid budget).
    Invalid(String),
}

impl std::fmt::Display for FoldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FoldError::Invalid(msg) => write!(f, "invalid fold spec: {msg}"),
        }
    }
}

impl std::error::Error for FoldError {}

/// Fold the spec's entries against the stock into a snapshot.
///
/// The prismatic profile only, phase 0: through-cut classification,
/// void union clipped to the stock, the burn surface map, provenance,
/// and escalation signals. `Volume` effects and top-open violations
/// escalate instead of failing; the fold still returns the best
/// prismatic approximation.
pub fn fold_effects(
    spec: &MaterialFoldSpec,
) -> Result<MaterialState, FoldError> {
    let (stock_polygons, thickness) = match &spec.stock {
        StockShape::Prismatic {
            polygons,
            thickness,
        } => (polygons, *thickness),
    };
    if stock_polygons.is_empty() {
        return Err(FoldError::Invalid("stock has no polygons".into()));
    }
    if thickness <= 0.0 || !thickness.is_finite() {
        return Err(FoldError::Invalid(
            "stock thickness must be a positive finite number".into(),
        ));
    }
    if spec.grid.px_per_mm <= 0.0 || !spec.grid.px_per_mm.is_finite() {
        return Err(FoldError::Invalid(
            "grid px_per_mm must be a positive finite number".into(),
        ));
    }

    // Toolpath Z convention: top surface at z = 0, bottom at
    // z = -thickness.
    let stock_bottom = -thickness;

    let mut escalation: Option<Escalation> = None;
    let mut provenance: Vec<String> = Vec::new();
    let mut void_candidates: Vec<Polygon> = Vec::new();
    let mut rasters: Vec<RasterView> = Vec::new();
    let mut raster_placements: Vec<crate::geo::matrix::Matrix> = Vec::new();

    for entry in &spec.entries {
        if entry.effects.is_empty() {
            continue;
        }
        provenance.push(entry.source_key.clone());
        for effect in &entry.effects {
            match effect {
                MaterialEffect::Vector {
                    polygons,
                    z_from,
                    z_to,
                } => {
                    if polygons.is_empty() {
                        continue;
                    }
                    let open_to_top = z_from.map_or(true, |z| z >= -Z_EPS);
                    let through =
                        z_to.map_or(true, |z| z <= stock_bottom + Z_EPS);
                    if !open_to_top && escalation.is_none() {
                        escalation = Some(Escalation::TopOpenViolation {
                            source_key: entry.source_key.clone(),
                        });
                    }
                    if open_to_top && through {
                        void_candidates.extend(transform_polygons(
                            polygons,
                            &entry.placement,
                        ));
                    }
                }
                MaterialEffect::Raster { power, grid, .. } => {
                    if grid.size_px.0 == 0 || grid.size_px.1 == 0 {
                        continue;
                    }
                    if let Some(view) = RasterView::new(power, grid) {
                        rasters.push(view);
                        raster_placements.push(entry.placement);
                    }
                }
                MaterialEffect::Volume { .. } => {
                    if escalation.is_none() {
                        escalation = Some(Escalation::SolidProfileRequired {
                            source_key: entry.source_key.clone(),
                        });
                    }
                }
            }
        }
    }

    let void_polygons = if void_candidates.is_empty() {
        Vec::new()
    } else {
        let union = get_polygons_union(&void_candidates);
        get_polygons_group_intersection(&union, stock_polygons)
    };

    let (surface_map, grid) = if rasters.is_empty() {
        (None, None)
    } else {
        let bounds = polygons_bounds(stock_polygons).ok_or_else(|| {
            FoldError::Invalid("stock has no polygons".into())
        })?;
        let grid = resolve_grid(bounds, &spec.grid);
        let inverse: Vec<crate::geo::matrix::Matrix> =
            raster_placements.iter().map(|m| m.invert()).collect();
        let (ppm_x, ppm_y) = grid.px_per_mm;
        let (origin_x, origin_y) = grid.origin_mm;
        let width = grid.size_px.0;
        let height_px = grid.size_px.1;
        let rasters_ref = &rasters;
        let inverse_ref = &inverse;
        let data: Vec<u8> = (0..height_px)
            .into_par_iter()
            .flat_map_iter(|row| {
                let world_y = origin_y + (row as f64 + 0.5) / ppm_y;
                (0..width).map(move |col| {
                    let world_x = origin_x + (col as f64 + 0.5) / ppm_x;
                    let mut max: u8 = 0;
                    for (view, inv) in
                        rasters_ref.iter().zip(inverse_ref.iter())
                    {
                        let (lx, ly) = inv.transform_point(world_x, world_y);
                        let v = view.sample((lx, ly));
                        if v > max {
                            max = v;
                        }
                    }
                    max
                })
            })
            .collect();
        let compressed = crate::compressed_array::CompressedArray::from_vec_u8(
            data,
            vec![height_px, width],
        );
        (Some(compressed), Some(grid))
    };

    provenance.sort();
    provenance.dedup();

    Ok(MaterialState {
        profile: FoldProfile::Prismatic,
        void_polygons,
        depth_field: None,
        surface_map,
        grid,
        provenance,
        escalation,
    })
}
