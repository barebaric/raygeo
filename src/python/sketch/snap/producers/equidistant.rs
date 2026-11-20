//! Snap points at equally spaced positions.

use std::collections::HashSet;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::snap::types::{
    PyDragContext, PySnapLine, PySnapLineType, PySnapPoint,
};

#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.sketch.snap.producers", name = "EquidistantLinesProducer")]
pub struct PyEquidistantLinesProducer {
    spacing_tolerance: f64,
    max_spacing: f64,
    include_construction: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEquidistantLinesProducer {
    #[new]
    #[pyo3(signature = (spacing_tolerance=0.5, max_spacing=100.0, include_construction=true))]
    fn new(
        spacing_tolerance: f64,
        max_spacing: f64,
        include_construction: bool,
    ) -> Self {
        Self {
            spacing_tolerance,
            max_spacing,
            include_construction,
        }
    }

    #[getter]
    fn _spacing_tolerance(&self) -> f64 {
        self.spacing_tolerance
    }

    #[getter]
    fn _max_spacing(&self) -> f64 {
        self.max_spacing
    }

    #[getter]
    fn _include_construction(&self) -> bool {
        self.include_construction
    }

    fn produce(
        &self,
        _py: Python<'_>,
        _registry: &Bound<'_, PyEntityRegistry>,
        _drag_position: (f64, f64),
        _drag_context: &PyDragContext,
        _threshold: f64,
    ) -> Vec<Py<PySnapLine>> {
        Vec::new()
    }

    fn produce_points(
        &self,
        py: Python<'_>,
        registry: &Bound<'_, PyEntityRegistry>,
        drag_position: (f64, f64),
        drag_context: &PyDragContext,
        threshold: f64,
    ) -> Vec<Py<PySnapPoint>> {
        let reg = registry.borrow();
        let (x, y) = drag_position;
        let mut points = Vec::new();
        let points_list: Vec<_> = reg
            .inner
            .point_ids()
            .into_iter()
            .filter(|id| !drag_context.is_point_dragged(*id))
            .filter_map(|id| reg.inner.get_point(id))
            .collect();
        if !self.include_construction {
            // The reference only filters entities; points have no
            // construction flag here.
        }

        let (v_coords, axis_x) =
            self.collect_aligned(&points_list, None, Some(x), threshold);
        let (h_coords, axis_y) =
            self.collect_aligned(&points_list, Some(y), None, threshold);

        for (snap_coord, spacing, pattern) in
            self.find_snaps(&v_coords, y, threshold)
        {
            points.push(Py::new(
                py,
                PySnapPoint {
                    x: axis_x,
                    y: snap_coord,
                    line_type: PySnapLineType(
                        PySnapLineType::EQUIDISTANT_VALUE,
                    ),
                    source: None,
                    spacing: Some(spacing),
                    is_horizontal: true,
                    pattern_coords: Some(pattern),
                    axis_coord: Some(axis_x),
                },
            ).unwrap());
        }
        for (snap_coord, spacing, pattern) in
            self.find_snaps(&h_coords, x, threshold)
        {
            points.push(Py::new(
                py,
                PySnapPoint {
                    x: snap_coord,
                    y: axis_y,
                    line_type: PySnapLineType(
                        PySnapLineType::EQUIDISTANT_VALUE,
                    ),
                    source: None,
                    spacing: Some(spacing),
                    is_horizontal: false,
                    pattern_coords: Some(pattern),
                    axis_coord: Some(axis_y),
                },
            ).unwrap());
        }
        points
    }
}

impl PyEquidistantLinesProducer {
    fn collect_aligned(
        &self,
        points: &[crate::sketch::entities::point::SketchPoint],
        fixed_y: Option<f64>,
        fixed_x: Option<f64>,
        threshold: f64,
    ) -> (Vec<f64>, f64) {
        let mut coords = Vec::new();
        let mut axis_values = Vec::new();
        for point in points {
            if let Some(fx) = fixed_x {
                if (point.x - fx).abs() > threshold {
                    continue;
                }
                coords.push(point.y);
                axis_values.push(point.x);
            } else if let Some(fy) = fixed_y {
                if (point.y - fy).abs() > threshold {
                    continue;
                }
                coords.push(point.x);
                axis_values.push(point.y);
            }
        }
        coords.sort_by(f64::total_cmp);
        coords.dedup();
        let axis = if axis_values.is_empty() {
            0.0
        } else {
            axis_values.iter().sum::<f64>() / axis_values.len() as f64
        };
        (coords, axis)
    }

    fn find_snaps(
        &self,
        aligned: &[f64],
        drag_coord: f64,
        threshold: f64,
    ) -> Vec<(f64, f64, Vec<f64>)> {
        if aligned.len() < 2 {
            return Vec::new();
        }
        let mut spacings: HashSet<u64> = HashSet::new();
        for pair in aligned.windows(2) {
            let spacing = pair[1] - pair[0];
            if spacing > 1e-6 && spacing <= self.max_spacing {
                spacings.insert(spacing.to_bits());
            }
        }
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for spacing in spacings.into_iter().map(f64::from_bits) {
            for base in aligned {
                let n = ((drag_coord - base) / spacing).round();
                let snap = base + n * spacing;
                if seen.contains(&snap.to_bits()) {
                    continue;
                }
                if (snap - drag_coord).abs() > threshold {
                    continue;
                }
                let pattern = self.build_pattern(aligned, snap, spacing);
                if pattern.len() >= 3 {
                    seen.insert(snap.to_bits());
                    out.push((snap, spacing, pattern));
                }
            }
        }
        out
    }

    fn build_pattern(
        &self,
        aligned: &[f64],
        snap_coord: f64,
        spacing: f64,
    ) -> Vec<f64> {
        let mut all_coords: Vec<f64> = aligned.to_vec();
        all_coords.push(snap_coord);
        let mut pattern = Vec::new();
        let near = |coord: f64| {
            all_coords
                .iter()
                .any(|c| (coord - c).abs() < self.spacing_tolerance)
        };
        let min = all_coords.iter().copied().fold(f64::INFINITY, f64::min);
        let max = all_coords
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let mut coord = snap_coord;
        while near(coord) {
            pattern.push(coord);
            coord -= spacing;
            if coord < min - spacing {
                break;
            }
        }
        coord = snap_coord + spacing;
        while near(coord) {
            pattern.push(coord);
            coord += spacing;
            if coord > max + spacing {
                break;
            }
        }
        pattern.sort_by(f64::total_cmp);
        pattern
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyEquidistantLinesProducer>()?;
    Ok(())
}
