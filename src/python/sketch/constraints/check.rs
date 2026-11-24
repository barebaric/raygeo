use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import numpy
    import numpy.typing

    def check_grad(
        func: collections.abc.Callable[
            [numpy.typing.NDArray[numpy.float64]], float
        ],
        grad: collections.abc.Callable[
            [numpy.typing.NDArray[numpy.float64]],
            numpy.typing.NDArray[numpy.float64],
        ],
        x0: numpy.typing.ArrayLike,
        epsilon: float = 1e-6,
    ) -> float:
        """Compare an analytical gradient with forward differences.

        Mirrors ``scipy.optimize.check_grad``: the analytical gradient is
        evaluated at ``x0``, each coordinate is then perturbed by
        ``epsilon`` and ``func`` is re-evaluated, and the L2 norm of the
        difference between the analytical and numerical gradients is
        returned.

        :param func: Scalar function of a 1-D float64 array.
        :param grad: Analytical gradient of ``func``.
        :param x0: Point at which to check the gradient.
        :param epsilon: Forward-difference step size.
        :returns: L2 norm of the gradient difference.
        :complexity: O(n) function and gradient evaluations
        """
    "#,
    module = "raygeo.sketch.constraints"
)]
#[pyfunction(name = "check_grad")]
#[pyo3(signature = (func, grad, x0, epsilon=1e-6))]
fn py_check_grad(
    py: Python<'_>,
    func: &Bound<'_, PyAny>,
    grad: &Bound<'_, PyAny>,
    x0: &Bound<'_, PyAny>,
    epsilon: f64,
) -> PyResult<f64> {
    let numpy = py.import("numpy")?;
    let point = numpy
        .call_method1("asarray", (x0,))?
        .call_method1("astype", ("float64",))?;
    let dimension: usize = point.getattr("ndim")?.extract()?;
    if dimension != 1 {
        return Err(PyValueError::new_err("x0 must be one-dimensional"));
    }

    let base: f64 = numpy
        .call_method1("asarray", (func.call1((point.clone(),))?,))?
        .call_method0("item")?
        .extract()?;
    let analytical: Vec<f64> = numpy
        .call_method1("asarray", (grad.call1((point.clone(),))?,))?
        .call_method1("astype", ("float64",))?
        .call_method0("tolist")?
        .extract()?;
    let count: usize = point.len()?;
    if analytical.len() != count {
        return Err(PyValueError::new_err(
            "gradient length does not match x0 length",
        ));
    }

    let mut squared_difference = 0.0;
    for (index, analytical_value) in analytical.iter().enumerate() {
        let shifted = point.call_method0("copy")?;
        let value: f64 = shifted.get_item(index)?.extract()?;
        shifted.set_item(index, value + epsilon)?;
        let shifted_value: f64 = numpy
            .call_method1("asarray", (func.call1((shifted,))?,))?
            .call_method0("item")?
            .extract()?;
        let numerical = (shifted_value - base) / epsilon;
        let delta = *analytical_value - numerical;
        squared_difference += delta * delta;
    }

    Ok(squared_difference.sqrt())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_check_grad, m)?)?;
    Ok(())
}
