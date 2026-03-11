use tauri::AppHandle;

use crate::overlay::update_indicator_window;
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
pub(crate) fn refresh_devices(app: AppHandle, shared: tauri::State<'_, SharedState>) {
    crate::recording::refresh_sources(&app, &shared);
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
