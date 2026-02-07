//! Common paths for Perch data storage
//!
//! All Perch data is stored under ~/.config/perch/ on all platforms:
//! - config.toml - User configuration
//! - credentials.enc - Encrypted credentials
//! - perch.sqlite - Database

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Get the Perch data directory (~/.config/perch/)
/// 
/// This is consistent across all platforms for simplicity.
pub fn perch_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let perch_dir = home.join(".config").join("perch");
    fs::create_dir_all(&perch_dir).context("Failed to create perch directory")?;
    Ok(perch_dir)
}

/// Get the config file path (~/.config/perch/config.toml)
pub fn config_path() -> Result<PathBuf> {
    Ok(perch_dir()?.join("config.toml"))
}

/// Get the database file path (~/.config/perch/perch.sqlite)
pub fn database_path() -> Result<PathBuf> {
    Ok(perch_dir()?.join("perch.sqlite"))
}

/// Get the credentials file path (~/.config/perch/credentials.enc)
pub fn credentials_path() -> Result<PathBuf> {
    Ok(perch_dir()?.join("credentials.enc"))
}
