use anyhow::{anyhow, Context, Result};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, EventTarget, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::paste_last_transcript;
use crate::constants::*;
use crate::overlay::fallback_indicator_window_size;
use crate::recording;
use crate::state::{AppPhase, RecordingMode, Settings, SharedState};
use crate::storage::normalize_shortcut;
#[cfg(target_os = "macos")]
use tauri::menu::{MenuEvent, SubmenuBuilder};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub(crate) fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

pub(crate) fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub(crate) fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        show_main_window(app);
    }
}

fn emit_shell_action(app: &AppHandle, action: &str) {
    let _ = app.emit_to(
        EventTarget::webview_window("main"),
        EVENT_SHELL_ACTION,
        action,
    );

    if let Some(window) = app.get_webview_window("main") {
        if let (Ok(event_name), Ok(payload)) = (
            serde_json::to_string(EVENT_SHELL_ACTION),
            serde_json::to_string(action),
        ) {
            let _ = window.eval(format!(
                "window.dispatchEvent(new CustomEvent({event_name}, {{ detail: {payload} }}));"
            ));
        }
    }
}

fn show_main_window_and_emit(app: &AppHandle, action: &str) {
    show_main_window(app);
    emit_shell_action(app, action);
}

fn start_recording_from_tray(app: &AppHandle, mode: RecordingMode) {
    let shared = app.state::<SharedState>();
    let _ = recording::begin_recording(app, &shared, mode);
}

fn stop_or_cancel_from_tray(app: &AppHandle) {
    let shared = app.state::<SharedState>();
    let phase = {
        let core = shared.lock();
        core.phase.clone()
    };

    match phase {
        AppPhase::Recording => {
            let _ = recording::stop_recording(app, &shared);
        }
        AppPhase::Transcribing => {
            let _ = recording::cancel_current_operation(app, &shared);
        }
        _ => show_main_window(app),
    }
}

fn paste_last_from_tray(app: &AppHandle) {
    let shared = app.state::<SharedState>();
    let _ = paste_last_transcript(app, &shared);
}

fn format_shortcut_summary(shortcuts: &[String]) -> String {
    match shortcuts {
        [] => "No shortcuts active".to_string(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => format!(
            "{}, and {}",
            shortcuts[..shortcuts.len() - 1].join(", "),
            shortcuts[shortcuts.len() - 1]
        ),
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn build_app_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let about_item =
        MenuItem::with_id(app, MENU_APP_ABOUT_ID, "About Warble", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(
        app,
        MENU_APP_SETTINGS_ID,
        "Settings…",
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let quit_item = PredefinedMenuItem::quit(app, None)?;

    let transcribe_item = MenuItem::with_id(
        app,
        MENU_FILE_TRANSCRIBE_ID,
        "Transcribe Audio File…",
        true,
        Some("CmdOrCtrl+O"),
    )?;
    let paste_last_item = MenuItem::with_id(
        app,
        MENU_FILE_PASTE_LAST_ID,
        "Paste Last Transcript",
        true,
        None::<&str>,
    )?;
    let capture_item = MenuItem::with_id(
        app,
        MENU_VIEW_CAPTURE_ID,
        "Capture",
        true,
        Some("CmdOrCtrl+1"),
    )?;
    let models_item = MenuItem::with_id(
        app,
        MENU_VIEW_MODELS_ID,
        "Models",
        true,
        Some("CmdOrCtrl+2"),
    )?;
    let vocabulary_item = MenuItem::with_id(
        app,
        MENU_VIEW_VOCABULARY_ID,
        "Vocabulary",
        true,
        Some("CmdOrCtrl+3"),
    )?;
    let history_item = MenuItem::with_id(
        app,
        MENU_VIEW_HISTORY_ID,
        "History",
        true,
        Some("CmdOrCtrl+4"),
    )?;

    let troubleshooting_item = MenuItem::with_id(
        app,
        MENU_HELP_TROUBLESHOOTING_ID,
        "Troubleshooting…",
        true,
        None::<&str>,
    )?;
    let project_item = MenuItem::with_id(
        app,
        MENU_HELP_PROJECT_ID,
        "Project Page",
        true,
        None::<&str>,
    )?;

    let menu = Menu::new(app)?;

    let hide_item = PredefinedMenuItem::hide(app, None)?;
    let hide_others_item = PredefinedMenuItem::hide_others(app, None)?;
    let show_all_item = PredefinedMenuItem::show_all(app, None)?;

    let app_menu = SubmenuBuilder::new(app, "Warble")
        .item(&about_item)
        .item(&settings_item)
        .separator()
        .item(&hide_item)
        .item(&hide_others_item)
        .item(&show_all_item)
        .separator()
        .item(&quit_item)
        .build()?;
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&transcribe_item)
        .item(&paste_last_item)
        .build()?;
    let go_menu = SubmenuBuilder::new(app, "Go")
        .item(&capture_item)
        .item(&models_item)
        .item(&vocabulary_item)
        .item(&history_item)
        .build()?;
    let help_menu = SubmenuBuilder::new(app, "Help")
        .item(&troubleshooting_item)
        .separator()
        .item(&project_item)
        .build()?;

    menu.append(&app_menu)?;
    menu.append(&file_menu)?;
    menu.append(&go_menu)?;
    menu.append(&help_menu)?;

    Ok(menu)
}

#[cfg(target_os = "macos")]
pub(crate) fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_APP_ABOUT_ID => show_main_window_and_emit(app, SHELL_ACTION_OPEN_ABOUT),
        MENU_APP_SETTINGS_ID => show_main_window_and_emit(app, SHELL_ACTION_OPEN_SETTINGS),
        MENU_FILE_TRANSCRIBE_ID => {
            show_main_window_and_emit(app, SHELL_ACTION_TRANSCRIBE_FILE)
        }
        MENU_FILE_PASTE_LAST_ID => paste_last_from_tray(app),
        MENU_VIEW_CAPTURE_ID => show_main_window_and_emit(app, SHELL_ACTION_NAVIGATE_CAPTURE),
        MENU_VIEW_MODELS_ID => show_main_window_and_emit(app, SHELL_ACTION_NAVIGATE_MODELS),
        MENU_VIEW_VOCABULARY_ID => {
            show_main_window_and_emit(app, SHELL_ACTION_NAVIGATE_VOCABULARY)
        }
        MENU_VIEW_HISTORY_ID => show_main_window_and_emit(app, SHELL_ACTION_NAVIGATE_HISTORY),
        MENU_HELP_TROUBLESHOOTING_ID => {
            show_main_window_and_emit(app, SHELL_ACTION_OPEN_TROUBLESHOOTING)
        }
        MENU_HELP_PROJECT_ID => {
            let _ = tauri_plugin_opener::open_url(PROJECT_URL, None::<&str>);
        }
        _ => {}
    }
}

pub(crate) fn create_tray_icon(app: &AppHandle) -> Result<()> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }

    let show_item =
        MenuItem::with_id(app, TRAY_SHOW_ID, "Open Warble", true, None::<&str>)?;
    let settings_item =
        MenuItem::with_id(app, TRAY_SETTINGS_ID, "Settings…", true, None::<&str>)?;
    let paste_last_item = MenuItem::with_id(
        app,
        TRAY_PASTE_LAST_ID,
        "Paste Last Transcript",
        true,
        None::<&str>,
    )?;
    let hold_item = MenuItem::with_id(
        app,
        TRAY_RECORD_HOLD_ID,
        "Start Hold Dictation",
        true,
        None::<&str>,
    )?;
    let toggle_item = MenuItem::with_id(
        app,
        TRAY_RECORD_TOGGLE_ID,
        "Start Toggle Dictation",
        true,
        None::<&str>,
    )?;
    let stop_item = MenuItem::with_id(
        app,
        TRAY_STOP_OR_CANCEL_ID,
        "Stop / Cancel",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, TRAY_QUIT_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &settings_item,
            &paste_last_item,
            &PredefinedMenuItem::separator(app)?,
            &hold_item,
            &toggle_item,
            &stop_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;
    let icon = app
        .default_window_icon()
        .cloned()
        .context("missing default window icon")?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Warble")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW_ID => show_main_window(app),
            TRAY_SETTINGS_ID => show_main_window_and_emit(app, SHELL_ACTION_OPEN_SETTINGS),
            TRAY_PASTE_LAST_ID => paste_last_from_tray(app),
            TRAY_RECORD_HOLD_ID => start_recording_from_tray(app, RecordingMode::Hold),
            TRAY_RECORD_TOGGLE_ID => start_recording_from_tray(app, RecordingMode::Toggle),
            TRAY_STOP_OR_CANCEL_ID => stop_or_cancel_from_tray(app),
            TRAY_QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub(crate) fn register_shortcuts(app: &AppHandle, shared: &SharedState) -> Result<()> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let settings = {
            let core = shared.lock();
            core.settings.clone()
        };
        let hold_shortcut = normalize_shortcut(&settings.hold_shortcut);
        let toggle_shortcut = normalize_shortcut(&settings.toggle_shortcut);
        let paste_last_shortcut = normalize_shortcut(&settings.paste_last_shortcut);
        let cancel_shortcut = normalize_shortcut(CANCEL_SHORTCUT);

        if hold_shortcut == toggle_shortcut {
            return Err(anyhow!("Hold and toggle shortcuts must be different"));
        }
        if hold_shortcut == cancel_shortcut
            || toggle_shortcut == cancel_shortcut
            || (!paste_last_shortcut.is_empty() && paste_last_shortcut == cancel_shortcut)
        {
            return Err(anyhow!("Escape is reserved for cancel"));
        }
        if !paste_last_shortcut.is_empty()
            && (hold_shortcut == paste_last_shortcut
                || toggle_shortcut == paste_last_shortcut)
        {
            return Err(anyhow!(
                "Paste last transcript shortcut must be different from record shortcuts"
            ));
        }

        app.global_shortcut().unregister_all()?;

        let is_mod_only = crate::modifier_monitor::is_modifier_only_shortcut;

        if !is_mod_only(&settings.hold_shortcut) {
            app.global_shortcut()
                .register(settings.hold_shortcut.as_str())?;
        }
        if !is_mod_only(&settings.toggle_shortcut) {
            app.global_shortcut()
                .register(settings.toggle_shortcut.as_str())?;
        }
        if !paste_last_shortcut.is_empty() && !is_mod_only(&settings.paste_last_shortcut) {
            app.global_shortcut()
                .register(settings.paste_last_shortcut.as_str())?;
        }
        let escape_registered = app.global_shortcut().register(CANCEL_SHORTCUT).is_ok();

        let mut core = shared.lock();
        core.shortcuts_active = true;
        let mut shortcuts = vec![
            settings.hold_shortcut.clone(),
            settings.toggle_shortcut.clone(),
        ];
        if !paste_last_shortcut.is_empty() {
            shortcuts.push(settings.paste_last_shortcut.clone());
        }
        if escape_registered {
            shortcuts.push(CANCEL_SHORTCUT.to_string());
        }
        core.shortcut_message = format!("Listening for {}", format_shortcut_summary(&shortcuts));
        core.error_message = None;
    }

    Ok(())
}

pub(crate) fn create_indicator_window(app: &AppHandle) -> Result<()> {
    if app.get_webview_window("indicator").is_some() {
        return Ok(());
    }

    let (indicator_width, indicator_height) = fallback_indicator_window_size(&Settings::default());
    let builder = WebviewWindowBuilder::new(
        app,
        "indicator",
        WebviewUrl::App("index.html?indicator=1".into()),
    )
    .title("Warble Indicator")
    .transparent(true);
    let window = builder
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .focused(false)
        .inner_size(indicator_width as f64, indicator_height as f64)
        .build()?;

    let _ = window.set_focusable(true);
    let _ = window.set_ignore_cursor_events(false);
    let _ = window.set_shadow(false);

    Ok(())
}
