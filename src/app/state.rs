//! Application state

use anyhow::Result;

use crate::config::Config;
use crate::db::Database;
use crate::models::{Account, Network, Post};
use crate::theme::Theme;

/// Which panel is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPanel {
    #[default]
    Accounts,
    Timeline,
    Detail,
}

impl FocusedPanel {
    pub fn next(&self) -> Self {
        match self {
            Self::Accounts => Self::Timeline,
            Self::Timeline => Self::Detail,
            Self::Detail => Self::Accounts,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Accounts => Self::Detail,
            Self::Timeline => Self::Accounts,
            Self::Detail => Self::Timeline,
        }
    }
}

/// Current view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Timeline,
    Compose,
    Search,
    Help,
}

/// Timeline filter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineFilter {
    #[default]
    All,
    Mastodon,
    Bluesky,
}

impl TimelineFilter {
    pub fn next(&self) -> Self {
        match self {
            Self::All => Self::Mastodon,
            Self::Mastodon => Self::Bluesky,
            Self::Bluesky => Self::All,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Mastodon => "Mastodon",
            Self::Bluesky => "Bluesky",
        }
    }

    pub fn to_network(&self) -> Option<Network> {
        match self {
            Self::All => None,
            Self::Mastodon => Some(Network::Mastodon),
            Self::Bluesky => Some(Network::Bluesky),
        }
    }
}

/// Application state
pub struct AppState {
    /// Configuration
    pub config: Config,
    /// Database connection
    pub db: Database,
    /// Whether to quit
    pub should_quit: bool,
    /// Current theme
    pub theme: Theme,
    /// Current view
    pub view: View,
    /// Focused panel
    pub focused_panel: FocusedPanel,
    /// Timeline filter
    pub timeline_filter: TimelineFilter,

    /// Loaded accounts
    pub accounts: Vec<Account>,
    /// Selected account index
    pub selected_account: usize,

    /// Posts in the timeline
    pub posts: Vec<Post>,
    /// Selected post index
    pub selected_post: usize,
    /// Scroll offset for timeline
    pub timeline_scroll: usize,

    /// Compose text buffer
    pub compose_text: String,
    /// Networks to post to (for cross-posting)
    pub compose_networks: Vec<Network>,

    /// Search query
    pub search_query: String,
    /// Search results
    pub search_results: Vec<Post>,

    /// Status message (bottom bar)
    pub status: String,
    /// Is loading?
    pub loading: bool,

    /// Tick counter for animations
    tick: u64,
}

impl AppState {
    /// Create a new app state
    pub fn new(config: Config, db: Database) -> Result<Self> {
        let theme = config.theme;
        let accounts = db.get_accounts()?;
        let posts = db.get_cached_posts(None, config.post_limit)?;

        Ok(Self {
            config,
            db,
            should_quit: false,
            theme,
            view: View::Timeline,
            focused_panel: FocusedPanel::Timeline,
            timeline_filter: TimelineFilter::All,
            accounts,
            selected_account: 0,
            posts,
            selected_post: 0,
            timeline_scroll: 0,
            compose_text: String::new(),
            compose_networks: vec![Network::Mastodon, Network::Bluesky],
            search_query: String::new(),
            search_results: Vec::new(),
            status: String::new(),
            loading: false,
            tick: 0,
        })
    }

    /// Tick for animations
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    /// Get current tick
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Set status message
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status.clear();
    }

    /// Get the currently selected post
    pub fn selected_post(&self) -> Option<&Post> {
        self.posts.get(self.selected_post)
    }

    /// Get the currently selected account
    pub fn selected_account(&self) -> Option<&Account> {
        self.accounts.get(self.selected_account)
    }

    /// Move selection down in timeline
    pub fn select_next_post(&mut self) {
        if !self.posts.is_empty() {
            self.selected_post = (self.selected_post + 1).min(self.posts.len() - 1);
        }
    }

    /// Move selection up in timeline
    pub fn select_prev_post(&mut self) {
        self.selected_post = self.selected_post.saturating_sub(1);
    }

    /// Move selection down in accounts
    pub fn select_next_account(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_account = (self.selected_account + 1).min(self.accounts.len() - 1);
        }
    }

    /// Move selection up in accounts
    pub fn select_prev_account(&mut self) {
        self.selected_account = self.selected_account.saturating_sub(1);
    }

    /// Cycle through themes
    pub fn next_theme(&mut self) {
        self.theme = self.theme.next();
        self.config.theme = self.theme;
    }

    /// Cycle through timeline filters
    pub fn cycle_filter(&mut self) {
        self.timeline_filter = self.timeline_filter.next();
        // Reload posts with new filter
        if let Ok(posts) = self.db.get_cached_posts(
            self.timeline_filter.to_network(),
            self.config.post_limit,
        ) {
            self.posts = posts;
            self.selected_post = 0;
        }
    }

    /// Refresh data from database
    pub fn refresh_data(&mut self) -> Result<()> {
        self.accounts = self.db.get_accounts()?;
        self.posts = self.db.get_cached_posts(
            self.timeline_filter.to_network(),
            self.config.post_limit,
        )?;
        Ok(())
    }

    /// Open compose view
    pub fn open_compose(&mut self) {
        self.view = View::Compose;
        self.compose_text.clear();
    }

    /// Close compose view
    pub fn close_compose(&mut self) {
        self.view = View::Timeline;
    }

    /// Toggle network in compose
    pub fn toggle_compose_network(&mut self, network: Network) {
        if let Some(idx) = self.compose_networks.iter().position(|n| *n == network) {
            self.compose_networks.remove(idx);
        } else {
            self.compose_networks.push(network);
        }
    }
}
