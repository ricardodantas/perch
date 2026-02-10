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
    ImageLoaded { url: String, image: image::DynamicImage },
    /// Image loading failed
    ImageFailed { url: String, error: String },
    /// An error occurred
    Error { message: String },
    /// Status message (for progress updates)
    Status { message: String },
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

        match fetch_timeline(account, &token).await {
            Ok(posts) => {
                all_posts.extend(posts);
            }
            Err(e) => {
                errors.push(format!("@{}: {}", account.handle, e));
            }
        }
    }

    // Sort by timestamp (newest first)
    all_posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

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

async fn fetch_timeline(account: &Account, token: &str) -> Result<Vec<Post>> {
    let client = get_client(account, token).await?;
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

    let client = match get_client(&account, &token).await {
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

    let client = match get_client(&account, &token).await {
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

    let client = match get_client(&account, &token).await {
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

    let client = match get_client(&account, &token).await {
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

    let client = match get_client(&account, &token).await {
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

        let client = match get_client(account, &token).await {
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
async fn handle_load_image(result_tx: &mpsc::Sender<AsyncResult>, url: String) {
    log_debug(&format!("Loading image: {}", url));
    
    // Download the image
    let response = match reqwest::get(&url).await {
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

    image.resize(
        new_width,
        new_height,
        image::imageops::FilterType::Triangle,
    )
}
