use tauri::AppHandle;

use crate::overlay::update_indicator_window;
use crate::permissions::MicrophoneAccess;
use crate::state::*;
use crate::storage::*;

#[tauri::command]
pub(crate) fn get_snapshot(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<Snapshot, String> {
    crate::recording::refresh_sources(&app, &shared);
    Ok(build_snapshot(&app, &shared))
}

#[tauri::command]
pub(crate) fn refresh_devices(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    crate::recording::refresh_sources_after_permission_check(&app, &shared)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn prime_microphone_access(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    let access = crate::permissions::ensure_microphone_access(&app)
        .map_err(|error| error.to_string())?;

    if matches!(access, MicrophoneAccess::Authorized) {
        crate::recording::refresh_sources(&app, &shared);
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn prime_auto_paste_access(app: AppHandle) -> Result<(), String> {
    crate::permissions::request_post_event_access(&app)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_debug_logs_command(app: AppHandle) -> DebugLogs {
    DebugLogs {
        capture: read_capture_log(&app),
        live_preview: read_live_preview_log(&app),
    }
}

#[tauri::command]
pub(crate) fn report_indicator_layout_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let width = width.clamp(1, 4096) as i32;
    let height = height.clamp(1, 2048) as i32;
    let shared = shared.inner();

    let mut should_update = false;
    {
        let mut core = shared.lock();
        let next = (width, height);
        if core.indicator_window_size != Some(next) {
            core.indicator_window_size = Some(next);
            should_update = true;
        }
    }

    if should_update {
        update_indicator_window(&app, shared);
    }

    Ok(())
}
