use pyo3::prelude::*;

pub(crate) mod apply;
pub(crate) mod bidir_scan_offset;
pub(crate) mod clip;
pub(crate) mod lead_in_out;
pub(crate) mod link;
pub(crate) mod merge_lines;
pub(crate) mod multipass;
pub(crate) mod optimize;
pub(crate) mod overscan;
pub(crate) mod smooth;
pub(crate) mod tabs;

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let transform_mod = PyModule::new(ops_mod.py(), "transform")?;

    apply::register(&transform_mod)?;
    bidir_scan_offset::register(&transform_mod)?;
    clip::register(&transform_mod)?;
    lead_in_out::register(&transform_mod)?;
    link::register(&transform_mod)?;
    merge_lines::register(&transform_mod)?;
    multipass::register(&transform_mod)?;
    optimize::register(&transform_mod)?;
    overscan::register(&transform_mod)?;
    smooth::register(&transform_mod)?;
    tabs::register(&transform_mod)?;

    ops_mod.add_submodule(&transform_mod)?;

    let sys_modules = ops_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform", &transform_mod)?;

    Ok(())
}
