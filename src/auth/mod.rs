//! Authentication module (credential storage via keyring)

use anyhow::{Context, Result};
use keyring::Entry;

use crate::models::Account;

const SERVICE_NAME: &str = "perch";

/// Store credentials for an account
pub fn store_credentials(account: &Account, secret: &str) -> Result<()> {
    let key = account.keyring_key();
    let entry = Entry::new(SERVICE_NAME, &key).context("Failed to create keyring entry")?;
    entry
        .set_password(secret)
        .context("Failed to store credentials")?;
    Ok(())
}

/// Get credentials for an account
pub fn get_credentials(account: &Account) -> Result<Option<String>> {
    let key = account.keyring_key();
    let entry = Entry::new(SERVICE_NAME, &key).context("Failed to create keyring entry")?;

    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).context("Failed to get credentials"),
    }
}

/// Delete credentials for an account
pub fn delete_credentials(account: &Account) -> Result<()> {
    let key = account.keyring_key();
    let entry = Entry::new(SERVICE_NAME, &key).context("Failed to create keyring entry")?;

    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
        Err(e) => Err(e).context("Failed to delete credentials"),
    }
}

/// Store OAuth client credentials (for Mastodon instances)
pub fn store_oauth_client(instance: &str, client_id: &str, client_secret: &str) -> Result<()> {
    let key = format!("oauth:{}:client", instance);
    let value = format!("{}:{}", client_id, client_secret);
    let entry = Entry::new(SERVICE_NAME, &key).context("Failed to create keyring entry")?;
    entry
        .set_password(&value)
        .context("Failed to store OAuth client")?;
    Ok(())
}

/// Get OAuth client credentials for a Mastodon instance
pub fn get_oauth_client(instance: &str) -> Result<Option<(String, String)>> {
    let key = format!("oauth:{}:client", instance);
    let entry = Entry::new(SERVICE_NAME, &key).context("Failed to create keyring entry")?;

    match entry.get_password() {
        Ok(value) => {
            let parts: Vec<&str> = value.splitn(2, ':').collect();
            if parts.len() == 2 {
                Ok(Some((parts[0].to_string(), parts[1].to_string())))
            } else {
                Ok(None)
            }
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).context("Failed to get OAuth client"),
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
    // Note: keyring doesn't support listing entries
    // This would need a separate index or database lookup
    Vec::new()
}
