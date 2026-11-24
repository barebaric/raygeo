use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pyfunction,
};

use crate::formats::coreldraw::detect::{detect_format, CdrFormat};
use crate::formats::coreldraw::{parse, CdrDocument};
use crate::python::errors::{CdrError, CdrParseError, CdrUnsupportedFormat};
use crate::python::geo::geometry::Geometry;

pyo3_stub_gen::module_doc!("raygeo.formats.coreldraw", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CorelDRAW (``.cdr``) import.

Parses the drawing natively: RIFF and ZIP containers, compressed chunk
lists, object records and their transforms are all read in Rust, so no
external tool or C library is needed. Text is drawn as glyph outlines
with the system's closest font; bitmaps and object kinds the reader does
not know are reported in :attr:`CdrDocument.issues` instead of being
converted.
";

/// The kind of Corel document a byte stream holds.
#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.formats.coreldraw",
    name = "CdrFormat",
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyCdrFormat {
    /// A CorelDRAW drawing (``.cdr``).
    #[pyo3(name = "CDR")]
    Cdr,
    /// A Corel Presentation Exchange file (``.cmx``).
    #[pyo3(name = "CMX")]
    Cmx,
}

impl From<CdrFormat> for PyCdrFormat {
    fn from(format: CdrFormat) -> Self {
        match format {
            CdrFormat::Cdr => PyCdrFormat::Cdr,
            CdrFormat::Cmx => PyCdrFormat::Cmx,
        }
    }
}

#[pymethods]
impl PyCdrFormat {
    fn __repr__(&self) -> String {
        format!("CdrFormat.{}", self.name())
    }

    fn __str__(&self) -> String {
        self.value().to_string()
    }

    #[getter]
    fn name(&self) -> &str {
        match self {
            PyCdrFormat::Cdr => "CDR",
            PyCdrFormat::Cmx => "CMX",
        }
    }

    #[getter]
    fn value(&self) -> &str {
        match self {
            PyCdrFormat::Cdr => "cdr",
            PyCdrFormat::Cmx => "cmx",
        }
    }

    /// Enum members are singletons: copying returns the canonical member.
    fn __deepcopy__(
        &self,
        py: Python<'_>,
        _memo: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        Ok(py.get_type::<PyCdrFormat>().getattr(self.name())?.unbind())
    }

    /// Reconstruct from ``getattr(CdrFormat, name)`` so members can be
    /// pickled and deep-copied.
    fn __reduce__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let cls = py.get_type::<PyCdrFormat>();
        let builtins = py.import("builtins")?;
        let getattr = builtins.getattr("getattr")?;
        let args = (cls, self.name());
        Ok((getattr, args).into_pyobject(py)?.into_any().unbind())
    }
}

/// The geometry of a CorelDRAW page.
#[gen_stub_pyclass(module = "raygeo.formats.coreldraw")]
#[pyclass(
    name = "CdrDocument",
    skip_from_py_object,
    module = "raygeo.formats.coreldraw"
)]
#[derive(Clone)]
pub struct PyCdrDocument {
    /// The page width in inches.
    #[pyo3(get)]
    pub page_width: f64,
    /// The page height in inches.
    #[pyo3(get)]
    pub page_height: f64,
    /// The number of non-master pages in the document.
    #[pyo3(get)]
    pub page_count: usize,
    /// The layers of the first page, bottom to top, as
    /// ``(name, geometry)`` pairs. The geometry is in inches, relative to
    /// the page's bottom-left corner.
    #[pyo3(get)]
    pub layers: Vec<(String, Geometry)>,
    /// Content the reader did not convert, one note per category.
    #[pyo3(get)]
    pub issues: Vec<String>,
    /// Multiplier converting the native unit (inches) to millimetres.
    #[pyo3(get)]
    pub units_to_mm: f64,
}

impl From<CdrDocument> for PyCdrDocument {
    fn from(document: CdrDocument) -> Self {
        PyCdrDocument {
            page_width: document.page_width,
            page_height: document.page_height,
            page_count: document.page_count,
            layers: document
                .layers
                .into_iter()
                .map(|(name, geometry)| (name, Geometry { inner: geometry }))
                .collect(),
            issues: document.issues,
            units_to_mm: CdrDocument::UNITS_TO_MM,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo
    import raygeo.formats.coreldraw

    def detect_format(data: bytes) -> raygeo.formats.coreldraw.CdrFormat | None:
        """Return the kind of Corel document in ``data``.

        Recognizes a bare RIFF drawing, a ZIP package holding one, and the
        earliest ``WL`` format; CMX files are recognized but not read.

        :param data: The first bytes of the file, or the whole file.
        :returns: The document kind, or ``None`` when the data is not a
            Corel document.
        :complexity: O(n) where n = size of the data
        """
"#,
    module = "raygeo.formats.coreldraw"
)]
#[pyfunction(name = "detect_format")]
fn py_detect_format(data: &[u8]) -> Option<PyCdrFormat> {
    detect_format(data).map(PyCdrFormat::from)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo
    import raygeo.formats.coreldraw

    def parse(data: bytes) -> raygeo.formats.coreldraw.CdrDocument:
        """Read a CorelDRAW drawing into the geometry of its first page.

        The whole file is parsed in Rust: RIFF and ZIP containers,
        compressed chunk lists, object records and their transforms.
        Drawing geometry and text are extracted; text becomes glyph
        outlines of the system's closest font, since CorelDRAW's embedded
        fonts are not used. Bitmaps and unknown object kinds are listed in
        ``issues``. The layers of the first page come bottom to top, their
        geometry in inches with the page origin at the bottom-left corner.

        :param data: The document's bytes.
        :returns: The page size, its layers and the conversion issues.
        :raises CdrUnsupportedFormat: When the data is not a CorelDRAW
            document.
        :raises CdrParseError: When the container, a record or an argument
            table cannot be read, or when the file is a CMX file.
        :complexity: O(n) where n = size of the document
        """
"#,
    module = "raygeo.formats.coreldraw"
)]
#[pyfunction(name = "parse")]
fn py_parse(py: Python<'_>, data: &[u8]) -> PyResult<PyCdrDocument> {
    let document = py.detach(|| parse(data))?;
    Ok(document.into())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let sub = PyModule::new(py, "coreldraw")?;
    sub.setattr("__doc__", MODULE_DOC)?;
    sub.add(
        "__all__",
        vec![
            "CdrDocument",
            "CdrError",
            "CdrFormat",
            "CdrParseError",
            "CdrUnsupportedFormat",
            "detect_format",
            "parse",
        ],
    )?;
    sub.add_class::<PyCdrFormat>()?;
    sub.add_class::<PyCdrDocument>()?;
    sub.add("CdrError", py.get_type::<CdrError>())?;
    sub.add(
        "CdrUnsupportedFormat",
        py.get_type::<CdrUnsupportedFormat>(),
    )?;
    sub.add("CdrParseError", py.get_type::<CdrParseError>())?;
    sub.add_function(wrap_pyfunction!(py_detect_format, &sub)?)?;
    sub.add_function(wrap_pyfunction!(py_parse, &sub)?)?;
    m.add_submodule(&sub)?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.formats.coreldraw", &sub)?;
    Ok(())
}
