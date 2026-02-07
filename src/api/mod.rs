//! API clients for social networks

pub mod bluesky;
pub mod mastodon;

use anyhow::Result;

use crate::models::{Account, Network, Post};

/// Unified API trait for social networks
#[allow(async_fn_in_trait)]
pub trait SocialApi {
    /// Get the home timeline
    async fn timeline(&self, limit: usize) -> Result<Vec<Post>>;

    /// Get replies/context for a post
    async fn get_context(&self, post: &Post) -> Result<Vec<Post>>;

    /// Post a new status
    async fn post(&self, content: &str) -> Result<Post>;

    /// Reply to a post
    async fn reply(&self, content: &str, reply_to_id: &str) -> Result<Post>;

    /// Like/favorite a post
    async fn like(&self, post: &Post) -> Result<()>;

    /// Unlike/unfavorite a post
    async fn unlike(&self, post: &Post) -> Result<()>;

    /// Repost/boost a post
    async fn repost(&self, post: &Post) -> Result<()>;

    /// Unrepost/unboost a post
    async fn unrepost(&self, post: &Post) -> Result<()>;

    /// Verify credentials and get account info
    async fn verify_credentials(&self) -> Result<Account>;
}

/// Unified client that wraps network-specific implementations
pub enum Client {
    /// Mastodon API client
    Mastodon(mastodon::MastodonClient),
    /// Bluesky AT Protocol client
    Bluesky(bluesky::BlueskyClient),
}

impl Client {
    /// Get the home timeline
    pub async fn timeline(&self, limit: usize) -> Result<Vec<Post>> {
        match self {
            Client::Mastodon(c) => c.timeline(limit).await,
            Client::Bluesky(c) => c.timeline(limit).await,
        }
    }

    /// Get replies/context for a post
    pub async fn get_context(&self, post: &Post) -> Result<Vec<Post>> {
        match self {
            Client::Mastodon(c) => c.get_context(post).await,
            Client::Bluesky(c) => c.get_context(post).await,
        }
    }

    /// Post a new status
    pub async fn post(&self, content: &str) -> Result<Post> {
        match self {
            Client::Mastodon(c) => c.post(content).await,
            Client::Bluesky(c) => c.post(content).await,
        }
    }

    /// Reply to a post
    pub async fn reply(&self, content: &str, reply_to_id: &str) -> Result<Post> {
        match self {
            Client::Mastodon(c) => c.reply(content, reply_to_id).await,
            Client::Bluesky(c) => c.reply(content, reply_to_id).await,
        }
    }

    /// Like/favorite a post
    pub async fn like(&self, post: &Post) -> Result<()> {
        match self {
            Client::Mastodon(c) => c.like(post).await,
            Client::Bluesky(c) => c.like(post).await,
        }
    }

    /// Unlike/unfavorite a post
    pub async fn unlike(&self, post: &Post) -> Result<()> {
        match self {
            Client::Mastodon(c) => c.unlike(post).await,
            Client::Bluesky(c) => c.unlike(post).await,
        }
    }

    /// Repost/boost a post
    pub async fn repost(&self, post: &Post) -> Result<()> {
        match self {
            Client::Mastodon(c) => c.repost(post).await,
            Client::Bluesky(c) => c.repost(post).await,
        }
    }

    /// Unrepost/unboost a post
    pub async fn unrepost(&self, post: &Post) -> Result<()> {
        match self {
            Client::Mastodon(c) => c.unrepost(post).await,
            Client::Bluesky(c) => c.unrepost(post).await,
        }
    }

    /// Verify credentials and get account info
    pub async fn verify_credentials(&self) -> Result<Account> {
        match self {
            Client::Mastodon(c) => c.verify_credentials().await,
            Client::Bluesky(c) => c.verify_credentials().await,
        }
    }
}

/// Get the appropriate API client for an account
pub async fn get_client(account: &Account, token: &str) -> Result<Client> {
    match account.network {
        Network::Mastodon => {
            let client = mastodon::MastodonClient::new(&account.server, token);
            Ok(Client::Mastodon(client))
        }
        Network::Bluesky => {
            // For Bluesky, token is the app password, server is the PDS URL
            let pds_url = if account.server.is_empty() {
                bluesky::DEFAULT_PDS_URL
            } else {
                &account.server
            };
            let client = bluesky::BlueskyClient::login_with_pds(&account.handle, token, pds_url).await?;
            Ok(Client::Bluesky(client))
        }
    }
}
