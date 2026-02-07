//! Account model for stored credentials

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Network;

/// A stored account/credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Unique identifier
    pub id: Uuid,
    /// Which network this account belongs to
    pub network: Network,
    /// Display name (for UI)
    pub display_name: String,
    /// Handle/username (e.g., @user@mastodon.social or user.bsky.social)
    pub handle: String,
    /// Server/instance URL (for Mastodon) or PDS URL (for Bluesky)
    pub server: String,
    /// Whether this is the default account for the network
    pub is_default: bool,
    /// Avatar URL (cached)
    pub avatar_url: Option<String>,
    /// When the account was added
    pub created_at: DateTime<Utc>,
    /// Last used timestamp
    pub last_used_at: Option<DateTime<Utc>>,
}

impl Account {
    /// Create a new Mastodon account
    pub fn new_mastodon(handle: &str, server: &str, display_name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            network: Network::Mastodon,
            display_name: display_name.to_string(),
            handle: handle.to_string(),
            server: server.to_string(),
            is_default: false,
            avatar_url: None,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    /// Create a new Bluesky account
    pub fn new_bluesky(handle: &str, display_name: &str) -> Self {
        Self::new_bluesky_with_pds(handle, display_name, "https://bsky.social")
    }

    /// Create a new Bluesky account with a custom PDS URL
    pub fn new_bluesky_with_pds(handle: &str, display_name: &str, pds_url: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            network: Network::Bluesky,
            display_name: display_name.to_string(),
            handle: handle.to_string(),
            server: pds_url.to_string(),
            is_default: false,
            avatar_url: None,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    /// Get the full handle with instance (for Mastodon)
    pub fn full_handle(&self) -> String {
        match self.network {
            Network::Mastodon => {
                if self.handle.contains('@') {
                    self.handle.clone()
                } else {
                    // Extract domain from server URL
                    let domain = self
                        .server
                        .trim_start_matches("https://")
                        .trim_start_matches("http://")
                        .trim_end_matches('/');
                    format!("@{}@{}", self.handle, domain)
                }
            }
            Network::Bluesky => {
                if self.handle.ends_with(".bsky.social") {
                    format!("@{}", self.handle)
                } else {
                    format!("@{}", self.handle)
                }
            }
        }
    }

    /// Get the keyring key for storing credentials
    pub fn keyring_key(&self) -> String {
        format!("perch:{}:{}", self.network.name().to_lowercase(), self.id)
    }
}
