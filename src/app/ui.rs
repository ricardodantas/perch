//! UI rendering for the TUI

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

use super::state::{AppState, FocusedPanel, Mode, TimelineFilter, View};
use crate::theme::Theme;

/// ASCII art logo for Perch
const LOGO: &str = r#"
██████╗ ███████╗██████╗  ██████╗██╗  ██╗
██╔══██╗██╔════╝██╔══██╗██╔════╝██║  ██║
██████╔╝█████╗  ██████╔╝██║     ███████║
██╔═══╝ ██╔══╝  ██╔══██╗██║     ██╔══██║
██║     ███████╗██║  ██║╚██████╗██║  ██║
╚═╝     ╚══════╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝
"#;

/// Perch icon
const ICON: &str = "🐦";

/// Main render function
pub fn render(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();

    // Set background
    let area = frame.area();
    let bg_block = Block::default().style(Style::default().bg(colors.bg));
    frame.render_widget(bg_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    render_tabs(frame, state, chunks[0]);
    render_main(frame, state, chunks[1]);
    render_status_bar(frame, state, chunks[2]);

    // Render modal dialogs
    match state.mode {
        Mode::Help => render_help_popup(frame, state),
        Mode::ThemePicker => render_theme_picker(frame, state),
        Mode::About => render_about_dialog(frame, state),
        Mode::Compose => render_compose_popup(frame, state),
        Mode::Search => render_search_popup(frame, state),
        Mode::Normal => {}
    }
}

fn render_tabs(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    let titles: Vec<Line> = vec![
        format!(
            "{}  Timeline",
            if state.view == View::Timeline {
                "●"
            } else {
                "○"
            }
        ),
        format!(
            "{}  Accounts",
            if state.view == View::Accounts {
                "●"
            } else {
                "○"
            }
        ),
    ]
    .into_iter()
    .map(Line::from)
    .collect();

    let selected = match state.view {
        View::Timeline => 0,
        View::Accounts => 1,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(colors.block())
                .title(format!(" {} Perch ", ICON))
                .title_style(colors.logo_style_primary()),
        )
        .select(selected)
        .style(colors.tab())
        .highlight_style(colors.tab_active())
        .divider(Span::styled(" │ ", colors.text_muted()));

    frame.render_widget(tabs, area);
}

fn render_main(frame: &mut Frame, state: &AppState, area: Rect) {
    match state.view {
        View::Timeline => render_timeline_view(frame, state, area),
        View::Accounts => render_accounts_view(frame, state, area),
    }
}

fn render_timeline_view(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    // Layout: [Accounts 20%] [Timeline 40%] [Detail 40%]
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
        ])
        .split(area);

    // Accounts panel (sidebar)
    let accounts_block = Block::default()
        .title(" 👤 Accounts ")
        .title_style(colors.text_primary())
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
            let content = format!("{} {}", icon, account.display_name);
            let style = if i == state.selected_account {
                colors.selected()
            } else {
                colors.text()
            };
            ListItem::new(Line::from(vec![
                Span::styled(if i == state.selected_account { " ▸ " } else { "   " }, style),
                Span::styled(content, style),
            ]))
        })
        .collect();

    if account_items.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::styled("  No accounts", colors.text_muted()),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Run ", colors.text_dim()),
                Span::styled("perch auth", colors.text_primary()),
            ]),
        ])
        .block(accounts_block);
        frame.render_widget(empty, horizontal[0]);
    } else {
        let accounts_list = List::new(account_items).block(accounts_block);
        frame.render_widget(accounts_list, horizontal[0]);
    }

    // Timeline panel
    let filter_label = match state.timeline_filter {
        TimelineFilter::All => "🌐 All",
        TimelineFilter::Mastodon => "🐘 Mastodon",
        TimelineFilter::Bluesky => "🦋 Bluesky",
    };

    let timeline_block = Block::default()
        .title(format!(" 📰 Timeline ({}) ", filter_label))
        .title_style(colors.text_primary())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if state.focused_panel == FocusedPanel::Timeline {
            colors.block_focus()
        } else {
            colors.block()
        });

    // Show loading or empty state
    let post_items: Vec<ListItem> = if state.loading && state.posts.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("⏳ Loading...", colors.text_muted()),
        ]))]
    } else if state.posts.is_empty() {
        vec![
            ListItem::new(Line::from("")),
            ListItem::new(Line::from(vec![
                Span::styled("  ℹ ", colors.text_info()),
                Span::styled("No posts yet", colors.text_muted()),
            ])),
            ListItem::new(Line::from("")),
            ListItem::new(Line::from(vec![
                Span::styled("  Press ", colors.text_dim()),
                Span::styled("[r]", colors.key_hint()),
                Span::styled(" to refresh", colors.text_dim()),
            ])),
        ]
    } else {
        state
            .posts
            .iter()
            .enumerate()
            .map(|(i, post)| {
                let icon = post.network.emoji();
                let preview = post.preview(38);
                let time = post.relative_time();
                
                // Status indicators
                let mut indicators = String::new();
                if post.liked { indicators.push_str(" ❤️"); }
                if post.reposted { indicators.push_str(" 🔁"); }
                
                let is_selected = i == state.selected_post;
                let width = horizontal[1].width.saturating_sub(3) as usize;
                
                // Full-width background for selected item
                let base_style = if is_selected {
                    colors.selected()
                } else {
                    Style::default()
                };
                
                let author_text = format!(" {} @{} · {}{}", icon, post.author_handle, time, indicators);
                let content_text = format!("   {}", preview);
                
                // Pad lines to full width for selection highlight
                let author_padded = format!("{:<width$}", author_text, width = width);
                let content_padded = format!("{:<width$}", content_text, width = width);
                let spacer = format!("{:<width$}", "", width = width);

                ListItem::new(vec![
                    Line::styled(author_padded, base_style.patch(colors.text_primary())),
                    Line::styled(content_padded, base_style.patch(colors.text())),
                    Line::styled(spacer, Style::default()), // Spacer between posts
                ])
            })
            .collect()
    };

    let timeline_list = List::new(post_items).block(timeline_block);
    frame.render_widget(timeline_list, horizontal[1]);

    // Detail panel
    let detail_block = Block::default()
        .title(" 📝 Post Detail ")
        .title_style(colors.text_primary())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if state.focused_panel == FocusedPanel::Detail {
            colors.block_focus()
        } else {
            colors.block()
        });

    if let Some(post) = state.selected_post() {
        let like_icon = if post.liked { "❤️" } else { "♡" };
        let repost_icon = if post.reposted { "🔁" } else { "↻" };
        
        let detail_content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(post.network.emoji(), Style::default()),
                Span::styled(format!(" {} ", post.author_name), colors.text_primary().add_modifier(Modifier::BOLD)),
                Span::styled(format!("@{}", post.author_handle), colors.text_muted()),
            ]),
            Line::from(vec![
                Span::styled(format!("     {}", post.relative_time()), colors.text_muted()),
            ]),
            Line::from(""),
            // Content lines
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&post.content, colors.text()),
            ]),
            Line::from(""),
            Line::from("  ─────────────────────────────────"),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("{} {}", like_icon, post.like_count), 
                    if post.liked { colors.text_error() } else { colors.text_muted() }),
                Span::styled("   ", Style::default()),
                Span::styled(format!("{} {}", repost_icon, post.repost_count),
                    if post.reposted { colors.text_success() } else { colors.text_muted() }),
                Span::styled("   ", Style::default()),
                Span::styled(format!("💬 {}", post.reply_count), colors.text_muted()),
            ]),
        ];

        let detail = Paragraph::new(detail_content)
            .block(detail_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(detail, horizontal[2]);
    } else {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::styled("  Select a post", colors.text_muted()),
        ])
        .block(detail_block);
        frame.render_widget(empty, horizontal[2]);
    }
}

fn render_accounts_view(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    if state.accounts.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::styled("  No accounts connected", colors.text_muted()),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Run ", colors.text_dim()),
                Span::styled("perch auth mastodon", colors.text_primary()),
                Span::styled(" or ", colors.text_dim()),
                Span::styled("perch auth bluesky", colors.text_primary()),
            ]),
            Line::from(vec![
                Span::styled("  to connect an account", colors.text_dim()),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(colors.block())
                .title(" 👤 Connected Accounts ")
                .title_style(colors.text_primary()),
        );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state
        .accounts
        .iter()
        .enumerate()
        .map(|(i, account)| {
            let is_selected = i == state.selected_account;
            let cursor = if is_selected { "▸" } else { " " };
            let style = if is_selected {
                colors.selected()
            } else {
                colors.text()
            };

            let network_style = match account.network {
                crate::models::Network::Mastodon => colors.network_mastodon(),
                crate::models::Network::Bluesky => colors.network_bluesky(),
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!(" {} ", cursor), style),
                    Span::styled(account.network.emoji(), network_style),
                    Span::styled(format!(" {} ", account.display_name), style.add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("     ", style),
                    Span::styled(format!("@{}", account.handle), colors.text_muted()),
                ]),
                Line::from(vec![
                    Span::styled("     ", style),
                    Span::styled(
                        format!("Server: {}", account.server),
                        colors.text_dim(),
                    ),
                ]),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(colors.block_focus())
            .title(format!(" 👤 Connected Accounts ({}) ", state.accounts.len()))
            .title_style(colors.text_primary()),
    );

    frame.render_widget(list, area);
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

    let content = if !state.status.is_empty() {
        vec![
            Span::styled(" ", Style::default()),
            Span::styled(&loading_indicator, colors.text_secondary()),
            Span::styled(&state.status, colors.text_secondary()),
        ]
    } else {
        vec![
            Span::styled(" ", Style::default()),
            Span::styled(&loading_indicator, colors.text_secondary()),
            Span::styled("Tab", colors.key_hint()),
            Span::styled(": views  ", colors.text_muted()),
            Span::styled("?", colors.key_hint()),
            Span::styled(": help  ", colors.text_muted()),
            Span::styled("t", colors.key_hint()),
            Span::styled(": theme  ", colors.text_muted()),
            Span::styled("A", colors.key_hint()),
            Span::styled(": about  ", colors.text_muted()),
            Span::styled("q", colors.key_hint()),
            Span::styled(": quit", colors.text_muted()),
        ]
    };

    let status = Paragraph::new(Line::from(content))
        .style(Style::default().bg(colors.bg_secondary));
    frame.render_widget(status, area);
}

fn render_help_popup(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Same dimensions as Hazelnut: 50% width, 70% height
    let popup_area = centered_rect(50, 70, area);

    // First render a solid background block to cover everything underneath
    let bg_block = Block::default().style(Style::default().bg(colors.bg_secondary));
    frame.render_widget(Clear, popup_area);
    frame.render_widget(bg_block, popup_area);

    let help_content = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Navigation",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Tab              ", colors.key_hint()),
            Span::styled("Switch between views", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  h/l or ←/→       ", colors.key_hint()),
            Span::styled("Switch panels", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  j/k or ↑/↓       ", colors.key_hint()),
            Span::styled("Navigate items", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  g/G              ", colors.key_hint()),
            Span::styled("Go to first/last item", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Timeline View",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  n                ", colors.key_hint()),
            Span::styled("Compose new post", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  R                ", colors.key_hint()),
            Span::styled("Reply to post", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  r                ", colors.key_hint()),
            Span::styled("Refresh timeline", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  L                ", colors.key_hint()),
            Span::styled("Like/unlike post", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  b                ", colors.key_hint()),
            Span::styled("Boost/repost", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  o                ", colors.key_hint()),
            Span::styled("Open in browser", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  f                ", colors.key_hint()),
            Span::styled("Cycle filter (All/Mastodon/Bluesky)", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  /                ", colors.key_hint()),
            Span::styled("Search posts", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  General",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  t                ", colors.key_hint()),
            Span::styled("Open theme selector", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  A                ", colors.key_hint()),
            Span::styled("About Perch", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  ?                ", colors.key_hint()),
            Span::styled("Toggle this help", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  q                ", colors.key_hint()),
            Span::styled("Quit application", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", colors.text_muted()),
            Span::styled("Esc", colors.key_hint()),
            Span::styled(" or ", colors.text_muted()),
            Span::styled("?", colors.key_hint()),
            Span::styled(" to close", colors.text_muted()),
        ]),
    ];

    let help = Paragraph::new(help_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(colors.block_focus())
                .style(Style::default().bg(colors.bg_secondary))
                .title(" ⌨ Keyboard Shortcuts ")
                .title_style(colors.text_primary()),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(help, popup_area);
}

fn render_theme_picker(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Same dimensions as Hazelnut: 50% width, 70% height
    let popup_area = centered_rect(50, 70, area);

    // First render a solid background block to cover everything underneath
    let bg_block = Block::default().style(Style::default().bg(colors.bg));
    frame.render_widget(Clear, popup_area);
    frame.render_widget(bg_block, popup_area);

    let themes = Theme::all();
    let items: Vec<ListItem> = themes
        .iter()
        .enumerate()
        .map(|(i, theme_name)| {
            let palette = theme_name.palette();
            let selected = i == state.theme_picker_index;

            // Create color preview squares - exactly like Hazelnut
            let preview = format!(
                "  {} {} ",
                if selected { "▸" } else { " " },
                theme_name.display_name()
            );

            let style = if selected {
                Style::default()
                    .fg(palette.accent)
                    .bg(palette.selection)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.fg).bg(colors.bg)
            };

            ListItem::new(Line::from(vec![
                Span::styled(preview, style),
                Span::styled("█", Style::default().fg(palette.accent).bg(colors.bg)),
                Span::styled("█", Style::default().fg(palette.secondary).bg(colors.bg)),
                Span::styled("█", Style::default().fg(palette.success).bg(colors.bg)),
                Span::styled("█", Style::default().fg(palette.warning).bg(colors.bg)),
            ]))
        })
        .collect();

    let theme_list = List::new(items)
        .style(Style::default().bg(colors.bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.primary))
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(colors.bg))
                .title(format!(
                    " 🎨 Select Theme ({}/{}) ",
                    state.theme_picker_index + 1,
                    themes.len()
                ))
                .title_bottom(Line::from(" ↑↓ navigate │ ↵ apply │ Esc cancel ").centered()),
        );

    frame.render_widget(theme_list, popup_area);
}

fn render_about_dialog(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Same dimensions as Hazelnut: 80% width, 60% height
    let popup_area = centered_rect(80, 60, area);

    // First render a solid background block to cover everything underneath
    let bg_block = Block::default().style(Style::default().bg(colors.bg));
    frame.render_widget(Clear, popup_area);
    frame.render_widget(bg_block, popup_area);

    let version = env!("CARGO_PKG_VERSION");
    let repo = "https://github.com/ricardodantas/perch";

    // Static logo array like Hazelnut - all lines same color
    let logo = [
        "██████╗ ███████╗██████╗  ██████╗██╗  ██╗",
        "██╔══██╗██╔════╝██╔══██╗██╔════╝██║  ██║",
        "██████╔╝█████╗  ██████╔╝██║     ███████║",
        "██╔═══╝ ██╔══╝  ██╔══██╗██║     ██╔══██║",
        "██║     ███████╗██║  ██║╚██████╗██║  ██║",
        "╚═╝     ╚══════╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝",
    ];

    let mut lines: Vec<Line> = logo
        .iter()
        .map(|line| Line::from(Span::styled(*line, Style::default().fg(colors.primary))))
        .collect();
    lines.extend([
        Line::from(""),
        Line::from(Span::styled(
            "🐦 Terminal social client for Mastodon & Bluesky",
            Style::default()
                .fg(colors.fg)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Version: ", colors.text_muted()),
            Span::styled(
                version,
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Author: ", colors.text_muted()),
            Span::styled("Ricardo Dantas", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("License: ", colors.text_muted()),
            Span::styled("MIT", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("Repo: ", colors.text_muted()),
            Span::styled(repo, Style::default().fg(colors.primary)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Built with Rust 🦀 + Ratatui",
            colors.text_muted().add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [G] ",
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Open GitHub"),
            Span::raw("    "),
            Span::styled(" [Esc] ", colors.text_muted()),
            Span::raw("Close"),
        ]),
    ]);

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.primary))
            .style(Style::default().bg(colors.bg))
            .title(" 🐦 About Perch ")
            .title_style(
                Style::default()
                    .fg(colors.primary)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(paragraph, popup_area);
}

fn render_compose_popup(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    // Smaller dialog: 50% width, 40% height
    let popup_area = centered_rect(50, 40, area);
    
    // Clear and add background
    let bg_block = Block::default().style(Style::default().bg(colors.bg));
    frame.render_widget(Clear, popup_area);
    frame.render_widget(bg_block, popup_area);

    let networks_str: String = state
        .compose_networks
        .iter()
        .map(|n| n.emoji())
        .collect::<Vec<_>>()
        .join(" ");

    let char_count = state.compose_text.len();
    let max_chars = 500; // Mastodon default, Bluesky is 300
    
    let mut content = vec![Line::from("")];
    
    // Show reply context if replying
    if let Some(ref reply_to) = state.reply_to {
        content.push(Line::from(vec![
            Span::styled("  ↩ Replying to ", colors.text_dim()),
            Span::styled(format!("@{}", reply_to.author_handle), colors.text_primary()),
        ]));
        content.push(Line::from(""));
    }
    
    content.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            if state.compose_text.is_empty() {
                if state.reply_to.is_some() { "Write your reply..." } else { "What's on your mind?" }
            } else {
                &state.compose_text
            },
            if state.compose_text.is_empty() {
                colors.text_muted()
            } else {
                colors.text()
            },
        ),
    ]));
    content.push(Line::from(""));
    content.push(Line::from(vec![
        Span::styled(format!("  {}/{} ", char_count, max_chars), 
            if char_count > max_chars { colors.text_error() } else { colors.text_dim() }),
        Span::styled("│ ", colors.text_dim()),
        Span::styled("Ctrl+↵", colors.key_hint()),
        Span::styled(if state.reply_to.is_some() { " reply " } else { " post " }, colors.text_muted()),
        Span::styled("Esc", colors.key_hint()),
        Span::styled(" cancel", colors.text_muted()),
    ]));

    let title = if state.reply_to.is_some() {
        format!(" ↩ Reply → {} ", networks_str)
    } else if state.compose_networks.is_empty() {
        " 📝 Compose (Alt+1 🐘 Alt+2 🦋) ".to_string()
    } else {
        format!(" 📝 Compose → {} ", networks_str)
    };

    let compose = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(colors.block_focus())
                .style(Style::default().bg(colors.bg))
                .title(title)
                .title_style(colors.text_primary()),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(compose, popup_area);
    
    // Show cursor position - adjust for reply context
    let reply_offset = if state.reply_to.is_some() { 2u16 } else { 0 };
    let cursor_x = popup_area.x + 3 + state.compose_text.lines().last().map(|l| l.len()).unwrap_or(0) as u16;
    let cursor_y = popup_area.y + 2 + reply_offset + state.compose_text.lines().count().saturating_sub(1) as u16;
    if cursor_x < popup_area.x + popup_area.width - 1 && cursor_y < popup_area.y + popup_area.height - 2 {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_search_popup(frame: &mut Frame, state: &AppState) {
    let colors = state.theme.colors();
    let area = frame.area();

    let popup_area = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup_area);

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                if state.search_query.is_empty() {
                    "Type to search..."
                } else {
                    &state.search_query
                },
                if state.search_query.is_empty() {
                    colors.text_muted()
                } else {
                    colors.text()
                },
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("Enter", colors.key_hint()),
            Span::styled(" search  ", colors.text_muted()),
            Span::styled("Esc", colors.key_hint()),
            Span::styled(" cancel", colors.text_muted()),
        ]),
    ];

    let search = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(colors.block_focus())
                .style(Style::default().bg(colors.bg))
                .title(" 🔍 Search ")
                .title_style(colors.text_primary()),
        );

    frame.render_widget(search, popup_area);
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    Rect {
        x: r.x + (r.width.saturating_sub(popup_width)) / 2,
        y: r.y + (r.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    }
}
