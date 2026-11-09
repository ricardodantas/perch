//! Async operations for the TUI
//!
//! Uses channels to communicate between the sync TUI loop and async tasks.

use anyhow::Result;
use std::io::Write;
use tokio::sync::mpsc;

use super::state::ReplyItem;
use crate::api::get_client;
use crate::auth;
use crate::models::{Account, Network, Post};

/// Log debug messages to `/tmp/perch_debug.log`
fn log_debug(msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/perch_debug.log")
    {
        let _ = writeln!(f, "[{}] {}", chrono::Local::now().format("%H:%M:%S"), msg);
    }
}

/// Commands sent from the TUI to the async worker
#[derive(Debug, Clone)]
pub enum AsyncCommand {
    /// Refresh timeline for given accounts
    RefreshTimeline { accounts: Vec<Account> },
    /// Load recent posts with a hashtag (without the leading `#`)
    HashtagTimeline { tag: String, accounts: Vec<Account> },
    /// Fetch replies/context for a post
    FetchContext { post: Post, account: Account },
    /// Like a post
    Like { post: Post, account: Account },
    /// Unlike a post
    Unlike { post: Post, account: Account },
    /// Repost/boost a post
    Repost { post: Post, account: Account },
    /// Unrepost/unboost a post
    Unrepost { post: Post, account: Account },
    /// Post to networks
    Post {
        content: String,
        accounts: Vec<Account>,
        reply_to: Option<Post>,
    },
    /// Schedule a post for later
    SchedulePost {
        content: String,
        networks: Vec<Network>,
        scheduled_for: chrono::DateTime<chrono::Utc>,
    },
    /// Load an image from a URL
    LoadImage { url: String },
    /// Shutdown the worker
    Shutdown,
}

/// Results sent back from the async worker to the TUI
#[derive(Debug)]
pub enum AsyncResult {
    /// Timeline refreshed with new posts
    TimelineRefreshed { posts: Vec<Post> },
    /// Hashtag timeline loaded
    HashtagLoaded { tag: String, posts: Vec<Post> },
    /// Context/replies fetched for a post
    #[allow(dead_code)]
    ContextFetched {
        post_id: String,
        replies: Vec<ReplyItem>,
    },
    /// Post was liked
    #[allow(dead_code)]
    Liked { post_id: String },
    /// Post was unliked
    #[allow(dead_code)]
    Unliked { post_id: String },
    /// Post was reposted
    #[allow(dead_code)]
    Reposted { post_id: String },
    /// Post was unreposted
    #[allow(dead_code)]
    Unreposted { post_id: String },
    /// New post created
    Posted { posts: Vec<Post> },
    /// Post was scheduled
    Scheduled { id: String, scheduled_for: String },
    /// Image loaded successfully
    ImageLoaded {
        url: String,
        image: image::DynamicImage,
    },
    /// Image loading failed
    ImageFailed { url: String, error: String },
    /// An error occurred
    Error { message: String },
    /// Status message (for progress updates)
    Status { message: String },
    /// A Bluesky session reported a DID the account has no record of
    SessionDid { account_id: uuid::Uuid, did: String },
}

/// Channel handles for communicating with the async worker
pub struct AsyncHandle {
    /// Send commands to the worker
    pub cmd_tx: mpsc::Sender<AsyncCommand>,
    /// Receive results from the worker
    pub result_rx: mpsc::Receiver<AsyncResult>,
}

/// Spawn the async worker and return handles
pub fn spawn_worker() -> AsyncHandle {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<AsyncCommand>(32);
    let (result_tx, result_rx) = mpsc::channel::<AsyncResult>(32);

    // Spawn the worker task
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                AsyncCommand::Shutdown => break,
                AsyncCommand::RefreshTimeline { accounts } => {
                    handle_refresh(&result_tx, accounts).await;
                }
                AsyncCommand::HashtagTimeline { tag, accounts } => {
                    handle_hashtag_timeline(&result_tx, tag, accounts).await;
                }
                AsyncCommand::FetchContext { post, account } => {
                    handle_fetch_context(&result_tx, post, account).await;
                }
                AsyncCommand::Like { post, account } => {
                    handle_like(&result_tx, post, account).await;
                }
                AsyncCommand::Unlike { post, account } => {
                    handle_unlike(&result_tx, post, account).await;
                }
                AsyncCommand::Repost { post, account } => {
                    handle_repost(&result_tx, post, account).await;
                }
                AsyncCommand::Unrepost { post, account } => {
                    handle_unrepost(&result_tx, post, account).await;
                }
                AsyncCommand::Post {
                    content,
                    accounts,
                    reply_to,
                } => {
                    handle_post(&result_tx, content, accounts, reply_to).await;
                }
                AsyncCommand::SchedulePost {
                    content,
                    networks,
                    scheduled_for,
                } => {
                    handle_schedule_post(&result_tx, content, networks, scheduled_for).await;
                }
                AsyncCommand::LoadImage { url } => {
                    handle_load_image(&result_tx, url).await;
                }
            }
        }
    });

    AsyncHandle { cmd_tx, result_rx }
}

async fn handle_refresh(result_tx: &mpsc::Sender<AsyncResult>, accounts: Vec<Account>) {
    // Fetch once per network with its default account; duplicate account rows
    // for the same network would otherwise show every post twice.
    let accounts = Account::default_for_networks(&accounts, Network::all());

    let _ = result_tx
        .send(AsyncResult::Status {
            message: "Refreshing...".to_string(),
        })
        .await;

    if accounts.is_empty() {
        let _ = result_tx
            .send(AsyncResult::Error {
                message: "No accounts configured".to_string(),
            })
            .await;
        return;
    }

    let mut all_posts = Vec::new();
    let mut errors = Vec::new();

    for account in &accounts {
        let token = match auth::get_credentials(account) {
            Ok(Some(t)) => t,
            Ok(None) => {
                errors.push(format!("No credentials for @{}", account.handle));
                continue;
            }
            Err(e) => {
                errors.push(format!("Auth error for @{}: {}", account.handle, e));
                continue;
            }
        };

        match fetch_timeline(result_tx, account, &token).await {
            Ok(posts) => {
                all_posts.extend(posts);
            }
            Err(e) => {
                errors.push(format!("@{}: {}", account.handle, e));
            }
        }
    }

    let all_posts = merge_timeline_posts(all_posts);

    if all_posts.is_empty() && !errors.is_empty() {
        let _ = result_tx
            .send(AsyncResult::Error {
                message: errors.join("; "),
            })
            .await;
    } else {
        let _ = result_tx
            .send(AsyncResult::TimelineRefreshed { posts: all_posts })
            .await;

        if !errors.is_empty() {
            let _ = result_tx
                .send(AsyncResult::Status {
                    message: format!("Partial refresh: {}", errors.join("; ")),
                })
                .await;
        }
    }
}

async fn handle_hashtag_timeline(
    result_tx: &mpsc::Sender<AsyncResult>,
    tag: String,
    accounts: Vec<Account>,
) {
    // Hashtag timelines are public: one account (the network's default) per
    // network is enough; querying more would only duplicate results.
    let accounts = Account::default_for_networks(&accounts, Network::all());
    let mut posts = Vec::new();
    let mut errors = Vec::new();

    for account in &accounts {
        let result = match auth::get_credentials(account) {
            Ok(Some(token)) => match connect(result_tx, account, &token).await {
                Ok(client) => client.hashtag_timeline(&tag, 40).await,
                Err(e) => Err(e),
            },
            Ok(None) => {
                errors.push(format!("No credentials for @{}", account.handle));
                continue;
            }
            Err(e) => Err(e),
        };

        match result {
            Ok(found) => posts.extend(found),
            Err(e) => errors.push(format!("@{}: {}", account.handle, e)),
        }
    }

    let posts = merge_timeline_posts(posts);

    let result = if posts.is_empty() && !errors.is_empty() {
        AsyncResult::Error {
            message: errors.join("; "),
        }
    } else {
        AsyncResult::HashtagLoaded { tag, posts }
    };
    let _ = result_tx.send(result).await;
}

/// Sort posts newest first and drop repeated timeline entries (same network,
/// post id and reposter). Reposts by different people are kept.
fn merge_timeline_posts(mut posts: Vec<Post>) -> Vec<Post> {
    posts.sort_by_key(|p| std::cmp::Reverse(p.created_at));
    let mut seen = std::collections::HashSet::new();
    posts.retain(|p| seen.insert((p.network, p.network_id.clone(), p.repost_author.clone())));
    posts
}

/// Create an API client. If it is a Bluesky session whose DID the account
/// doesn't have stored, report it so the main loop can record it (no extra
/// request: the DID comes from `createSession`).
async fn connect(
    result_tx: &mpsc::Sender<AsyncResult>,
    account: &Account,
    token: &str,
) -> Result<crate::api::Client> {
    let client = get_client(account, token).await?;
    if let Some(did) = client.session_did()
        && account.did.as_deref() != Some(did)
    {
        let _ = result_tx
            .send(AsyncResult::SessionDid {
                account_id: account.id,
                did: did.to_string(),
            })
            .await;
    }
    Ok(client)
}

async fn fetch_timeline(
    result_tx: &mpsc::Sender<AsyncResult>,
    account: &Account,
    token: &str,
) -> Result<Vec<Post>> {
    let client = connect(result_tx, account, token).await?;
    client.timeline(50).await
}

async fn handle_fetch_context(result_tx: &mpsc::Sender<AsyncResult>, post: Post, account: Account) {
    let token = match auth::get_credentials(&account) {
        Ok(Some(t)) => t,
        _ => {
            log_debug("No token for account");
            return;
        }
    };

    let client = match connect(result_tx, &account, &token).await {
        Ok(c) => c,
        Err(e) => {
            log_debug(&format!("Failed to get client: {}", e));
            return;
        }
    };

    match client.get_context(&post).await {
        Ok(flat_replies) => {
            log_debug(&format!(
                "Got {} flat replies for {}",
                flat_replies.len(),
                post.network_id
            ));
            if !flat_replies.is_empty() {
                log_debug(&format!(
                    "  First reply reply_to_id: {:?}",
                    flat_replies[0].reply_to_id
                ));
                log_debug(&format!("  Post uri: {:?}", post.uri));
            }
            // Build threaded reply list with depth
            let reply_items = build_reply_tree(&post, &flat_replies);
            log_debug(&format!("  Built {} reply items", reply_items.len()));

            let _ = result_tx
                .send(AsyncResult::ContextFetched {
                    post_id: post.network_id,
                    replies: reply_items,
                })
                .await;
        }
        Err(e) => {
            log_debug(&format!("Failed to fetch context: {}", e));
        }
    }
}

/// Build a flat list of replies with depth from a threaded structure
fn build_reply_tree(root_post: &Post, replies: &[Post]) -> Vec<ReplyItem> {
    let mut result = Vec::new();

    // Get the root URI - replies reference this, not just the network_id
    let root_uri = root_post.uri.as_deref().unwrap_or(&root_post.network_id);

    // Find direct replies to the root and recursively add their children
    fn add_replies(
        parent_uri: &str,
        parent_id: &str,
        all_replies: &[Post],
        result: &mut Vec<ReplyItem>,
        depth: usize,
    ) {
        for reply in all_replies {
            // Check both URI and network_id for parent match
            let is_reply_to_parent = reply.reply_to_id.as_deref() == Some(parent_uri)
                || reply.reply_to_id.as_deref() == Some(parent_id);

            if is_reply_to_parent {
                let reply_uri = reply.uri.as_deref().unwrap_or(&reply.network_id);
                result.push(ReplyItem {
                    post: reply.clone(),
                    depth,
                });
                // Recursively add replies to this reply
                add_replies(reply_uri, &reply.network_id, all_replies, result, depth + 1);
            }
        }
    }

    add_replies(root_uri, &root_post.network_id, replies, &mut result, 0);
    result
}

async fn handle_like(result_tx: &mpsc::Sender<AsyncResult>, post: Post, account: Account) {
    let token = match auth::get_credentials(&account) {
        Ok(Some(t)) => t,
        Ok(None) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: "No credentials".to_string(),
                })
                .await;
            return;
        }
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: e.to_string(),
                })
                .await;
            return;
        }
    };

    let client = match connect(result_tx, &account, &token).await {
        Ok(c) => c,
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: e.to_string(),
                })
                .await;
            return;
        }
    };

    match client.like(&post).await {
        Ok(()) => {
            let _ = result_tx
                .send(AsyncResult::Liked {
                    post_id: post.network_id,
                })
                .await;
        }
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: format!("Like failed: {e}"),
                })
                .await;
        }
    }
}

async fn handle_unlike(result_tx: &mpsc::Sender<AsyncResult>, post: Post, account: Account) {
    let token = match auth::get_credentials(&account) {
        Ok(Some(t)) => t,
        _ => return,
    };

    let client = match connect(result_tx, &account, &token).await {
        Ok(c) => c,
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: e.to_string(),
                })
                .await;
            return;
        }
    };

    match client.unlike(&post).await {
        Ok(()) => {
            let _ = result_tx
                .send(AsyncResult::Unliked {
                    post_id: post.network_id,
                })
                .await;
        }
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: format!("Unlike failed: {e}"),
                })
                .await;
        }
    }
}

async fn handle_repost(result_tx: &mpsc::Sender<AsyncResult>, post: Post, account: Account) {
    let token = match auth::get_credentials(&account) {
        Ok(Some(t)) => t,
        _ => return,
    };

    let client = match connect(result_tx, &account, &token).await {
        Ok(c) => c,
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: e.to_string(),
                })
                .await;
            return;
        }
    };

    match client.repost(&post).await {
        Ok(()) => {
            let _ = result_tx
                .send(AsyncResult::Reposted {
                    post_id: post.network_id,
                })
                .await;
        }
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: format!("Repost failed: {e}"),
                })
                .await;
        }
    }
}

async fn handle_unrepost(result_tx: &mpsc::Sender<AsyncResult>, post: Post, account: Account) {
    let token = match auth::get_credentials(&account) {
        Ok(Some(t)) => t,
        _ => return,
    };

    let client = match connect(result_tx, &account, &token).await {
        Ok(c) => c,
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: e.to_string(),
                })
                .await;
            return;
        }
    };

    match client.unrepost(&post).await {
        Ok(()) => {
            let _ = result_tx
                .send(AsyncResult::Unreposted {
                    post_id: post.network_id,
                })
                .await;
        }
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: format!("Unrepost failed: {e}"),
                })
                .await;
        }
    }
}

async fn handle_post(
    result_tx: &mpsc::Sender<AsyncResult>,
    content: String,
    accounts: Vec<Account>,
    reply_to: Option<Post>,
) {
    let action = if reply_to.is_some() {
        "Replying..."
    } else {
        "Posting..."
    };
    let _ = result_tx
        .send(AsyncResult::Status {
            message: format!("{} (to {} accounts)", action, accounts.len()),
        })
        .await;

    let mut posted = Vec::new();
    let mut errors = Vec::new();

    for account in &accounts {
        let token = match auth::get_credentials(account) {
            Ok(Some(t)) => t,
            Ok(None) => {
                errors.push(format!(
                    "No credentials for {} (@{})",
                    account.network.name(),
                    account.handle
                ));
                continue;
            }
            Err(e) => {
                errors.push(format!("Auth error for {}: {}", account.network.name(), e));
                continue;
            }
        };

        let client = match connect(result_tx, account, &token).await {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{}: {}", account.network.name(), e));
                continue;
            }
        };

        // Check if we're replying and this account matches the reply network
        let reply_id = reply_to
            .as_ref()
            .filter(|p| p.network == account.network)
            .map(|p| p.network_id.clone());

        let result = if let Some(ref reply_id) = reply_id {
            client.reply(&content, reply_id).await
        } else {
            client.post(&content).await
        };

        match result {
            Ok(post) => {
                posted.push(post);
            }
            Err(e) => {
                errors.push(format!("{}: {}", account.network.name(), e));
            }
        }
    }

    if !posted.is_empty() {
        let _ = result_tx.send(AsyncResult::Posted { posts: posted }).await;
    }

    let success_msg = if reply_to.is_some() {
        "Replied successfully!"
    } else {
        "Posted successfully!"
    };
    if errors.is_empty() {
        let _ = result_tx
            .send(AsyncResult::Status {
                message: success_msg.to_string(),
            })
            .await;
    } else {
        let _ = result_tx
            .send(AsyncResult::Error {
                message: errors.join("; "),
            })
            .await;
    }
}

async fn handle_schedule_post(
    result_tx: &mpsc::Sender<AsyncResult>,
    content: String,
    networks: Vec<Network>,
    scheduled_for: chrono::DateTime<chrono::Utc>,
) {
    let _ = result_tx
        .send(AsyncResult::Status {
            message: "Scheduling post...".to_string(),
        })
        .await;

    // Save to database
    let db = match crate::Database::open() {
        Ok(db) => db,
        Err(e) => {
            let _ = result_tx
                .send(AsyncResult::Error {
                    message: format!("Database error: {}", e),
                })
                .await;
            return;
        }
    };

    let scheduled_post = crate::ScheduledPost::new(content, networks, scheduled_for);
    let id = scheduled_post.id.to_string();
    let scheduled_for_str = scheduled_post.scheduled_time_display();
    let time_until = scheduled_post.time_until();

    if let Err(e) = db.save_scheduled_post(&scheduled_post) {
        let _ = result_tx
            .send(AsyncResult::Error {
                message: format!("Failed to schedule: {}", e),
            })
            .await;
        return;
    }

    let _ = result_tx
        .send(AsyncResult::Scheduled {
            id: id[..8].to_string(),
            scheduled_for: scheduled_for_str.clone(),
        })
        .await;

    let _ = result_tx
        .send(AsyncResult::Status {
            message: format!("📅 Scheduled for {} (in {})", scheduled_for_str, time_until),
        })
        .await;
}

/// Handle image loading from URL
/// HTTP client for image downloads, built once so connections are reused.
///
/// The 30s timeout matches `images::loader`; a timeout surfaces as a normal
/// `ImageFailed` result.
fn image_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            // GoToSocial rejects requests without a User-Agent (HTTP 418)
            .user_agent(format!("Perch/{}", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default()
    })
}

async fn handle_load_image(result_tx: &mpsc::Sender<AsyncResult>, url: String) {
    log_debug(&format!("Loading image: {}", url));

    let response = match image_client().get(&url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            log_debug(&format!("Failed to fetch image: {}", e));
            let _ = result_tx
                .send(AsyncResult::ImageFailed {
                    url,
                    error: e.to_string(),
                })
                .await;
            return;
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        log_debug(&format!("Image fetch failed with status: {}", status));
        let _ = result_tx
            .send(AsyncResult::ImageFailed {
                url,
                error: format!("HTTP {}", status),
            })
            .await;
        return;
    }

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            log_debug(&format!("Failed to read image bytes: {}", e));
            let _ = result_tx
                .send(AsyncResult::ImageFailed {
                    url,
                    error: e.to_string(),
                })
                .await;
            return;
        }
    };

    // Decode the image
    let image = match image::load_from_memory(&bytes) {
        Ok(img) => img,
        Err(e) => {
            log_debug(&format!("Failed to decode image: {}", e));
            let _ = result_tx
                .send(AsyncResult::ImageFailed {
                    url,
                    error: e.to_string(),
                })
                .await;
            return;
        }
    };

    // Optionally resize if too large
    let image = resize_if_needed(image);

    log_debug(&format!(
        "Image loaded successfully: {}x{}",
        image.width(),
        image.height()
    ));

    let _ = result_tx
        .send(AsyncResult::ImageLoaded { url, image })
        .await;
}

/// Resize image if it's too large (to save memory and rendering time).
fn resize_if_needed(image: image::DynamicImage) -> image::DynamicImage {
    const MAX_DIMENSION: u32 = 800;

    let (width, height) = (image.width(), image.height());

    if width <= MAX_DIMENSION && height <= MAX_DIMENSION {
        return image;
    }

    // Calculate new dimensions maintaining aspect ratio
    let ratio = f64::from(width) / f64::from(height);
    let (new_width, new_height) = if width > height {
        (MAX_DIMENSION, (f64::from(MAX_DIMENSION) / ratio) as u32)
    } else {
        ((f64::from(MAX_DIMENSION) * ratio) as u32, MAX_DIMENSION)
    };

    image.resize(new_width, new_height, image::imageops::FilterType::Triangle)
}

#[cfg(test)]
mod tests {
    use super::merge_timeline_posts;

    #[test]
    fn test_merge_timeline_posts_dedupes_and_sorts() {
        let posts = crate::demo::demo_posts();
        let mut boosted = posts[0].clone();
        boosted.is_repost = true;
        boosted.repost_author = Some("someone".to_string());

        // Same posts fetched twice (e.g. two accounts on one network) + a boost
        let mut input = posts.clone();
        input.extend(posts.iter().cloned());
        input.push(boosted);

        let merged = merge_timeline_posts(input);
        assert_eq!(merged.len(), posts.len() + 1);
        assert!(
            merged
                .windows(2)
                .all(|w| w[0].created_at >= w[1].created_at)
        );
    }
}
