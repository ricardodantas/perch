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
//! - [`db`] — SQLite database for accounts, cache, drafts
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

pub mod api;
pub mod app;
pub mod auth;
pub mod config;
pub mod db;
pub mod demo;
pub mod models;
pub mod sync;
pub mod theme;

// Re-export main types for convenience
pub use app::AppState;
pub use config::Config;
pub use db::Database;
pub use models::{Account, Network, Post};
pub use theme::{Theme, ThemeColors};

// Re-export theme types from ratatui-themes crate
pub use ratatui_themes::{ThemeName, ThemePalette};

/// ASCII logo for the application
pub const LOGO: &str = r#"
    ___                 __  
   / _ \___ _______/ /  
  / ___/ -_) __/ __/ _ \ 
 /_/   \__/_/  \__/_//_/ 
"#;

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Repository URL
pub const REPO_URL: &str = "https://github.com/ricardodantas/perch";
