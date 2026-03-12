use anyhow::{bail, Result};
use arboard::Clipboard;
use serde::Serialize;
#[cfg(target_os = "macos")]
use std::sync::mpsc;
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

#[cfg(target_os = "macos")]
pub fn supports_dynamic_island(app: &AppHandle) -> bool {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;

    run_on_main_thread_and_wait(app, move || {
        let Some(mtm) = MainThreadMarker::new() else {
            return false;
        };
        let Some(screen) = NSScreen::mainScreen(mtm) else {
            return false;
        };

        let safe_area = screen.safeAreaInsets();
        if safe_area.top > 0.0 {
            return true;
        }

        let left_area = screen.auxiliaryTopLeftArea();
        let right_area = screen.auxiliaryTopRightArea();
        left_area.size.width > 0.0 || right_area.size.width > 0.0
    })
    .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
pub fn supports_dynamic_island(_app: &AppHandle) -> bool {
    false
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

#[cfg(target_os = "macos")]
pub fn detect_caret_anchor() -> Option<CaretAnchor> {
    use std::ffi::c_void;
    use std::ptr;

    type AXUIElementRef = *mut c_void;
    type AXValueRef = *mut c_void;
    type AXError = i32;
    type CFTypeRef = *const c_void;
    type CFStringRef = *const c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CFRange {
        location: i64,
        length: i64,
    }

    const K_AX_ERROR_SUCCESS: AXError = 0;

    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementCopyParameterizedAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            parameter: CFTypeRef,
            result: *mut CFTypeRef,
        ) -> AXError;
        fn AXValueCreate(value_type: u32, value: *const c_void) -> AXValueRef;
        fn AXValueGetValue(value: AXValueRef, value_type: u32, value_out: *mut c_void) -> bool;
        fn CFRelease(cf: *const c_void);
    }

    #[allow(non_upper_case_globals)]
    const kAXValueTypeCGRect: u32 = 3;
    #[allow(non_upper_case_globals)]
    const kAXValueTypeCFRange: u32 = 4;

    macro_rules! cfstr {
        ($s:expr) => {{
            use std::sync::Once;
            static mut PTR: *const c_void = ptr::null();
            static INIT: Once = Once::new();
            #[allow(unused_unsafe)]
            INIT.call_once(|| unsafe {
                let cstr = std::ffi::CString::new($s).unwrap();
                extern "C" {
                    fn CFStringCreateWithCString(
                        alloc: *const c_void,
                        cstr: *const i8,
                        encoding: u32,
                    ) -> *const c_void;
                }
                PTR = CFStringCreateWithCString(ptr::null(), cstr.as_ptr(), 0x08000100);
            });
            #[allow(unused_unsafe)]
            unsafe { PTR }
        }};
    }

    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }

        let mut focused_element: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(
            system_wide,
            cfstr!("AXFocusedUIElement"),
            &mut focused_element,
        );
        CFRelease(system_wide as *const c_void);
        if err != K_AX_ERROR_SUCCESS || focused_element.is_null() {
            return None;
        }

        let mut range_value: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(
            focused_element as AXUIElementRef,
            cfstr!("AXSelectedTextRange"),
            &mut range_value,
        );
        if err != K_AX_ERROR_SUCCESS || range_value.is_null() {
            CFRelease(focused_element);
            return None;
        }

        let mut range = CFRange { location: 0, length: 0 };
        let ok = AXValueGetValue(range_value as AXValueRef, kAXValueTypeCFRange, &mut range as *mut CFRange as *mut c_void);
        CFRelease(range_value);
        if !ok {
            CFRelease(focused_element);
            return None;
        }

        let range_param = AXValueCreate(kAXValueTypeCFRange, &range as *const CFRange as *const c_void);
        if range_param.is_null() {
            CFRelease(focused_element);
            return None;
        }

        let mut bounds_value: CFTypeRef = ptr::null();
        let err = AXUIElementCopyParameterizedAttributeValue(
            focused_element as AXUIElementRef,
            cfstr!("AXBoundsForRange"),
            range_param as CFTypeRef,
            &mut bounds_value,
        );
        CFRelease(range_param as *const c_void);
        CFRelease(focused_element);

        if err != K_AX_ERROR_SUCCESS || bounds_value.is_null() {
            return None;
        }

        #[repr(C)]
        #[derive(Default)]
        struct CGRect {
            x: f64,
            y: f64,
            w: f64,
            h: f64,
        }

        let mut rect = CGRect::default();
        let ok = AXValueGetValue(bounds_value as AXValueRef, kAXValueTypeCGRect, &mut rect as *mut CGRect as *mut c_void);
        CFRelease(bounds_value);
        if !ok {
            return None;
        }

        Some(CaretAnchor {
            x: rect.x as i32 + 16,
            y: (rect.y + rect.h) as i32 + 14,
        })
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn detect_caret_anchor() -> Option<CaretAnchor> {
    None
}

#[cfg(target_os = "macos")]
fn run_on_main_thread_and_wait<T, F>(app: &AppHandle, task: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    if unsafe { libc::pthread_main_np() == 1 } {
        return Ok(task());
    }

    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(task());
    })?;

    receiver.recv().map_err(Into::into)
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
