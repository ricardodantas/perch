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

    // Layout: [Timeline 50%] [Detail 50%]
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

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
                let time = post.relative_time();
                
                // Status indicators
                let mut indicators = String::new();
                if post.liked { indicators.push_str(" ❤️"); }
                if post.reposted { indicators.push_str(" 🔁"); }
                
                let is_selected = i == state.selected_post;
                let width = horizontal[0].width.saturating_sub(3) as usize;
                
                // Preview should use available width minus the indent
                let preview_width = width.saturating_sub(4); // 3 spaces indent + margin
                let preview = post.preview(preview_width);
                
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

    let timeline_list = List::new(post_items)
        .block(timeline_block)
        .highlight_style(colors.selected());
    
    // Create a ListState for scrolling
    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(state.selected_post));
    
    frame.render_stateful_widget(timeline_list, horizontal[0], &mut list_state);

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
        
        let mut detail_content = vec![
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
        
        // Add replies section
        if !state.current_replies.is_empty() {
            detail_content.push(Line::from(""));
            detail_content.push(Line::from(vec![
                Span::styled("  ── Replies ──────────────────────", colors.text_dim()),
            ]));
            
            // Show all replies (no limit since we show nested structure)
            for (idx, reply_item) in state.current_replies.iter().enumerate() {
                let is_selected = state.selected_reply == Some(idx) && state.focused_panel == FocusedPanel::Detail;
                let indent = "  ".repeat(reply_item.depth + 1);
                let tree_char = if reply_item.depth > 0 { "└ " } else { "" };
                let prefix = if is_selected { "▶" } else { " " };
                let handle_style = if is_selected {
                    colors.text_primary().add_modifier(Modifier::BOLD).add_modifier(Modifier::REVERSED)
                } else {
                    colors.text_primary()
                };
                
                detail_content.push(Line::from(""));
                detail_content.push(Line::from(vec![
                    Span::styled(indent.clone(), Style::default()),
                    Span::styled(tree_char, colors.text_dim()),
                    Span::styled(prefix, if is_selected { colors.text_primary() } else { Style::default() }),
                    Span::styled(format!("@{}", reply_item.post.author_handle), handle_style),
                    Span::styled(format!(" · {}", reply_item.post.relative_time()), colors.text_muted()),
                ]));
                
                // Content indent (depth + 2 for alignment after handle)
                let content_indent = "  ".repeat(reply_item.depth + 2);
                
                // Show full content
                if reply_item.post.content.contains('\n') {
                    // Multi-line content
                    for line in reply_item.post.content.lines() {
                        detail_content.push(Line::from(vec![
                            Span::styled(content_indent.clone(), Style::default()),
                            Span::styled(line.to_string(), colors.text()),
                        ]));
                    }
                } else {
                    // Single line content
                    detail_content.push(Line::from(vec![
                        Span::styled(content_indent.clone(), Style::default()),
                        Span::styled(reply_item.post.content.clone(), colors.text()),
                    ]));
                }
                detail_content.push(Line::from(vec![
                    Span::styled(format!("{}♡ {}  ↻ {}  💬 {}", content_indent, reply_item.post.like_count, reply_item.post.repost_count, reply_item.post.reply_count), colors.text_dim()),
                ]));
            }
        } else if state.loading_replies {
            detail_content.push(Line::from(""));
            detail_content.push(Line::from(vec![
                Span::styled("  ⏳ Loading replies...", colors.text_muted()),
            ]));
        }

        let detail = Paragraph::new(detail_content)
            .block(detail_block)
            .wrap(Wrap { trim: false })
            .scroll((state.detail_scroll, 0));
        frame.render_widget(detail, horizontal[1]);
    } else {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::styled("  Select a post", colors.text_muted()),
        ])
        .block(detail_block);
        frame.render_widget(empty, horizontal[1]);
    }
}

fn render_accounts_view(frame: &mut Frame, state: &AppState, area: Rect) {
    let colors = state.theme.colors();

    // Split into main area and bottom bar for account actions
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let main_area = layout[0];
    let action_bar_area = layout[1];

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
        frame.render_widget(empty, main_area);
        return;
    }

    // Calculate width for full-line selection
    let content_width = main_area.width.saturating_sub(2) as usize; // -2 for borders

    let items: Vec<ListItem> = state
        .accounts
        .iter()
        .enumerate()
        .map(|(i, account)| {
            let is_selected = i == state.selected_account;
            let cursor = if is_selected { "▸" } else { " " };
            
            let bg_style = if is_selected {
                colors.selected()
            } else {
                Style::default()
            };

            let network_style = if is_selected {
                colors.selected().add_modifier(Modifier::BOLD)
            } else {
                match account.network {
                    crate::models::Network::Mastodon => colors.network_mastodon(),
                    crate::models::Network::Bluesky => colors.network_bluesky(),
                }
            };

            let text_style = if is_selected {
                colors.selected().add_modifier(Modifier::BOLD)
            } else {
                colors.text()
            };

            let muted_style = if is_selected {
                colors.selected()
            } else {
                colors.text_muted()
            };

            let dim_style = if is_selected {
                colors.selected()
            } else {
                colors.text_dim()
            };

            let default_marker = if account.is_default { " ★" } else { "" };

            // Build lines with full-width background
            let line1 = format!(" {} {} {} {}", cursor, account.network.emoji(), account.display_name, default_marker);
            let line1_padded = format!("{:width$}", line1, width = content_width);
            
            let line2 = format!("     @{}", account.handle);
            let line2_padded = format!("{:width$}", line2, width = content_width);
            
            let line3 = format!("     Server: {}", account.server);
            let line3_padded = format!("{:width$}", line3, width = content_width);

            ListItem::new(vec![
                Line::from(Span::styled(line1_padded, text_style.patch(bg_style))),
                Line::from(Span::styled(line2_padded, muted_style.patch(bg_style))),
                Line::from(Span::styled(line3_padded, dim_style.patch(bg_style))),
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

    frame.render_widget(list, main_area);

    // Render action bar with account-specific keys
    let action_bar = Line::from(vec![
        Span::styled(" ", Style::default().bg(colors.bg_secondary)),
        Span::styled("[d]", colors.key_hint()),
        Span::styled(" Set default  ", colors.text_muted()),
        Span::styled("[D]", colors.key_hint()),
        Span::styled(" Delete  ", colors.text_muted()),
        Span::styled("[r]", colors.key_hint()),
        Span::styled(" Refresh token  ", colors.text_muted()),
        Span::styled("[Enter]", colors.key_hint()),
        Span::styled(" View timeline", colors.text_muted()),
    ]);

    let action_bar_bg = Paragraph::new(action_bar)
        .style(Style::default().bg(colors.bg_secondary));
    frame.render_widget(action_bar_bg, action_bar_area);
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
            Span::styled("Navigate items / replies", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  g/G              ", colors.key_hint()),
            Span::styled("Go to first/last item", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Timeline Actions",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  n                ", colors.key_hint()),
            Span::styled("Compose new post", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  R                ", colors.key_hint()),
            Span::styled("Reply to post/reply", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  r                ", colors.key_hint()),
            Span::styled("Repost/unrepost (toggle)", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  L                ", colors.key_hint()),
            Span::styled("Like/unlike post", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  b                ", colors.key_hint()),
            Span::styled("Refresh timeline", colors.text()),
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
            "  Detail Panel (post + replies)",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  j/k              ", colors.key_hint()),
            Span::styled("Select reply", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  J/K (shift)      ", colors.key_hint()),
            Span::styled("Scroll content", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d/u         ", colors.key_hint()),
            Span::styled("Page down/up", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  R                ", colors.key_hint()),
            Span::styled("Reply to selected reply", colors.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Compose",
            colors.text_primary().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  1                  ", colors.key_hint()),
            Span::styled("Toggle Mastodon", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  2                  ", colors.key_hint()),
            Span::styled("Toggle Bluesky", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Enter       ", colors.key_hint()),
            Span::styled("Send post", colors.text()),
        ]),
        Line::from(vec![
            Span::styled("  Esc              ", colors.key_hint()),
            Span::styled("Cancel", colors.text()),
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
    
    // Show network selection toggles - cleaner pill-style
    let has_mastodon = state.accounts.iter().any(|a| a.network == crate::models::Network::Mastodon);
    let has_bluesky = state.accounts.iter().any(|a| a.network == crate::models::Network::Bluesky);
    let mastodon_selected = state.compose_networks.contains(&crate::models::Network::Mastodon);
    let bluesky_selected = state.compose_networks.contains(&crate::models::Network::Bluesky);
    
    let mut network_spans = vec![Span::styled("  ", Style::default())];
    
    if has_mastodon {
        if mastodon_selected {
            network_spans.push(Span::styled(" 🐘 Mastodon ✓ ", colors.selected()));
        } else {
            network_spans.push(Span::styled(" 🐘 Mastodon ", colors.text_dim()));
        }
        network_spans.push(Span::styled(" ", Style::default()));
    }
    
    if has_bluesky {
        if bluesky_selected {
            network_spans.push(Span::styled(" 🦋 Bluesky ✓ ", colors.selected()));
        } else {
            network_spans.push(Span::styled(" 🦋 Bluesky ", colors.text_dim()));
        }
    }
    
    if !has_mastodon && !has_bluesky {
        network_spans.push(Span::styled("No accounts configured!", colors.text_error()));
    }
    
    content.push(Line::from(network_spans));
    content.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled("Press ", colors.text_dim()),
        Span::styled("1", colors.key_hint()),
        Span::styled("/", colors.text_dim()),
        Span::styled("2", colors.key_hint()),
        Span::styled(" to toggle", colors.text_dim()),
    ]));
    content.push(Line::from(""));
    
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
        " ↩ Reply ".to_string()
    } else {
        " 📝 Compose ".to_string()
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
    
    // Show cursor position - adjust for reply context and network line
    let reply_offset = if state.reply_to.is_some() { 2u16 } else { 0 };
    let network_offset = 3u16; // network pills + hint line + empty line
    let cursor_x = popup_area.x + 3 + state.compose_text.lines().last().map(|l| l.len()).unwrap_or(0) as u16;
    let cursor_y = popup_area.y + 2 + reply_offset + network_offset + state.compose_text.lines().count().saturating_sub(1) as u16;
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
