use tauri::AppHandle;
use uuid::Uuid;

use crate::state::*;
use crate::storage::*;
use crate::transcript::normalize_replacement_rules;

#[tauri::command]
pub(crate) fn add_replacement_rule(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    variants: Vec<String>,
    replacement: String,
) -> Result<(), String> {
    if variants.iter().all(|variant| variant.trim().is_empty()) {
        return Err("Enter at least one spoken variant.".to_string());
    }
    if replacement.trim().is_empty() {
        return Err("Enter the text Warble should insert.".to_string());
    }

    let normalized = normalize_replacement_rules(&[ReplacementRule {
        id: Uuid::new_v4().to_string(),
        variants,
        replacement,
    }])
    .into_iter()
    .next()
    .ok_or_else(|| "Enter at least one spoken variant and a replacement.".to_string())?;

    {
        let mut core = shared.lock();
        if core.settings.replacement_rules.iter().any(|existing| {
            existing.variants == normalized.variants
                && existing.replacement == normalized.replacement
        }) {
            return Ok(());
        }

        core.settings.replacement_rules.push(normalized);
    }

    persist_and_emit_settings_change(&app, &shared)
}

#[tauri::command]
pub(crate) fn remove_replacement_rule(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    id: String,
) -> Result<(), String> {
    if id.trim().is_empty() {
        return Ok(());
    }

    {
        let mut core = shared.lock();
        core.settings
            .replacement_rules
            .retain(|rule| rule.id != id.trim());
    }

    persist_and_emit_settings_change(&app, &shared)
}
