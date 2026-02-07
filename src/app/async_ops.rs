//! Async operations for the TUI
//!
//! Uses channels to communicate between the sync TUI loop and async tasks.

use anyhow::Result;
use tokio::sync::mpsc;

use crate::api::get_client;
use crate::auth;
use crate::models::{Account, Post};
use super::state::ReplyItem;

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
    ContextFetched { post_id: String, replies: Vec<ReplyItem> },
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
                AsyncCommand::Post { content, accounts, reply_to } => {
                    handle_post(&result_tx, content, accounts, reply_to).await;
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
        _ => return,
    };

    let client = match get_client(&account, &token).await {
        Ok(c) => c,
        Err(_) => return,
    };

    match client.get_context(&post).await {
        Ok(flat_replies) => {
            // Build threaded reply list with depth
            let reply_items = build_reply_tree(&post.network_id, &flat_replies);
            
            let _ = result_tx
                .send(AsyncResult::ContextFetched {
                    post_id: post.network_id,
                    replies: reply_items,
                })
                .await;
        }
        Err(_) => {
            // Silently fail - replies are optional
        }
    }
}

/// Build a flat list of replies with depth from a threaded structure
fn build_reply_tree(root_id: &str, replies: &[Post]) -> Vec<ReplyItem> {
    let mut result = Vec::new();
    
    // Find direct replies to the root and recursively add their children
    fn add_replies(parent_id: &str, all_replies: &[Post], result: &mut Vec<ReplyItem>, depth: usize) {
        for reply in all_replies {
            if reply.reply_to_id.as_deref() == Some(parent_id) {
                result.push(ReplyItem {
                    post: reply.clone(),
                    depth,
                });
                // Recursively add replies to this reply
                add_replies(&reply.network_id, all_replies, result, depth + 1);
            }
        }
    }
    
    add_replies(root_id, replies, &mut result, 0);
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
                    message: format!("Like failed: {}", e),
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
                    message: format!("Unlike failed: {}", e),
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
                    message: format!("Repost failed: {}", e),
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
                    message: format!("Unrepost failed: {}", e),
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
    let action = if reply_to.is_some() { "Replying..." } else { "Posting..." };
    let _ = result_tx
        .send(AsyncResult::Status {
            message: action.to_string(),
        })
        .await;

    let mut posted = Vec::new();
    let mut errors = Vec::new();

    for account in &accounts {
        let token = match auth::get_credentials(account) {
            Ok(Some(t)) => {
                t
            },
            Ok(None) => {
                errors.push(format!("No credentials for {}", account.network.name()));
                continue;
            }
            _ => {
                errors.push(format!("No credentials for {}", account.network.name()));
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
        let reply_id = reply_to.as_ref()
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
        let _ = result_tx
            .send(AsyncResult::Posted { posts: posted })
            .await;
    }

    let success_msg = if reply_to.is_some() { "Replied successfully!" } else { "Posted successfully!" };
    if !errors.is_empty() {
        let _ = result_tx
            .send(AsyncResult::Error {
                message: errors.join("; "),
            })
            .await;
    } else {
        let _ = result_tx
            .send(AsyncResult::Status {
                message: success_msg.to_string(),
            })
            .await;
    }
}
