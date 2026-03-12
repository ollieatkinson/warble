use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use super::history::prune_history_audio;
use super::paths::diagnostics_dir;
use super::persistence::save_persisted_state;
use crate::constants::EVENT_SNAPSHOT;
use crate::models::{built_in_parakeet_status, current_model_status, installed_model_sizes};
use crate::overlay::update_indicator_window;
use crate::platform;
use crate::state::{SharedState, Snapshot};

pub(crate) fn live_preview_log_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(diagnostics_dir(app)?.join("live-preview.log"))
}

pub(crate) fn capture_log_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(diagnostics_dir(app)?.join("capture.log"))
}

pub(crate) fn append_live_preview_log(app: &AppHandle, line: &str) {
    let Ok(path) = live_preview_log_path(app) else {
        return;
    };

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };

    let _ = writeln!(file, "{line}");
}

pub(crate) fn append_capture_log(app: &AppHandle, line: &str) {
    let Ok(path) = capture_log_path(app) else {
        return;
    };

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };

    let _ = writeln!(file, "{line}");
}

fn read_log_tail(path: PathBuf, max_bytes: usize) -> String {
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };

    let Ok(file_len) = file.metadata().map(|metadata| metadata.len()) else {
        return String::new();
    };

    let start = file_len.saturating_sub(max_bytes as u64);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return String::new();
    }

    let mut bytes = Vec::with_capacity((file_len - start) as usize);
    if file.read_to_end(&mut bytes).is_err() {
        return String::new();
    }

    if start > 0 {
        if let Some(first_newline) = bytes.iter().position(|byte| *byte == b'\n') {
            bytes.drain(..=first_newline);
        }
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

pub(crate) fn read_live_preview_log(app: &AppHandle) -> String {
    let Ok(path) = live_preview_log_path(app) else {
        return String::new();
    };
    read_log_tail(path, 24 * 1024)
}

pub(crate) fn read_capture_log(app: &AppHandle) -> String {
    let Ok(path) = capture_log_path(app) else {
        return String::new();
    };
    read_log_tail(path, 24 * 1024)
}

pub(crate) fn build_snapshot(app: &AppHandle, shared: &SharedState) -> Snapshot {
    let core = shared.lock();
    let mut preview_diagnostics = core.preview_diagnostics.clone();
    let mut capture_diagnostics = core.capture_diagnostics.clone();
    preview_diagnostics.log_path = live_preview_log_path(app)
        .ok()
        .map(|path| path.to_string_lossy().into_owned());
    capture_diagnostics.log_path = capture_log_path(app)
        .ok()
        .map(|path| path.to_string_lossy().into_owned());
    Snapshot {
        phase: core.phase.clone(),
        platform: platform::current_platform(),
        auto_paste_support: platform::auto_paste_support(),
        settings: core.settings.clone(),
        sources: core.sources.clone(),
        history: core.history.clone(),
        model_status: core.model_status.clone(),
        parakeet_model_status: core.parakeet_model_status.clone(),
        installed_model_sizes: installed_model_sizes(app, &core.settings),
        model_downloads: core.model_downloads.clone(),
        system_profile: core.system_profile.clone(),
        shortcuts_active: core.shortcuts_active,
        shortcut_message: core.shortcut_message.clone(),
        status_message: core.status_message.clone(),
        error_message: core.error_message.clone(),
        preview_diagnostics,
        capture_diagnostics,
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
