use anyhow::Result;
use std::fs;
use tauri::AppHandle;

use super::paths::persisted_state_path;
use crate::constants::BACKGROUND_ARG;
use crate::state::{PersistedState, Settings, SharedState};

pub(crate) fn normalize_shortcut(shortcut: &str) -> String {
    shortcut.to_ascii_lowercase().replace(' ', "")
}

pub(crate) fn launched_in_background() -> bool {
    std::env::args().any(|arg| arg == BACKGROUND_ARG)
}

pub(crate) fn save_persisted_state(app: &AppHandle, shared: &SharedState) -> Result<()> {
    let persisted = {
        let core = shared.lock();
        PersistedState {
            settings: core.settings.clone(),
            history: core.history.clone(),
        }
    };

    let path = persisted_state_path(app)?;
    fs::write(path, serde_json::to_vec_pretty(&persisted)?)?;
    Ok(())
}

pub(crate) fn load_persisted_state(app: &AppHandle) -> PersistedState {
    let Some(path) = persisted_state_path(app).ok() else {
        return PersistedState {
            settings: Settings::default(),
            history: Vec::new(),
        };
    };

    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(parsed) = serde_json::from_str::<PersistedState>(&content) {
            return parsed;
        }
    }

    PersistedState {
        settings: Settings::default(),
        history: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_shortcut_lowercases_and_removes_spaces() {
        assert_eq!(normalize_shortcut("Ctrl + Shift + A"), "ctrl+shift+a");
        assert_eq!(normalize_shortcut("F8"), "f8");
        assert_eq!(normalize_shortcut(""), "");
    }

    #[test]
    fn normalize_shortcut_handles_mixed_case() {
        assert_eq!(normalize_shortcut("Alt + F4"), "alt+f4");
        assert_eq!(normalize_shortcut("CTRL+C"), "ctrl+c");
    }
}
