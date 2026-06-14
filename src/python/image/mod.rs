use pyo3::prelude::*;

mod convert;
mod dither;
mod grayscale;
mod preprocess;
mod srgb;

pyo3_stub_gen::module_doc!("raygeo.image", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Image processing functions for laser cutting applications.

Provides sRGB/linear color space conversions, RGBA-to-grayscale/binary \
conversions with alpha unpremultiplication, grayscale normalization \
with auto-levels, and dithering algorithms (Floyd-Steinberg, Bayer, \
minimum run length) for converting grayscale images to binary output.
";

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let image_mod = PyModule::new(m.py(), "image")?;

    image_mod.setattr("__doc__", MODULE_DOC)?;
    image_mod.add(
        "__all__",
        vec![
            "srgb_to_linear",
            "linear_to_srgb",
            "compute_auto_levels",
            "normalize_grayscale",
            "apply_floyd_steinberg_dither",
            "apply_minimum_run_length",
            "apply_bayer_dither",
            "rgba_to_grayscale",
            "rgba_to_binary",
            "rgba_to_grayscale_inplace",
            "grayscale_to_binary",
            "get_component_areas",
            "filter_components",
        ],
    )?;

    convert::register(&image_mod)?;
    srgb::register(&image_mod)?;
    grayscale::register(&image_mod)?;
    dither::register(&image_mod)?;
    preprocess::register(&image_mod)?;

    m.add_submodule(&image_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image", &image_mod)?;

    Ok(())
}
