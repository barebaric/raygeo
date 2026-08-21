//! The fold kernel: aggregate material effects against one stock.
//!
//! Pure Rust, deterministic, and order-independent — union and
//! max-reduce are commutative and associative, and provenance is
//! sorted — so cache tokens and snapshot equality are stable
//! regardless of entry order.

use rayon::prelude::*;

use super::grid::{resolve_grid, RasterView};
use crate::compressed_array::CompressedArray;
use crate::geo::shape::polygon::{
    get_polygon_group_bounds, get_polygons_group_intersection,
    get_polygons_union, transform_polygons,
};
use crate::geo::types::{Polygon, Rect};
use crate::ops::material::spec::{FoldEntry, MaterialFoldSpec, StockShape};
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

/// Intermediate classification of one fold's entries, gathered in a
/// single pass so the fold can compose its outputs.
struct FoldParts<'a> {
    escalation: Option<Escalation>,
    provenance: Vec<String>,
    void_candidates: Vec<Polygon>,
    rasters: Vec<RasterView<'a>>,
    raster_placements: Vec<crate::geo::matrix::Matrix>,
}

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
    match &spec.stock {
        StockShape::Prismatic { .. } => fold_prismatic(spec),
        StockShape::Cylinder { .. } => fold_cylindrical(spec),
    }
}

/// Fold against a prismatic stock: through-cut classification, void
/// union clipped to the stock, the burn surface map, provenance, and
/// escalation signals. `Volume` effects and top-open violations
/// escalate instead of failing; the fold still returns the best
/// prismatic approximation.
fn fold_prismatic(spec: &MaterialFoldSpec) -> Result<MaterialState, FoldError> {
    let (stock_polygons, thickness) = validate_and_extract_stock(spec)?;
    let parts = classify_entries(&spec.entries, -thickness);

    let void_polygons = resolve_voids(&parts.void_candidates, stock_polygons);
    let bounds = get_polygon_group_bounds(stock_polygons);
    let (surface_map, grid) = build_surface_map(
        &parts.rasters,
        &parts.raster_placements,
        &bounds,
        &spec.grid,
    );

    Ok(MaterialState {
        profile: FoldProfile::Prismatic,
        void_polygons,
        depth_field: None,
        surface_map,
        grid,
        provenance: parts.provenance,
        escalation: parts.escalation,
    })
}

/// Fold against a cylindrical (rotary) stock: burn surface map over
/// the unrolled axial × circumference domain only. Laser ops
/// contribute to the burn; vector and volume effects are not modeled
/// for rotary stock yet and are ignored.
fn fold_cylindrical(
    spec: &MaterialFoldSpec,
) -> Result<MaterialState, FoldError> {
    let (diameter, length) = validate_and_extract_cylinder(spec)?;
    // Domain matches the rotary work area: axial x in [0, length],
    // arc-length y centered on the machine origin (the beam sits at
    // y = 0), spanning one full circumference.
    let half_circumference = std::f64::consts::PI * diameter / 2.0;
    let parts = classify_raster_entries(&spec.entries);

    let bounds =
        Rect::new(0.0, -half_circumference, length, half_circumference);
    let (surface_map, grid) = build_surface_map(
        &parts.rasters,
        &parts.raster_placements,
        &bounds,
        &spec.grid,
    );

    Ok(MaterialState {
        profile: FoldProfile::Cylindrical,
        void_polygons: Vec::new(),
        depth_field: None,
        surface_map,
        grid,
        provenance: parts.provenance,
        escalation: parts.escalation,
    })
}

/// Validate the spec and return the stock's outline and thickness.
fn validate_and_extract_stock(
    spec: &MaterialFoldSpec,
) -> Result<(&[Polygon], f64), FoldError> {
    let StockShape::Prismatic {
        polygons,
        thickness,
    } = &spec.stock
    else {
        return Err(FoldError::Invalid("expected a prismatic stock".into()));
    };
    if polygons.is_empty() {
        return Err(FoldError::Invalid("stock has no polygons".into()));
    }
    if *thickness <= 0.0 || !thickness.is_finite() {
        return Err(FoldError::Invalid(
            "stock thickness must be a positive finite number".into(),
        ));
    }
    if spec.grid.px_per_mm <= 0.0 || !spec.grid.px_per_mm.is_finite() {
        return Err(FoldError::Invalid(
            "grid px_per_mm must be a positive finite number".into(),
        ));
    }
    Ok((polygons, *thickness))
}

/// Validate a cylindrical stock spec and return `(diameter, length)`.
fn validate_and_extract_cylinder(
    spec: &MaterialFoldSpec,
) -> Result<(f64, f64), FoldError> {
    let StockShape::Cylinder { diameter, length } = &spec.stock else {
        return Err(FoldError::Invalid("expected a cylinder stock".into()));
    };
    if *diameter <= 0.0 || !diameter.is_finite() {
        return Err(FoldError::Invalid(
            "stock diameter must be a positive finite number".into(),
        ));
    }
    if *length <= 0.0 || !length.is_finite() {
        return Err(FoldError::Invalid(
            "stock length must be a positive finite number".into(),
        ));
    }
    if spec.grid.px_per_mm <= 0.0 || !spec.grid.px_per_mm.is_finite() {
        return Err(FoldError::Invalid(
            "grid px_per_mm must be a positive finite number".into(),
        ));
    }
    Ok((*diameter, *length))
}

/// Classify every entry's effects in a single order-independent pass.
///
/// Through-cut vectors become void candidates, rasters are
/// decompressed for later sampling, `Volume` effects and top-open
/// violations set escalation signals. `stock_bottom` is the toolpath
/// Z of the stock's lower surface (negative).
fn classify_entries<'a>(
    entries: &'a [FoldEntry],
    stock_bottom: f64,
) -> FoldParts<'a> {
    let mut parts = FoldParts {
        escalation: None,
        provenance: Vec::new(),
        void_candidates: Vec::new(),
        rasters: Vec::new(),
        raster_placements: Vec::new(),
    };
    for entry in entries {
        if entry.effects.is_empty() {
            continue;
        }
        parts.provenance.push(entry.source_key.clone());
        for effect in &entry.effects {
            classify_effect(effect, entry, stock_bottom, &mut parts);
        }
    }
    parts.provenance.sort();
    parts.provenance.dedup();
    parts
}

/// Classify every entry's raster effects for a cylindrical fold.
///
/// Only rasters matter on the unrolled burn domain; entries without
/// raster effects are ignored entirely (no provenance).
fn classify_raster_entries(entries: &[FoldEntry]) -> FoldParts<'_> {
    let mut parts = FoldParts {
        escalation: None,
        provenance: Vec::new(),
        void_candidates: Vec::new(),
        rasters: Vec::new(),
        raster_placements: Vec::new(),
    };
    for entry in entries {
        let mut has_raster = false;
        for effect in &entry.effects {
            if let MaterialEffect::Raster { power, grid, .. } = effect {
                if grid.size_px.0 == 0 || grid.size_px.1 == 0 {
                    continue;
                }
                if let Some(view) = RasterView::new(power, grid) {
                    parts.rasters.push(view);
                    parts.raster_placements.push(entry.placement);
                    has_raster = true;
                }
            }
        }
        if has_raster {
            parts.provenance.push(entry.source_key.clone());
        }
    }
    parts.provenance.sort();
    parts.provenance.dedup();
    parts
}

/// Fold a single effect into the shared classification.
fn classify_effect<'a>(
    effect: &'a MaterialEffect,
    entry: &'a FoldEntry,
    stock_bottom: f64,
    parts: &mut FoldParts<'a>,
) {
    match effect {
        MaterialEffect::Vector {
            polygons,
            z_from,
            z_to,
        } => {
            if polygons.is_empty() {
                return;
            }
            let open_to_top = z_from.is_none_or(|z| z >= -Z_EPS);
            let through = z_to.is_none_or(|z| z <= stock_bottom + Z_EPS);
            if !open_to_top && parts.escalation.is_none() {
                parts.escalation = Some(Escalation::TopOpenViolation {
                    source_key: entry.source_key.clone(),
                });
            }
            if open_to_top && through {
                parts
                    .void_candidates
                    .extend(transform_polygons(polygons, &entry.placement));
            }
        }
        MaterialEffect::Raster { power, grid, .. } => {
            if grid.size_px.0 == 0 || grid.size_px.1 == 0 {
                return;
            }
            if let Some(view) = RasterView::new(power, grid) {
                parts.rasters.push(view);
                parts.raster_placements.push(entry.placement);
            }
        }
        MaterialEffect::Volume { .. } => {
            if parts.escalation.is_none() {
                parts.escalation = Some(Escalation::SolidProfileRequired {
                    source_key: entry.source_key.clone(),
                });
            }
        }
    }
}

/// Union through-cut candidates and clip the result to the stock.
fn resolve_voids(
    void_candidates: &[Polygon],
    stock_polygons: &[Polygon],
) -> Vec<Polygon> {
    if void_candidates.is_empty() {
        return Vec::new();
    }
    let union = get_polygons_union(void_candidates);
    get_polygons_group_intersection(&union, stock_polygons)
}

/// Sample every raster into one max-reduced stock-grid surface map.
///
/// Returns `(None, None)` when no rasters contributed. The map is
/// built in parallel: for each stock-grid pixel, each raster is
/// sampled at the world point mapped through its inverse placement.
fn build_surface_map<'a>(
    rasters: &[RasterView<'a>],
    raster_placements: &[crate::geo::matrix::Matrix],
    bounds: &crate::geo::types::Rect,
    budget: &super::spec::GridBudget,
) -> (Option<CompressedArray>, Option<super::spec::GridSpec>) {
    if rasters.is_empty() {
        return (None, None);
    }
    let grid = resolve_grid(bounds, budget);
    let inverse: Vec<crate::geo::matrix::Matrix> =
        raster_placements.iter().map(|m| m.invert()).collect();
    let (ppm_x, ppm_y) = grid.px_per_mm;
    let (origin_x, origin_y) = grid.origin_mm;
    let width = grid.size_px.0;
    let height_px = grid.size_px.1;
    let data: Vec<u8> = (0..height_px)
        .into_par_iter()
        .flat_map_iter(|row| {
            let world_y = origin_y + (row as f64 + 0.5) / ppm_y;
            let rasters = &rasters;
            let inverse = &inverse;
            (0..width).map(move |col| {
                let world_x = origin_x + (col as f64 + 0.5) / ppm_x;
                let mut max: u8 = 0;
                for (view, inv) in rasters.iter().zip(inverse.iter()) {
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
    let compressed = CompressedArray::from_vec_u8(data, vec![height_px, width]);
    (Some(compressed), Some(grid))
}
