use tauri::AppHandle;

use crate::models::*;
use crate::state::*;

#[tauri::command]
pub(crate) fn inspect_model_path(path: String) -> Result<ModelPathInspection, String> {
    inspect_model_candidate(&path)
}

#[tauri::command]
pub(crate) fn install_catalog_model(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    model_id: String,
    model_kind: TranscriptionModelKind,
    path: String,
) -> Result<(), String> {
    let inspection = inspect_model_candidate(&path)?;
    if !inspection.ready || !inspection.compatible {
        return Err(match model_kind {
            TranscriptionModelKind::Parakeet => {
                "That folder is not a usable Parakeet TDT model.".to_string()
            }
            TranscriptionModelKind::ParakeetCtc => {
                "That folder is not a usable Parakeet CTC model.".to_string()
            }
        });
    }

    if inspection.model_kind != model_kind {
        return Err(match model_kind {
            TranscriptionModelKind::Parakeet => {
                "Choose a compatible Parakeet TDT folder for this catalog entry.".to_string()
            }
            TranscriptionModelKind::ParakeetCtc => {
                "Choose a compatible Parakeet CTC folder for this catalog entry.".to_string()
            }
        });
    }

    activate_catalog_model(&app, &shared, model_id, model_kind, inspection)
}

#[tauri::command]
pub(crate) fn download_catalog_model(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    model_id: String,
) -> Result<(), String> {
    crate::models::download_catalog_model(&app, &shared, model_id)
}

#[tauri::command]
pub(crate) fn remove_catalog_model(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    model_id: String,
) -> Result<(), String> {
    crate::models::remove_catalog_model(&app, &shared, model_id)
}
