use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const WINDOWS_RUNTIME_FILES: &[&str] = &[
    "onnxruntime.dll",
    "DirectML.dll",
    "dxcompiler.dll",
    "dxil.dll",
];

fn main() {
    tauri_build::build();
    copy_windows_ort_runtime();
}

fn copy_windows_ort_runtime() {
    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }

    let Some(source_dir) = find_windows_ort_runtime_dir() else {
        println!("cargo:warning=failed to locate a Windows ONNX Runtime bundle to copy");
        return;
    };

    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let Some(target_dir) = PathBuf::from(out_dir)
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
    else {
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

        copy_runtime_files(&source_dir, &destination_dir);
    }
}

fn copy_runtime_files(source_dir: &Path, destination_dir: &Path) {
    for file_name in WINDOWS_RUNTIME_FILES {
        let source = source_dir.join(file_name);
        if !source.exists() {
            continue;
        }

        let destination = destination_dir.join(file_name);
        if destination.is_symlink() {
            let _ = fs::remove_file(&destination);
        }

        if let Err(error) = fs::copy(&source, &destination) {
            println!(
                "cargo:warning=failed to copy {} to {}: {}",
                file_name,
                destination.display(),
                error
            );
        }
    }
}

fn find_windows_ort_runtime_dir() -> Option<PathBuf> {
    env::var_os("WARBLE_ORT_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| runtime_dir_ready(path))
        .or_else(find_workspace_onnxruntime_node_dir)
}

fn find_workspace_onnxruntime_node_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?);
    let pnpm_dir = manifest_dir.parent()?.join("node_modules").join(".pnpm");
    let entries = fs::read_dir(pnpm_dir).ok()?;

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if !file_name.starts_with("onnxruntime-node@") {
            continue;
        }

        let candidate = entry
            .path()
            .join("node_modules")
            .join("onnxruntime-node")
            .join("bin")
            .join("napi-v6")
            .join("win32")
            .join("x64");
        if runtime_dir_ready(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn runtime_dir_ready(path: &Path) -> bool {
    WINDOWS_RUNTIME_FILES
        .iter()
        .all(|file_name| path.join(file_name).exists())
}
