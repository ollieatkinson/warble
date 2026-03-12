mod audio;
mod commands;
mod constants;
mod inference;
mod live_preview;
mod media;
mod model_catalog;
mod models;
mod overlay;
pub mod parakeet;
mod permissions;
mod platform;
mod recording;
mod runtime;
mod shell;
mod state;
mod storage;
mod streaming_preview;
mod system;
mod transcript;
mod transcription;

use audio::enumerate_sources;
use commands::*;
use constants::*;
use models::{built_in_parakeet_status, current_model_status};
#[cfg(target_os = "macos")]
use shell::{build_app_menu, handle_menu_event};
use shell::{
    create_indicator_window, create_tray_icon, hide_main_window, register_shortcuts,
    show_main_window,
};
use state::*;
use storage::*;
use tauri::WindowEvent;
use tauri_plugin_autostart::ManagerExt as AutostartExt;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let recorder = recording::spawn_recorder_thread();
    let transcriber = TranscriberHandle::default();
    let preview_control = PreviewControl::default();
    let shared = SharedState::new({
        let placeholder = PersistedState {
            settings: Settings::default(),
            history: Vec::new(),
        };
        AppCore::new(placeholder.settings, placeholder.history)
    });

    let builder = tauri::Builder::default()
        .manage(shared.clone())
        .manage(recorder)
        .manage(transcriber)
        .manage(preview_control);

    #[cfg(target_os = "macos")]
    let builder = builder
        .menu(build_app_menu)
        .on_menu_event(handle_menu_event);

    #[cfg(not(target_os = "macos"))]
    let builder = builder;

    builder
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![BACKGROUND_ARG]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            let runtime_error = runtime::ensure_ort_initialized().err();
            let persisted = load_persisted_state(app.handle());
            {
                let mut core = shared.lock();
                core.settings = persisted.settings;
                core.history = persisted.history;
                core.sources = enumerate_sources();
                core.model_status = current_model_status(app.handle(), &core.settings);
                core.parakeet_model_status = built_in_parakeet_status(app.handle());
                if let Some(error) = runtime_error.as_ref() {
                    core.error_message = Some(format!("Couldn't initialize ONNX Runtime: {error}"));
                    core.status_message = "ONNX Runtime needs attention".to_string();
                }
            }
            if prune_history_audio(app.handle(), &shared) {
                let _ = save_persisted_state(app.handle(), &shared);
            }

            create_tray_icon(app.handle())?;
            create_indicator_window(app.handle())?;

            if let Err(error) = app.autolaunch().enable() {
                let mut core = shared.lock();
                core.error_message = Some(format!("Couldn't enable launch at login: {error}"));
            }

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                let state_for_shortcuts = shared.clone();
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, shortcut, event| {
                            let shortcut_text = shortcut.to_string();
                            let (hold_shortcut, toggle_shortcut) = {
                                let core = state_for_shortcuts.lock();
                                (
                                    normalize_shortcut(&core.settings.hold_shortcut),
                                    normalize_shortcut(&core.settings.toggle_shortcut),
                                )
                            };
                            let shortcut_text = normalize_shortcut(&shortcut_text);
                            let cancel_shortcut = normalize_shortcut(CANCEL_SHORTCUT);

                            if shortcut_text == cancel_shortcut
                                && matches!(event.state, ShortcutState::Pressed)
                            {
                                let _ =
                                    recording::cancel_current_operation(app, &state_for_shortcuts);
                                return;
                            }

                            if shortcut_text == hold_shortcut {
                                match event.state {
                                    ShortcutState::Pressed => {
                                        let _ = recording::begin_recording(
                                            app,
                                            &state_for_shortcuts,
                                            RecordingMode::Hold,
                                        );
                                    }
                                    ShortcutState::Released => {
                                        let phase = {
                                            let core = state_for_shortcuts.lock();
                                            core.phase.clone()
                                        };
                                        if matches!(phase, AppPhase::Recording) {
                                            let _ = recording::stop_recording(
                                                app,
                                                &state_for_shortcuts,
                                            );
                                        }
                                    }
                                }
                                return;
                            }

                            if shortcut_text == toggle_shortcut
                                && matches!(event.state, ShortcutState::Pressed)
                            {
                                let phase = {
                                    let core = state_for_shortcuts.lock();
                                    core.phase.clone()
                                };

                                if matches!(phase, AppPhase::Recording) {
                                    let _ = recording::stop_recording(app, &state_for_shortcuts);
                                } else {
                                    let _ = recording::begin_recording(
                                        app,
                                        &state_for_shortcuts,
                                        RecordingMode::Toggle,
                                    );
                                }
                            }
                        })
                        .build(),
                )?;

                if let Err(error) = register_shortcuts(app.handle(), &shared) {
                    {
                        let mut core = shared.lock();
                        core.settings = Settings::default();
                        core.shortcuts_active = false;
                        core.shortcut_message = "Falling back to default shortcuts".to_string();
                        core.status_message = format!("Shortcut defaults were restored: {error}");
                    }
                    if let Err(retry_error) = register_shortcuts(app.handle(), &shared) {
                        let mut core = shared.lock();
                        core.shortcuts_active = false;
                        core.shortcut_message = "Global shortcuts are unavailable".to_string();
                        core.status_message = "Running without global shortcuts".to_string();
                        core.error_message = Some(retry_error.to_string());
                    } else {
                        let _ = save_persisted_state(app.handle(), &shared);
                    }
                }
            }

            if launched_in_background() {
                hide_main_window(app.handle());
            } else {
                show_main_window(app.handle());
            }

            emit_snapshot(app.handle(), &shared);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            refresh_devices,
            prime_microphone_access,
            prime_auto_paste_access,
            get_debug_logs_command,
            inspect_model_path,
            install_catalog_model,
            download_catalog_model,
            remove_catalog_model,
            update_settings_command,
            add_cleanup_term,
            remove_cleanup_term,
            restore_default_cleanup_terms,
            add_replacement_rule,
            remove_replacement_rule,
            clear_error_message_command,
            remove_history_item,
            clear_history,
            report_indicator_layout_command,
            start_manual_recording,
            stop_manual_recording,
            transcribe_media_file_command,
            cancel_current_operation_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
