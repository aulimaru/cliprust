use crate::Cli;
use clap::ValueEnum;
use dirs;
use serde::Deserialize;
use serde::Serialize;
use std::path::{Path, PathBuf};
use toml;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Deserialize, Serialize)]
pub enum ThumbMode {
    Wofi,
    Rofi,
    None,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub db_dir_path: PathBuf,
    pub max_dedupe_depth: usize,
    pub max_items: usize,
    pub preview_width: usize,
    pub generate_thumb: ThumbMode,
}

impl Config {
    pub fn cli_override(&mut self, cli: &Cli) {
        if let Some(db_path) = &cli.db_path {
            self.db_dir_path = db_path.clone();
        }
        if let Some(max_dedupe_depth) = cli.max_dedupe_depth {
            self.max_dedupe_depth = max_dedupe_depth;
        }
        if let Some(max_items) = cli.max_items {
            self.max_items = max_items;
        }
        if let Some(preview_width) = cli.max_preview_width {
            self.preview_width = preview_width;
        }
        if let Some(generate_thumb) = &cli.generate_thumb {
            self.generate_thumb = generate_thumb.clone();
        }
    }
    pub fn from_file(path: &PathBuf) -> Self {
        let config_str = std::fs::read_to_string(path).expect("Failed to read config");
        let mut config: Config = toml::from_str(&config_str).expect("Failed to parse config");
        config.db_dir_path = expand_tilde(&config.db_dir_path);
        config
    }
    pub fn to_file(&self, path: &PathBuf) {
        let config_str = toml::to_string(self).expect("Failed to serialize config");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create config directory");
        }
        std::fs::write(path, config_str).expect("Failed to write config");
    }
}

pub fn default_config_path() -> PathBuf {
    let mut path = dirs::config_dir().expect("Failed to find config directory");
    path.push("cliprust/config.toml");
    path
}

pub fn default_config() -> Config {
    Config {
        db_dir_path: default_db_dir_path(),
        max_dedupe_depth: 100,
        max_items: 750,
        preview_width: 100,
        generate_thumb: ThumbMode::None,
    }
}

fn default_db_dir_path() -> PathBuf {
    let mut path = dirs::data_dir().expect("Failed to find data directory");
    path.push("cliprust");
    path
}

fn expand_tilde(path: &Path) -> PathBuf {
    match path.to_str() {
        Some(p) if p == "~" || p.starts_with("~/") => {
            let home =
                dirs::home_dir().expect("Could not determine home directory for tilde expansion");
            if p == "~" {
                home
            } else {
                home.join(&p[2..])
            }
        }
        _ => path.to_path_buf(),
    }
}
