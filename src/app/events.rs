//! Event handling

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::async_ops::AsyncCommand;
use super::state::{AppState, FocusedPanel, Mode, View};
use crate::models::Network;
use crate::theme::Theme;

/// Handle key events, returning an optional async command
pub fn handle_key(state: &mut AppState, key: KeyEvent) -> Option<AsyncCommand> {
    // Handle mode-specific input first
    match state.mode {
        Mode::ThemePicker => {
            handle_theme_picker_key(state, key);
            return None;
        }
        Mode::Help => {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter) {
                state.mode = Mode::Normal;
            }
            return None;
        }
        Mode::About => {
            handle_about_key(state, key);
            return None;
        }
        Mode::Compose => {
            return handle_compose_key(state, key);
        }
        Mode::Search => {
            return handle_search_key(state, key);
        }
        Mode::Normal => {}
    }

    // Global shortcuts (work in normal mode)
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Char('q')) => {
            state.should_quit = true;
            return None;
        }
        (_, KeyCode::Char('?')) | (_, KeyCode::F(1)) => {
            state.mode = Mode::Help;
            return None;
        }
        (_, KeyCode::Tab) => {
            state.next_view();
            return None;
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            state.prev_view();
            return None;
        }
        // Number keys for quick navigation
        (_, KeyCode::Char('1')) => {
            state.view = View::Timeline;
            return None;
        }
        (_, KeyCode::Char('2')) => {
            state.view = View::Accounts;
            return None;
        }
        // Theme picker
        (_, KeyCode::Char('t')) => {
            state.theme_picker_index = Theme::all()
                .iter()
                .position(|t| *t == state.theme.inner())
                .unwrap_or(0);
            state.mode = Mode::ThemePicker;
            return None;
        }
        // About dialog
        (KeyModifiers::SHIFT, KeyCode::Char('A')) => {
            state.mode = Mode::About;
            return None;
        }
        _ => {}
    }

    // View-specific handling
    match state.view {
        View::Timeline => handle_timeline_key(state, key),
        View::Accounts => handle_accounts_key(state, key),
    }
}

fn handle_timeline_key(state: &mut AppState, key: KeyEvent) -> Option<AsyncCommand> {
    match (key.modifiers, key.code) {
        // Panel navigation (when in timeline view)
        (_, KeyCode::Left) | (_, KeyCode::Char('h')) => {
            state.focused_panel = state.focused_panel.prev();
            None
        }
        (_, KeyCode::Right) | (_, KeyCode::Char('l')) => {
            state.focused_panel = state.focused_panel.next();
            None
        }

        // Navigation within panel
        (_, KeyCode::Char('j') | KeyCode::Down) => {
            match state.focused_panel {
                FocusedPanel::Accounts => {
                    state.select_next_account();
                    None
                }
                FocusedPanel::Detail => {
                    // Navigate through replies (None -> 0 -> 1 -> ... -> max)
                    if state.current_replies.is_empty() {
                        // No replies, just scroll
                        state.detail_scroll = state.detail_scroll.saturating_add(1);
                    } else {
                        match state.selected_reply {
                            None => state.selected_reply = Some(0),
                            Some(i) if i < state.current_replies.len().saturating_sub(1) => {
                                state.selected_reply = Some(i + 1);
                            }
                            _ => {} // Already at last reply
                        }
                    }
                    None
                }
                FocusedPanel::Timeline => {
                    state.select_next_post();
                    // Fetch replies for newly selected post
                    if let Some(post) = state.selected_post().cloned() {
                        if let Some(account) = find_account_for_post(state, &post) {
                            return Some(AsyncCommand::FetchContext { post, account });
                        }
                    }
                    None
                }
            }
        }
        (_, KeyCode::Char('k') | KeyCode::Up) => {
            match state.focused_panel {
                FocusedPanel::Accounts => {
                    state.select_prev_account();
                    None
                }
                FocusedPanel::Detail => {
                    // Navigate through replies (max -> ... -> 1 -> 0 -> None)
                    if state.current_replies.is_empty() {
                        // No replies, just scroll
                        state.detail_scroll = state.detail_scroll.saturating_sub(1);
                    } else {
                        match state.selected_reply {
                            Some(0) => state.selected_reply = None,
                            Some(i) => state.selected_reply = Some(i - 1),
                            None => {} // Already at main post
                        }
                    }
                    None
                }
                FocusedPanel::Timeline => {
                    state.select_prev_post();
                    // Fetch replies for newly selected post
                    if let Some(post) = state.selected_post().cloned() {
                        if let Some(account) = find_account_for_post(state, &post) {
                            return Some(AsyncCommand::FetchContext { post, account });
                        }
                    }
                    None
                }
            }
        }

        // Scroll detail panel (Shift+J/K or Ctrl+D/U)
        (KeyModifiers::SHIFT, KeyCode::Char('J')) => {
            if state.focused_panel == FocusedPanel::Detail {
                state.detail_scroll = state.detail_scroll.saturating_add(3);
            }
            None
        }
        (KeyModifiers::SHIFT, KeyCode::Char('K')) => {
            if state.focused_panel == FocusedPanel::Detail {
                state.detail_scroll = state.detail_scroll.saturating_sub(3);
            }
            None
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            if state.focused_panel == FocusedPanel::Detail {
                state.detail_scroll = state.detail_scroll.saturating_add(10);
            }
            None
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            if state.focused_panel == FocusedPanel::Detail {
                state.detail_scroll = state.detail_scroll.saturating_sub(10);
            }
            None
        }

        // Jump to top/bottom
        (_, KeyCode::Char('g')) => {
            state.selected_post = 0;
            None
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            if !state.posts.is_empty() {
                state.selected_post = state.posts.len() - 1;
            }
            None
        }

        // Actions
        (_, KeyCode::Char('n')) => {
            state.open_compose();
            None
        }
        (_, KeyCode::Char('/')) => {
            state.mode = Mode::Search;
            None
        }
        (_, KeyCode::Char('b')) => {
            // Refresh timeline (b for "buffer refresh")
            if !state.loading && !state.accounts.is_empty() {
                state.loading = true;
                state.set_status("Refreshing...");
                Some(AsyncCommand::RefreshTimeline {
                    accounts: state.accounts.clone(),
                })
            } else {
                None
            }
        }
        (KeyModifiers::SHIFT, KeyCode::Char('R')) => {
            // Reply to selected post or reply
            let reply_target = if state.focused_panel == FocusedPanel::Detail {
                // If a reply is selected, reply to that reply
                if let Some(idx) = state.selected_reply {
                    state.current_replies.get(idx).map(|r| r.post.clone())
                } else {
                    // Reply to main post
                    state.selected_post().cloned()
                }
            } else {
                // Reply to main post
                state.selected_post().cloned()
            };
            
            if let Some(post) = reply_target {
                state.open_reply(post);
            }
            None
        }
        (_, KeyCode::Char('o')) => {
            // Open selected post in browser
            if let Some(post) = state.selected_post() {
                if let Some(url) = &post.url {
                    let _ = open::that(url);
                    state.set_status("✓ Opened in browser");
                }
            }
            None
        }
        (KeyModifiers::NONE, KeyCode::Char('L')) | (KeyModifiers::SHIFT, KeyCode::Char('L')) => {
            // Like/favorite
            if let Some(post) = state.selected_post().cloned() {
                if let Some(account) = find_account_for_post(state, &post) {
                    if post.liked {
                        state.set_status("Unliking...");
                        return Some(AsyncCommand::Unlike { post, account });
                    } else {
                        state.set_status("Liking...");
                        return Some(AsyncCommand::Like { post, account });
                    }
                } else {
                    state.set_status("⚠ No matching account for this network");
                }
            }
            None
        }
        (_, KeyCode::Char('r')) => {
            // Repost/boost (toggle)
            if let Some(post) = state.selected_post().cloned() {
                if let Some(account) = find_account_for_post(state, &post) {
                    if post.reposted {
                        state.set_status("Undoing repost...");
                        return Some(AsyncCommand::Unrepost { post, account });
                    } else {
                        state.set_status("Reposting...");
                        return Some(AsyncCommand::Repost { post, account });
                    }
                } else {
                    state.set_status("⚠ No matching account for this network");
                }
            }
            None
        }

        // Filter
        (_, KeyCode::Char('f')) => {
            state.cycle_filter();
            state.set_status(format!("Filter: {}", state.timeline_filter.name()));
            None
        }

        // Enter detail view
        (_, KeyCode::Enter) if state.focused_panel == FocusedPanel::Timeline => {
            state.focused_panel = FocusedPanel::Detail;
            None
        }

        (_, KeyCode::Esc) => {
            state.clear_status();
            None
        }

        _ => None,
    }
}

fn handle_accounts_key(state: &mut AppState, key: KeyEvent) -> Option<AsyncCommand> {
    match (key.modifiers, key.code) {
        (_, KeyCode::Down | KeyCode::Char('j')) => {
            state.select_next_account();
            None
        }
        (_, KeyCode::Up | KeyCode::Char('k')) => {
            state.select_prev_account();
            None
        }
        (_, KeyCode::Char('g')) => {
            state.selected_account = 0;
            None
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            if !state.accounts.is_empty() {
                state.selected_account = state.accounts.len() - 1;
            }
            None
        }
        (_, KeyCode::Char('d')) => {
            // Set as default account for this network
            if let Some(account) = state.accounts.get(state.selected_account) {
                let account_id = account.id;
                let network = account.network;
                if let Err(e) = state.db.set_default_account(account_id, network) {
                    state.set_status(format!("❌ Failed to set default: {}", e));
                } else {
                    // Reload accounts to reflect change
                    if let Ok(accounts) = state.db.get_accounts() {
                        state.accounts = accounts;
                    }
                    state.set_status("★ Set as default account");
                }
            }
            None
        }
        (KeyModifiers::SHIFT, KeyCode::Char('D')) => {
            // Delete account
            if let Some(account) = state.accounts.get(state.selected_account) {
                let account_id = account.id;
                let handle = account.handle.clone();
                if let Err(e) = state.db.delete_account(account_id) {
                    state.set_status(format!("❌ Failed to delete: {}", e));
                } else {
                    // Remove credentials
                    let _ = crate::auth::delete_credentials(&state.accounts[state.selected_account]);
                    // Reload accounts
                    if let Ok(accounts) = state.db.get_accounts() {
                        state.accounts = accounts;
                        if state.selected_account >= state.accounts.len() && !state.accounts.is_empty() {
                            state.selected_account = state.accounts.len() - 1;
                        }
                    }
                    state.set_status(format!("🗑 Deleted @{}", handle));
                }
            }
            None
        }
        (_, KeyCode::Enter) => {
            // Switch to timeline view filtered by this account's network
            if let Some(account) = state.accounts.get(state.selected_account) {
                state.timeline_filter = match account.network {
                    crate::models::Network::Mastodon => crate::app::state::TimelineFilter::Mastodon,
                    crate::models::Network::Bluesky => crate::app::state::TimelineFilter::Bluesky,
                };
                state.view = crate::app::state::View::Timeline;
                state.focused_panel = FocusedPanel::Timeline;
                state.set_status(format!("Viewing {} timeline", account.network.emoji()));
            }
            None
        }
        (_, KeyCode::Esc) => {
            state.clear_status();
            None
        }
        _ => None,
    }
}

fn handle_compose_key(state: &mut AppState, key: KeyEvent) -> Option<AsyncCommand> {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => {
            state.close_compose();
            None
        }
        // Ctrl+Enter or Ctrl+S or Tab to post
        (KeyModifiers::CONTROL, KeyCode::Enter) 
        | (KeyModifiers::CONTROL, KeyCode::Char('s')) 
        | (_, KeyCode::Tab) => {
            // Post
            if !state.compose_text.is_empty() && !state.compose_networks.is_empty() {
                let content = state.compose_text.clone();
                let reply_to = state.reply_to.clone();
                // Find accounts matching selected networks
                let accounts: Vec<_> = state
                    .accounts
                    .iter()
                    .filter(|a| state.compose_networks.contains(&a.network))
                    .cloned()
                    .collect();
                
                if accounts.is_empty() {
                    state.set_status("⚠ No accounts for selected networks");
                    return None;
                }
                
                state.loading = true;
                state.close_compose();
                Some(AsyncCommand::Post { content, accounts, reply_to })
            } else {
                if state.compose_text.is_empty() {
                    state.set_status("⚠ Write something first!");
                } else {
                    state.set_status("⚠ Select at least one network");
                }
                None
            }
        }
        (_, KeyCode::F(1)) => {
            state.toggle_compose_network(Network::Mastodon);
            None
        }
        (_, KeyCode::F(2)) => {
            state.toggle_compose_network(Network::Bluesky);
            None
        }
        (_, KeyCode::Char(c)) => {
            state.compose_text.push(c);
            None
        }
        (_, KeyCode::Backspace) => {
            state.compose_text.pop();
            None
        }
        (_, KeyCode::Enter) => {
            state.compose_text.push('\n');
            None
        }
        _ => None,
    }
}

fn handle_search_key(state: &mut AppState, key: KeyEvent) -> Option<AsyncCommand> {
    match key.code {
        KeyCode::Esc => {
            state.mode = Mode::Normal;
            state.search_query.clear();
        }
        KeyCode::Enter => {
            if !state.search_query.is_empty() {
                // Filter posts by search query (local search)
                let query = state.search_query.to_lowercase();
                let filtered: Vec<_> = state.posts.iter()
                    .filter(|p| {
                        p.content.to_lowercase().contains(&query) ||
                        p.author_handle.to_lowercase().contains(&query) ||
                        p.author_name.to_lowercase().contains(&query)
                    })
                    .cloned()
                    .collect();
                
                let count = filtered.len();
                state.posts = filtered;
                state.selected_post = 0;
                state.mode = Mode::Normal;
                state.set_status(format!("✓ Found {} posts matching '{}'", count, state.search_query));
                state.search_query.clear();
            }
        }
        KeyCode::Char(c) => {
            state.search_query.push(c);
        }
        KeyCode::Backspace => {
            state.search_query.pop();
        }
        _ => {}
    }
    None
}

fn handle_theme_picker_key(state: &mut AppState, key: KeyEvent) {
    let themes = Theme::all();
    let len = themes.len();

    match key.code {
        KeyCode::Esc => {
            // Cancel - restore original theme
            state.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            // Apply selected theme
            let selected_theme = Theme::from(themes[state.theme_picker_index]);
            state.theme = selected_theme;
            state.config.theme = selected_theme;

            state.mode = Mode::Normal;
            state.set_status(format!("✓ Theme set to {}", selected_theme.name()));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.theme_picker_index = (state.theme_picker_index + 1) % len;
            // Preview theme
            state.theme = Theme::from(themes[state.theme_picker_index]);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.theme_picker_index = state.theme_picker_index.checked_sub(1).unwrap_or(len - 1);
            // Preview theme
            state.theme = Theme::from(themes[state.theme_picker_index]);
        }
        KeyCode::Home | KeyCode::Char('g') => {
            state.theme_picker_index = 0;
            state.theme = Theme::from(themes[state.theme_picker_index]);
        }
        KeyCode::End | KeyCode::Char('G') => {
            state.theme_picker_index = len - 1;
            state.theme = Theme::from(themes[state.theme_picker_index]);
        }
        _ => {}
    }
}

fn handle_about_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
            state.mode = Mode::Normal;
        }
        KeyCode::Char('g') | KeyCode::Char('G') => {
            // Open GitHub repository
            let _ = open::that("https://github.com/ricardodantas/perch");
        }
        _ => {}
    }
}

/// Find an account that matches the network of a post
fn find_account_for_post(state: &AppState, post: &crate::models::Post) -> Option<crate::models::Account> {
    state
        .accounts
        .iter()
        .find(|a| a.network == post.network)
        .cloned()
}
