//! Scheduled post model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Network;

/// Status of a scheduled post
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ScheduledPostStatus {
    /// Waiting to be posted
    #[default]
    Pending,
    /// Currently being posted
    Posting,
    /// Successfully posted
    Posted,
    /// Failed to post
    Failed,
    /// Cancelled by user
    Cancelled,
}

impl ScheduledPostStatus {
    /// Get status as string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Posting => "posting",
            Self::Posted => "posted",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse status from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(Self::Pending),
            "posting" => Some(Self::Posting),
            "posted" => Some(Self::Posted),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Get emoji for status
    pub const fn emoji(&self) -> &'static str {
        match self {
            Self::Pending => "⏳",
            Self::Posting => "📤",
            Self::Posted => "✅",
            Self::Failed => "❌",
            Self::Cancelled => "🚫",
        }
    }
}

/// A scheduled post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledPost {
    /// Unique identifier
    pub id: Uuid,
    /// Post content
    pub content: String,
    /// Target networks
    pub networks: Vec<Network>,
    /// When to post
    pub scheduled_for: DateTime<Utc>,
    /// Current status
    pub status: ScheduledPostStatus,
    /// Error message if failed
    pub error: Option<String>,
    /// When this was created
    pub created_at: DateTime<Utc>,
}

impl ScheduledPost {
    /// Create a new scheduled post
    pub fn new(
        content: impl Into<String>,
        networks: Vec<Network>,
        scheduled_for: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: content.into(),
            networks,
            scheduled_for,
            status: ScheduledPostStatus::Pending,
            error: None,
            created_at: Utc::now(),
        }
    }

    /// Check if this post is due (scheduled time has passed)
    pub fn is_due(&self) -> bool {
        self.status == ScheduledPostStatus::Pending && Utc::now() >= self.scheduled_for
    }

    /// Get networks as comma-separated string
    pub fn networks_str(&self) -> String {
        self.networks
            .iter()
            .map(|n| format!("{:?}", n).to_lowercase())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Parse networks from comma-separated string
    pub fn networks_from_str(s: &str) -> Vec<Network> {
        s.split(',')
            .filter_map(|n| Network::from_str(n.trim()))
            .collect()
    }

    /// Human-readable time until posting
    pub fn time_until(&self) -> String {
        let now = Utc::now();
        if self.scheduled_for <= now {
            return "now".to_string();
        }

        let duration = self.scheduled_for - now;
        let seconds = duration.num_seconds();

        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m", seconds / 60)
        } else if seconds < 86400 {
            let hours = seconds / 3600;
            let mins = (seconds % 3600) / 60;
            if mins > 0 {
                format!("{}h {}m", hours, mins)
            } else {
                format!("{}h", hours)
            }
        } else {
            let days = seconds / 86400;
            let hours = (seconds % 86400) / 3600;
            if hours > 0 {
                format!("{}d {}h", days, hours)
            } else {
                format!("{}d", days)
            }
        }
    }

    /// Format scheduled time for display
    pub fn scheduled_time_display(&self) -> String {
        self.scheduled_for.format("%Y-%m-%d %H:%M UTC").to_string()
    }
}
