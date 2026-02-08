//! Mastodon API client

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::{Account, MediaAttachment, MediaType, Network, Post};

use super::SocialApi;

/// Mastodon API client
pub struct MastodonClient {
    client: Client,
    instance: String,
    access_token: String,
}

impl MastodonClient {
    /// Create a new Mastodon client
    pub fn new(instance: &str, access_token: &str) -> Self {
        Self {
            client: Client::new(),
            instance: instance.trim_end_matches('/').to_string(),
            access_token: access_token.to_string(),
        }
    }

    /// Build API URL
    fn api_url(&self, endpoint: &str) -> String {
        format!("{}/api/v1{}", self.instance, endpoint)
    }
}

impl SocialApi for MastodonClient {
    async fn timeline(&self, limit: usize) -> Result<Vec<Post>> {
        let url = self.api_url(&format!("/timelines/home?limit={limit}"));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .context("Failed to fetch timeline")?;

        let statuses: Vec<MastodonStatus> = response
            .json()
            .await
            .context("Failed to parse timeline response")?;

        Ok(statuses
            .into_iter()
            .map(MastodonStatus::into_post)
            .collect())
    }

    async fn get_context(&self, post: &Post) -> Result<Vec<Post>> {
        let url = self.api_url(&format!("/statuses/{}/context", post.network_id));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .context("Failed to fetch context")?;

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct ContextResponse {
            ancestors: Vec<MastodonStatus>,
            descendants: Vec<MastodonStatus>,
        }

        let context: ContextResponse = response
            .json()
            .await
            .context("Failed to parse context response")?;

        // Return descendants (replies) only
        Ok(context
            .descendants
            .into_iter()
            .map(MastodonStatus::into_post)
            .collect())
    }

    async fn post(&self, content: &str) -> Result<Post> {
        let url = self.api_url("/statuses");

        let request = PostStatusRequest {
            status: content.to_string(),
            visibility: Some("public".to_string()),
            ..Default::default()
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&request)
            .send()
            .await
            .context("Failed to post status")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Mastodon error {status}: {body}");
        }

        let status: MastodonStatus = response
            .json()
            .await
            .context("Failed to parse post response")?;

        Ok(status.into_post())
    }

    async fn reply(&self, content: &str, reply_to_id: &str) -> Result<Post> {
        let url = self.api_url("/statuses");

        let request = PostStatusRequest {
            status: content.to_string(),
            visibility: Some("public".to_string()),
            in_reply_to_id: Some(reply_to_id.to_string()),
            ..Default::default()
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&request)
            .send()
            .await
            .context("Failed to post reply")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Mastodon error {status}: {body}");
        }

        let status: MastodonStatus = response
            .json()
            .await
            .context("Failed to parse reply response")?;

        Ok(status.into_post())
    }

    async fn like(&self, post: &Post) -> Result<()> {
        let url = self.api_url(&format!("/statuses/{}/favourite", post.network_id));

        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .context("Failed to like post")?;

        Ok(())
    }

    async fn unlike(&self, post: &Post) -> Result<()> {
        let url = self.api_url(&format!("/statuses/{}/unfavourite", post.network_id));

        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .context("Failed to unlike post")?;

        Ok(())
    }

    async fn repost(&self, post: &Post) -> Result<()> {
        let url = self.api_url(&format!("/statuses/{}/reblog", post.network_id));

        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .context("Failed to repost")?;

        Ok(())
    }

    async fn unrepost(&self, post: &Post) -> Result<()> {
        let url = self.api_url(&format!("/statuses/{}/unreblog", post.network_id));

        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .context("Failed to unrepost")?;

        Ok(())
    }

    async fn verify_credentials(&self) -> Result<Account> {
        let url = self.api_url("/accounts/verify_credentials");

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .context("Failed to verify credentials")?;

        let account: MastodonAccount = response
            .json()
            .await
            .context("Failed to parse account response")?;

        Ok(Account {
            id: uuid::Uuid::new_v4(),
            network: Network::Mastodon,
            display_name: account.display_name,
            handle: account.username,
            server: self.instance.clone(),
            is_default: true, // First account of this network is default
            avatar_url: Some(account.avatar),
            created_at: Utc::now(),
            last_used_at: None,
        })
    }
}

// ==================== API Types ====================

#[derive(Debug, Serialize, Default)]
struct PostStatusRequest {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    in_reply_to_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sensitive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spoiler_text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MastodonStatus {
    id: String,
    created_at: String,
    content: String,
    url: Option<String>,
    account: MastodonAccount,
    reblog: Option<Box<Self>>,
    favourites_count: u32,
    reblogs_count: u32,
    replies_count: u32,
    favourited: Option<bool>,
    reblogged: Option<bool>,
    in_reply_to_id: Option<String>,
    media_attachments: Vec<MastodonMedia>,
}

#[derive(Debug, Deserialize)]
struct MastodonAccount {
    #[allow(dead_code)]
    id: String,
    username: String,
    display_name: String,
    avatar: String,
    #[serde(default)]
    acct: String,
}

#[derive(Debug, Deserialize)]
struct MastodonMedia {
    url: String,
    preview_url: Option<String>,
    #[serde(rename = "type")]
    media_type: String,
    description: Option<String>,
}

impl MastodonStatus {
    fn into_post(self) -> Post {
        // Handle reposts
        if let Some(reblog) = self.reblog {
            let mut post = reblog.into_post();
            post.is_repost = true;
            post.repost_author = Some(self.account.display_name);
            return post;
        }

        // Strip HTML from content
        let content = html_escape::decode_html_entities(&self.content)
            .to_string()
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("</p><p>", "\n\n");

        // Simple HTML tag removal
        let content = regex_lite::Regex::new(r"<[^>]+>")
            .map(|re| re.replace_all(&content, "").to_string())
            .unwrap_or(content);

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

        Post {
            id: uuid::Uuid::new_v4(),
            network_id: self.id,
            network: Network::Mastodon,
            author_handle: if self.account.acct.is_empty() {
                self.account.username
            } else {
                self.account.acct
            },
            author_name: self.account.display_name,
            author_avatar: Some(self.account.avatar),
            content,
            content_raw: Some(self.content),
            created_at,
            url: self.url,
            is_repost: false,
            repost_author: None,
            like_count: self.favourites_count,
            repost_count: self.reblogs_count,
            reply_count: self.replies_count,
            liked: self.favourited.unwrap_or(false),
            reposted: self.reblogged.unwrap_or(false),
            reply_to_id: self.in_reply_to_id,
            media: self
                .media_attachments
                .into_iter()
                .map(|m| MediaAttachment {
                    url: m.url,
                    preview_url: m.preview_url,
                    media_type: match m.media_type.as_str() {
                        "image" => MediaType::Image,
                        "video" => MediaType::Video,
                        "gifv" => MediaType::Gifv,
                        "audio" => MediaType::Audio,
                        _ => MediaType::Unknown,
                    },
                    alt_text: m.description,
                })
                .collect(),
            cid: None,
            uri: None,
        }
    }
}

/// OAuth authentication flow for Mastodon
pub mod oauth {
    use super::{Client, Context, Deserialize, Result};

    /// Registered OAuth application credentials
    #[derive(Debug, Deserialize)]
    pub struct OAuthApp {
        /// OAuth client ID
        pub client_id: String,
        /// OAuth client secret
        pub client_secret: String,
    }

    /// OAuth access token response
    #[derive(Debug, Deserialize)]
    pub struct OAuthToken {
        /// Access token for API requests
        pub access_token: String,
        /// Token type (usually "Bearer")
        pub token_type: String,
    }

    /// Register an OAuth application with an instance
    pub async fn register_app(instance: &str) -> Result<OAuthApp> {
        let client = Client::new();
        let url = format!("{}/api/v1/apps", instance.trim_end_matches('/'));

        let params = [
            ("client_name", "Perch"),
            ("redirect_uris", "urn:ietf:wg:oauth:2.0:oob"),
            ("scopes", "read write follow"),
            ("website", "https://github.com/ricardodantas/perch"),
        ];

        let response = client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to register app")?;

        response
            .json()
            .await
            .context("Failed to parse app registration response")
    }

    /// Get the authorization URL for the user to visit
    pub fn get_auth_url(instance: &str, client_id: &str) -> String {
        format!(
            "{}/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=read+write+follow",
            instance.trim_end_matches('/'),
            client_id,
            urlencoding::encode("urn:ietf:wg:oauth:2.0:oob")
        )
    }

    /// Exchange authorization code for access token
    pub async fn get_token(
        instance: &str,
        client_id: &str,
        client_secret: &str,
        code: &str,
    ) -> Result<OAuthToken> {
        let client = Client::new();
        let url = format!("{}/oauth/token", instance.trim_end_matches('/'));

        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
            ("code", code),
            ("scope", "read write follow"),
        ];

        let response = client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to get access token")?;

        response
            .json()
            .await
            .context("Failed to parse token response")
    }
}
