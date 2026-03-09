use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::process::Command;
use tauri::{AppHandle, Emitter, Manager};
#[cfg(target_os = "windows")]
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

use crate::constants::{
    BACKGROUND_ARG, DEFAULT_HOLD_SHORTCUT, DEFAULT_TOGGLE_SHORTCUT, EVENT_SNAPSHOT,
    LEGACY_HOLD_SHORTCUT, LEGACY_TOGGLE_SHORTCUT, MANAGED_MODELS_DIR, PERSISTED_STATE_FILE,
    RECORDINGS_DIR,
};
use crate::models::{built_in_parakeet_status, current_model_status, installed_model_sizes};
use crate::state::{
    AudioRetentionPolicy, HistoryItem, PersistedState, Settings, SharedState, Snapshot,
    SystemProfile,
};
use crate::update_indicator_window;

pub(crate) fn normalize_shortcut(shortcut: &str) -> String {
    shortcut.to_ascii_lowercase().replace(' ', "")
}

pub(crate) fn launched_in_background() -> bool {
    std::env::args().any(|arg| arg == BACKGROUND_ARG)
}

pub(crate) fn migrate_legacy_shortcuts(settings: &mut Settings) {
    if normalize_shortcut(&settings.hold_shortcut) == normalize_shortcut(LEGACY_HOLD_SHORTCUT)
        && normalize_shortcut(&settings.toggle_shortcut)
            == normalize_shortcut(LEGACY_TOGGLE_SHORTCUT)
    {
        settings.hold_shortcut = DEFAULT_HOLD_SHORTCUT.to_string();
        settings.toggle_shortcut = DEFAULT_TOGGLE_SHORTCUT.to_string();
    }
}

pub(crate) fn app_data_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data directory")?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(crate) fn persisted_state_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app_data_dir(app)?.join(PERSISTED_STATE_FILE))
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

pub(crate) fn detect_system_profile() -> SystemProfile {
    let logical_cores = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(4);

    #[cfg(target_os = "windows")]
    let (total_memory_bytes, gpu_name, gpu_memory_bytes, directml_available) = {
        let mut status = MEMORYSTATUSEX::default();
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        let total_memory_bytes = unsafe {
            if GlobalMemoryStatusEx(&mut status).is_ok() {
                status.ullTotalPhys
            } else {
                0
            }
        };
        let (gpu_name, gpu_memory_bytes) = detect_primary_gpu();
        let directml_available = gpu_name.is_some() && directml_runtime_available();
        (
            total_memory_bytes,
            gpu_name,
            gpu_memory_bytes,
            directml_available,
        )
    };

    #[cfg(not(target_os = "windows"))]
    let (total_memory_bytes, gpu_name, gpu_memory_bytes, directml_available) =
        (0, None, 0, false);

    SystemProfile {
        logical_cores,
        total_memory_bytes,
        gpu_name,
        gpu_memory_bytes,
        directml_available,
    }
}

#[cfg(target_os = "windows")]
fn directml_runtime_available() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("DirectML.dll")))
        .map(|path| path.exists())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn detect_primary_gpu() -> (Option<String>, u64) {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            "try { $gpu = Get-CimInstance Win32_VideoController | Sort-Object -Property AdapterRAM -Descending | Select-Object -First 1 Name,AdapterRAM; if ($gpu) { $gpu | ConvertTo-Json -Compress } } catch { '' }",
        ])
        .output();

    let Ok(output) = output else {
        return (None, 0);
    };

    if !output.status.success() {
        return (None, 0);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return (None, 0);
    }

    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return (None, 0);
    };

    let name = value
        .get("Name")
        .and_then(|field| field.as_str())
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .map(ToOwned::to_owned);
    let gpu_memory_bytes = value
        .get("AdapterRAM")
        .and_then(|field| match field {
            Value::Number(value) => value.as_u64(),
            Value::String(value) => value.parse::<u64>().ok(),
            _ => None,
        })
        .unwrap_or(0);

    (name, gpu_memory_bytes)
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

pub(crate) fn audio_retention_duration(policy: &AudioRetentionPolicy) -> ChronoDuration {
    match policy {
        AudioRetentionPolicy::OneDay => ChronoDuration::days(1),
        AudioRetentionPolicy::SevenDays => ChronoDuration::days(7),
        AudioRetentionPolicy::ThirtyDays => ChronoDuration::days(30),
    }
}

pub(crate) fn history_item_created_at(item: &HistoryItem) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&item.created_at)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

pub(crate) fn remove_history_audio_file(item: &HistoryItem) {
    let Some(path) = item.audio_path.as_ref() else {
        return;
    };

    let _ = fs::remove_file(path);
}

pub(crate) fn write_recording_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("failed to create wav file at {}", path.display()))?;

    for sample in samples {
        let scaled = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
        writer.write_sample(scaled)?;
    }

    writer.finalize()?;
    Ok(())
}

pub(crate) fn save_history_audio(
    app: &AppHandle,
    item_id: &str,
    samples: &[f32],
    sample_rate: u32,
) -> Result<String> {
    let path = recordings_dir(app)?.join(format!("{item_id}.wav"));
    write_recording_wav(&path, samples, sample_rate)?;
    Ok(path.display().to_string())
}

pub(crate) fn prune_history_audio(app: &AppHandle, shared: &SharedState) -> bool {
    let now = Utc::now();
    let mut removed_paths = Vec::new();
    let mut changed = false;

    {
        let mut core = shared.lock();
        let cutoff = now - audio_retention_duration(&core.settings.audio_retention_policy);

        for item in &mut core.history {
            let Some(path) = item.audio_path.as_ref() else {
                continue;
            };

            let should_expire = history_item_created_at(item)
                .map(|created_at| created_at < cutoff)
                .unwrap_or(false);
            let file_missing = !Path::new(path).exists();
            if should_expire || file_missing {
                if should_expire {
                    removed_paths.push(path.clone());
                }
                item.audio_path = None;
                changed = true;
            }
        }
    }

    for path in removed_paths {
        let _ = fs::remove_file(path);
    }

    if let Ok(directory) = recordings_dir(app) {
        let referenced_paths = {
            let core = shared.lock();
            core.history
                .iter()
                .filter_map(|item| item.audio_path.clone())
                .collect::<Vec<_>>()
        };

        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                let path_string = path.display().to_string();
                if path.extension().and_then(|value| value.to_str()) == Some("wav")
                    && !referenced_paths.iter().any(|existing| existing == &path_string)
                {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }

    changed
}

pub(crate) fn save_persisted_state(app: &AppHandle, shared: &SharedState) -> Result<()> {
    let persisted = {
        let core = shared.lock();
        PersistedState {
            settings: core.settings.clone(),
            history: core.history.clone(),
        }
    };

    let path = persisted_state_path(app)?;
    fs::write(path, serde_json::to_vec_pretty(&persisted)?)?;
    Ok(())
}

pub(crate) fn load_persisted_state(app: &AppHandle) -> PersistedState {
    let Some(path) = persisted_state_path(app).ok() else {
        return PersistedState {
            settings: Settings::default(),
            history: Vec::new(),
        };
    };

    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(parsed) = serde_json::from_str::<PersistedState>(&content) {
            let mut parsed = parsed;
            migrate_legacy_shortcuts(&mut parsed.settings);
            return parsed;
        }
    }

    PersistedState {
        settings: Settings::default(),
        history: Vec::new(),
    }
}

pub(crate) fn build_snapshot(app: &AppHandle, shared: &SharedState) -> Snapshot {
    let core = shared.lock();
    Snapshot {
        phase: core.phase.clone(),
        settings: core.settings.clone(),
        sources: core.sources.clone(),
        history: core.history.clone(),
        model_status: core.model_status.clone(),
        parakeet_model_status: core.parakeet_model_status.clone(),
        installed_model_sizes: installed_model_sizes(app, &core.settings),
        system_profile: core.system_profile.clone(),
        shortcuts_active: core.shortcuts_active,
        shortcut_message: core.shortcut_message.clone(),
        status_message: core.status_message.clone(),
        error_message: core.error_message.clone(),
        overlay: core.overlay.clone(),
    }
}

pub(crate) fn emit_snapshot(app: &AppHandle, shared: &SharedState) {
    let snapshot = build_snapshot(app, shared);
    let _ = app.emit(EVENT_SNAPSHOT, snapshot);
}

pub(crate) fn persist_and_emit_settings_change(
    app: &AppHandle,
    shared: &SharedState,
) -> Result<(), String> {
    let audio_changed = prune_history_audio(app, shared);
    {
        let mut core = shared.lock();
        core.model_status = current_model_status(app, &core.settings);
        core.parakeet_model_status = built_in_parakeet_status(app);
        if audio_changed && core.history.iter().all(|item| item.audio_path.is_none()) {
            core.status_message = "Expired audio clips were cleaned up".to_string();
        }
    }
    save_persisted_state(app, shared).map_err(|error| error.to_string())?;
    update_indicator_window(app, shared);
    emit_snapshot(app, shared);
    Ok(())
}
