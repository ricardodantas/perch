//! Background sync module for timeline refresh

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, interval};

use crate::api::get_client;
use crate::db::Database;
use crate::models::{Account, Post};

/// Sync manager for background timeline updates
pub struct SyncManager {
    db: Arc<Mutex<Database>>,
    accounts: Vec<Account>,
    tokens: Vec<String>,
}

impl SyncManager {
    /// Create a new sync manager
    pub fn new(db: Database, accounts: Vec<Account>, tokens: Vec<String>) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            accounts,
            tokens,
        }
    }

    /// Refresh all timelines once
    pub async fn refresh_all(&self) -> Result<Vec<Post>> {
        let mut all_posts = Vec::new();

        for (account, token) in self.accounts.iter().zip(self.tokens.iter()) {
            match self.refresh_account(account, token).await {
                Ok(posts) => all_posts.extend(posts),
                Err(e) => {
                    tracing::warn!("Failed to refresh {}: {}", account.handle, e);
                }
            }
        }

        // Sort by timestamp (newest first)
        all_posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Cache posts
        {
            let db = self.db.lock().await;
            for post in &all_posts {
                if let Err(e) = db.cache_post(post) {
                    tracing::warn!("Failed to cache post: {}", e);
                }
            }
        }

        Ok(all_posts)
    }

    /// Refresh a single account's timeline
    async fn refresh_account(&self, account: &Account, token: &str) -> Result<Vec<Post>> {
        let client = get_client(account, token).await?;
        let posts = client.timeline(50).await?;
        Ok(posts)
    }

    /// Start a background refresh loop
    pub async fn start_background_refresh(self: Arc<Self>, interval_secs: u64) {
        if interval_secs == 0 {
            return; // Manual refresh only
        }

        let mut interval = interval(Duration::from_secs(interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.refresh_all().await {
                tracing::error!("Background refresh failed: {}", e);
            }
        }
    }
}

/// Fetch timeline for a single account
pub async fn fetch_timeline(account: &Account, token: &str, limit: usize) -> Result<Vec<Post>> {
    let client = get_client(account, token).await?;
    client.timeline(limit).await
}

/// Post to multiple accounts (cross-post)
pub async fn cross_post(
    content: &str,
    accounts: &[Account],
    tokens: &[String],
) -> Vec<Result<Post>> {
    let mut results = Vec::new();

    for (account, token) in accounts.iter().zip(tokens.iter()) {
        let result = async {
            let client = get_client(account, token).await?;
            client.post(content).await
        }
        .await;

        results.push(result);
    }

    results
}
