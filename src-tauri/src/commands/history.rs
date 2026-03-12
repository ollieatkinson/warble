use tauri::AppHandle;

use crate::platform;
use crate::state::*;
use crate::storage::*;

pub(crate) fn paste_last_transcript(app: &AppHandle, shared: &SharedState) -> Result<(), String> {
    let Some(text) = ({
        let core = shared.lock();
        core.last_transcript_text.clone()
    }) else {
        let mut core = shared.lock();
        core.status_message = "No recent transcript to paste".to_string();
        core.error_message = None;
        emit_snapshot(app, shared);
        return Ok(());
    };

    let paste_result = platform::paste_text(app, &text);
    {
        let mut core = shared.lock();
        match paste_result.as_ref() {
            Ok(platform::PasteOutcome::ActiveApp) => {
                core.status_message = "Pasted the last transcript".to_string();
                core.error_message = None;
            }
            Ok(platform::PasteOutcome::ClipboardOnly) => {
                core.status_message = "Copied the last transcript to the clipboard".to_string();
                core.error_message = None;
            }
            Err(error) => {
                core.status_message = "Paste last transcript failed".to_string();
                core.error_message = Some(error.to_string());
            }
        }
    }

    emit_snapshot(app, shared);
    paste_result.map(|_| ()).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn paste_last_transcript_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    paste_last_transcript(&app, &shared)
}

#[tauri::command]
pub(crate) fn remove_history_item(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    id: String,
) -> Result<(), String> {
    {
        let mut core = shared.lock();
        let Some(index) = core.history.iter().position(|item| item.id == id) else {
            return Ok(());
        };

        let item = core.history.remove(index);
        remove_history_audio_file(&item);
        core.status_message = "History item removed".to_string();
        core.error_message = None;
    }

    save_persisted_state(&app, &shared).map_err(|error| error.to_string())?;
    emit_snapshot(&app, &shared);
    Ok(())
}

#[tauri::command]
pub(crate) fn clear_history(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    {
        let mut core = shared.lock();
        if core.history.is_empty() {
            return Ok(());
        }

        let items = std::mem::take(&mut core.history);
        for item in &items {
            remove_history_audio_file(item);
        }

        core.status_message = "History cleared".to_string();
        core.error_message = None;
    }

    save_persisted_state(&app, &shared).map_err(|error| error.to_string())?;
    emit_snapshot(&app, &shared);
    Ok(())
}
