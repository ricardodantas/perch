//! Image loading and caching for terminal display.
//!
//! This module handles downloading, caching, and preparing images for
//! rendering in the terminal using various graphics protocols (Sixel,
//! Kitty, iTerm2) with fallback to Unicode halfblocks.

mod cache;
mod loader;

pub use cache::ImageCache;
pub use loader::ImageLoader;

use ratatui_image::picker::Picker;
use std::sync::OnceLock;

/// Global picker instance (initialized once at startup)
static PICKER: OnceLock<Option<Picker>> = OnceLock::new();

/// Initialize the image picker by querying terminal capabilities.
///
/// This should be called once at startup, before entering the TUI.
/// Returns `true` if a graphics protocol is available.
pub fn init_picker() -> bool {
    let picker = PICKER.get_or_init(|| {
        // Try to detect terminal graphics support
        // This queries the terminal for sixel/kitty/iterm2 support
        match Picker::from_query_stdio() {
            Ok(p) => {
                tracing::info!("Image support detected: {:?}", p.protocol_type());
                Some(p)
            }
            Err(e) => {
                tracing::debug!("No image protocol support: {e}");
                // Fall back to halfblocks (Unicode-based, works everywhere)
                Some(Picker::halfblocks())
            }
        }
    });
    picker.is_some()
}

/// Get the global picker instance.
pub fn picker() -> Option<&'static Picker> {
    PICKER.get().and_then(|p| p.as_ref())
}

/// Check if terminal supports native graphics (not just halfblocks).
pub fn has_native_graphics() -> bool {
    picker()
        .map(|p| !matches!(p.protocol_type(), ratatui_image::picker::ProtocolType::Halfblocks))
        .unwrap_or(false)
}
