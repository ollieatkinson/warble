use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CaretAnchor {
    pub x: i32,
    pub y: i32,
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

#[cfg(target_os = "windows")]
pub fn paste_text(text: &str) -> Result<()> {
    use arboard::Clipboard;
    use std::mem::size_of;
    use std::thread;
    use std::time::Duration;
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

    let mut clipboard = Clipboard::new()?;
    let previous = clipboard.get_text().ok();
    clipboard.set_text(text.to_owned())?;

    unsafe {
        let inputs = [
            key_input(VK_CONTROL.0, KEYBD_EVENT_FLAGS(0)),
            key_input(0x56, KEYBD_EVENT_FLAGS(0)),
            key_input(0x56, KEYEVENTF_KEYUP),
            key_input(VK_CONTROL.0, KEYEVENTF_KEYUP),
        ];
        let _ = SendInput(&inputs, size_of::<INPUT>() as i32);
    }

    if let Some(previous_text) = previous {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(250));
            if let Ok(mut clipboard) = Clipboard::new() {
                let _ = clipboard.set_text(previous_text);
            }
        });
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn paste_text(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text.to_owned())?;
    Ok(())
}
