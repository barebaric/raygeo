use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::cylindrical as rust_cylindrical;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def transform_to_cylinder(
        verts: numpy.typing.NDArray[numpy.float32],
        diameter: float,
        colors: numpy.typing.NDArray[numpy.float32] | None = None,
        degrees_input: bool = False,
    ) -> tuple[
        numpy.typing.NDArray[numpy.float32],
        numpy.typing.NDArray[numpy.float32] | None,
        numpy.typing.NDArray[numpy.int32],
    ]:
        """Transform flat vertex pairs to cylindrical coordinates.

        The input has angular values on axis 1 (Y) and linear position
        on axis 0 (X). The output maps to:
            X = linear position along cylinder
            Y = r * sin(theta)
            Z = r * cos(theta)

        Line segments are subdivided as needed to follow the cylinder
        surface instead of cutting through the interior.

        :param verts: Float32 array of shape (N, 3) with X, Y, Z per vertex.
                      Vertices are in pairs (line segments for GL_LINES).
        :param diameter: Cylinder diameter in mm.
        :param colors: Optional float32 array of shape (N, 4) RGBA colors.
        :param degrees_input: If True, Y values are in degrees and converted
                              directly. If False (default), they are in motor
                              units (mu) and converted via mu_to_degrees.
        :returns: Tuple of (transformed_vertices, expanded_colors, cum_subs).
                  cum_subs is a cumulative subdivision count array.
        :complexity: O(N * subdivisions)
        """
    "#,
    module = "raygeo.geo.algo.cylindrical"
)]
#[pyfunction(name = "transform_to_cylinder")]
#[pyo3(signature = (verts, diameter, colors=None, degrees_input=false))]
fn py_transform_to_cylinder(
    py: Python<'_>,
    verts: &Bound<'_, PyAny>,
    diameter: f64,
    colors: Option<&Bound<'_, PyAny>>,
    degrees_input: bool,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;

    let vert_flat: Vec<f32> = {
        let arr = numpy.call_method1("asarray", (verts,))?;
        arr.call_method0("flatten")?
            .call_method0("tolist")?
            .extract()?
    };

    let col_flat: Option<Vec<f32>> = if let Some(c) = colors {
        let c_arr = numpy.call_method1("asarray", (c,))?;
        Some(
            c_arr
                .call_method0("flatten")?
                .call_method0("tolist")?
                .extract()?,
        )
    } else {
        None
    };

    let (result_verts, result_colors, cum_subs) =
        rust_cylindrical::transform_to_cylinder(
            &vert_flat,
            diameter,
            col_flat.as_deref(),
            degrees_input,
        );

    let verts_arr = result_verts.into_pyarray(py);
    let verts_py = verts_arr
        .call_method1("reshape", (-1i32, 3i32))?
        .into_any()
        .unbind();

    let cum_subs_py = cum_subs.into_pyarray(py).into_any().unbind();

    let items: Vec<Py<PyAny>> = if let Some(cols) = result_colors {
        let colors_arr = cols.into_pyarray(py);
        let colors_py = colors_arr
            .call_method1("reshape", (-1i32, 4i32))?
            .into_any()
            .unbind();
        vec![verts_py, colors_py, cum_subs_py]
    } else {
        vec![verts_py, py.None(), cum_subs_py]
    };

    let tuple = PyTuple::new(py, &items)?;
    Ok(tuple.into_any().unbind())
}

pub(crate) fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "cylindrical")?;
    m.setattr(
        "__doc__",
        "Cylindrical coordinate transformation utilities.",
    )?;
    register_functions!(m, py_transform_to_cylinder);
    algo_mod.add_submodule(&m)?;
    Ok(())
}
