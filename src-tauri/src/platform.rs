use anyhow::{bail, Result};
use arboard::Clipboard;
use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CaretAnchor {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformKind {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutoPasteSupport {
    ActiveApp,
    ClipboardOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteOutcome {
    ActiveApp,
    ClipboardOnly,
}

pub fn current_platform() -> PlatformKind {
    match std::env::consts::OS {
        "windows" => PlatformKind::Windows,
        "macos" => PlatformKind::Macos,
        "linux" => PlatformKind::Linux,
        _ => PlatformKind::Linux,
    }
}

pub fn auto_paste_support() -> AutoPasteSupport {
    #[cfg(target_os = "windows")]
    {
        return AutoPasteSupport::ActiveApp;
    }

    #[cfg(target_os = "macos")]
    {
        return AutoPasteSupport::ActiveApp;
    }

    #[cfg(target_os = "linux")]
    {
        if x11_display_available() {
            return AutoPasteSupport::ActiveApp;
        }

        return AutoPasteSupport::ClipboardOnly;
    }

    #[allow(unreachable_code)]
    AutoPasteSupport::ClipboardOnly
}

#[cfg(target_os = "windows")]
pub fn detect_caret_anchor() -> Option<CaretAnchor> {
    use std::mem::size_of;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
    };

    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.0.is_null() {
            return None;
        }

        let mut process_id = 0u32;
        let thread_id = GetWindowThreadProcessId(foreground, Some(&mut process_id));
        if process_id == GetCurrentProcessId() {
            return None;
        }

        let mut gui_info = GUITHREADINFO {
            cbSize: size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };

        if GetGUIThreadInfo(thread_id, &mut gui_info).is_ok() && !gui_info.hwndCaret.0.is_null() {
            let mut top_left = POINT {
                x: gui_info.rcCaret.left,
                y: gui_info.rcCaret.top,
            };
            let mut bottom_right = POINT {
                x: gui_info.rcCaret.right,
                y: gui_info.rcCaret.bottom,
            };

            let _ = ClientToScreen(gui_info.hwndCaret, &mut top_left);
            let _ = ClientToScreen(gui_info.hwndCaret, &mut bottom_right);

            return Some(CaretAnchor {
                x: bottom_right.x + 16,
                y: bottom_right.y + 14,
            });
        }
    }

    None
}

#[cfg(not(target_os = "windows"))]
pub fn detect_caret_anchor() -> Option<CaretAnchor> {
    None
}

pub fn paste_text(_app: &AppHandle, text: &str) -> Result<PasteOutcome> {
    #[cfg(target_os = "windows")]
    {
        return paste_text_windows(text);
    }

    #[cfg(target_os = "macos")]
    {
        return paste_text_macos(_app, text);
    }

    #[cfg(target_os = "linux")]
    {
        return paste_text_linux(text);
    }

    #[allow(unreachable_code)]
    copy_to_clipboard(text).map(|()| PasteOutcome::ClipboardOnly)
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_owned())?;
    Ok(())
}

fn prepare_paste(text: &str) -> Result<Option<String>> {
    let mut clipboard = Clipboard::new()?;
    let previous = clipboard.get_text().ok();
    clipboard.set_text(text.to_owned())?;
    Ok(previous)
}

fn restore_clipboard(previous: Option<String>) {
    let Some(previous_text) = previous else {
        return;
    };

    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(previous_text);
    }
}

fn restore_clipboard_after_delay(previous: Option<String>) {
    let Some(previous_text) = previous else {
        return;
    };

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(250));
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(previous_text);
        }
    });
}

#[cfg(target_os = "windows")]
fn paste_text_windows(text: &str) -> Result<PasteOutcome> {
    use std::mem::size_of;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VIRTUAL_KEY, VK_CONTROL,
    };

    fn key_input(vk: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let previous = prepare_paste(text)?;

    unsafe {
        let inputs = [
            key_input(VK_CONTROL.0, KEYBD_EVENT_FLAGS(0)),
            key_input(0x56, KEYBD_EVENT_FLAGS(0)),
            key_input(0x56, KEYEVENTF_KEYUP),
            key_input(VK_CONTROL.0, KEYEVENTF_KEYUP),
        ];
        let inserted = SendInput(&inputs, size_of::<INPUT>() as i32);
        if inserted != inputs.len() as u32 {
            restore_clipboard(previous);
            bail!("Windows paste event injection failed");
        }
    }

    restore_clipboard_after_delay(previous);
    Ok(PasteOutcome::ActiveApp)
}

#[cfg(target_os = "macos")]
fn paste_text_macos(app: &AppHandle, text: &str) -> Result<PasteOutcome> {
    crate::permissions::ensure_post_event_access(app)?;

    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEYCODE_COMMAND: u16 = 55;
    const KEYCODE_V: u16 = 9;

    let previous = prepare_paste(text)?;
    let source = match CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
        Ok(source) => source,
        Err(()) => {
            restore_clipboard(previous.clone());
            bail!("failed to create macOS event source");
        }
    };

    let command_down = match CGEvent::new_keyboard_event(source.clone(), KEYCODE_COMMAND, true) {
        Ok(event) => event,
        Err(()) => {
            restore_clipboard(previous.clone());
            bail!("failed to prepare Command key event");
        }
    };
    command_down.post(CGEventTapLocation::HID);

    let v_down = match CGEvent::new_keyboard_event(source.clone(), KEYCODE_V, true) {
        Ok(event) => event,
        Err(()) => {
            restore_clipboard(previous.clone());
            bail!("failed to prepare V key event");
        }
    };
    v_down.set_flags(CGEventFlags::CGEventFlagCommand);
    v_down.post(CGEventTapLocation::HID);

    let v_up = match CGEvent::new_keyboard_event(source.clone(), KEYCODE_V, false) {
        Ok(event) => event,
        Err(()) => {
            restore_clipboard(previous.clone());
            bail!("failed to prepare V key release");
        }
    };
    v_up.set_flags(CGEventFlags::CGEventFlagCommand);
    v_up.post(CGEventTapLocation::HID);

    let command_up = match CGEvent::new_keyboard_event(source, KEYCODE_COMMAND, false) {
        Ok(event) => event,
        Err(()) => {
            restore_clipboard(previous.clone());
            bail!("failed to prepare Command key release");
        }
    };
    command_up.post(CGEventTapLocation::HID);

    restore_clipboard_after_delay(previous);
    Ok(PasteOutcome::ActiveApp)
}

#[cfg(target_os = "linux")]
fn paste_text_linux(text: &str) -> Result<PasteOutcome> {
    if !x11_display_available() {
        copy_to_clipboard(text)?;
        return Ok(PasteOutcome::ClipboardOnly);
    }

    let previous = prepare_paste(text)?;

    unsafe {
        use std::ffi::CString;
        use std::ptr;
        use x11::{xlib, xtest};

        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            copy_to_clipboard(text)?;
            return Ok(PasteOutcome::ClipboardOnly);
        }

        let control_keysym = CString::new("Control_L").expect("static keysym");
        let v_keysym = CString::new("v").expect("static keysym");

        let control_keycode =
            xlib::XKeysymToKeycode(display, xlib::XStringToKeysym(control_keysym.as_ptr()) as _);
        let v_keycode =
            xlib::XKeysymToKeycode(display, xlib::XStringToKeysym(v_keysym.as_ptr()) as _);

        if control_keycode == 0 || v_keycode == 0 {
            xlib::XCloseDisplay(display);
            restore_clipboard(previous);
            bail!("failed to resolve X11 paste keycodes");
        }

        if xtest::XTestFakeKeyEvent(display, control_keycode as u32, 1, 0) == 0
            || xtest::XTestFakeKeyEvent(display, v_keycode as u32, 1, 0) == 0
            || xtest::XTestFakeKeyEvent(display, v_keycode as u32, 0, 0) == 0
            || xtest::XTestFakeKeyEvent(display, control_keycode as u32, 0, 0) == 0
        {
            xlib::XCloseDisplay(display);
            restore_clipboard(previous);
            bail!("X11 paste event injection failed");
        }

        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
    }

    restore_clipboard_after_delay(previous);
    Ok(PasteOutcome::ActiveApp)
}

#[cfg(target_os = "linux")]
fn x11_display_available() -> bool {
    unsafe {
        use std::ptr;
        use x11::xlib;

        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return false;
        }

        xlib::XCloseDisplay(display);
    }

    true
}
