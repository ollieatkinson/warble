use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::constants::{MANAGED_MODELS_DIR, RECORDINGS_DIR};

pub(crate) fn app_data_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data directory")?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(crate) fn persisted_state_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app_data_dir(app)?.join(crate::constants::PERSISTED_STATE_FILE))
}

pub(crate) fn model_root_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app_data_dir(app)?.join("models");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(crate) fn recordings_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app_data_dir(app)?.join(RECORDINGS_DIR);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(crate) fn managed_models_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = model_root_dir(app)?.join(MANAGED_MODELS_DIR);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(crate) fn diagnostics_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app_data_dir(app)?.join("logs");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(crate) fn managed_model_dir_for_id(app: &AppHandle, model_id: &str) -> Result<PathBuf> {
    Ok(managed_models_dir(app)?.join(model_id))
}

pub(crate) fn is_managed_model_path(app: &AppHandle, path: &Path) -> bool {
    let Ok(managed_root) = managed_models_dir(app) else {
        return false;
    };

    let managed_root = managed_root.canonicalize().unwrap_or(managed_root);
    let candidate = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    candidate.starts_with(&managed_root)
}

pub(crate) fn path_size_bytes(path: &Path) -> u64 {
    let Ok(metadata) = fs::metadata(path) else {
        return 0;
    };

    if metadata.is_file() {
        return metadata.len();
    }

    if !metadata.is_dir() {
        return 0;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| path_size_bytes(&entry.path()))
        .sum()
}

pub(crate) fn path_if_not_empty(value: Option<String>) -> Option<String> {
    value.and_then(|path| {
        let trimmed = path.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}
