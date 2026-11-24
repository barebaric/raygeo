import numpy as np
import pytest

from raygeo.sketch.constraints import check_grad


def test_matching_gradient_is_zero():
    def func(x):
        return float(np.sum(x * x))

    def grad(x):
        return 2.0 * x

    result = check_grad(func, grad, np.array([1.0, 2.0, -3.0]))

    assert result == pytest.approx(0.0, abs=1e-5)


def test_wrong_gradient_is_detected():
    def func(x):
        return float(np.sum(x * x))

    def grad(x):
        return np.zeros_like(x)

    result = check_grad(func, grad, np.array([1.0, -2.0]))

    assert result > 1.0


def test_accepts_python_sequences():
    def func(x):
        return x[0] ** 2 + 3.0 * x[1]

    def grad(x):
        return np.array([2.0 * x[0], 3.0])

    result = check_grad(func, grad, [2.0, 4.0])

    assert result == pytest.approx(0.0, abs=1e-5)


def test_rejects_mismatched_gradient_length():
    def func(x):
        return float(np.sum(x))

    def grad(x):
        return np.zeros(3)

    with pytest.raises(ValueError, match="length"):
        check_grad(func, grad, np.zeros(2))


def test_rejects_non_1d_input():
    with pytest.raises(ValueError, match="one-dimensional"):
        check_grad(lambda x: 0.0, lambda x: x, np.zeros((2, 2)))
