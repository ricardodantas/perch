//! Network type definitions

use serde::{Deserialize, Serialize};

/// Supported social networks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    /// Mastodon (and compatible ActivityPub servers)
    #[default]
    Mastodon,
    /// Bluesky (AT Protocol)
    Bluesky,
}

impl Network {
    /// Get all supported networks
    pub const fn all() -> &'static [Self] {
        &[Self::Mastodon, Self::Bluesky]
    }

    /// Get the display name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Mastodon => "Mastodon",
            Self::Bluesky => "Bluesky",
        }
    }

    /// Get the emoji icon
    pub const fn emoji(&self) -> &'static str {
        match self {
            Self::Mastodon => "🐘",
            Self::Bluesky => "🦋",
        }
    }

    /// Get the color for theming (hex)
    pub const fn color(&self) -> &'static str {
        match self {
            Self::Mastodon => "#6364FF", // Mastodon purple
            Self::Bluesky => "#0085FF",  // Bluesky blue
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mastodon" | "masto" => Some(Self::Mastodon),
            "bluesky" | "bsky" => Some(Self::Bluesky),
            _ => None,
        }
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
