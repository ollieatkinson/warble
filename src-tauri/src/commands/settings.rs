use tauri::AppHandle;

use crate::models::*;
use crate::shell::register_shortcuts;
use crate::state::*;
use crate::storage::*;

#[tauri::command]
pub(crate) fn update_settings_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    update: SettingsUpdate,
) -> Result<(), String> {
    let previous_settings = {
        let core = shared.lock();
        core.settings.clone()
    };

    {
        let mut core = shared.lock();
        if core
            .settings
            .apply_update(&update)
        {
            core.overlay.detail.clear();
        }
        if matches!(core.phase, AppPhase::Recording | AppPhase::Transcribing) {
            core.overlay.limit_ms = selected_model_audio_limit_ms(&core.settings);
        }
    }

    if let Err(error) = register_shortcuts(&app, &shared) {
        {
            let mut core = shared.lock();
            core.settings = previous_settings.clone();
            core.shortcuts_active = false;
            core.shortcut_message = "Global shortcuts are unavailable".to_string();
            core.status_message = "Shortcut update failed".to_string();
            core.error_message = Some(error.to_string());
        }
        let _ = register_shortcuts(&app, &shared);
        return Err(error.to_string());
    }
    persist_and_emit_settings_change(&app, &shared)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn clear_error_message_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    {
        let mut core = shared.lock();
        core.error_message = None;
        if matches!(core.phase, AppPhase::Error) {
            core.phase = AppPhase::Idle;
            if core.status_message == "Transcription failed" {
                core.status_message = "Ready".to_string();
            }
        }
    }

    emit_snapshot(&app, &shared);
    Ok(())
}
