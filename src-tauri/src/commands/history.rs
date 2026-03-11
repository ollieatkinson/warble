use tauri::AppHandle;

use crate::state::*;
use crate::storage::*;

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
