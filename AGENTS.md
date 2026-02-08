# AGENTS.md - Perch

Guidelines for AI agents working on this codebase.

## Project Overview

Perch is a beautiful terminal social client for Mastodon and Bluesky. Write once, post everywhere.

**Stack:** Rust, Ratatui (TUI), Tokio (async), SQLite (cache), reqwest (HTTP)

## Architecture

```
src/
├── api/           # Network API clients
│   ├── mod.rs     # SocialApi trait, unified Client enum
│   ├── mastodon.rs # Mastodon API implementation
│   └── bluesky.rs  # Bluesky AT Protocol implementation
├── app/           # TUI application
│   ├── mod.rs     # Main app loop, async worker
│   ├── state.rs   # AppState, Mode, View, FocusedPanel
│   ├── events.rs  # Key event handling
│   ├── ui.rs      # Ratatui rendering
│   └── async_ops.rs # Background async operations
├── auth/          # Credential storage (AES-GCM encrypted)
├── models/        # Data models (Post, Account, Network)
├── db.rs          # SQLite database
├── config.rs      # Configuration
├── theme.rs       # Theme support (15 themes via ratatui-themes)
├── update.rs      # Auto-update (crates.io + Homebrew detection)
└── main.rs        # CLI interface
```

## Key Patterns

### API Trait
Both Mastodon and Bluesky implement `SocialApi`:
- `timeline()`, `post()`, `reply()`, `like()`, `unlike()`, `repost()`, `unrepost()`
- Use the unified `Client` enum for network-agnostic operations

### Async Operations
- Main loop is sync (Ratatui requires it)
- Background async worker handles API calls via channels
- `AsyncCommand` → worker → `AsyncResult`

### State Management
- `Mode` enum: Normal, Compose, Search, Help, ThemePicker, About, UpdateConfirm, Updating
- `View` enum: Timeline, Accounts
- `FocusedPanel`: Timeline, Detail, Accounts

### Update System
- Background check on startup via `check_for_updates_crates_io()`
- Yellow banner at top when update available
- Press `[U]` from anywhere to update
- Detects Homebrew vs Cargo installation

## CLI Commands

```bash
perch                    # Launch TUI
perch auth mastodon <instance>  # Add Mastodon account
perch auth bluesky       # Add Bluesky account
perch post "text"        # Post from CLI
perch timeline           # View timeline in CLI
perch accounts           # List accounts
```

## Development

```bash
cargo run                # Run TUI
cargo run -- --demo      # Demo mode with mock data
cargo test               # Run tests
cargo clippy             # Lint
cargo fmt                # Format
```

## CI Requirements

All PRs must pass:
- `cargo check`
- `cargo test`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- Build on Linux, macOS, Windows

## Code Style

- Use `anyhow` for error handling
- Prefer `?` over `.unwrap()` except in tests
- Document public APIs
- Keep functions focused and small
- Use the existing clippy allows in `lib.rs`
