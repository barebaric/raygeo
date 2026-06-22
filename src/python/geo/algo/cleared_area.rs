pyo3_stub_gen::module_doc!("raygeo.geo.algo.cleared_area", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Incremental cleared-area tracker for adaptive clearing.

Maintains a union of tool-swept polygons and provides a spatial-indexed
windowed query for efficient engagement computation.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::algo::cleared_area::ClearedArea as RustClearedArea;
use crate::types::Rect;

#[gen_stub_pyclass(module = "raygeo.geo.algo.cleared_area")]
#[pyclass]
pub struct ClearedArea {
    pub(crate) inner: RustClearedArea,
}

#[gen_stub_pymethods]
#[pymethods]
impl ClearedArea {
    #[new]
    #[pyo3(signature = (initial = None))]
    pub fn new(initial: Option<Vec<Vec<(f64, f64)>>>) -> Self {
        match initial {
            Some(polys) => {
                let polygons: Vec<crate::types::Polygon> = polys
                    .into_iter()
                    .map(|v| {
                        v.into_iter()
                            .map(|(x, y)| crate::types::Point::new(x, y))
                            .collect()
                    })
                    .collect();
                ClearedArea {
                    inner: RustClearedArea::from_polygons(&polygons),
                }
            }
            None => ClearedArea {
                inner: RustClearedArea::new(),
            },
        }
    }

    /// :complexity: O(n) where n = number of path points
    pub fn expand(&mut self, tool_path: Vec<(f64, f64)>, tool_radius: f64) {
        let path: Vec<crate::types::Point> = tool_path
            .into_iter()
            .map(|(x, y)| crate::types::Point::new(x, y))
            .collect();
        self.inner.expand(&path, tool_radius);
    }

    /// :complexity: O(n) where n = total vertices across all polygons
    pub fn add_cleared_polygons(&mut self, polygons: Vec<Vec<(f64, f64)>>) {
        let polys: Vec<crate::types::Polygon> = polygons
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        self.inner.add_cleared_polygons(&polys);
    }

    /// :complexity: O(m + k) where m = number of fragments, k = output vertices
    pub fn query_window(
        &self,
        bbox: (f64, f64, f64, f64),
    ) -> Vec<Vec<(f64, f64)>> {
        let rect = Rect::new(bbox.0, bbox.1, bbox.2, bbox.3);
        let frags = self.inner.query_window(rect);
        frags
            .into_iter()
            .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// :complexity: O(n * m) where n = bounds vertices, m = fragments
    pub fn remaining(
        &self,
        bounds: Vec<Vec<(f64, f64)>>,
    ) -> Vec<Vec<(f64, f64)>> {
        let bounds_polys: Vec<crate::types::Polygon> = bounds
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let remaining = self.inner.remaining(&bounds_polys);
        remaining
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Add polygons, returning only the newly-added portion.
    /// Faster than add_cleared_polygons when inputs don't overlap
    /// existing fragments (skips the full union).
    /// :complexity: O(n log n) worst case when union required,
    ///              O(n) when inputs are disjoint from existing fragments
    pub fn incorporate(
        &mut self,
        polygons: Vec<Vec<(f64, f64)>>,
    ) -> Vec<Vec<(f64, f64)>> {
        let polys: Vec<crate::types::Polygon> = polygons
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let new = self.inner.incorporate(&polys);
        new.into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Return a unioned, simplified snapshot of the current outer boundary.
    /// :param simplify_tol: tolerance in mm for polyline simplification
    /// :complexity: O(n log n)
    pub fn frontier(&self, simplify_tol: f64) -> Vec<Vec<(f64, f64)>> {
        let f = self.inner.frontier(simplify_tol);
        f.into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Compute the "bites" — new material reachable by expanding the
    /// current frontier outward by step_over, clipping to valid_area,
    /// and subtracting already-cleared portions.
    /// :param step_over: lateral step-over in mm
    /// :param valid_area: list of polygons defining the valid tool-centre region
    /// :param simplify_tol: tolerance in mm for frontier simplification
    /// :complexity: O(n log n)
    pub fn bites(
        &self,
        step_over: f64,
        valid_area: Vec<Vec<(f64, f64)>>,
        simplify_tol: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        let valid: Vec<crate::types::Polygon> = valid_area
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let bites = self.inner.bites(step_over, &valid, simplify_tol);
        bites
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Like :py:meth:`bites` but filters to only the bites whose centroid
    /// lies within *max_angle* radians of the direction from the current
    /// cleared region's centre toward *target*.
    /// useful for steering the clearing direction along a MAT branch.
    /// :param step_over: lateral step-over in mm
    /// :param valid_area: list of polygons defining the valid tool-centre region
    /// :param simplify_tol: tolerance in mm for frontier simplification
    /// :param target: (x, y) target point to steer toward
    /// :param max_angle: maximum deviation from the target direction (radians)
    /// :complexity: O(n log n)
    pub fn bite_in_direction(
        &self,
        step_over: f64,
        valid_area: Vec<Vec<(f64, f64)>>,
        simplify_tol: f64,
        target: (f64, f64),
        max_angle: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        let valid: Vec<crate::types::Polygon> = valid_area
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let bites = self.inner.bite_in_direction(
            step_over,
            &valid,
            simplify_tol,
            crate::types::Point::new(target.0, target.1),
            max_angle,
        );
        bites
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Iteratively call :py:meth:`bites` + :py:meth:`incorporate` until
    /// the valid area is fully cleared.
    ///
    /// Returns all passes, each pass being a list of bite polygons.
    /// The cleared area is fully cleared after this call.
    /// :param step_over: lateral step-over in mm
    /// :param valid_area: list of polygons defining the valid tool-centre region
    /// :param simplify_tol: tolerance in mm for frontier simplification
    /// :complexity: O(k n log n) where k = number of passes
    pub fn all_bites(
        &mut self,
        step_over: f64,
        valid_area: Vec<Vec<(f64, f64)>>,
        simplify_tol: f64,
    ) -> Vec<Vec<Vec<(f64, f64)>>> {
        let valid: Vec<crate::types::Polygon> = valid_area
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let passes = self.inner.all_bites(step_over, &valid, simplify_tol);
        passes
            .into_iter()
            .map(|pass| {
                pass.into_iter()
                    .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
                    .collect()
            })
            .collect()
    }

    /// :complexity: O(1)
    pub fn total_area(&self) -> f64 {
        self.inner.total_area()
    }

    /// Return the union of all polygons currently tracked as cleared.
    ///
    /// Each fragment is a closed polygon (list of ``(x, y)`` vertices)
    /// representing an area that has already been cut.  The fragment set
    /// grows as ``incorporate`` or ``add_cleared_polygons`` are called.
    ///
    /// This is useful for determining which parts of a bite polygon
    /// lie outside the cleared area (i.e. the cutting arc), for example
    /// when used with :py:func:`raygeo.ops.assembly.hsm.find_cutting_arc`.
    /// :complexity: O(m) where m = number of fragments
    pub fn fragments(&self) -> Vec<Vec<(f64, f64)>> {
        self.inner
            .fragments()
            .iter()
            .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Compute the inset region of *boundary* by *radius* (excluding
    /// *obstacles*), then return the portions of that region not covered
    /// by stored fragments, together with the original obstacle polygons.
    ///
    /// :param boundary: Outer boundary polygon.
    /// :param obstacles: Obstacle (hole) polygons to exclude.
    /// :param radius: Inset distance applied to *boundary* and *obstacles*.
    /// :returns: List of polygons — the obstacles plus the uncovered
    ///           portion of the inset region.
    /// :complexity: O(n log n) for the inset and difference operations.
    #[pyo3(signature = (boundary, obstacles = None, radius = 3.0))]
    pub fn remaining_in_inset(
        &self,
        boundary: Vec<(f64, f64)>,
        obstacles: Option<Vec<Vec<(f64, f64)>>>,
        radius: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        let boundary_poly: crate::types::Polygon = boundary
            .into_iter()
            .map(|(x, y)| crate::types::Point::new(x, y))
            .collect();
        let obstacles_polys: Vec<crate::types::Polygon> = obstacles
            .unwrap_or_default()
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let result = self.inner.remaining_in_inset(
            &boundary_poly,
            &obstacles_polys,
            radius,
        );
        result
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("ClearedArea({} fragments)", self.inner.len())
    }
}

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "cleared_area")?;
    m.setattr("__doc__", MODULE_DOC)?;

    m.add_class::<ClearedArea>()?;

    algo_mod.add_submodule(&m)?;
    Ok(())
}
