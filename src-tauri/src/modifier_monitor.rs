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
    use objc2_app_kit::{NSEvent, NSEventMask, NSEventType};
    use std::ptr::NonNull;

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
        let mask = NSEventMask::FlagsChanged.union(NSEventMask::KeyDown);

        let block = block2::RcBlock::new(move |event: NonNull<NSEvent>| {
            let event = unsafe { event.as_ref() };
            let event_type = event.r#type();
            let keycode = event.keyCode();

            // NSEventTypeKeyDown — non-modifier key pressed, cancel pending
            if event_type == NSEventType::KeyDown {
                let mut lock = pending_for_block.lock().unwrap();
                *lock = None;
                return;
            }

            // NSEventTypeFlagsChanged
            if event_type != NSEventType::FlagsChanged {
                return;
            }

            let shortcut_id = match keycode_to_shortcut_id(keycode) {
                Some(id) => id,
                None => return,
            };

            let modifier_flags = event.modifierFlags().bits();

            let flag_bit: usize = match keycode {
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
        });

        let _monitor =
            NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &block);

        // Keep the thread alive so the monitor stays registered.
        extern "C" {
            fn CFRunLoopRun();
        }
        unsafe {
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
