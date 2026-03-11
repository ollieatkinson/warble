use anyhow::{anyhow, Context, Result};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::constants::*;
use crate::overlay::fallback_indicator_window_size;
use crate::state::{Settings, SharedState};
use crate::storage::normalize_shortcut;
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

pub(crate) fn create_tray_icon(app: &AppHandle) -> Result<()> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }

    let show_item = MenuItem::with_id(app, TRAY_SHOW_ID, "Open Warble", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, TRAY_HIDE_ID, "Hide Window", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, TRAY_QUIT_ID, "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &separator, &quit_item])?;
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
            TRAY_HIDE_ID => hide_main_window(app),
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
        let cancel_shortcut = normalize_shortcut(CANCEL_SHORTCUT);

        if hold_shortcut == toggle_shortcut {
            return Err(anyhow!("Hold and toggle shortcuts must be different"));
        }
        if hold_shortcut == cancel_shortcut || toggle_shortcut == cancel_shortcut {
            return Err(anyhow!("Escape is reserved for cancel"));
        }

        app.global_shortcut().unregister_all()?;
        app.global_shortcut()
            .register(settings.hold_shortcut.as_str())?;
        app.global_shortcut()
            .register(settings.toggle_shortcut.as_str())?;
        let escape_registered = app.global_shortcut().register(CANCEL_SHORTCUT).is_ok();

        let mut core = shared.lock();
        core.shortcuts_active = true;
        core.shortcut_message = if escape_registered {
            format!(
                "Listening for {}, {}, and Esc",
                settings.hold_shortcut, settings.toggle_shortcut
            )
        } else {
            format!(
                "Listening for {} and {}",
                settings.hold_shortcut, settings.toggle_shortcut
            )
        };
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
