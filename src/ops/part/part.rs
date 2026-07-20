use std::collections::HashMap;

use crate::constants::{EPSILON_BOUNDARY, EPSILON_MERGE};
use crate::geo::algo::fitting::linearize_data;
use crate::geo::algo::topology::{
    split_inner_and_outer_contours, split_into_contours,
};
use crate::geo::shape::polygon::clean_polygon;
use crate::geo::Geometry;
use crate::types::{Point, Polygon};

use super::cleared_area::ClearedArea;
use super::image_source::ImageSource;
use super::stock_region::StockRegion;

/// State for one face of a multi-face part.
///
/// Each face carries its own geometry, stock region, and cleared
/// area. Assemblers operate on one face at a time via
/// [`AssembleCtx::face`](crate::ops::assembly::AssembleCtx).
#[derive(Clone, Debug)]
pub struct FaceState {
    /// Vector geometry for this face.
    pub geometry: Option<Geometry>,
    /// Boundary and islands extracted from `geometry`.
    pub stock_region: StockRegion,
    /// Accumulated cleared-area state for this face.
    pub cleared: ClearedArea,
}

impl FaceState {
    pub fn new(geometry: Option<Geometry>) -> Self {
        let stock_region = match &geometry {
            Some(_) => {
                let (boundary, islands) =
                    Self::extract_boundary_from_geometry(geometry.as_ref());
                StockRegion::new(boundary.unwrap_or_default(), islands)
            }
            None => StockRegion::empty(),
        };
        FaceState {
            geometry,
            stock_region,
            cleared: ClearedArea::new(),
        }
    }

    /// Extract boundary + islands from this face's geometry.
    pub fn extract_boundary(&self) -> (Option<Polygon>, Vec<Polygon>) {
        Self::extract_boundary_from_geometry(self.geometry.as_ref())
    }

    /// Standalone boundary extraction from a geometry reference.
    pub(crate) fn extract_boundary_from_geometry(
        geo: Option<&Geometry>,
    ) -> (Option<Polygon>, Vec<Polygon>) {
        let geo = match geo {
            Some(g) => g,
            None => return (None, vec![]),
        };

        let mut linearized = geo.copy();
        if !linearized.data.is_empty() {
            linearized.data =
                linearize_data(&linearized.data, EPSILON_BOUNDARY);
        }
        let contours = split_into_contours(&linearized);
        if contours.is_empty() {
            return (None, vec![]);
        }

        let refs: Vec<&Geometry> = contours.iter().collect();
        let (inner_indices, outer_indices) =
            split_inner_and_outer_contours(&refs);

        let contour_to_poly = |i: usize| -> Option<Polygon> {
            let segs = contours[i].segments();
            for seg in &segs {
                if seg.len() < 3 {
                    continue;
                }
                let poly: Polygon =
                    seg.iter().map(|p| Point::new(p.x, p.y)).collect();
                if let Some(cleaned) =
                    clean_polygon(&poly, 0.01 * EPSILON_MERGE)
                {
                    return Some(cleaned);
                }
                if poly.len() >= 3 {
                    return Some(poly);
                }
            }
            None
        };

        let outers: Vec<Polygon> = outer_indices
            .iter()
            .filter_map(|&i| contour_to_poly(i))
            .collect();

        let islands: Vec<Polygon> = inner_indices
            .iter()
            .filter_map(|&i| contour_to_poly(i))
            .collect();

        let boundary = outers.into_iter().max_by(|a, b| {
            crate::utils::sort_f64(
                crate::geo::shape::polygon::get_polygon_area(a),
                crate::geo::shape::polygon::get_polygon_area(b),
            )
        });

        (boundary, islands)
    }
}

/// Unified workpiece description shared by all assemblers.
///
/// Carries physical metadata (size, pixels_per_mm, image_source)
/// at the part level, plus a per-face map of geometry, stock region,
/// and cleared area.  Most assemblers operate on a single face —
/// the `AssembleCtx` provides `face: &mut FaceState` for that.
///
/// Not `Clone`: an [`ImageSource`] is an opaque trait object. Callers
/// that previously cloned a `Part` should construct a fresh one or
/// borrow `&mut Part` instead.
pub struct Part {
    /// Per-face state. The empty string `""` is the default face.
    /// A part constructed the old (single-face) way has exactly one
    /// entry for `""`.
    pub faces: HashMap<String, FaceState>,

    /// Physical size of the part in millimetres `(width, height)`.
    pub size_mm: (f64, f64),

    /// Pixel density `(x, y)` in pixels per millimetre.
    ///
    /// Required for raster operations; `None` for purely vector work.
    pub pixels_per_mm: Option<(f64, f64)>,

    /// Optional lazy source of pixel data for raster/shrinkwrap
    /// assemblers.
    ///
    /// Set by the stage before calling an assembler. The assembler
    /// pulls rows via [`ImageSource::read_slab`] (or
    /// [`ImageSource::read_all`] for full-buffer passes) instead of
    /// reading a separate image argument. `None` for vector-only
    /// work.
    pub image_source: Option<Box<dyn ImageSource>>,
}

impl std::fmt::Debug for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let img = match &self.image_source {
            None => "None",
            Some(_) => "<ImageSource>",
        };
        f.debug_struct("Part")
            .field("faces", &self.faces)
            .field("size_mm", &self.size_mm)
            .field("pixels_per_mm", &self.pixels_per_mm)
            .field("image_source", &img)
            .finish()
    }
}

impl Part {
    /// Create a new `Part` from geometry and size.
    ///
    /// The single default face `""` is populated from `geometry`.
    /// The `StockRegion` is extracted from `geometry` (empty if `None`).
    /// The `ClearedArea` starts empty.
    pub fn new(geometry: Option<Geometry>, size_mm: (f64, f64)) -> Self {
        let mut faces = HashMap::new();
        faces.insert(String::new(), FaceState::new(geometry));
        Part {
            faces,
            size_mm,
            pixels_per_mm: None,
            image_source: None,
        }
    }

    /// Build a `Part` from a boundary polygon and optional islands.
    ///
    /// Constructs a `Geometry` containing the boundary as the first
    /// closed contour and each island as an additional contour, then
    /// wraps it in a `Part` with the given `size_mm`.
    /// The default face's `StockRegion` is set directly from
    /// `boundary`/`islands`.
    pub fn from_polygons(
        boundary: &Polygon,
        islands: &[Polygon],
        size_mm: (f64, f64),
    ) -> Self {
        let mut geo = Geometry::new();
        if let Some(first) = boundary.first() {
            geo.move_to(first.x, first.y, 0.0);
            for p in boundary.iter().skip(1) {
                geo.line_to(p.x, p.y, 0.0);
            }
            geo.close_path();
        }
        for island in islands {
            if let Some(first) = island.first() {
                geo.move_to(first.x, first.y, 0.0);
                for p in island.iter().skip(1) {
                    geo.line_to(p.x, p.y, 0.0);
                }
                geo.close_path();
            }
        }
        let stock_region = StockRegion::new(boundary.clone(), islands.to_vec());
        let mut faces = HashMap::new();
        faces.insert(
            String::new(),
            FaceState {
                geometry: Some(geo),
                stock_region,
                cleared: ClearedArea::new(),
            },
        );
        Part {
            faces,
            size_mm,
            pixels_per_mm: None,
            image_source: None,
        }
    }

    /// Build a Part from polygons, pre-seeding the cleared area with
    /// `initial` fragments (e.g. a seed circle for adaptive clearing).
    pub fn from_polygons_initial(
        boundary: &Polygon,
        islands: &[Polygon],
        initial: &[Polygon],
        size_mm: (f64, f64),
    ) -> Self {
        let mut geo = Geometry::new();
        if let Some(first) = boundary.first() {
            geo.move_to(first.x, first.y, 0.0);
            for p in boundary.iter().skip(1) {
                geo.line_to(p.x, p.y, 0.0);
            }
            geo.close_path();
        }
        for island in islands {
            if let Some(first) = island.first() {
                geo.move_to(first.x, first.y, 0.0);
                for p in island.iter().skip(1) {
                    geo.line_to(p.x, p.y, 0.0);
                }
                geo.close_path();
            }
        }
        let stock_region = StockRegion::new(boundary.clone(), islands.to_vec());
        let cleared = ClearedArea::with_fragments(initial);
        let mut faces = HashMap::new();
        faces.insert(
            String::new(),
            FaceState {
                geometry: Some(geo),
                stock_region,
                cleared,
            },
        );
        Part {
            faces,
            size_mm,
            pixels_per_mm: None,
            image_source: None,
        }
    }

    /// Add a new face. Panics if `id` already exists.
    pub fn add_face(
        &mut self,
        id: &str,
        geometry: Option<Geometry>,
    ) -> &mut FaceState {
        let old = self.faces.insert(id.to_string(), FaceState::new(geometry));
        assert!(old.is_none(), "face {id} already exists");
        self.faces.get_mut(id).unwrap()
    }

    /// Borrow the state for a face. Returns the default face `""` for
    /// unknown ids so callers that don't know about faces get the
    /// single-face behaviour.
    ///
    /// When `id` is not found, the default face is inserted (lazy
    /// init) so that mutations work.
    pub fn face_mut(&mut self, id: &str) -> &mut FaceState {
        let entry = self.faces.entry(id.to_string());
        entry.or_insert_with(|| FaceState::new(None))
    }

    /// Immutable borrow of a face's state. Returns `None` for unknown
    /// ids.
    pub fn face(&self, id: &str) -> Option<&FaceState> {
        self.faces.get(id)
    }

    /// Convenience: access the default face's geometry (for callers
    /// that don't care about faces).
    pub fn geometry(&self) -> Option<&Geometry> {
        self.faces.get("").and_then(|f| f.geometry.as_ref())
    }

    /// Convenience: access the default face's stock_region.
    pub fn stock_region(&self) -> &StockRegion {
        // The default face always exists.
        &self.faces.get("").unwrap().stock_region
    }

    /// Convenience: mutable access to the default face's stock_region.
    pub fn stock_region_mut(&mut self) -> &mut StockRegion {
        &mut self.faces.get_mut("").unwrap().stock_region
    }

    /// Convenience: access the default face's cleared area.
    pub fn cleared(&self) -> &ClearedArea {
        &self.faces.get("").unwrap().cleared
    }

    /// Convenience: mutable access to the default face's cleared area.
    pub fn cleared_mut(&mut self) -> &mut ClearedArea {
        &mut self.faces.get_mut("").unwrap().cleared
    }

    /// Extract the outer boundary and island polygons from the
    /// default face's geometry.
    pub fn extract_boundary(&self) -> (Option<Polygon>, Vec<Polygon>) {
        self.faces
            .get("")
            .map(|f| f.extract_boundary())
            .unwrap_or((None, vec![]))
    }
}
