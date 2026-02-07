//! Bluesky (AT Protocol) API client

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::{Account, MediaAttachment, MediaType, Network, Post};

use super::SocialApi;

/// Default PDS URL for Bluesky
pub const DEFAULT_PDS_URL: &str = "https://bsky.social";

/// Bluesky API client
pub struct BlueskyClient {
    client: Client,
    pds_url: String,
    access_jwt: String,
    did: String,
}

impl BlueskyClient {
    /// Login to Bluesky using the default PDS
    pub async fn login(handle: &str, app_password: &str) -> Result<Self> {
        Self::login_with_pds(handle, app_password, DEFAULT_PDS_URL).await
    }

    /// Login to Bluesky with a custom PDS URL
    pub async fn login_with_pds(handle: &str, app_password: &str, pds_url: &str) -> Result<Self> {
        let client = Client::new();
        let pds_url = pds_url.trim_end_matches('/').to_string();

        let url = format!("{}/xrpc/com.atproto.server.createSession", pds_url);

        let request = CreateSessionRequest {
            identifier: handle.to_string(),
            password: app_password.to_string(),
        };

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to login to Bluesky")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Bluesky login failed: {}", error_text);
        }

        let session: CreateSessionResponse = response
            .json()
            .await
            .context("Failed to parse login response")?;

        Ok(Self {
            client,
            pds_url,
            access_jwt: session.access_jwt,
            did: session.did,
        })
    }

    /// Create a new client with existing credentials
    pub fn new(pds_url: &str, access_jwt: &str, did: &str) -> Self {
        Self {
            client: Client::new(),
            pds_url: pds_url.to_string(),
            access_jwt: access_jwt.to_string(),
            did: did.to_string(),
        }
    }
}

impl SocialApi for BlueskyClient {
    async fn timeline(&self, limit: usize) -> Result<Vec<Post>> {
        let url = format!(
            "{}/xrpc/app.bsky.feed.getTimeline?limit={}",
            self.pds_url, limit
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_jwt))
            .send()
            .await
            .context("Failed to fetch timeline")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to fetch timeline: {}", error_text);
        }

        let timeline: GetTimelineResponse = response
            .json()
            .await
            .context("Failed to parse timeline response")?;

        Ok(timeline
            .feed
            .into_iter()
            .map(|item| item.into_post())
            .collect())
    }

    async fn post(&self, content: &str) -> Result<Post> {
        let url = format!("{}/xrpc/com.atproto.repo.createRecord", self.pds_url);

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let record = PostRecord {
            text: content.to_string(),
            created_at: now.clone(),
            r#type: "app.bsky.feed.post".to_string(),
        };

        let request = CreateRecordRequest {
            repo: self.did.clone(),
            collection: "app.bsky.feed.post".to_string(),
            record,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_jwt))
            .json(&request)
            .send()
            .await
            .context("Failed to post")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to post: {}", error_text);
        }

        let result: CreateRecordResponse = response
            .json()
            .await
            .context("Failed to parse post response")?;

        // Return a simple post object
        Ok(Post {
            id: uuid::Uuid::new_v4(),
            network_id: result.uri.split('/').last().unwrap_or(&result.uri).to_string(),
            network: Network::Bluesky,
            author_handle: self.did.clone(),
            author_name: String::new(),
            author_avatar: None,
            content: content.to_string(),
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
            cid: Some(result.cid),
            uri: Some(result.uri),
        })
    }

    async fn like(&self, post: &Post) -> Result<()> {
        let cid = post.cid.as_ref().context("Post missing CID for like")?;
        let uri = post.uri.as_ref().context("Post missing URI for like")?;

        let url = format!("{}/xrpc/com.atproto.repo.createRecord", self.pds_url);

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let record = LikeRecord {
            subject: RecordRef {
                uri: uri.clone(),
                cid: cid.clone(),
            },
            created_at: now,
            r#type: "app.bsky.feed.like".to_string(),
        };

        let request = CreateRecordRequest {
            repo: self.did.clone(),
            collection: "app.bsky.feed.like".to_string(),
            record,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_jwt))
            .json(&request)
            .send()
            .await
            .context("Failed to like post")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to like post: {}", error_text);
        }

        Ok(())
    }

    async fn unlike(&self, post: &Post) -> Result<()> {
        let uri = post.uri.as_ref().context("Post missing URI for unlike")?;
        
        // First, find the like record
        let list_url = format!(
            "{}/xrpc/app.bsky.feed.getLikes?uri={}&limit=100",
            self.pds_url,
            urlencoding::encode(uri)
        );
        
        let response = self
            .client
            .get(&list_url)
            .header("Authorization", format!("Bearer {}", self.access_jwt))
            .send()
            .await
            .context("Failed to get likes")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to get likes: {}", error_text);
        }
        
        #[derive(Debug, Deserialize)]
        struct LikesResponse {
            likes: Vec<LikeItem>,
        }
        
        #[derive(Debug, Deserialize)]
        struct LikeItem {
            actor: ActorRef,
            #[serde(rename = "indexedAt")]
            _indexed_at: String,
        }
        
        let _likes: LikesResponse = response.json().await.context("Failed to parse likes")?;
        
        // Find our like in the actor's repo
        let records_url = format!(
            "{}/xrpc/com.atproto.repo.listRecords?repo={}&collection=app.bsky.feed.like&limit=100",
            self.pds_url, self.did
        );
        
        let response = self
            .client
            .get(&records_url)
            .header("Authorization", format!("Bearer {}", self.access_jwt))
            .send()
            .await
            .context("Failed to list like records")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to list like records: {}", error_text);
        }
        
        #[derive(Debug, Deserialize)]
        struct ListRecordsResponse {
            records: Vec<RecordItem>,
        }
        
        #[derive(Debug, Deserialize)]
        struct RecordItem {
            uri: String,
            value: LikeRecordValue,
        }
        
        #[derive(Debug, Deserialize)]
        struct LikeRecordValue {
            subject: RecordRef,
        }
        
        #[derive(Debug, Deserialize)]
        struct RecordRef {
            uri: String,
        }
        
        let records: ListRecordsResponse = response.json().await.context("Failed to parse records")?;
        
        // Find the like record for this post
        let like_record = records.records.iter().find(|r| r.value.subject.uri == *uri);
        
        let Some(record) = like_record else {
            // Already unliked or not found
            return Ok(());
        };
        
        // Extract rkey from the record URI
        let rkey = record.uri.split('/').last().context("Invalid record URI")?;
        
        // Delete the like record
        let delete_url = format!("{}/xrpc/com.atproto.repo.deleteRecord", self.pds_url);
        
        #[derive(Debug, Serialize)]
        struct DeleteRequest {
            repo: String,
            collection: String,
            rkey: String,
        }
        
        let delete_request = DeleteRequest {
            repo: self.did.clone(),
            collection: "app.bsky.feed.like".to_string(),
            rkey: rkey.to_string(),
        };
        
        let response = self
            .client
            .post(&delete_url)
            .header("Authorization", format!("Bearer {}", self.access_jwt))
            .json(&delete_request)
            .send()
            .await
            .context("Failed to delete like")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to unlike: {}", error_text);
        }
        
        Ok(())
    }

    async fn repost(&self, post: &Post) -> Result<()> {
        let cid = post.cid.as_ref().context("Post missing CID for repost")?;
        let uri = post.uri.as_ref().context("Post missing URI for repost")?;

        let url = format!("{}/xrpc/com.atproto.repo.createRecord", self.pds_url);

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let record = RepostRecord {
            subject: RecordRef {
                uri: uri.clone(),
                cid: cid.clone(),
            },
            created_at: now,
            r#type: "app.bsky.feed.repost".to_string(),
        };

        let request = CreateRecordRequest {
            repo: self.did.clone(),
            collection: "app.bsky.feed.repost".to_string(),
            record,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_jwt))
            .json(&request)
            .send()
            .await
            .context("Failed to repost")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to repost: {}", error_text);
        }

        Ok(())
    }

    async fn verify_credentials(&self) -> Result<Account> {
        let url = format!(
            "{}/xrpc/app.bsky.actor.getProfile?actor={}",
            self.pds_url, self.did
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_jwt))
            .send()
            .await
            .context("Failed to get profile")?;

        let profile: ProfileResponse = response
            .json()
            .await
            .context("Failed to parse profile response")?;

        Ok(Account {
            id: uuid::Uuid::new_v4(),
            network: Network::Bluesky,
            display_name: profile.display_name.unwrap_or_else(|| profile.handle.clone()),
            handle: profile.handle,
            server: self.pds_url.clone(),
            is_default: false,
            avatar_url: profile.avatar,
            created_at: Utc::now(),
            last_used_at: None,
        })
    }
}

// ==================== API Types ====================

#[derive(Debug, Serialize)]
struct CreateSessionRequest {
    identifier: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct CreateSessionResponse {
    #[serde(rename = "accessJwt")]
    access_jwt: String,
    did: String,
    #[allow(dead_code)]
    handle: String,
}

#[derive(Debug, Deserialize)]
struct GetTimelineResponse {
    feed: Vec<FeedViewPost>,
    #[allow(dead_code)]
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FeedViewPost {
    post: PostView,
    #[serde(default)]
    reason: Option<RepostReason>,
}

#[derive(Debug, Deserialize)]
struct RepostReason {
    #[serde(rename = "$type")]
    reason_type: String,
    by: ActorRef,
}

#[derive(Debug, Deserialize)]
struct ActorRef {
    #[allow(dead_code)]
    did: String,
    handle: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostView {
    uri: String,
    cid: String,
    author: Author,
    record: PostRecord,
    #[serde(rename = "replyCount", default)]
    reply_count: u32,
    #[serde(rename = "repostCount", default)]
    repost_count: u32,
    #[serde(rename = "likeCount", default)]
    like_count: u32,
    #[serde(rename = "indexedAt")]
    indexed_at: String,
    #[serde(default)]
    embed: Option<Embed>,
    /// Viewer state (liked, reposted, etc.)
    #[serde(default)]
    viewer: Option<ViewerState>,
}

#[derive(Debug, Deserialize)]
struct Author {
    #[allow(dead_code)]
    did: String,
    handle: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    avatar: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostRecord {
    text: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "$type")]
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct Embed {
    #[allow(dead_code)]
    #[serde(rename = "$type")]
    embed_type: String,
    images: Option<Vec<EmbedImage>>,
}

#[derive(Debug, Deserialize)]
struct EmbedImage {
    #[allow(dead_code)]
    thumb: String,
    fullsize: String,
    alt: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateRecordRequest<T> {
    repo: String,
    collection: String,
    record: T,
}

#[derive(Debug, Deserialize)]
struct CreateRecordResponse {
    uri: String,
    cid: String,
}

#[derive(Debug, Serialize)]
struct LikeRecord {
    subject: RecordRef,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "$type")]
    r#type: String,
}

#[derive(Debug, Serialize)]
struct RepostRecord {
    subject: RecordRef,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "$type")]
    r#type: String,
}

#[derive(Debug, Serialize)]
struct RecordRef {
    uri: String,
    cid: String,
}

#[derive(Debug, Deserialize)]
struct ProfileResponse {
    handle: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    avatar: Option<String>,
}

/// Viewer state for a post (whether current user liked/reposted)
#[derive(Debug, Deserialize, Default)]
struct ViewerState {
    /// URI of the like record if liked by viewer
    like: Option<String>,
    /// URI of the repost record if reposted by viewer
    repost: Option<String>,
}

impl FeedViewPost {
    fn into_post(self) -> Post {
        let created_at = DateTime::parse_from_rfc3339(&self.post.record.created_at)
            .or_else(|_| DateTime::parse_from_rfc3339(&self.post.indexed_at))
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let (is_repost, repost_author) = if let Some(reason) = &self.reason {
            if reason.reason_type == "app.bsky.feed.defs#reasonRepost" {
                (
                    true,
                    Some(
                        reason
                            .by
                            .display_name
                            .clone()
                            .unwrap_or_else(|| reason.by.handle.clone()),
                    ),
                )
            } else {
                (false, None)
            }
        } else {
            (false, None)
        };

        let media = self
            .post
            .embed
            .and_then(|e| e.images)
            .map(|images| {
                images
                    .into_iter()
                    .map(|img| MediaAttachment {
                        url: img.fullsize,
                        preview_url: None,
                        media_type: MediaType::Image,
                        alt_text: img.alt,
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Build URL from URI
        let url = format!(
            "https://bsky.app/profile/{}/post/{}",
            self.post.author.handle,
            self.post.uri.split('/').last().unwrap_or("")
        );

        // Check viewer state for liked/reposted
        let (liked, reposted) = self.post.viewer.as_ref().map_or((false, false), |v| {
            (v.like.is_some(), v.repost.is_some())
        });

        Post {
            id: uuid::Uuid::new_v4(),
            network_id: self.post.uri.split('/').last().unwrap_or(&self.post.uri).to_string(),
            network: Network::Bluesky,
            author_handle: self.post.author.handle,
            author_name: self.post.author.display_name.unwrap_or_default(),
            author_avatar: self.post.author.avatar,
            content: self.post.record.text,
            content_raw: None,
            created_at,
            url: Some(url),
            is_repost,
            repost_author,
            like_count: self.post.like_count,
            repost_count: self.post.repost_count,
            reply_count: self.post.reply_count,
            liked,
            reposted,
            reply_to_id: None,
            media,
            cid: Some(self.post.cid),
            uri: Some(self.post.uri),
        }
    }
}
