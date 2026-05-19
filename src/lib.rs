use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3_stub_gen::{StubInfo, StubGenConfig};

mod geo;
mod ops;

#[pymodule(gil_used = false)]
fn raygeo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    geo::register(m)?;
    ops::register(m)?;
    m.add_function(wrap_pyfunction!(generate_stubs, m)?)?;
    Ok(())
}

#[pyfunction]
fn generate_stubs(path: &str) -> PyResult<()> {
    let stub_info = StubInfo::from_project_root("raygeo".to_string(), path.into(), false, StubGenConfig::default())
        .map_err(|e| PyRuntimeError::new_err(format!("StubInfo failed: {}", e)))?;
    for module in stub_info.modules.values() {
        let content = module.format_with_config(false);
        let parts: Vec<&str> = module.name.split('.').collect();
        if parts.len() == 1 {
            // Root module: raygeo -> __init__.pyi
            std::fs::write(format!("{}/__init__.pyi", path), content)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        } else {
            // Strip "raygeo." prefix to get relative path components
            let rel_parts = &parts[1..];
            let has_submodules = stub_info.modules.values().any(|m| {
                m.name != module.name && m.name.starts_with(&format!("{}.", module.name))
            });
            if has_submodules {
                // Intermediate module -> directory with __init__.pyi
                let subdir = format!("{}/{}", path, rel_parts.join("/"));
                std::fs::create_dir_all(&subdir)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
                std::fs::write(format!("{}/__init__.pyi", subdir), content)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            } else {
                // Leaf module -> .pyi file in parent directory
                let parent = &rel_parts[..rel_parts.len() - 1];
                let filename = rel_parts.last().unwrap();
                let subdir = if parent.is_empty() {
                    path.to_string()
                } else {
                    format!("{}/{}", path, parent.join("/"))
                };
                std::fs::create_dir_all(&subdir)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
                std::fs::write(format!("{}/{}.pyi", subdir, filename), content)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            }
        }
    }
    Ok(())
}
