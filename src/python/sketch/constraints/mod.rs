//! Python bindings for the sketch constraints, mirroring the
//! `sketch::constraints` module tree: one leaf module per constraint.

pub(crate) mod angle;
pub(crate) mod aspect_ratio;
pub(crate) mod coincident;
pub(crate) mod collinear;
pub(crate) mod diameter;
pub(crate) mod distance;
pub(crate) mod drag;
pub(crate) mod equal_distance;
pub(crate) mod equal_length;
pub(crate) mod horizontal;
pub(crate) mod parallelogram;
pub(crate) mod perpendicular;
pub(crate) mod point_on_curve;
pub(crate) mod point_on_line;
pub(crate) mod radius;
pub(crate) mod rotational;
pub(crate) mod symmetry;
pub(crate) mod tangent;
pub(crate) mod vertical;

use pyo3::prelude::*;

/// Type of a leaf-module registration function.
type LeafRegister = fn(&Bound<'_, PyModule>) -> PyResult<()>;

pub(crate) const MODULE_DOC: &str = "\
Sketch constraint classes.

One leaf module per constraint type, mirroring the core
sketch::constraints hierarchy: raygeo.sketch.constraints.horizontal,
.distance, .perpendicular, and so on.  Constraint objects carry the
resolved point ids and dimension values of one constraint of a
sketch problem and are passed to raygeo.sketch.Solver.
";

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let constraints_mod = PyModule::new(py, "constraints")?;

    constraints_mod.setattr("__doc__", MODULE_DOC)?;

    let leaves: [(&str, LeafRegister); 19] = [
        ("angle", angle::register),
        ("aspect_ratio", aspect_ratio::register),
        ("coincident", coincident::register),
        ("collinear", collinear::register),
        ("diameter", diameter::register),
        ("distance", distance::register),
        ("drag", drag::register),
        ("equal_distance", equal_distance::register),
        ("equal_length", equal_length::register),
        ("horizontal", horizontal::register),
        ("parallelogram", parallelogram::register),
        ("perpendicular", perpendicular::register),
        ("point_on_curve", point_on_curve::register),
        ("point_on_line", point_on_line::register),
        ("radius", radius::register),
        ("rotational", rotational::register),
        ("symmetry", symmetry::register),
        ("tangent", tangent::register),
        ("vertical", vertical::register),
    ];
    for (name, register_leaf) in leaves {
        let leaf_mod = PyModule::new(py, name)?;
        register_leaf(&leaf_mod)?;
        constraints_mod.add_submodule(&leaf_mod)?;
    }

    m.add_submodule(&constraints_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints", &constraints_mod)?;

    Ok(())
}
