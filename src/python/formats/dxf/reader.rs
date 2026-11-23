use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::formats::dxf::reader::{read as read_core, DxfHeader};
use crate::geo::read_output::ReadOutput;
use crate::python::geo::geometry::Geometry;

pyo3_stub_gen::module_doc!("raygeo.formats.dxf.reader", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Reading DXF documents.

The reader never fails on a single unreadable entity: anything it could
not represent is reported in ``ReadOutput.issues``, so a file that is
partly unsupported still imports the parts that are.
";

/// The geometry and metadata a DXF reader extracted from a document.
#[gen_stub_pyclass(module = "raygeo.formats.dxf.reader")]
#[pyclass(
    name = "ReadOutput",
    skip_from_py_object,
    module = "raygeo.formats.dxf.reader"
)]
#[derive(Clone)]
pub struct PyReadOutput {
    /// One geometry per layer, as ``(layer_name, geometry)`` pairs in
    /// document order.
    #[pyo3(get)]
    pub layers: Vec<(String, Geometry)>,
    /// Bounding rectangle of all drawn content, in native units, as
    /// ``(x_min, y_min, x_max, y_max)``.
    #[pyo3(get)]
    pub bounds: (f64, f64, f64, f64),
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
            bounds: (
                output.bounds.min.x,
                output.bounds.min.y,
                output.bounds.max.x,
                output.bounds.max.y,
            ),
            units_to_mm: output.units_to_mm,
            is_y_down: output.is_y_down,
            min_z,
            max_z,
            issues: output.issues,
        }
    }
}

/// Details of a DXF document that are not geometry.
#[gen_stub_pyclass(module = "raygeo.formats.dxf.reader")]
#[pyclass(
    name = "DxfHeader",
    skip_from_py_object,
    module = "raygeo.formats.dxf.reader"
)]
#[derive(Clone)]
pub struct PyDxfHeader {
    /// The ``$INSUNITS`` value, or 0 when the file states no unit.
    #[pyo3(get)]
    pub insunits: i64,
    /// The drawing format marker from ``$ACADVER``.
    #[pyo3(get)]
    pub acad_version: String,
    /// The declared drawing extents as ``(x_min, y_min, x_max, y_max)``,
    /// or ``None`` when the file does not state them.
    #[pyo3(get)]
    pub extents: Option<(f64, f64, f64, f64)>,
}

impl From<DxfHeader> for PyDxfHeader {
    fn from(header: DxfHeader) -> Self {
        PyDxfHeader {
            insunits: header.insunits,
            acad_version: header.acad_version,
            extents: header
                .extents
                .map(|r| (r.min.x, r.min.y, r.max.x, r.max.y)),
        }
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

    def read(data: str) -> raygeo.formats.dxf.reader.ReadOutput:
        """Read a DXF document into geometry.

        Supports lines, arcs, circles, ellipses, polylines with bulges,
        splines (including NURBS) and block insertions, which are
        flattened into the drawing. Content the reader cannot represent is
        listed in ``issues`` rather than raising.

        :param data: The DXF file contents as text.
        :returns: Per-layer geometry in native units, Y up, with the
            drawing bounds, the unit conversion factor and any issues.
        :raises ValueError: When the input is not a group-code stream.
        :complexity: O(n) where n = size of the document
        """
"#,
    module = "raygeo.formats.dxf.reader"
)]
#[pyfunction(name = "read")]
fn py_read(data: &str) -> PyResult<PyReadOutput> {
    let output = read_core(data)?;
    Ok(PyReadOutput::from_core(output))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub = PyModule::new(m.py(), "reader")?;
    sub.setattr("__doc__", MODULE_DOC)?;
    sub.add("__all__", vec!["DxfHeader", "ReadOutput", "read"])?;
    sub.add_class::<PyReadOutput>()?;
    sub.add_class::<PyDxfHeader>()?;
    sub.add_function(wrap_pyfunction!(py_read, &sub)?)?;
    m.add_submodule(&sub)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.formats.dxf.reader", &sub)?;
    Ok(())
}
