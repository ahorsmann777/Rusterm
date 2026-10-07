use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub background_opacity: f32,
    pub window_decorations: bool,
    pub rounded_corners: bool,
    pub font_size: f32,
    pub font_family: String,
    pub cursor_blink: bool,
    pub scrollback_lines: usize,
    pub shell: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            background_opacity: 0.7,
            window_decorations: false,
            rounded_corners: true,
            font_size: 16.0,
            font_family: "Hack Nerd Font Mono".to_string(),
            cursor_blink: true,
            scrollback_lines: 10000,
            shell: std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string()),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Path::new("rusterm.conf");
        if config_path.exists() {
            let content = fs::read_to_string(config_path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_else(|_| Config::default())
        } else {
            let default_config = Config::default();
            let toml = toml::to_string(&default_config).unwrap_or_default();
            fs::write(config_path, toml).ok();
            default_config
        }
    }
}
