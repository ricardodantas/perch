# CLAUDE.md - Perch

Quick reference for Claude/AI assistants working on Perch.

## What is Perch?

A terminal social client for Mastodon and Bluesky. TUI built with Ratatui, async with Tokio.

## Quick Commands

```bash
cargo run              # Run the TUI
cargo run -- --demo    # Demo mode (no real accounts needed)
cargo check            # Fast compile check
cargo clippy -- -D warnings  # Must pass for CI
cargo fmt              # Format code
```

## File Locations

| What | Where |
|------|-------|
| API clients | `src/api/{mastodon,bluesky}.rs` |
| UI rendering | `src/app/ui.rs` |
| Key handling | `src/app/events.rs` |
| App state | `src/app/state.rs` |
| Async ops | `src/app/async_ops.rs` |
| Auto-update | `src/update.rs` |

## Common Tasks

### Add a new keybinding
1. Edit `src/app/events.rs`
2. Find the relevant `handle_*_key` function
3. Add match arm for the key
4. Update help dialog in `src/app/ui.rs` → `render_help_popup`

### Add a new API feature
1. Add method to `SocialApi` trait in `src/api/mod.rs`
2. Implement in both `mastodon.rs` and `bluesky.rs`
3. Add `AsyncCommand` variant in `src/app/async_ops.rs`
4. Handle result in `handle_async_result` in `src/app/mod.rs`

### Add a new dialog/mode
1. Add variant to `Mode` enum in `src/app/state.rs`
2. Add render function in `src/app/ui.rs`
3. Call it from `render()` based on mode
4. Add key handler in `src/app/events.rs`

## CI Gotchas

- Many clippy lints are allowed in `lib.rs` - check there before adding new allows
- Format with `cargo fmt` before committing
- Tests are minimal - focus on compile checks

## Architecture Notes

- Ratatui requires sync main loop → async operations via channels
- `AppState` is the single source of truth
- Credentials are AES-GCM encrypted, key derived from machine ID
- Both networks implement same `SocialApi` trait for parity
