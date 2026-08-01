use std::collections::HashMap;

use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_centroid, is_point_inside_polygon,
};
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
    /// Boundary + islands for a face: the largest outer contour as the
    /// boundary and every inner contour as an island.
    fn boundary_and_islands(geo: &Geometry) -> (Option<Polygon>, Vec<Polygon>) {
        let (outers, islands) = geo.split_inner_and_outer_polygons();
        let boundary = outers.into_iter().max_by(|a, b| {
            crate::utils::sort_f64(get_polygon_area(a), get_polygon_area(b))
        });
        (boundary, islands)
    }

    pub fn new(geometry: Option<Geometry>) -> Self {
        let stock_region = match &geometry {
            Some(geo) => {
                let (boundary, islands) = Self::boundary_and_islands(geo);
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
        match &self.geometry {
            Some(geo) => Self::boundary_and_islands(geo),
            None => (None, vec![]),
        }
    }

    /// Seed the cleared area with a circle at `entry_pt` when empty.
    ///
    /// The seed radius is 25% of `r_max`, floored at `tool_radius` so
    /// the tool's own disk fits inside the seed and the stepper has
    /// meaningful engagement on the first step.
    pub fn seed_circle(
        &mut self,
        entry_pt: Point,
        r_max: f64,
        tool_radius: f64,
    ) {
        if !self.cleared.is_empty() || r_max <= 0.0 {
            return;
        }
        let seed_r = (r_max * 0.25).max(tool_radius);
        let seed = crate::geo::shape::polygon::get_circle_polygon(
            entry_pt, seed_r, 32,
        );
        self.cleared.set_fragments(vec![seed]);
    }

    /// Seed the cleared area from the largest inscribed circle of the
    /// stock region when empty.
    pub fn seed_from_largest_circle(&mut self, tool_radius: f64) {
        let (entry_pt, r_max) =
            crate::geo::algo::polylabel::find_largest_circle(
                &self.stock_region.boundary,
                &self.stock_region.islands,
                0.1,
            )
            .unwrap_or_default();
        self.seed_circle(entry_pt, r_max, tool_radius);
    }

    /// Build a face scoped to `boundary`, keeping only the stock
    /// islands and cleared fragments whose centroid lies inside
    /// `boundary`.
    pub fn scoped_to(&self, boundary: Polygon) -> FaceState {
        let islands = self
            .stock_region
            .islands
            .iter()
            .filter(|isl| {
                let c = get_polygon_centroid(isl);
                is_point_inside_polygon(c, &boundary)
            })
            .cloned()
            .collect();
        let fragments = self
            .cleared
            .fragments()
            .iter()
            .filter(|f| {
                let c = get_polygon_centroid(f);
                is_point_inside_polygon(c, &boundary)
            })
            .cloned()
            .collect::<Vec<Polygon>>();
        FaceState {
            geometry: None,
            stock_region: StockRegion::new(boundary, islands),
            cleared: ClearedArea::with_fragments(&fragments),
        }
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

impl Clone for Part {
    fn clone(&self) -> Self {
        Part {
            faces: self.faces.clone(),
            size_mm: self.size_mm,
            pixels_per_mm: self.pixels_per_mm,
            image_source: None,
        }
    }
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

    /// Create a `Part` from geometry, auto-detecting separate pockets
    /// (disconnected outer contours) and creating one face per pocket.
    ///
    /// The largest pocket becomes the default face `""`; the others get
    /// ids `"1"`, `"2"`, ... (sorted by area descending). Island (inner)
    /// contours are assigned to the outer that contains their centroid.
    /// Single-pocket geometry produces a single face `""` — backward
    /// compatible with [`Part::new`].
    ///
    /// Each face's [`StockRegion`] is set directly from its outer +
    /// islands and `ClearedArea` starts empty. When `geometry` is empty
    /// or has no contours, a single empty default face is produced
    /// (matching [`Part::new`]'s behaviour).
    pub fn from_geometry_multi_face(
        geometry: Geometry,
        size_mm: (f64, f64),
    ) -> Self {
        // 1. Linearize + split into contours, classified by nesting
        //    depth into outers and inners.
        let (outers, inners) = geometry.split_inner_and_outer_polygons();
        if outers.is_empty() {
            return Part::new(Some(geometry), size_mm);
        }

        // 2. Sort outers by area descending so the largest becomes "".
        let mut sorted_outers = outers;
        sorted_outers.sort_by(|a, b| {
            let aa = get_polygon_area(a);
            let ab = get_polygon_area(b);
            ab.partial_cmp(&aa).unwrap_or(std::cmp::Ordering::Equal)
        });

        // 3. Associate each island with the containing outer (centroid
        //    test). An island goes to the first (largest) outer that
        //    contains it, so nested islands stay with their pocket.
        let mut used_inner = vec![false; inners.len()];
        let mut faces = HashMap::new();

        for (face_idx, outer_poly) in sorted_outers.iter().enumerate() {
            let mut islands = Vec::new();
            for (j, inner_poly) in inners.iter().enumerate() {
                if used_inner[j] || inner_poly.len() < 3 {
                    continue;
                }
                let cx = get_polygon_centroid(inner_poly);
                if is_point_inside_polygon(cx, outer_poly) {
                    islands.push(inner_poly.clone());
                    used_inner[j] = true;
                }
            }

            let face_id = if face_idx == 0 {
                String::new()
            } else {
                face_idx.to_string()
            };

            // Build the geometry for this face (outer + islands) so
            // assemblers that read `FaceState::geometry` see the same
            // pocket.
            let mut face_geo = Geometry::new();
            if let Some(first) = outer_poly.first() {
                face_geo.move_to(first.x, first.y, 0.0);
                for p in outer_poly.iter().skip(1) {
                    face_geo.line_to(p.x, p.y, 0.0);
                }
                face_geo.close_path();
            }
            for island in &islands {
                if let Some(first) = island.first() {
                    face_geo.move_to(first.x, first.y, 0.0);
                    for p in island.iter().skip(1) {
                        face_geo.line_to(p.x, p.y, 0.0);
                    }
                    face_geo.close_path();
                }
            }

            let stock_region = StockRegion::new(outer_poly.clone(), islands);
            faces.insert(
                face_id,
                FaceState {
                    geometry: Some(face_geo),
                    stock_region,
                    cleared: ClearedArea::new(),
                },
            );
        }

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
