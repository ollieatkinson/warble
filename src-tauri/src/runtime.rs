#[cfg(target_os = "windows")]
use anyhow::Context;
use anyhow::{anyhow, Result};
#[cfg(target_os = "windows")]
use std::path::PathBuf;
use std::sync::OnceLock;

static ORT_INIT_RESULT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

pub(crate) fn ensure_ort_initialized() -> Result<()> {
    let result =
        ORT_INIT_RESULT.get_or_init(|| initialize_ort_runtime().map_err(|error| error.to_string()));
    result.clone().map_err(|error| anyhow!(error))
}

#[cfg(target_os = "windows")]
pub(crate) fn directml_runtime_available() -> bool {
    let Some(runtime_dir) = ort_runtime_dir() else {
        return false;
    };

    [
        "onnxruntime.dll",
        "DirectML.dll",
        "dxcompiler.dll",
        "dxil.dll",
    ]
    .iter()
    .all(|file_name| runtime_dir.join(file_name).exists())
}

#[cfg(target_os = "windows")]
fn initialize_ort_runtime() -> Result<()> {
    let dylib_path = ort_dylib_path()
        .ok_or_else(|| anyhow!("onnxruntime.dll was not found next to the application"))?;
    ort::init_from(&dylib_path)
        .with_context(|| {
            format!(
                "failed to prepare ONNX Runtime from {}",
                dylib_path.display()
            )
        })?
        .commit();
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn initialize_ort_runtime() -> Result<()> {
    ort::init().commit();
    Ok(())
}

#[cfg(target_os = "windows")]
fn ort_dylib_path() -> Option<PathBuf> {
    std::env::var_os("ORT_DYLIB_PATH")
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| {
            ort_runtime_dir()
                .map(|dir| dir.join("onnxruntime.dll"))
                .filter(|path| path.exists())
        })
}

#[cfg(target_os = "windows")]
fn ort_runtime_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
}
