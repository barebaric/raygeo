use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::geometry::Geometry;
use crate::image::render::{geometry_to_image, RenderOptions};
use crate::ops::assembly::material_test_grid::{
    generate_material_test_grid, MaterialTestGridParams,
};
use crate::ops::assembly::tracelet::Tracelet;
use crate::python::ops::assembly::result::PyAssemblyResult;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "material_test_grid")?;
    m.add_function(pyo3::wrap_pyfunction!(
        generate_material_test_grid_py,
        m.clone()
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        generate_material_test_grid_preview_py,
        m.clone()
    )?)?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.material_test_grid", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def generate_material_test_grid(
        size_mm: tuple[float, float],
        cols: int = 5,
        rows: int = 5,
        min_speed: float = 100.0,
        max_speed: float = 500.0,
        min_power: float = 10.0,
        max_power: float = 100.0,
        min_passes: int = 1,
        max_passes: int = 5,
        fixed_speed: float = 1000.0,
        fixed_power: float = 50.0,
        shape_size: float = 10.0,
        spacing: float = 2.0,
        line_interval_mm: float = 0.1,
        mode: str = "engrave",
        grid_mode: str = "Power vs Speed",
        include_labels: bool = True,
        label_power_percent: float = 10.0,
        label_speed: float = 1000.0,
        min_offset: float = -0.5,
        max_offset: float = 0.5,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate a material test grid with varying speed and power.

        Creates grid cells in rows x cols arrangement, each with baked-in
        power, speed, and pass count. When *include_labels* is True (default),
        column headers, row labels, and axis titles are generated using
        raygeo's built-in text-to-geometry (swash/fontdb).

        :param size_mm: The (width, height) of the workpiece in mm.
        :param cols: Number of columns (default 5).
        :param rows: Number of rows (default 5).
        :param min_speed: Minimum speed in mm/min (default 100.0).
        :param max_speed: Maximum speed in mm/min (default 500.0).
        :param min_power: Minimum power in percent (default 10.0).
        :param max_power: Maximum power in percent (default 100.0).
        :param min_passes: Minimum number of passes (default 1).
        :param max_passes: Maximum number of passes (default 5).
        :param fixed_speed: Fixed speed for Power vs Passes mode (default 1000.0).
        :param fixed_power: Fixed power for Speed vs Passes mode (default 50.0).
        :param shape_size: Size of each grid cell in mm (default 10.0).
        :param spacing: Spacing between cells in mm (default 2.0).
        :param line_interval_mm: Line spacing for engrave mode (default 0.1).
        :param mode: "engrave" or "cut" (default "engrave").
        :param grid_mode: "Power vs Speed", "Power vs Passes", "Speed vs Passes",
                         or "Speed vs Offset" (default "Power vs Speed").
        :param include_labels: Generate text labels (default True).
        :param label_power_percent: Power for label engraving in percent (default 10.0).
        :param label_speed: Feed rate for label engraving in mm/min (default 1000.0).
        :param min_offset: Minimum bidirectional scan offset in mm for
                         Speed vs Offset mode (default -0.5).
        :param max_offset: Maximum bidirectional scan offset in mm for
                         Speed vs Offset mode (default 0.5).
        :returns: An :class:`AssemblyResult` with grid cell paths and labels.
        """
    "#,
    module = "raygeo.ops.assembly.material_test_grid"
)]
#[allow(clippy::too_many_arguments)]
#[pyfunction(name = "generate_material_test_grid")]
#[pyo3(signature = (
    size_mm,
    cols = 5,
    rows = 5,
    min_speed = 100.0,
    max_speed = 500.0,
    min_power = 10.0,
    max_power = 100.0,
    min_passes = 1,
    max_passes = 5,
    fixed_speed = 1000.0,
    fixed_power = 50.0,
    shape_size = 10.0,
    spacing = 2.0,
    line_interval_mm = 0.1,
    mode = "engrave",
    grid_mode = "Power vs Speed",
    include_labels = true,
    label_power_percent = 10.0,
    label_speed = 1000.0,
    min_offset = -0.5,
    max_offset = 0.5,
))]
fn generate_material_test_grid_py(
    size_mm: (f64, f64),
    cols: u32,
    rows: u32,
    min_speed: f64,
    max_speed: f64,
    min_power: f64,
    max_power: f64,
    min_passes: u32,
    max_passes: u32,
    fixed_speed: f64,
    fixed_power: f64,
    shape_size: f64,
    spacing: f64,
    line_interval_mm: f64,
    mode: &str,
    grid_mode: &str,
    include_labels: bool,
    label_power_percent: f64,
    label_speed: f64,
    min_offset: f64,
    max_offset: f64,
) -> PyResult<PyAssemblyResult> {
    let params = MaterialTestGridParams {
        cols,
        rows,
        min_speed,
        max_speed,
        min_power,
        max_power,
        min_passes,
        max_passes,
        fixed_speed,
        fixed_power,
        shape_size,
        spacing,
        line_interval_mm,
        mode: mode.to_string(),
        grid_mode: grid_mode.to_string(),
        include_labels,
        label_power: label_power_percent / 100.0,
        label_speed: label_speed as i32,
        min_offset,
        max_offset,
    };

    let mut trace = Tracelet::new();
    let meta = generate_material_test_grid(&params, size_mm, &mut trace)?;
    let trace_events = trace.drain();
    let trace_attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(
        ops,
        meta,
        trace_attrs,
        trace_events,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import raygeo

    def generate_material_test_grid_preview(
        size_mm: tuple[float, float],
        dpi: float = 96.0,
        cols: int = 5,
        rows: int = 5,
        min_speed: float = 100.0,
        max_speed: float = 500.0,
        min_power: float = 10.0,
        max_power: float = 100.0,
        min_passes: int = 1,
        max_passes: int = 5,
        fixed_speed: float = 1000.0,
        fixed_power: float = 50.0,
        shape_size: float = 10.0,
        spacing: float = 2.0,
        line_interval_mm: float = 0.1,
        mode: str = "engrave",
        grid_mode: str = "Power vs Speed",
        include_labels: bool = True,
        label_power_percent: float = 10.0,
        label_speed: float = 1000.0,
        min_offset: float = -0.5,
        max_offset: float = 0.5,
    ) -> numpy.ndarray:
        """Generate a raster preview of the material test grid.

        Creates the same grid as :func:`generate_material_test_grid` but
        renders it to an RGBA numpy array instead of returning Ops.

        :param size_mm: The (width, height) of the workpiece in mm.
        :param dpi: Output resolution in dots per inch (default 96.0).
        :param cols: Number of columns (default 5).
        :param rows: Number of rows (default 5).
        :param min_speed: Minimum speed in mm/min (default 100.0).
        :param max_speed: Maximum speed in mm/min (default 500.0).
        :param min_power: Minimum power in percent (default 10.0).
        :param max_power: Maximum power in percent (default 100.0).
        :param min_passes: Minimum number of passes (default 1).
        :param max_passes: Maximum number of passes (default 5).
        :param fixed_speed: Fixed speed for Power vs Passes mode (default 1000.0).
        :param fixed_power: Fixed power for Speed vs Passes mode (default 50.0).
        :param shape_size: Size of each grid cell in mm (default 10.0).
        :param spacing: Spacing between cells in mm (default 2.0).
        :param line_interval_mm: Line spacing for engrave mode (default 0.1).
        :param mode: "engrave" or "cut" (default "engrave").
        :param grid_mode: "Power vs Speed", "Power vs Passes", "Speed vs Passes",
                         or "Speed vs Offset" (default "Power vs Speed").
        :param include_labels: Generate text labels (default True).
        :param label_power_percent: Power for label engraving in percent (default 10.0).
        :param label_speed: Feed rate for label engraving in mm/min (default 1000.0).
        :param min_offset: Minimum bidirectional scan offset in mm for
                         Speed vs Offset mode (default -0.5).
        :param max_offset: Maximum bidirectional scan offset in mm for
                         Speed vs Offset mode (default 0.5).
        :returns: A (H, W, 4) RGBA uint8 numpy array.
        """
    "#,
    module = "raygeo.ops.assembly.material_test_grid"
)]
#[allow(clippy::too_many_arguments)]
#[pyfunction(name = "generate_material_test_grid_preview")]
#[pyo3(signature = (
    size_mm,
    dpi = 96.0,
    cols = 5,
    rows = 5,
    min_speed = 100.0,
    max_speed = 500.0,
    min_power = 10.0,
    max_power = 100.0,
    min_passes = 1,
    max_passes = 5,
    fixed_speed = 1000.0,
    fixed_power = 50.0,
    shape_size = 10.0,
    spacing = 2.0,
    line_interval_mm = 0.1,
    mode = "engrave",
    grid_mode = "Power vs Speed",
    include_labels = true,
    label_power_percent = 10.0,
    label_speed = 1000.0,
    min_offset = -0.5,
    max_offset = 0.5,
))]
fn generate_material_test_grid_preview_py(
    py: Python<'_>,
    size_mm: (f64, f64),
    dpi: f64,
    cols: u32,
    rows: u32,
    min_speed: f64,
    max_speed: f64,
    min_power: f64,
    max_power: f64,
    min_passes: u32,
    max_passes: u32,
    fixed_speed: f64,
    fixed_power: f64,
    shape_size: f64,
    spacing: f64,
    line_interval_mm: f64,
    mode: &str,
    grid_mode: &str,
    include_labels: bool,
    label_power_percent: f64,
    label_speed: f64,
    min_offset: f64,
    max_offset: f64,
) -> PyResult<Py<PyAny>> {
    let params = MaterialTestGridParams {
        cols,
        rows,
        min_speed,
        max_speed,
        min_power,
        max_power,
        min_passes,
        max_passes,
        fixed_speed,
        fixed_power,
        shape_size,
        spacing,
        line_interval_mm,
        mode: mode.to_string(),
        grid_mode: grid_mode.to_string(),
        include_labels,
        label_power: label_power_percent / 100.0,
        label_speed: label_speed as i32,
        min_offset,
        max_offset,
    };

    // Use the SAME code path as the Ops: generate the full grid + labels,
    // then rasterise the resulting geometry.  The Ops are already Y-down
    // (generate_material_test_grid applies scale(1,-1)+translate(0,height)).
    // geometry_to_image expects Y-up and flips to Y-down, so we invert
    // twice and get Y-up output — which matches the canvas coordinate
    // system used for ops rendering.
    let mut trace = Tracelet::new();
    let _meta = generate_material_test_grid(&params, size_mm, &mut trace)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let ops = trace.into_ops();
    let geo = ops.to_geometry();
    let empty = Geometry::new();
    let opts = RenderOptions {
        dpi,
        ..Default::default()
    };
    let (buf, height, width) = geometry_to_image(&geo, &empty, size_mm, &opts);

    if buf.is_empty() {
        let numpy = py.import("numpy")?;
        let arr = numpy.call_method1("zeros", ((0, 0, 4), "uint8"))?;
        return Ok(arr.into_pyobject(py)?.into_any().unbind());
    }

    let py_bytes = PyBytes::new(py, &buf);
    let numpy = py.import("numpy")?;
    let arr = numpy
        .call_method1("frombuffer", (py_bytes, numpy.getattr("uint8")?))?;
    let arr =
        arr.call_method1("reshape", (height as i64, width as i64, 4i64))?;
    Ok(arr.into_pyobject(py)?.into_any().unbind())
}
