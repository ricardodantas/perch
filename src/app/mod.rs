//! TUI Application module

mod async_ops;
mod events;
mod state;
mod ui;

pub use state::AppState;
pub use state::FocusedPanel;
pub use state::ReplyItem;

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io::stdout;
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::config::Config;
use crate::db::Database;
use crate::demo;

use async_ops::{AsyncCommand, AsyncHandle, AsyncResult, spawn_worker};

/// Run the TUI application
pub fn run() -> Result<()> {
    // Create tokio runtime
    let rt = Runtime::new()?;

    // Load config
    let config = Config::load()?;

    // Open database
    let db = Database::open()?;

    // Spawn async worker
    let async_handle = rt.block_on(async { spawn_worker() });

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Create app state
    let mut state = AppState::new(config, db)?;

    // Trigger initial refresh if we have accounts
    if !state.accounts.is_empty() {
        let _ = async_handle
            .cmd_tx
            .blocking_send(AsyncCommand::RefreshTimeline {
                accounts: state.accounts.clone(),
            });
        state.loading = true;
        state.set_status("Loading timeline...");
    }

    // Spawn background update check
    std::thread::spawn(|| {
        if let crate::VersionCheck::UpdateAvailable { latest, .. } =
            crate::check_for_updates_crates_io()
        {
            // We can't easily send to main thread without channels,
            // but we can use a static or file. For simplicity, we'll check in the main loop.
            // This thread just warms up the check.
            let _ = latest;
        }
    });

    // Main loop
    let result = run_app(&mut terminal, &mut state, async_handle, &rt);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Background message types
enum BackgroundMsg {
    UpdateAvailable(String),
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
    mut async_handle: AsyncHandle,
    _rt: &Runtime,
) -> Result<()> {
    // Channel for background messages
    let (bg_tx, bg_rx) = std::sync::mpsc::channel::<BackgroundMsg>();

    // Spawn background update check
    std::thread::spawn(move || {
        if let crate::VersionCheck::UpdateAvailable { latest, .. } =
            crate::check_for_updates_crates_io()
        {
            let _ = bg_tx.send(BackgroundMsg::UpdateAvailable(latest));
        }
    });

    loop {
        // Check for background messages (non-blocking)
        if let Ok(msg) = bg_rx.try_recv() {
            match msg {
                BackgroundMsg::UpdateAvailable(version) => {
                    state.set_update_available(version);
                }
            }
        }

        // Process any async results
        while let Ok(result) = async_handle.result_rx.try_recv() {
            if let Some(cmd) = handle_async_result(state, result) {
                let _ = async_handle.cmd_tx.blocking_send(cmd);
            }
        }

        // Draw UI
        terminal.draw(|frame| ui::render(frame, state))?;

        // Process pending update after draw (so "Updating..." is visible)
        if state.pending_update {
            events::process_pending_update(state);
            // Redraw immediately after update completes
            terminal.draw(|frame| ui::render(frame, state))?;
        }

        // Handle events
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && let Some(cmd) = events::handle_key(state, key)
        {
            let _ = async_handle.cmd_tx.blocking_send(cmd);
        }

        // Queue image loading for current post
        let images_to_load = state.get_images_to_load();
        if !images_to_load.is_empty() {
            state.mark_images_loading(&images_to_load);
            for url in images_to_load {
                let _ = async_handle
                    .cmd_tx
                    .blocking_send(AsyncCommand::LoadImage { url });
            }
        }

        // Tick for animations
        state.tick();

        if state.should_quit {
            // Shutdown async worker
            let _ = async_handle.cmd_tx.blocking_send(AsyncCommand::Shutdown);
            break;
        }
    }

    // Save config on exit
    state.config.save()?;

    Ok(())
}

fn handle_async_result(state: &mut AppState, result: AsyncResult) -> Option<AsyncCommand> {
    match result {
        AsyncResult::TimelineRefreshed { posts } => {
            // Cache posts to database
            for post in &posts {
                let _ = state.db.cache_post(post);
            }
            state.posts = posts;
            state.selected_post = 0;
            state.loading = false;
            state.set_status(format!("Loaded {} posts", state.posts.len()));

            // Fetch replies for the first post
            if let Some(post) = state.selected_post().cloned()
                && let Some(account) = state.accounts.iter().find(|a| a.network == post.network)
            {
                return Some(AsyncCommand::FetchContext {
                    post,
                    account: account.clone(),
                });
            }
            None
        }
        AsyncResult::ContextFetched {
            post_id: _,
            replies,
        } => {
            state.current_replies = replies;
            state.loading_replies = false;
            None
        }
        AsyncResult::Liked { post_id } => {
            // Update the post in our local state
            if let Some(post) = state.posts.iter_mut().find(|p| p.network_id == post_id) {
                post.liked = true;
                post.like_count += 1;
            }
            state.set_status("❤️ Liked!");
            None
        }
        AsyncResult::Unliked { post_id } => {
            if let Some(post) = state.posts.iter_mut().find(|p| p.network_id == post_id) {
                post.liked = false;
                post.like_count = post.like_count.saturating_sub(1);
            }
            state.set_status("💔 Unliked");
            None
        }
        AsyncResult::Reposted { post_id } => {
            if let Some(post) = state.posts.iter_mut().find(|p| p.network_id == post_id) {
                post.reposted = true;
                post.repost_count += 1;
            }
            state.set_status("🔁 Reposted!");
            None
        }
        AsyncResult::Unreposted { post_id } => {
            if let Some(post) = state.posts.iter_mut().find(|p| p.network_id == post_id) {
                post.reposted = false;
                post.repost_count = post.repost_count.saturating_sub(1);
            }
            state.set_status("↩️ Unreposted");
            None
        }
        AsyncResult::Posted { posts } => {
            let networks: Vec<_> = posts.iter().map(|p| p.network.emoji()).collect();
            state.set_status(format!("✅ Posted to {}", networks.join(" ")));
            state.loading = false;
            None
        }
        AsyncResult::Scheduled { id, scheduled_for } => {
            state.set_status(format!("📅 Scheduled [{}] for {}", id, scheduled_for));
            state.loading = false;
            None
        }
        AsyncResult::Error { message } => {
            state.set_status(format!("❌ {message}"));
            state.loading = false;
            None
        }
        AsyncResult::Status { message } => {
            state.set_status(message);
            None
        }
        AsyncResult::ImageLoaded { url, image } => {
            state.loading_images.remove(&url);
            state.image_cache.insert(&url, image);
            // No status message - images load quietly
            None
        }
        AsyncResult::ImageFailed { url, error } => {
            state.loading_images.remove(&url);
            tracing::warn!("Failed to load image {}: {}", url, error);
            // Don't show error in status bar - would be too noisy
            None
        }
    }
}

/// Run the TUI in demo mode with mock data (for screenshots)
pub fn run_demo() -> Result<()> {
    // Load config
    let config = Config::load()?;

    // Open database
    let db = Database::open()?;

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Create app state with demo data
    let mut state = AppState::new(config, db)?;
    state.accounts = demo::demo_accounts();
    state.posts = demo::demo_posts();
    state.focused_panel = state::FocusedPanel::Timeline;
    // Use Dracula theme for screenshots
    state.theme = crate::theme::Theme(ratatui_themes::ThemeName::Dracula);
    state.set_status(format!(
        "Demo mode | {} posts | Press ? for help | q to quit",
        state.posts.len()
    ));

    // Main loop (simpler, no async)
    loop {
        // Draw UI
        terminal.draw(|frame| ui::render(frame, &state))?;

        // Handle events
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            // Simple event handling for demo
            events::handle_key(&mut state, key);
        }

        // Tick for animations
        state.tick();

        if state.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Save config on exit
    state.config.save()?;

    Ok(())
}
