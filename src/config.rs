use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub max_edit_distance: i32,
    pub enabled_by_default: bool,
    pub undo_timeout_seconds: u64,
    pub hotkey_toggle: String,
    pub auto_check_updates: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_edit_distance: 2,
            enabled_by_default: true,
            undo_timeout_seconds: 5,
            hotkey_toggle: "Ctrl+Shift+A".to_string(),
            auto_check_updates: true,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, confy::ConfyError> {
        match confy::load("autocorrect", Some("config")) {
            Ok(config) => Ok(config),
            Err(err) => {
                eprintln!("Failed to load config, using defaults: {err}");
                Ok(Self::default())
            }
        }
    }

    pub fn save(&self) -> Result<(), confy::ConfyError> {
        confy::store("autocorrect", Some("config"), self)
    }
}
