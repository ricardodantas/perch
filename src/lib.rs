//! # Perch 🐦
//!
//! A beautiful terminal social client for Mastodon and Bluesky.
//!
//! ## Overview
//!
//! Perch is a multi-network social client that lets you read, post, and engage
//! across Mastodon and Bluesky from your terminal. Write once, post everywhere.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                          App                                │
//! │  Orchestrates all components and runs the main event loop   │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!          ┌───────────────────┼───────────────────┐
//!          ▼                   ▼                   ▼
//! ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
//! │     Config      │ │       API       │ │       UI        │
//! │                 │ │                 │ │                 │
//! │ • Load/Save     │ │ • Mastodon      │ │ • Render panels │
//! │ • Theme         │ │ • Bluesky       │ │ • Handle input  │
//! │ • Preferences   │ │ • Unified trait │ │ • Compose       │
//! └─────────────────┘ └─────────────────┘ └─────────────────┘
//!          │                   │                   │
//!          └───────────────────┴───────────────────┘
//!                              │
//!          ┌───────────────────┼───────────────────┐
//!          ▼                   ▼                   ▼
//! ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
//! │    Database     │ │      Auth       │ │     Models      │
//! │                 │ │                 │ │                 │
//! │ • Accounts      │ │ • Keyring       │ │ • Post          │
//! │ • Post cache    │ │ • OAuth flow    │ │ • Account       │
//! │ • Drafts        │ │ • Credentials   │ │ • Network       │
//! └─────────────────┘ └─────────────────┘ └─────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`api`] — Network API clients (Mastodon, Bluesky)
//! - [`app`] — TUI application state and event loop
//! - [`auth`] — Credential storage via system keyring
//! - [`config`] — Configuration management
//! - [`db`] — `SQLite` database for accounts, cache, drafts
//! - [`models`] — Data models (Post, Account, Network)
//! - [`theme`] — Theme support via ratatui-themes
//!
//! ## Example
//!
//! ```no_run
//! use perch::app;
//!
//! fn main() -> anyhow::Result<()> {
//!     app::run()
//! }
//! ```
//!
//! ## Features
//!
//! - **Multi-Network** — Mastodon and Bluesky in one place
//! - **Cross-Post** — Write once, post to multiple networks
//! - **Beautiful TUI** — Three-panel interface with 15 themes
//! - **Offline Cache** — Read your timeline without internet
//! - **Secure** — Credentials stored in system keyring
//! - **Fast** — Async networking with Tokio

#![doc(html_root_url = "https://docs.rs/perch/0.1.0")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![allow(clippy::unused_async)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::if_not_else)]
#![allow(clippy::single_match_else)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::use_self)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::similar_names)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::branches_sharing_code)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::return_self_not_must_use)]

pub mod api;
pub mod app;
pub mod auth;
pub mod config;
pub mod db;
pub mod demo;
pub mod models;
pub mod paths;
pub mod schedule;
pub mod sync;
pub mod theme;
pub mod update;

// Re-export main types for convenience
pub use app::AppState;
pub use config::Config;
pub use db::Database;
pub use models::{Account, Network, Post, ScheduledPost, ScheduledPostStatus};
pub use theme::{Theme, ThemeColors};
pub use update::{
    PackageManager, VersionCheck, check_for_updates_crates_io, detect_package_manager, run_update,
};

// Re-export theme types from ratatui-themes crate
pub use ratatui_themes::{ThemeName, ThemePalette};

/// ASCII logo for the application
pub const LOGO: &str = r"
    ___                 __  
   / _ \___ _______/ /  
  / ___/ -_) __/ __/ _ \ 
 /_/   \__/_/  \__/_//_/ 
";

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Repository URL
pub const REPO_URL: &str = "https://github.com/ricardodantas/perch";
