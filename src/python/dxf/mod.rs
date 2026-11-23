use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::dxf::reader::read as read_core;
use crate::dxf::writer::write as write_core;
use crate::geo::read_output::ReadOutput;
use crate::geo::types::Rect as GeoRect;
use crate::python::geo::geometry::Geometry;

pyo3_stub_gen::module_doc!("raygeo.dxf", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
DXF (Drawing Exchange Format) reading and writing.

Reads the ASCII DXF files CAD applications export for laser cutting:
lines, arcs, circles, ellipses, polylines with bulges, splines, text
and the block insertions that 3D CAD tools wrap their export in.

The reader never fails on a single unreadable entity. Anything it could
not represent is reported in ``ReadOutput.issues``, so a file that is
partly unsupported still imports the parts that are. Coordinates are in
native drawing units with Y pointing up; multiply by
``ReadOutput.units_to_mm`` to convert.
";

/// A rectangle in the drawing's own coordinates.
#[gen_stub_pyclass(module = "raygeo.dxf")]
#[pyclass(name = "Rect", skip_from_py_object, module = "raygeo.dxf")]
#[derive(Clone)]
pub struct PyRect {
    /// Left edge.
    #[pyo3(get)]
    pub min_x: f64,
    /// Top or bottom edge, depending on the drawing's Y direction.
    #[pyo3(get)]
    pub min_y: f64,
    /// Right edge.
    #[pyo3(get)]
    pub max_x: f64,
    /// The other extreme of the vertical axis.
    #[pyo3(get)]
    pub max_y: f64,
}

impl From<GeoRect> for PyRect {
    fn from(rect: GeoRect) -> Self {
        PyRect {
            min_x: rect.min.x,
            min_y: rect.min.y,
            max_x: rect.max.x,
            max_y: rect.max.y,
        }
    }
}

/// The geometry and metadata a DXF reader extracted from a document.
#[gen_stub_pyclass(module = "raygeo.dxf")]
#[pyclass(name = "ReadOutput", skip_from_py_object, module = "raygeo.dxf")]
#[derive(Clone)]
pub struct PyReadOutput {
    /// One geometry per layer, as ``(layer_name, geometry)`` pairs in
    /// document order.
    #[pyo3(get)]
    pub layers: Vec<(String, Geometry)>,
    /// Bounding rectangle of all drawn content, in native units.
    #[pyo3(get)]
    pub bounds: PyRect,
    /// Multiplier converting native units to millimetres.
    #[pyo3(get)]
    pub units_to_mm: f64,
    /// Whether the native Y axis points down. Always ``False`` for DXF.
    #[pyo3(get)]
    pub is_y_down: bool,
    /// Lowest Z value the drawing spans, in native units.
    #[pyo3(get)]
    pub min_z: f64,
    /// Highest Z value the drawing spans, in native units.
    #[pyo3(get)]
    pub max_z: f64,
    /// Notes about content that was skipped or approximated.
    #[pyo3(get)]
    pub issues: Vec<String>,
}

impl PyReadOutput {
    fn from_core(output: ReadOutput) -> Self {
        let (min_z, max_z) = output.z_range;
        PyReadOutput {
            layers: output
                .layers
                .into_iter()
                .map(|(name, geo)| (name, Geometry { inner: geo }))
                .collect(),
            bounds: output.bounds.into(),
            units_to_mm: output.units_to_mm,
            is_y_down: output.is_y_down,
            min_z,
            max_z,
            issues: output.issues,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyRect {
    /// Width of the rectangle.
    #[getter]
    fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// Height of the rectangle.
    #[getter]
    fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    fn __repr__(&self) -> String {
        format!(
            "Rect({}, {}, {}, {})",
            self.min_x, self.min_y, self.max_x, self.max_y
        )
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyReadOutput {
    /// True when the drawing spans more than one Z level.
    ///
    /// :param tolerance: Smallest Z difference that counts as variation,
    ///     in native units.
    #[pyo3(signature = (tolerance=1e-4))]
    fn has_z_variation(&self, tolerance: f64) -> bool {
        (self.max_z - self.min_z) > tolerance
    }

    /// True when nothing drawable was found.
    fn is_empty(&self) -> bool {
        self.layers.iter().all(|(_, geo)| geo.inner.is_empty())
    }

    fn __repr__(&self) -> String {
        format!(
            "ReadOutput(layers={}, units_to_mm={}, issues={})",
            self.layers.len(),
            self.units_to_mm,
            self.issues.len()
        )
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def read(data: str) -> raygeo.dxf.ReadOutput:
        """Read a DXF document into geometry.

        Supports lines, arcs, circles, ellipses, polylines with bulges,
        splines (including NURBS), text rendered through the system font,
        and block insertions, which are flattened into the drawing.
        Content the reader cannot represent is listed in ``issues``
        rather than raising.

        :param data: The DXF file contents as text.
        :returns: Per-layer geometry in native units, Y up, with the
            drawing bounds, the unit conversion factor and any issues.
        :raises ValueError: When the input is not a group-code stream.
        :complexity: O(n) where n = size of the document
        """
"#,
    module = "raygeo.dxf"
)]
#[pyfunction(name = "read")]
fn py_read(data: &str) -> PyResult<PyReadOutput> {
    let output = read_core(data)?;
    Ok(PyReadOutput::from_core(output))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def write(geometry: raygeo.geo.Geometry) -> str:
        """Write geometry as a DXF document.

        The output is R12-compatible ASCII DXF with millimetres declared
        as the unit, which is the variant every CAD tool and laser cutter
        controller accepts. Consecutive lines become one ``LWPOLYLINE``,
        arcs remain ``ARC`` entities, and beziers are written as splines.

        :param geometry: The geometry to write, in millimetres.
        :returns: The DXF document as text.
        :complexity: O(n) where n = number of commands
        """
"#,
    module = "raygeo.dxf"
)]
#[pyfunction(name = "write")]
fn py_write(geometry: &Geometry) -> String {
    write_core(&geometry.inner)
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let dxf_mod = PyModule::new(py, "dxf")?;

    dxf_mod.setattr("__doc__", MODULE_DOC)?;
    dxf_mod.add("__all__", vec!["ReadOutput", "Rect", "read", "write"])?;
    dxf_mod.add_class::<PyReadOutput>()?;
    dxf_mod.add_class::<PyRect>()?;
    dxf_mod.add_function(wrap_pyfunction!(py_read, &dxf_mod)?)?;
    dxf_mod.add_function(wrap_pyfunction!(py_write, &dxf_mod)?)?;

    m.add_submodule(&dxf_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.dxf", &dxf_mod)?;

    Ok(())
}
