"""Visualise 1D root-finding methods: bisection, secant, Illinois."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.algo.rootfind import (
    bisect,
    bisect_tracked,
    illinois,
    illinois_tracked,
    secant,
    secant_tracked,
)


def _f_cubic(x):
    return x**3 - 2 * x - 5


def _f_quad(x):
    return x * x - 2


def generate_rootfind():
    """3-panel: bisection, secant, Illinois on x^3 - 2x - 5."""
    f = _f_cubic
    a, b = 2.0, 3.0
    true_root = 2.0945514815423265

    fig, axes = plt.subplots(1, 3, figsize=(15, 5))
    xs = np.linspace(a, b, 200)
    ys = f(xs)

    panels = [
        ("Bisection", bisect_tracked(f, a, b, 1e-8, 50), "o"),
        ("Secant", secant_tracked(f, a, b, 1e-8, 50), "s"),
        ("Illinois", illinois_tracked(f, a, b, 1e-8, 50), "^"),
    ]

    for ax, (name, (root, status, iters, estimates), marker) in zip(
        axes, panels
    ):
        ax.plot(xs, ys, "k-", linewidth=1.5, label="f(x)")
        ax.axhline(0, color="gray", linewidth=0.5)
        ax.axvline(
            true_root,
            color="green",
            linestyle="--",
            linewidth=1,
            alpha=0.7,
            label="Root",
        )
        ax.plot(root, f(root), "r*", markersize=14, zorder=5)

        # Show iteration estimates as markers on the curve.
        est_vals = [f(e) for e in estimates]
        ax.plot(
            estimates,
            est_vals,
            color="blue",
            marker=marker,
            linestyle="None",
            markersize=5,
            alpha=0.6,
            zorder=4,
        )

        ax.set_title(f"{name} ({iters} iters)")
        ax.set_xlabel("x")
        ax.set_ylabel("f(x)")
        ax.legend(fontsize=7)
        ax.grid(True, alpha=0.3)

    fig.suptitle("Root-Finding: $x^3 - 2x - 5 = 0$", fontsize=13)
    fig.tight_layout()
    return fig


def generate_convergence():
    """Error vs iteration for all three methods."""
    f = _f_cubic
    true_root = 2.0945514815423265

    fig, ax = plt.subplots(figsize=(8, 5))

    for name, solver_fn in [
        ("Bisection", lambda: bisect(f, 2.0, 3.0, 1e-12, 50)),
        ("Secant", lambda: secant(f, 2.0, 3.0, 1e-12, 50)),
        ("Illinois", lambda: illinois(f, 2.0, 3.0, 1e-12, 50)),
    ]:
        root, status, iters = solver_fn()
        errors = []
        x0, x1 = 2.0, 3.0
        for i in range(min(iters + 1, 50)):
            if name == "Bisection":
                r, _, _ = bisect(f, x0, x1, 1e-12, i + 1)
            elif name == "Secant":
                r, _, _ = secant(f, x0, x1, 1e-12, i + 1)
            else:
                r, _, _ = illinois(f, x0, x1, 1e-12, i + 1)
            errors.append(abs(r - true_root))
        ax.semilogy(
            range(1, len(errors) + 1),
            errors,
            label=name,
            linewidth=1.5,
            marker="o",
            markersize=3,
        )

    ax.set_xlabel("Iteration")
    ax.set_ylabel("Error")
    ax.set_title("Convergence Rate Comparison")
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3, which="both")
    fig.tight_layout()
    return fig


def generate_precision():
    """Iterations needed vs required precision for sqrt(2)."""
    f = _f_quad

    fig, ax = plt.subplots(figsize=(7, 4))

    for name, solver_fn in [
        ("Bisection", lambda tol: bisect(f, 0.0, 2.0, tol, 200)),
        ("Secant", lambda tol: secant(f, 1.0, 2.0, tol, 200)),
    ]:
        tols = [10 ** (-k) for k in range(1, 13)]
        iters_list = []
        for tol in tols:
            _, _, it = solver_fn(tol)
            iters_list.append(it)
        ax.semilogx(tols, iters_list, label=name, marker="o", markersize=3)

    ax.set_xlabel("Tolerance")
    ax.set_ylabel("Iterations")
    ax.set_title("Iterations vs Required Precision")
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.rootfind.md"]
__images__ = [
    {
        "heading": None,
        "caption": ("Bisection, secant, and Illinois on $x^3 - 2x - 5$."),
        "function": generate_rootfind,
    },
    {
        "heading": "bisect",
        "caption": (
            "Error vs iteration count: secant fastest, bisection slowest."
        ),
        "function": generate_convergence,
    },
    {
        "heading": "bisect",
        "caption": (
            "Iterations to reach a given tolerance for sqrt(2):"
            " secant needs far fewer than bisection."
        ),
        "function": generate_precision,
    },
]
