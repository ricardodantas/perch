//! Theme configuration and colors.
//!
//! Perch supports popular terminal color schemes out of the box.
//! Theme palettes are provided by the `ratatui-themes` crate.

use ratatui::style::{Color, Modifier, Style};
use ratatui_themes::{ThemeName, ThemePalette};
use serde::{Deserialize, Serialize};

/// Theme wrapper around `ThemeName` from ratatui-themes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Theme(pub ThemeName);

impl Theme {
    /// Get all available theme names.
    #[must_use]
    pub const fn all() -> &'static [ThemeName] {
        ThemeName::all()
    }

    /// Get the next theme in rotation
    #[must_use]
    pub fn next(&self) -> Self {
        Self(self.0.next())
    }

    /// Get the previous theme in rotation
    #[must_use]
    pub fn prev(&self) -> Self {
        Self(self.0.prev())
    }

    /// Get the display name for the theme.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.0.display_name()
    }

    /// Get the color palette for this theme
    #[must_use]
    pub fn colors(&self) -> ThemeColors {
        ThemeColors::from_palette(self.0.palette())
    }

    /// Get the raw color palette for this theme.
    #[must_use]
    pub const fn palette(&self) -> ThemePalette {
        self.0.palette()
    }

    /// Get the inner `ThemeName`
    #[must_use]
    pub const fn inner(&self) -> ThemeName {
        self.0
    }

    /// Get the kebab-case slug for config files
    #[must_use]
    pub const fn slug(&self) -> &'static str {
        self.0.slug()
    }
}

impl From<ThemeName> for Theme {
    fn from(name: ThemeName) -> Self {
        Self(name)
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Extended color palette for UI elements.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Base colors (from palette)
    /// Primary background color
    pub bg: Color,
    /// Secondary background for panels/cards
    pub bg_secondary: Color,
    /// Highlighted/hover background
    pub bg_highlight: Color,
    /// Primary foreground/text color
    pub fg: Color,
    /// Dimmed text color
    pub fg_dim: Color,
    /// Muted text color (lowest contrast)
    pub fg_muted: Color,

    // Accent colors
    /// Primary accent color
    pub primary: Color,
    /// Secondary accent color
    pub secondary: Color,
    /// Tertiary accent color
    pub accent: Color,

    // Semantic colors
    /// Success state color (green)
    pub success: Color,
    /// Warning state color (yellow/orange)
    pub warning: Color,
    /// Error state color (red)
    pub error: Color,
    /// Info state color (blue)
    pub info: Color,

    // UI elements
    /// Border color (unfocused)
    pub border: Color,
    /// Border color (focused)
    pub border_focus: Color,
    /// Selection/highlight background
    pub selection: Color,

    // Network-specific colors
    /// Mastodon brand color (purple)
    pub mastodon: Color,
    /// Bluesky brand color (blue)
    pub bluesky: Color,

    // Logo colors
    /// Logo primary color
    pub logo_primary: Color,
    /// Logo secondary color
    pub logo_secondary: Color,
}

impl ThemeColors {
    /// Create `ThemeColors` from a `ThemePalette`
    #[must_use]
    pub fn from_palette(p: ThemePalette) -> Self {
        let bg_secondary = Self::adjust_brightness(p.bg, 10);
        let bg_highlight = Self::adjust_brightness(p.bg, 20);

        Self {
            bg: p.bg,
            bg_secondary,
            bg_highlight,
            fg: p.fg,
            fg_dim: p.muted,
            fg_muted: p.muted,

            primary: p.accent,
            secondary: p.secondary,
            accent: p.secondary,

            success: p.success,
            warning: p.warning,
            error: p.error,
            info: p.info,

            border: p.muted,
            border_focus: p.accent,
            selection: p.selection,

            // Network colors (approximate)
            mastodon: Color::Rgb(99, 100, 255), // #6364FF
            bluesky: Color::Rgb(0, 133, 255),   // #0085FF

            // Logo colors
            logo_primary: p.accent,
            logo_secondary: p.secondary,
        }
    }

    /// Adjust color brightness
    fn adjust_brightness(color: Color, amount: i16) -> Color {
        if let Color::Rgb(r, g, b) = color {
            let adjust = |c: u8| -> u8 {
                if amount > 0 {
                    c.saturating_add(amount as u8)
                } else {
                    c.saturating_sub((-amount) as u8)
                }
            };
            Color::Rgb(adjust(r), adjust(g), adjust(b))
        } else {
            color
        }
    }

    // Style helpers

    /// Default text style
    #[must_use]
    pub fn text(&self) -> Style {
        Style::default().fg(self.fg)
    }

    /// Dimmed text style
    #[must_use]
    pub fn text_dim(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    /// Muted text style
    #[must_use]
    pub fn text_muted(&self) -> Style {
        Style::default().fg(self.fg_muted)
    }

    /// Primary accent style
    #[must_use]
    pub fn text_primary(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Secondary accent style
    #[must_use]
    pub fn text_secondary(&self) -> Style {
        Style::default().fg(self.secondary)
    }

    /// Success style
    #[must_use]
    pub fn text_success(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Warning style
    #[must_use]
    pub fn text_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Error style
    #[must_use]
    pub fn text_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Info style
    #[must_use]
    pub fn text_info(&self) -> Style {
        Style::default().fg(self.info)
    }

    /// Block border style
    #[must_use]
    pub fn block(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Focused block border style
    #[must_use]
    pub fn block_focus(&self) -> Style {
        Style::default().fg(self.border_focus)
    }

    /// Selected item style
    #[must_use]
    pub fn selected(&self) -> Style {
        Style::default()
            .bg(self.selection)
            .fg(self.fg)
            .add_modifier(Modifier::BOLD)
    }

    /// Tab style
    #[must_use]
    pub fn tab(&self) -> Style {
        Style::default().fg(self.fg_muted)
    }

    /// Active tab style
    #[must_use]
    pub fn tab_active(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Key hint style (for shortcuts)
    #[must_use]
    pub fn key_hint(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Mastodon network style
    #[must_use]
    pub fn network_mastodon(&self) -> Style {
        Style::default().fg(self.mastodon)
    }

    /// Bluesky network style
    #[must_use]
    pub fn network_bluesky(&self) -> Style {
        Style::default().fg(self.bluesky)
    }

    /// Logo primary style
    #[must_use]
    pub fn logo_style_primary(&self) -> Style {
        Style::default()
            .fg(self.logo_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Logo secondary style
    #[must_use]
    pub fn logo_style_secondary(&self) -> Style {
        Style::default()
            .fg(self.logo_secondary)
            .add_modifier(Modifier::BOLD)
    }
}
