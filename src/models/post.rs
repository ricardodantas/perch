//! Post/Status model (unified across networks)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Network;

/// A post/status (unified model for all networks)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    /// Internal ID for caching
    pub id: Uuid,
    /// Network-specific ID
    pub network_id: String,
    /// Which network this post is from
    pub network: Network,
    /// Author handle
    pub author_handle: String,
    /// Author display name
    pub author_name: String,
    /// Author avatar URL
    pub author_avatar: Option<String>,
    /// Post content (plain text, HTML stripped)
    pub content: String,
    /// Original content (HTML for Mastodon, facets for Bluesky)
    pub content_raw: Option<String>,
    /// When the post was created
    pub created_at: DateTime<Utc>,
    /// URL to the post on the web
    pub url: Option<String>,
    /// Whether this is a repost/boost
    pub is_repost: bool,
    /// Original author (if repost)
    pub repost_author: Option<String>,
    /// Number of likes/favorites
    pub like_count: u32,
    /// Number of reposts/boosts
    pub repost_count: u32,
    /// Number of replies
    pub reply_count: u32,
    /// Whether the current user has liked this post
    pub liked: bool,
    /// Whether the current user has reposted this post
    pub reposted: bool,
    /// Reply-to post ID (if this is a reply)
    pub reply_to_id: Option<String>,
    /// Media attachments (URLs)
    pub media: Vec<MediaAttachment>,
    /// CID for Bluesky (needed for likes/reposts)
    pub cid: Option<String>,
    /// URI for Bluesky (at:// URI)
    pub uri: Option<String>,
}

/// Media attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAttachment {
    /// Media URL
    pub url: String,
    /// Preview/thumbnail URL
    pub preview_url: Option<String>,
    /// Media type (image, video, gifv, audio)
    pub media_type: MediaType,
    /// Alt text description
    pub alt_text: Option<String>,
}

/// Media type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    /// Image (JPEG, PNG, GIF, WebP)
    Image,
    /// Video (MP4, WebM)
    Video,
    /// Animated GIF (Mastodon-specific)
    Gifv,
    /// Audio file
    Audio,
    /// Unknown or unsupported media type
    Unknown,
}

impl Post {
    /// Create a new post from network data
    pub fn new(network: Network, network_id: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            network_id: network_id.to_string(),
            network,
            author_handle: String::new(),
            author_name: String::new(),
            author_avatar: None,
            content: String::new(),
            content_raw: None,
            created_at: Utc::now(),
            url: None,
            is_repost: false,
            repost_author: None,
            like_count: 0,
            repost_count: 0,
            reply_count: 0,
            liked: false,
            reposted: false,
            reply_to_id: None,
            media: Vec::new(),
            cid: None,
            uri: None,
        }
    }

    /// Get a short preview of the content (for list display)
    pub fn preview(&self, max_len: usize) -> String {
        let content = self.content.replace('\n', " ");
        if content.len() <= max_len {
            content
        } else {
            format!("{}...", &content[..max_len.saturating_sub(3)])
        }
    }

    /// Get relative time string (e.g., "5m", "2h", "3d")
    pub fn relative_time(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.created_at);

        if duration.num_seconds() < 60 {
            format!("{}s", duration.num_seconds())
        } else if duration.num_minutes() < 60 {
            format!("{}m", duration.num_minutes())
        } else if duration.num_hours() < 24 {
            format!("{}h", duration.num_hours())
        } else if duration.num_days() < 7 {
            format!("{}d", duration.num_days())
        } else {
            self.created_at.format("%b %d").to_string()
        }
    }
}
