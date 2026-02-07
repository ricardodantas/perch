//! Authentication module (encrypted file-based credential storage)
//!
//! Stores credentials encrypted with AES-256-GCM in ~/.config/perch/credentials.enc
//! The encryption key is derived from machine-specific identifiers.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::models::Account;
use crate::paths;

const NONCE_SIZE: usize = 12;

/// Get the credentials file path
fn credentials_path() -> Result<PathBuf> {
    paths::credentials_path()
}

/// Get machine ID for key derivation (cross-platform)
fn get_machine_id() -> String {
    // Try platform-specific machine IDs first
    
    // Linux: /etc/machine-id or /var/lib/dbus/machine-id
    #[cfg(target_os = "linux")]
    {
        if let Ok(id) = fs::read_to_string("/etc/machine-id") {
            return id.trim().to_string();
        }
        if let Ok(id) = fs::read_to_string("/var/lib/dbus/machine-id") {
            return id.trim().to_string();
        }
    }
    
    // macOS: IOPlatformUUID via ioreg
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("IOPlatformUUID") {
                    if let Some(uuid) = line.split('"').nth(3) {
                        return uuid.to_string();
                    }
                }
            }
        }
    }
    
    // Windows: MachineGuid from registry
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\Cryptography",
                "/v",
                "MachineGuid",
            ])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("MachineGuid") {
                    if let Some(guid) = line.split_whitespace().last() {
                        return guid.to_string();
                    }
                }
            }
        }
    }
    
    // Fallback: use home directory path (always available via dirs crate)
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "perch-fallback-key".to_string())
}

/// Derive encryption key from machine-specific data
fn derive_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    
    // Primary: machine-specific ID
    hasher.update(get_machine_id().as_bytes());
    
    // Secondary: home directory path (cross-platform via dirs crate)
    if let Some(home) = dirs::home_dir() {
        hasher.update(home.to_string_lossy().as_bytes());
    }
    
    // Tertiary: data directory path
    if let Some(data) = dirs::data_dir() {
        hasher.update(data.to_string_lossy().as_bytes());
    }
    
    // Fixed salt for this app
    hasher.update(b"perch-social-client-v1");
    
    hasher.finalize().into()
}

/// Load all credentials from encrypted file
fn load_credentials() -> Result<HashMap<String, String>> {
    let path = credentials_path()?;
    
    if !path.exists() {
        return Ok(HashMap::new());
    }
    
    let encrypted = fs::read(&path).context("Failed to read credentials file")?;
    
    if encrypted.len() < NONCE_SIZE {
        return Ok(HashMap::new());
    }
    
    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).expect("Invalid key length");
    
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Failed to decrypt credentials"))?;
    
    let json = String::from_utf8(plaintext).context("Invalid UTF-8 in credentials")?;
    let creds: HashMap<String, String> = serde_json::from_str(&json)?;
    
    Ok(creds)
}

/// Save all credentials to encrypted file
fn save_credentials(creds: &HashMap<String, String>) -> Result<()> {
    let path = credentials_path()?;
    
    let json = serde_json::to_string(creds)?;
    
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).expect("Invalid key length");
    
    let mut rng = rand::rng();
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rng.fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher
        .encrypt(nonce, json.as_bytes())
        .map_err(|_| anyhow::anyhow!("Failed to encrypt credentials"))?;
    
    let mut output = nonce_bytes.to_vec();
    output.extend(ciphertext);
    
    fs::write(&path, output).context("Failed to write credentials file")?;
    
    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }
    
    Ok(())
}

/// Store credentials for an account
pub fn store_credentials(account: &Account, secret: &str) -> Result<()> {
    let mut creds = load_credentials().unwrap_or_default();
    creds.insert(account.keyring_key(), secret.to_string());
    save_credentials(&creds)
}

/// Get credentials for an account
pub fn get_credentials(account: &Account) -> Result<Option<String>> {
    let creds = load_credentials()?;
    Ok(creds.get(&account.keyring_key()).cloned())
}

/// Delete credentials for an account
pub fn delete_credentials(account: &Account) -> Result<()> {
    let mut creds = load_credentials().unwrap_or_default();
    creds.remove(&account.keyring_key());
    save_credentials(&creds)
}

/// Store OAuth client credentials (for Mastodon instances)
pub fn store_oauth_client(instance: &str, client_id: &str, client_secret: &str) -> Result<()> {
    let mut creds = load_credentials().unwrap_or_default();
    let key = format!("oauth:{}:client", instance);
    let value = format!("{}:{}", client_id, client_secret);
    creds.insert(key, value);
    save_credentials(&creds)
}

/// Get OAuth client credentials for a Mastodon instance
pub fn get_oauth_client(instance: &str) -> Result<Option<(String, String)>> {
    let creds = load_credentials()?;
    let key = format!("oauth:{}:client", instance);
    
    match creds.get(&key) {
        Some(value) => {
            let parts: Vec<&str> = value.splitn(2, ':').collect();
            if parts.len() == 2 {
                Ok(Some((parts[0].to_string(), parts[1].to_string())))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// Check if credentials exist for an account
pub fn has_credentials(account: &Account) -> bool {
    get_credentials(account)
        .map(|c| c.is_some())
        .unwrap_or(false)
}

/// Get all stored credential keys (for debugging)
pub fn list_credential_keys() -> Vec<String> {
    load_credentials()
        .map(|c| c.keys().cloned().collect())
        .unwrap_or_default()
}
