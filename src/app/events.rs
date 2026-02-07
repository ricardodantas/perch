//! Event handling

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::async_ops::AsyncCommand;
use super::state::{AppState, FocusedPanel, View};
use crate::models::Network;

/// Handle key events, returning an optional async command
pub fn handle_key(state: &mut AppState, key: KeyEvent) -> Option<AsyncCommand> {
    // Global shortcuts (work in any view)
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Char('q')) => {
            if state.view == View::Timeline {
                state.should_quit = true;
                return None;
            }
        }
        (_, KeyCode::Char('?')) => {
            state.view = if state.view == View::Help {
                View::Timeline
            } else {
                View::Help
            };
            return None;
        }
        _ => {}
    }

    // View-specific handling
    match state.view {
        View::Timeline => handle_timeline_key(state, key),
        View::Compose => handle_compose_key(state, key),
        View::Search => handle_search_key(state, key),
        View::Help => handle_help_key(state, key),
    }
}

fn handle_timeline_key(state: &mut AppState, key: KeyEvent) -> Option<AsyncCommand> {
    match (key.modifiers, key.code) {
        // Quit
        (_, KeyCode::Char('q')) => {
            state.should_quit = true;
            None
        }

        // Navigation
        (_, KeyCode::Tab) => {
            state.focused_panel = state.focused_panel.next();
            None
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            state.focused_panel = state.focused_panel.prev();
            None
        }

        // Panel-specific navigation
        (_, KeyCode::Char('j') | KeyCode::Down) => {
            match state.focused_panel {
                FocusedPanel::Accounts => state.select_next_account(),
                FocusedPanel::Timeline | FocusedPanel::Detail => state.select_next_post(),
            }
            None
        }
        (_, KeyCode::Char('k') | KeyCode::Up) => {
            match state.focused_panel {
                FocusedPanel::Accounts => state.select_prev_account(),
                FocusedPanel::Timeline | FocusedPanel::Detail => state.select_prev_post(),
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
            state.view = View::Search;
            None
        }
        (_, KeyCode::Char('r')) => {
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
        (_, KeyCode::Char('o')) => {
            // Open selected post in browser
            if let Some(post) = state.selected_post() {
                if let Some(url) = &post.url {
                    let _ = open::that(url);
                    state.set_status("Opened in browser");
                }
            }
            None
        }
        (_, KeyCode::Char('l')) => {
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
                    state.set_status("No matching account for this network");
                }
            }
            None
        }
        (_, KeyCode::Char('b')) => {
            // Boost/repost
            if let Some(post) = state.selected_post().cloned() {
                if let Some(account) = find_account_for_post(state, &post) {
                    state.set_status("Reposting...");
                    return Some(AsyncCommand::Repost { post, account });
                } else {
                    state.set_status("No matching account for this network");
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

        // Theme
        (_, KeyCode::Char('t')) => {
            state.next_theme();
            state.set_status(format!("Theme: {}", state.theme.name()));
            None
        }

        // Enter detail view
        (_, KeyCode::Enter) if state.focused_panel == FocusedPanel::Timeline => {
            state.focused_panel = FocusedPanel::Detail;
            None
        }

        // Back from detail
        (_, KeyCode::Char('h') | KeyCode::Left) if state.focused_panel == FocusedPanel::Detail => {
            state.focused_panel = FocusedPanel::Timeline;
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
        (KeyModifiers::CONTROL, KeyCode::Enter) => {
            // Post
            if !state.compose_text.is_empty() && !state.compose_networks.is_empty() {
                let content = state.compose_text.clone();
                // Find accounts matching selected networks
                let accounts: Vec<_> = state
                    .accounts
                    .iter()
                    .filter(|a| state.compose_networks.contains(&a.network))
                    .cloned()
                    .collect();
                
                if accounts.is_empty() {
                    state.set_status("No accounts for selected networks");
                    return None;
                }
                
                state.loading = true;
                state.close_compose();
                Some(AsyncCommand::Post { content, accounts })
            } else {
                if state.compose_text.is_empty() {
                    state.set_status("Write something first!");
                } else {
                    state.set_status("Select at least one network");
                }
                None
            }
        }
        (KeyModifiers::ALT, KeyCode::Char('1')) => {
            state.toggle_compose_network(Network::Mastodon);
            None
        }
        (KeyModifiers::ALT, KeyCode::Char('2')) => {
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
            state.view = View::Timeline;
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
                state.view = View::Timeline;
                state.set_status(format!("Found {} posts matching '{}'", count, state.search_query));
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

fn handle_help_key(state: &mut AppState, key: KeyEvent) -> Option<AsyncCommand> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            state.view = View::Timeline;
        }
        _ => {}
    }
    None
}

/// Find an account that matches the network of a post
fn find_account_for_post(state: &AppState, post: &crate::models::Post) -> Option<crate::models::Account> {
    state
        .accounts
        .iter()
        .find(|a| a.network == post.network)
        .cloned()
}
