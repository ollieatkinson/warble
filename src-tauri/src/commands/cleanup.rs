use tauri::AppHandle;

use crate::state::*;
use crate::storage::*;
use crate::transcript::{
    default_cleanup_terms, normalize_cleanup_term, normalize_cleanup_terms,
};

#[tauri::command]
pub(crate) fn add_cleanup_term(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    term: String,
) -> Result<(), String> {
    let normalized = normalize_cleanup_term(&term)
        .ok_or_else(|| "Enter a filler word or phrase to remove.".to_string())?;

    {
        let mut core = shared.lock();
        if core
            .settings
            .cleanup_terms
            .iter()
            .any(|existing| existing == &normalized)
        {
            return Ok(());
        }
        core.settings.cleanup_terms.push(normalized);
        core.settings.cleanup_terms = normalize_cleanup_terms(&core.settings.cleanup_terms);
    }

    persist_and_emit_settings_change(&app, &shared)
}

#[tauri::command]
pub(crate) fn remove_cleanup_term(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    term: String,
) -> Result<(), String> {
    let Some(normalized) = normalize_cleanup_term(&term) else {
        return Ok(());
    };

    {
        let mut core = shared.lock();
        core.settings
            .cleanup_terms
            .retain(|existing| existing != &normalized);
    }

    persist_and_emit_settings_change(&app, &shared)
}

#[tauri::command]
pub(crate) fn restore_default_cleanup_terms(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    {
        let mut core = shared.lock();
        core.settings.cleanup_enabled = true;
        core.settings.cleanup_terms = default_cleanup_terms();
    }

    persist_and_emit_settings_change(&app, &shared)
}
