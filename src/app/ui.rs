//! UI rendering

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, BorderType};

use super::state::{AppState, FocusedPanel, View};

/// Main render function
pub fn render(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();

    // Clear with background color
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(colors.bg)),
        area,
    );

    match state.view {
        View::Timeline => render_timeline(frame, state),
        View::Compose => render_compose(frame, state),
        View::Search => render_search(frame, state),
        View::Help => render_help(frame, state),
    }
}

fn render_timeline(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Layout: [Accounts 20%] [Timeline 40%] [Detail 40%]
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    let content_area = main_chunks[0];
    let status_area = main_chunks[1];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
        ])
        .split(content_area);

    // Accounts panel
    let accounts_block = Block::default()
        .title(" 👤 Accounts ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if state.focused_panel == FocusedPanel::Accounts {
            colors.block_focus()
        } else {
            colors.block()
        });

    let account_items: Vec<ListItem> = state
        .accounts
        .iter()
        .enumerate()
        .map(|(i, account)| {
            let icon = account.network.emoji();
            let content = format!("{} {} ({})", icon, account.display_name, account.handle);
            let style = if i == state.selected_account {
                colors.selected()
            } else {
                colors.text()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let accounts_list = List::new(account_items).block(accounts_block);
    frame.render_widget(accounts_list, horizontal[0]);

    // Timeline panel
    let filter_label = match state.timeline_filter {
        super::state::TimelineFilter::All => "🌐 All",
        super::state::TimelineFilter::Mastodon => "🐘 Mastodon",
        super::state::TimelineFilter::Bluesky => "🦋 Bluesky",
    };

    let timeline_block = Block::default()
        .title(format!(" 📰 Timeline ({}) ", filter_label))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if state.focused_panel == FocusedPanel::Timeline {
            colors.block_focus()
        } else {
            colors.block()
        });

    // Show loading or empty state
    let post_items: Vec<ListItem> = if state.loading && state.posts.is_empty() {
        vec![ListItem::new("  Loading...").style(colors.text_muted())]
    } else if state.posts.is_empty() {
        vec![ListItem::new("  No posts yet. Press 'r' to refresh.").style(colors.text_muted())]
    } else {
        state
            .posts
            .iter()
            .enumerate()
            .map(|(i, post)| {
                let icon = post.network.emoji();
                let preview = post.preview(50);
                let time = post.relative_time();
                // Show liked/reposted indicators
                let indicators = format!(
                    "{}{}",
                    if post.liked { "❤️" } else { "" },
                    if post.reposted { "🔁" } else { "" }
                );
                let content = format!(
                    "{} @{} · {} {}\n  {}",
                    icon, post.author_handle, time, indicators, preview
                );
                let style = if i == state.selected_post {
                    colors.selected()
                } else {
                    colors.text()
                };
                ListItem::new(content).style(style)
            })
            .collect()
    };

    let timeline_list = List::new(post_items).block(timeline_block);
    frame.render_widget(timeline_list, horizontal[1]);

    // Detail panel
    let detail_block = Block::default()
        .title(" 📝 Post ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if state.focused_panel == FocusedPanel::Detail {
            colors.block_focus()
        } else {
            colors.block()
        });

    let detail_content = if let Some(post) = state.selected_post() {
        let header = format!(
            "{} {} (@{})\n{}\n\n",
            post.network.emoji(),
            post.author_name,
            post.author_handle,
            post.relative_time()
        );
        // Show status icons for liked/reposted
        let like_icon = if post.liked { "❤️" } else { "♡" };
        let repost_icon = if post.reposted { "🔁" } else { "↻" };
        let stats = format!(
            "\n\n{} {}  {} {}  💬 {}",
            like_icon, post.like_count,
            repost_icon, post.repost_count,
            post.reply_count
        );
        format!("{}{}{}", header, post.content, stats)
    } else {
        "No post selected".to_string()
    };

    let detail = Paragraph::new(detail_content)
        .block(detail_block)
        .wrap(Wrap { trim: false })
        .style(colors.text());
    frame.render_widget(detail, horizontal[2]);

    // Status bar
    render_status_bar(frame, state, status_area);
}

fn render_compose(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Center the compose box
    let compose_area = centered_rect(60, 60, area);

    let networks_str: String = state
        .compose_networks
        .iter()
        .map(|n| n.emoji())
        .collect::<Vec<_>>()
        .join(" ");

    let compose_block = Block::default()
        .title(format!(" 📝 Compose → {} ", networks_str))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.block_focus());

    let char_count = state.compose_text.len();
    let text = format!(
        "{}\n\n─────────────────────────────────\n{} characters | Ctrl+Enter to post | Esc to cancel\nAlt+1: 🐘 Mastodon | Alt+2: 🦋 Bluesky",
        if state.compose_text.is_empty() {
            "What's on your mind?"
        } else {
            &state.compose_text
        },
        char_count
    );

    let compose = Paragraph::new(text)
        .block(compose_block)
        .wrap(Wrap { trim: false })
        .style(if state.compose_text.is_empty() {
            colors.text_muted()
        } else {
            colors.text()
        });

    // Clear area first
    frame.render_widget(Block::default().style(Style::default().bg(colors.bg)), compose_area);
    frame.render_widget(compose, compose_area);
}

fn render_search(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    let search_area = centered_rect(60, 20, area);

    let search_block = Block::default()
        .title(" 🔍 Search ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.block_focus());

    let text = format!(
        "{}\n\nEnter to search | Esc to cancel",
        if state.search_query.is_empty() {
            "Type to search..."
        } else {
            &state.search_query
        }
    );

    let search = Paragraph::new(text)
        .block(search_block)
        .style(colors.text());

    frame.render_widget(Block::default().style(Style::default().bg(colors.bg)), search_area);
    frame.render_widget(search, search_area);
}

fn render_help(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    let help_area = centered_rect(70, 80, area);

    let help_block = Block::default()
        .title(" ❓ Help ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.block_focus());

    let help_text = r#"
  NAVIGATION
    j/↓         Move down
    k/↑         Move up
    Tab         Switch panel
    g           Jump to top
    G           Jump to bottom

  ACTIONS
    n           New post (compose)
    /           Search
    r           Refresh timeline
    o           Open in browser
    l           Like/favorite
    b           Boost/repost

  VIEW
    f           Cycle filter (All/Mastodon/Bluesky)
    t           Change theme
    ?           Toggle help

  COMPOSE
    Ctrl+Enter  Post
    Alt+1       Toggle Mastodon
    Alt+2       Toggle Bluesky
    Esc         Cancel

  GENERAL
    q           Quit
    Esc         Close dialog / Clear status
"#;

    let help = Paragraph::new(help_text)
        .block(help_block)
        .style(colors.text());

    frame.render_widget(Block::default().style(Style::default().bg(colors.bg)), help_area);
    frame.render_widget(help, help_area);
}

fn render_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    // Spinner animation frames
    const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    let loading_indicator = if state.loading {
        let frame_idx = (state.current_tick() / 2) as usize % SPINNER.len();
        format!("{} ", SPINNER[frame_idx])
    } else {
        String::new()
    };

    let status_text = if state.status.is_empty() {
        format!(
            " {}🐦 Perch | {} | {} posts | ? for help | q to quit",
            loading_indicator,
            state.theme.name(),
            state.posts.len()
        )
    } else {
        format!(" {}{} ", loading_indicator, state.status)
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(colors.fg_muted).bg(colors.bg_secondary));

    frame.render_widget(status, area);
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
