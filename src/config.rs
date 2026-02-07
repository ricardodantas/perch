//! Configuration module for Perch

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::theme::Theme;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Selected theme
    #[serde(default)]
    pub theme: Theme,

    /// Default timeline view (home, local, federated, unified)
    #[serde(default = "default_timeline")]
    pub default_timeline: String,

    /// Enable vim-like keybindings
    #[serde(default = "default_vim_mode")]
    pub vim_mode: bool,

    /// Auto-refresh interval in seconds (0 = manual only)
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,

    /// Number of posts to fetch per request
    #[serde(default = "default_post_limit")]
    pub post_limit: usize,

    /// Whether to show media previews in the timeline
    #[serde(default = "default_show_media")]
    pub show_media: bool,

    /// Default networks to post to (for cross-posting)
    #[serde(default)]
    pub default_post_networks: Vec<String>,
}

fn default_timeline() -> String {
    "home".to_string()
}

fn default_vim_mode() -> bool {
    true
}

fn default_refresh_interval() -> u64 {
    0 // Manual refresh by default
}

fn default_post_limit() -> usize {
    50
}

fn default_show_media() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            default_timeline: default_timeline(),
            vim_mode: default_vim_mode(),
            refresh_interval_secs: default_refresh_interval(),
            post_limit: default_post_limit(),
            show_media: default_show_media(),
            default_post_networks: Vec::new(),
        }
    }
}

impl Config {
    /// Get the default config file path
    pub fn default_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("perch");
        Ok(config_dir.join("config.toml"))
    }

    /// Load config from the default path or create default
    pub fn load() -> Result<Self> {
        let path = Self::default_path()?;
        Self::load_from(&path)
    }

    /// Load config from a specific path
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path).context("Failed to read config file")?;
            toml::from_str(&content).context("Failed to parse config file")
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to the default path
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path()?;
        self.save_to(&path)
    }

    /// Save config to a specific path
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(path, content).context("Failed to write config file")?;

        Ok(())
    }
}
