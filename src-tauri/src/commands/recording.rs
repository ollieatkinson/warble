use tauri::{AppHandle, Manager};

use crate::state::*;

#[tauri::command]
pub(crate) fn start_manual_recording(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    mode: RecordingMode,
) -> Result<(), String> {
    crate::recording::begin_recording(&app, &shared, mode).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn stop_manual_recording(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    crate::recording::stop_recording(&app, &shared).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn transcribe_media_file_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    path: String,
) -> Result<(), String> {
    let transcriber = app.state::<TranscriberHandle>().inner().clone();
    crate::transcription::transcribe_media_file(app, shared.inner().clone(), transcriber, path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn cancel_current_operation_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    crate::recording::cancel_current_operation(&app, &shared).map_err(|error| error.to_string())
}
