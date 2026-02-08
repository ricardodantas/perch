//! Application state

use anyhow::Result;

use crate::config::Config;
use crate::db::Database;
use crate::models::{Account, Network, Post};
use crate::theme::Theme;

/// A reply with its depth level for display
#[derive(Debug, Clone)]
pub struct ReplyItem {
    /// The post content
    pub post: Post,
    /// Nesting depth (0 = direct reply, 1+ = nested)
    pub depth: usize,
}

/// Which panel is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPanel {
    /// Accounts sidebar panel (only in Accounts view)
    Accounts,
    /// Timeline posts list panel
    #[default]
    Timeline,
    /// Post detail panel
    Detail,
}

impl FocusedPanel {
    /// Get the next panel in tab order (for Timeline view: Timeline <-> Detail)
    pub const fn next(&self) -> Self {
        match self {
            Self::Accounts => Self::Timeline,
            Self::Timeline => Self::Detail,
            Self::Detail => Self::Timeline,
        }
    }

    /// Get the previous panel in tab order
    pub const fn prev(&self) -> Self {
        match self {
            Self::Accounts => Self::Detail,
            Self::Timeline => Self::Detail,
            Self::Detail => Self::Timeline,
        }
    }
}

/// Current view/tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Timeline,
    Accounts,
}

/// Modal mode for dialogs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Normal navigation mode
    #[default]
    Normal,
    /// Compose new post
    Compose,
    /// Search posts
    Search,
    /// Help dialog
    Help,
    /// Theme picker dialog
    ThemePicker,
    /// About dialog
    About,
    /// Update confirmation dialog
    UpdateConfirm,
    /// Update in progress
    Updating,
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
    pub const fn next(&self) -> Self {
        match self {
            Self::All => Self::Mastodon,
            Self::Mastodon => Self::Bluesky,
            Self::Bluesky => Self::All,
        }
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Mastodon => "Mastodon",
            Self::Bluesky => "Bluesky",
        }
    }

    pub const fn to_network(&self) -> Option<Network> {
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
    /// Current view/tab
    pub view: View,
    /// Current modal mode
    pub mode: Mode,
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
    /// Replies to currently selected post
    pub current_replies: Vec<ReplyItem>,
    /// Loading replies?
    pub loading_replies: bool,
    /// Scroll offset for detail panel
    pub detail_scroll: u16,
    /// Selected reply index (None = main post selected, Some(i) = reply i selected)
    pub selected_reply: Option<usize>,

    /// Compose text buffer
    pub compose_text: String,
    /// Networks to post to (for cross-posting)
    pub compose_networks: Vec<Network>,
    /// Reply-to post (if replying)
    pub reply_to: Option<Post>,

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

    /// Theme picker index
    pub theme_picker_index: usize,

    /// Update available (version string if newer version exists)
    pub update_available: Option<String>,
    /// Package manager for updates
    pub package_manager: crate::update::PackageManager,
    /// Update status message
    pub update_status: Option<String>,
    /// Flag to trigger update on next tick
    pub pending_update: bool,
}

impl AppState {
    /// Create a new app state
    pub fn new(config: Config, db: Database) -> Result<Self> {
        let theme = config.theme;
        let accounts = db.get_accounts()?;
        let posts = db.get_cached_posts(None, config.post_limit)?;

        // Find current theme index
        let theme_picker_index = Theme::all()
            .iter()
            .position(|t| *t == theme.inner())
            .unwrap_or(0);

        Ok(Self {
            config,
            db,
            should_quit: false,
            theme,
            view: View::Timeline,
            mode: Mode::Normal,
            focused_panel: FocusedPanel::Timeline,
            timeline_filter: TimelineFilter::All,
            accounts,
            selected_account: 0,
            posts,
            selected_post: 0,
            timeline_scroll: 0,
            current_replies: Vec::new(),
            loading_replies: false,
            detail_scroll: 0,
            selected_reply: None,
            compose_text: String::new(),
            compose_networks: vec![Network::Mastodon, Network::Bluesky],
            reply_to: None,
            search_query: String::new(),
            search_results: Vec::new(),
            status: String::new(),
            loading: false,
            tick: 0,
            theme_picker_index,
            update_available: None,
            package_manager: crate::update::detect_package_manager(),
            update_status: None,
            pending_update: false,
        })
    }

    /// Tick for animations
    pub const fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    /// Get current tick
    pub const fn current_tick(&self) -> u64 {
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

    /// Set update available (called from background task)
    pub fn set_update_available(&mut self, version: String) {
        self.update_available = Some(version.clone());
        self.set_status(format!(
            "Update available: v{} (current: v{})",
            version,
            crate::update::VERSION
        ));
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
            let old = self.selected_post;
            self.selected_post = (self.selected_post + 1).min(self.posts.len() - 1);
            if old != self.selected_post {
                self.current_replies.clear();
                self.loading_replies = true;
                self.detail_scroll = 0;
                self.selected_reply = None;
            }
        }
    }

    /// Move selection up in timeline
    pub fn select_prev_post(&mut self) {
        let old = self.selected_post;
        self.selected_post = self.selected_post.saturating_sub(1);
        if old != self.selected_post {
            self.current_replies.clear();
            self.loading_replies = true;
            self.detail_scroll = 0;
            self.selected_reply = None;
        }
    }

    /// Move selection down in accounts
    pub fn select_next_account(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_account = (self.selected_account + 1).min(self.accounts.len() - 1);
        }
    }

    /// Move selection up in accounts
    pub const fn select_prev_account(&mut self) {
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
        if let Ok(posts) = self
            .db
            .get_cached_posts(self.timeline_filter.to_network(), self.config.post_limit)
        {
            self.posts = posts;
            self.selected_post = 0;
        }
    }

    /// Refresh data from database
    pub fn refresh_data(&mut self) -> Result<()> {
        self.accounts = self.db.get_accounts()?;
        self.posts = self
            .db
            .get_cached_posts(self.timeline_filter.to_network(), self.config.post_limit)?;
        Ok(())
    }

    /// Open compose view
    pub fn open_compose(&mut self) {
        self.mode = Mode::Compose;
        self.compose_text.clear();
        self.reply_to = None;
        // Pre-select networks based on configured accounts
        self.compose_networks = self
            .accounts
            .iter()
            .map(|a| a.network)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
    }

    /// Open reply view for a specific post
    pub fn open_reply(&mut self, post: Post) {
        self.mode = Mode::Compose;
        self.compose_text = format!("@{} ", post.author_handle);
        self.reply_to = Some(post.clone());
        // Only select the network of the post we're replying to
        self.compose_networks = vec![post.network];
    }

    /// Close compose view
    pub fn close_compose(&mut self) {
        self.mode = Mode::Normal;
        self.reply_to = None;
    }

    /// Toggle network in compose
    pub fn toggle_compose_network(&mut self, network: Network) {
        if let Some(idx) = self.compose_networks.iter().position(|n| *n == network) {
            self.compose_networks.remove(idx);
        } else {
            self.compose_networks.push(network);
        }
    }

    /// Navigate to the next view
    pub const fn next_view(&mut self) {
        self.view = match self.view {
            View::Timeline => View::Accounts,
            View::Accounts => View::Timeline,
        };
    }

    /// Navigate to the previous view
    pub const fn prev_view(&mut self) {
        self.view = match self.view {
            View::Timeline => View::Accounts,
            View::Accounts => View::Timeline,
        };
    }
}
