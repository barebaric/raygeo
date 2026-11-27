use glam::DMat4;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::formats::dxf::reader::{
    read as read_core, read_with_inserts as read_with_inserts_core, DxfHeader,
    InsertOutput, StructuredOutput,
};
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

/// One `INSERT`, with the block content it places in world space.
#[gen_stub_pyclass(module = "raygeo.formats.dxf.reader")]
#[pyclass(
    name = "Insert",
    skip_from_py_object,
    module = "raygeo.formats.dxf.reader"
)]
#[derive(Clone)]
pub struct PyInsert {
    /// The name of the inserted block.
    #[pyo3(get)]
    pub block_name: String,
    /// The layer the `INSERT` is on, after layer-0 inheritance.
    #[pyo3(get)]
    pub layer: String,
    /// The block-to-world transform as a row-major 4x4 list of lists.
    #[pyo3(get)]
    pub transform: Vec<Vec<f64>>,
    /// The block's geometry per layer, as ``(layer_name, geometry)``
    /// pairs in document order, already in world space.
    #[pyo3(get)]
    pub layers: Vec<(String, Geometry)>,
}

impl PyInsert {
    fn from_core(insert: InsertOutput) -> Self {
        PyInsert {
            block_name: insert.block_name,
            layer: insert.layer,
            transform: matrix_to_list(insert.transform),
            layers: insert
                .layers
                .into_iter()
                .map(|(name, geo)| (name, Geometry { inner: geo }))
                .collect(),
        }
    }
}

/// A read that keeps block insertions apart.
#[gen_stub_pyclass(module = "raygeo.formats.dxf.reader")]
#[pyclass(
    name = "ReadStructure",
    skip_from_py_object,
    module = "raygeo.formats.dxf.reader"
)]
#[derive(Clone)]
pub struct PyReadStructure {
    /// The same flattened result ``read()`` returns.
    #[pyo3(get)]
    pub output: PyReadOutput,
    /// One entry per ``INSERT`` in the ENTITIES section, in document
    /// order. Insertions nested inside a block are resolved into the
    /// entry of the insertion that placed that block.
    #[pyo3(get)]
    pub inserts: Vec<PyInsert>,
    /// Geometry of entities that are not part of any ``INSERT``, as
    /// ``(layer_name, geometry)`` pairs in document order.
    #[pyo3(get)]
    pub standalone_layers: Vec<(String, Geometry)>,
}

impl PyReadStructure {
    fn from_core(structure: StructuredOutput) -> Self {
        PyReadStructure {
            output: PyReadOutput::from_core(structure.output),
            inserts: structure
                .inserts
                .into_iter()
                .map(PyInsert::from_core)
                .collect(),
            standalone_layers: structure
                .standalone_layers
                .into_iter()
                .map(|(name, geo)| (name, Geometry { inner: geo }))
                .collect(),
        }
    }
}

/// Formats a transform as a row-major 4x4 list of lists.
fn matrix_to_list(matrix: DMat4) -> Vec<Vec<f64>> {
    vec![
        vec![
            matrix.x_axis.x,
            matrix.y_axis.x,
            matrix.z_axis.x,
            matrix.w_axis.x,
        ],
        vec![
            matrix.x_axis.y,
            matrix.y_axis.y,
            matrix.z_axis.y,
            matrix.w_axis.y,
        ],
        vec![
            matrix.x_axis.z,
            matrix.y_axis.z,
            matrix.z_axis.z,
            matrix.w_axis.z,
        ],
        vec![
            matrix.x_axis.w,
            matrix.y_axis.w,
            matrix.z_axis.w,
            matrix.w_axis.w,
        ],
    ]
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

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def read_with_inserts(data: str) -> raygeo.formats.dxf.reader.ReadStructure:
        """Read a DXF document, keeping each block insertion apart.

        The returned structure's ``output`` is the same flattened result
        ``read()`` returns. ``inserts`` holds one entry per ``INSERT`` in
        the drawing, with the block's name, its layer, its block-to-world
        transform and its geometry per layer, already in world space.
        ``standalone_layers`` holds the content that is not part of any
        block, so a caller can keep blocks as units without duplicating
        geometry.

        Insertions nested inside a block are resolved into the entry of
        the insertion that placed that block; they are not reported
        separately.

        :param data: The DXF file contents as text.
        :returns: The flattened result, the per-INSERT structure and the
            content outside blocks.
        :raises ValueError: When the input is not a group-code stream.
        :complexity: O(n) where n = size of the document
        """
"#,
    module = "raygeo.formats.dxf.reader"
)]
#[pyfunction(name = "read_with_inserts")]
fn py_read_with_inserts(data: &str) -> PyResult<PyReadStructure> {
    let structure = read_with_inserts_core(data)?;
    Ok(PyReadStructure::from_core(structure))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub = PyModule::new(m.py(), "reader")?;
    sub.setattr("__doc__", MODULE_DOC)?;
    sub.add(
        "__all__",
        vec![
            "DxfHeader",
            "Insert",
            "ReadOutput",
            "ReadStructure",
            "read",
            "read_with_inserts",
        ],
    )?;
    sub.add_class::<PyReadOutput>()?;
    sub.add_class::<PyDxfHeader>()?;
    sub.add_class::<PyInsert>()?;
    sub.add_class::<PyReadStructure>()?;
    sub.add_function(wrap_pyfunction!(py_read, &sub)?)?;
    sub.add_function(wrap_pyfunction!(py_read_with_inserts, &sub)?)?;
    m.add_submodule(&sub)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.formats.dxf.reader", &sub)?;
    Ok(())
}
