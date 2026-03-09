use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    tauri_build::build();
    copy_directml_runtime();
}

fn copy_directml_runtime() {
    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }

    let Some(source) = find_directml_dll() else {
        return;
    };

    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let Some(target_dir) = PathBuf::from(out_dir).ancestors().nth(3).map(Path::to_path_buf) else {
        return;
    };

    for destination_dir in [
        target_dir.clone(),
        target_dir.join("deps"),
        target_dir.join("examples"),
    ] {
        if !destination_dir.exists() {
            continue;
        }

        let destination = destination_dir.join("DirectML.dll");
        if destination.is_symlink() {
            let _ = fs::remove_file(&destination);
        }

        if let Err(error) = fs::copy(&source, &destination) {
            println!(
                "cargo:warning=failed to copy DirectML.dll to {}: {}",
                destination.display(),
                error
            );
        }
    }
}

fn find_directml_dll() -> Option<PathBuf> {
    let local_app_data = env::var_os("LOCALAPPDATA")?;
    let target = env::var("TARGET").ok()?;
    let cache_root = PathBuf::from(local_app_data)
        .join("ort.pyke.io")
        .join("dfbin")
        .join(target);
    let entries = fs::read_dir(cache_root).ok()?;

    for entry in entries.flatten() {
        let candidate = entry.path().join("onnxruntime").join("lib").join("DirectML.dll");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}
