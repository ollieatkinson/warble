use crate::commands::paste_last_transcript;
use crate::recording;
use crate::state::{AppPhase, RecordingMode, SharedState};
use crate::storage::normalize_shortcut;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

/// Known modifier-only shortcut identifiers sent from the frontend.
const MODIFIER_SHORTCUTS: &[&str] = &[
    "RightMeta",
    "LeftMeta",
    "RightControl",
    "LeftControl",
    "RightShift",
    "LeftShift",
    "RightAlt",
    "LeftAlt",
];

pub(crate) fn is_modifier_only_shortcut(shortcut: &str) -> bool {
    MODIFIER_SHORTCUTS.contains(&shortcut)
}

#[cfg(target_os = "macos")]
pub(crate) fn start_modifier_monitor(app: AppHandle, shared: SharedState) {
    use std::ffi::c_void;

    // Raw objc_msgSend with correct C-variadic signature
    extern "C" {
        fn objc_msgSend(receiver: *mut c_void, sel: *mut c_void, ...) -> u64;
        fn objc_getClass(name: *const i8) -> *mut c_void;
        fn sel_registerName(name: *const i8) -> *mut c_void;
        fn CFRunLoopRun();
    }

    fn keycode_to_shortcut_id(keycode: u16) -> Option<&'static str> {
        match keycode {
            54 => Some("RightMeta"),
            55 => Some("LeftMeta"),
            60 => Some("RightShift"),
            56 => Some("LeftShift"),
            61 => Some("RightAlt"),
            58 => Some("LeftAlt"),
            62 => Some("RightControl"),
            59 => Some("LeftControl"),
            _ => None,
        }
    }

    let pending: Arc<Mutex<Option<&'static str>>> = Arc::new(Mutex::new(None));
    let pending_for_block = pending.clone();
    let app_for_block = app.clone();
    let shared_for_block = shared.clone();

    std::thread::spawn(move || {
        // NSEventMaskFlagsChanged = 1 << 12, NSEventMaskKeyDown = 1 << 10
        let combined_mask: u64 = (1u64 << 12) | (1u64 << 10);

        let block = block2::RcBlock::new(move |event: *mut c_void| {
            if event.is_null() {
                return;
            }

            unsafe {
                let type_sel = sel_registerName(b"type\0".as_ptr() as *const i8);
                let event_type: u64 = objc_msgSend(event, type_sel);

                let keycode_sel = sel_registerName(b"keyCode\0".as_ptr() as *const i8);
                let keycode: u64 = objc_msgSend(event, keycode_sel);
                let keycode = keycode as u16;

                // NSEventTypeKeyDown = 10 — non-modifier key pressed, cancel pending
                if event_type == 10 {
                    let mut lock = pending_for_block.lock().unwrap();
                    *lock = None;
                    return;
                }

                // NSEventTypeFlagsChanged = 12
                if event_type != 12 {
                    return;
                }

                let shortcut_id = match keycode_to_shortcut_id(keycode) {
                    Some(id) => id,
                    None => return,
                };

                let flags_sel = sel_registerName(b"modifierFlags\0".as_ptr() as *const i8);
                let modifier_flags: u64 = objc_msgSend(event, flags_sel);

                let flag_bit: u64 = match keycode {
                    54 | 55 => 1 << 20, // Command
                    60 | 56 => 1 << 17, // Shift
                    61 | 58 => 1 << 19, // Option
                    62 | 59 => 1 << 18, // Control
                    _ => return,
                };

                let is_pressed = (modifier_flags & flag_bit) != 0;

                let mut lock = pending_for_block.lock().unwrap();
                if is_pressed {
                    *lock = Some(shortcut_id);
                } else if *lock == Some(shortcut_id) {
                    *lock = None;
                    drop(lock);
                    fire_modifier_shortcut(&app_for_block, &shared_for_block, shortcut_id);
                } else {
                    *lock = None;
                }
            }
        });

        unsafe {
            let ns_event_class = objc_getClass(b"NSEvent\0".as_ptr() as *const i8);
            if ns_event_class.is_null() {
                return;
            }

            let sel = sel_registerName(
                b"addGlobalMonitorForEventsMatchingMask:handler:\0".as_ptr() as *const i8,
            );

            let _monitor: u64 = objc_msgSend(
                ns_event_class,
                sel,
                combined_mask,
                &*block as *const _ as *const c_void,
            );

            CFRunLoopRun();
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn start_modifier_monitor(_app: AppHandle, _shared: SharedState) {}

fn fire_modifier_shortcut(app: &AppHandle, shared: &SharedState, shortcut_id: &str) {
    let (hold, toggle, paste_last) = {
        let core = shared.lock();
        (
            normalize_shortcut(&core.settings.hold_shortcut),
            normalize_shortcut(&core.settings.toggle_shortcut),
            normalize_shortcut(&core.settings.paste_last_shortcut),
        )
    };

    let normalized = normalize_shortcut(shortcut_id);

    if normalized == hold || normalized == toggle {
        let phase = {
            let core = shared.lock();
            core.phase.clone()
        };
        if matches!(phase, AppPhase::Recording) {
            let _ = recording::stop_recording(app, shared);
        } else {
            let _ = recording::begin_recording(app, shared, RecordingMode::Toggle);
        }
        return;
    }

    if normalized == paste_last {
        let _ = paste_last_transcript(app, shared);
    }
}
