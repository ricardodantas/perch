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
    /// Stable account identifier: the DID for Bluesky (survives handle
    /// changes). `None` for Mastodon and for Bluesky accounts stored before
    /// it was recorded.
    #[serde(default)]
    pub did: Option<String>,
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
            did: None,
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
            did: None,
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

    /// The account to act with on `network`: the explicit default if any,
    /// otherwise the most recently added account for that network.
    ///
    /// Mirrors `Database::get_default_account`. Every "which account for this
    /// network" decision should go through this.
    pub fn default_for_network(accounts: &[Self], network: Network) -> Option<Self> {
        accounts
            .iter()
            .filter(|a| a.network == network)
            .max_by_key(|a| (a.is_default, a.created_at))
            .cloned()
    }

    /// Pick one account per network (in `networks` order, duplicates ignored)
    /// using [`Self::default_for_network`], so a network is never posted to or
    /// fetched more than once even if several accounts exist for it.
    pub fn default_for_networks(accounts: &[Self], networks: &[Network]) -> Vec<Self> {
        let mut selected: Vec<Self> = Vec::new();
        for network in networks {
            if selected.iter().any(|a| a.network == *network) {
                continue;
            }
            if let Some(account) = Self::default_for_network(accounts, *network) {
                selected.push(account);
            }
        }
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_for_networks_picks_one_account_per_network() {
        let mut old_bsky = Account::new_bluesky("me.bsky.social", "Me");
        old_bsky.created_at -= chrono::Duration::hours(1);
        let new_bsky = Account::new_bluesky("me.bsky.social", "Me");
        let mut masto_default = Account::new_mastodon("me", "https://a.example", "Me");
        masto_default.is_default = true;
        masto_default.created_at -= chrono::Duration::hours(1);
        let masto_newer = Account::new_mastodon("alt", "https://b.example", "Alt");
        let accounts = vec![
            old_bsky,
            new_bsky.clone(),
            masto_default.clone(),
            masto_newer,
        ];

        let picked = Account::default_for_networks(
            &accounts,
            &[Network::Bluesky, Network::Mastodon, Network::Bluesky],
        );
        let ids: Vec<_> = picked.iter().map(|a| a.id).collect();
        assert_eq!(ids, vec![new_bsky.id, masto_default.id]);

        let picked = Account::default_for_networks(&accounts[..2], &[Network::Mastodon]);
        assert!(picked.is_empty());

        let picked = Account::default_for_network(&accounts, Network::Mastodon);
        assert_eq!(picked.map(|a| a.id), Some(masto_default.id));
        let picked = Account::default_for_network(&accounts, Network::Bluesky);
        assert_eq!(picked.map(|a| a.id), Some(new_bsky.id));
    }
}
