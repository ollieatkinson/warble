use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const WINDOWS_RUNTIME_BUNDLE_DIR: &str = "target/windows-runtime-bundle";
const WINDOWS_RUNTIME_FILES: &[&str] = &[
    "onnxruntime.dll",
    "DirectML.dll",
    "dxcompiler.dll",
    "dxil.dll",
];

const MACOS_DAWN_BUNDLE_DIR: &str = "target/macos-dawn-bundle";
const MACOS_DAWN_DYLIB: &str = "libwebgpu_dawn.dylib";

fn main() {
    prepare_windows_ort_runtime_bundle_dir();
    stage_macos_dawn_dylib();
    tauri_build::build();
    copy_windows_ort_runtime();
}

fn prepare_windows_ort_runtime_bundle_dir() {
    let Some(bundle_dir) = windows_ort_runtime_bundle_dir() else {
        return;
    };

    if let Err(error) = fs::create_dir_all(&bundle_dir) {
        println!(
            "cargo:warning=failed to create {}: {}",
            bundle_dir.display(),
            error
        );
        return;
    }

    clear_runtime_files(&bundle_dir);
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

    let mut destination_dirs = Vec::new();
    if let Some(bundle_dir) = windows_ort_runtime_bundle_dir() {
        destination_dirs.push(bundle_dir);
    }
    destination_dirs.push(target_dir.clone());
    destination_dirs.push(target_dir.join("deps"));
    destination_dirs.push(target_dir.join("examples"));

    for destination_dir in destination_dirs {
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

fn clear_runtime_files(destination_dir: &Path) {
    for file_name in WINDOWS_RUNTIME_FILES {
        let destination = destination_dir.join(file_name);
        if destination.exists() || destination.is_symlink() {
            let _ = fs::remove_file(destination);
        }
    }
}

fn find_windows_ort_runtime_dir() -> Option<PathBuf> {
    env::var_os("WARBLE_ORT_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| runtime_dir_ready(path))
        .or_else(find_workspace_onnxruntime_node_dir)
}

fn windows_ort_runtime_bundle_dir() -> Option<PathBuf> {
    Some(PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?).join(WINDOWS_RUNTIME_BUNDLE_DIR))
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

fn stage_macos_dawn_dylib() {
    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("macos") {
        return;
    }

    let bundle_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join(MACOS_DAWN_BUNDLE_DIR);
    let _ = fs::create_dir_all(&bundle_dir);

    // Tell the linker to add @executable_path/../Frameworks as an rpath so
    // the app bundle can find libwebgpu_dawn.dylib at runtime.
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");

    if let Some(source) = find_macos_dawn_dylib() {
        let destination = bundle_dir.join(MACOS_DAWN_DYLIB);
        if let Err(error) = fs::copy(&source, &destination) {
            println!(
                "cargo:warning={} copy failed: {}",
                MACOS_DAWN_DYLIB, error
            );
        }
    } else {
        println!(
            "cargo:warning={} not found in build artifacts — WebGPU will not be available at runtime",
            MACOS_DAWN_DYLIB
        );
    }
}

fn find_macos_dawn_dylib() -> Option<PathBuf> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR")?);
    // Walk up from OUT_DIR to the target profile dir (e.g. target/debug)
    let target_dir = out_dir.ancestors().nth(3)?;
    let build_dir = target_dir.join("build");
    let entries = fs::read_dir(&build_dir).ok()?;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("ort-sys-") {
            continue;
        }

        // Search common output locations within the ort-sys build dir
        for sub in &["out/lib", "out", "lib"] {
            let candidate = entry.path().join(sub).join(MACOS_DAWN_DYLIB);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

