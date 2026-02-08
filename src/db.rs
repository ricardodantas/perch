//! Database module for `SQLite` storage (accounts, drafts, cache, scheduled posts)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use uuid::Uuid;

use crate::models::{Account, Network, Post, ScheduledPost, ScheduledPostStatus};
use crate::paths;

/// Database connection wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database at the default location
    pub fn open() -> Result<Self> {
        let path = Self::default_path()?;
        Self::open_path(&path)
    }

    /// Open or create the database at a specific path
    pub fn open_path(path: &PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create data directory")?;
        }

        let conn = Connection::open(path).context("Failed to open database")?;

        let db = Self { conn };
        db.init()?;

        Ok(db)
    }

    /// Get the default database path
    pub fn default_path() -> Result<PathBuf> {
        paths::database_path()
    }

    /// Initialize the database schema
    fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            r"
            -- Accounts table
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                network TEXT NOT NULL,
                display_name TEXT NOT NULL,
                handle TEXT NOT NULL,
                server TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                avatar_url TEXT,
                created_at TEXT NOT NULL,
                last_used_at TEXT
            );

            -- Drafts table
            CREATE TABLE IF NOT EXISTS drafts (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                networks TEXT NOT NULL,
                reply_to_id TEXT,
                reply_to_network TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Scheduled posts table
            CREATE TABLE IF NOT EXISTS scheduled_posts (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                networks TEXT NOT NULL,
                scheduled_for TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                error TEXT,
                created_at TEXT NOT NULL
            );

            -- Post cache table
            CREATE TABLE IF NOT EXISTS post_cache (
                id TEXT PRIMARY KEY,
                network_id TEXT NOT NULL,
                network TEXT NOT NULL,
                author_handle TEXT NOT NULL,
                author_name TEXT NOT NULL,
                author_avatar TEXT,
                content TEXT NOT NULL,
                content_raw TEXT,
                created_at TEXT NOT NULL,
                url TEXT,
                is_repost INTEGER NOT NULL DEFAULT 0,
                repost_author TEXT,
                like_count INTEGER NOT NULL DEFAULT 0,
                repost_count INTEGER NOT NULL DEFAULT 0,
                reply_count INTEGER NOT NULL DEFAULT 0,
                liked INTEGER NOT NULL DEFAULT 0,
                reposted INTEGER NOT NULL DEFAULT 0,
                reply_to_id TEXT,
                cid TEXT,
                uri TEXT,
                media_json TEXT DEFAULT '[]',
                cached_at TEXT NOT NULL,
                UNIQUE(network, network_id)
            );

            -- Indexes
            CREATE INDEX IF NOT EXISTS idx_accounts_network ON accounts(network);
            CREATE INDEX IF NOT EXISTS idx_post_cache_network ON post_cache(network);
            CREATE INDEX IF NOT EXISTS idx_post_cache_cached_at ON post_cache(cached_at);
            CREATE INDEX IF NOT EXISTS idx_scheduled_posts_status ON scheduled_posts(status);
            CREATE INDEX IF NOT EXISTS idx_scheduled_posts_scheduled_for ON scheduled_posts(scheduled_for);
            ",
        )?;

        Ok(())
    }

    // ==================== Accounts ====================

    /// Insert a new account
    pub fn insert_account(&self, account: &Account) -> Result<()> {
        self.conn.execute(
            r"INSERT INTO accounts (id, network, display_name, handle, server, is_default, avatar_url, created_at, last_used_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                account.id.to_string(),
                format!("{:?}", account.network).to_lowercase(),
                account.display_name,
                account.handle,
                account.server,
                i32::from(account.is_default),
                account.avatar_url,
                account.created_at.to_rfc3339(),
                account.last_used_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Get all accounts
    pub fn get_accounts(&self) -> Result<Vec<Account>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, network, display_name, handle, server, is_default, avatar_url, created_at, last_used_at 
             FROM accounts ORDER BY network, display_name"
        )?;

        let accounts = stmt.query_map([], |row| {
            let network_str: String = row.get(1)?;
            let network = Network::from_str(&network_str).unwrap_or_default();

            Ok(Account {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                network,
                display_name: row.get(2)?,
                handle: row.get(3)?,
                server: row.get(4)?,
                is_default: row.get::<_, i32>(5)? != 0,
                avatar_url: row.get(6)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_used_at: row
                    .get::<_, Option<String>>(8)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        })?;

        accounts.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get accounts for a specific network
    pub fn get_accounts_for_network(&self, network: Network) -> Result<Vec<Account>> {
        let network_str = format!("{network:?}").to_lowercase();
        let mut stmt = self.conn.prepare(
            "SELECT id, network, display_name, handle, server, is_default, avatar_url, created_at, last_used_at 
             FROM accounts WHERE network = ?1 ORDER BY display_name"
        )?;

        let accounts = stmt.query_map(params![network_str], |row| {
            let net_str: String = row.get(1)?;
            let net = Network::from_str(&net_str).unwrap_or_default();

            Ok(Account {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                network: net,
                display_name: row.get(2)?,
                handle: row.get(3)?,
                server: row.get(4)?,
                is_default: row.get::<_, i32>(5)? != 0,
                avatar_url: row.get(6)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_used_at: row
                    .get::<_, Option<String>>(8)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        })?;

        accounts.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get the default account for a network
    pub fn get_default_account(&self, network: Network) -> Result<Option<Account>> {
        let network_str = format!("{network:?}").to_lowercase();
        let mut stmt = self.conn.prepare(
            "SELECT id, network, display_name, handle, server, is_default, avatar_url, created_at, last_used_at 
             FROM accounts WHERE network = ?1 AND is_default = 1"
        )?;

        let result = stmt.query_row(params![network_str], |row| {
            let net_str: String = row.get(1)?;
            let net = Network::from_str(&net_str).unwrap_or_default();

            Ok(Account {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                network: net,
                display_name: row.get(2)?,
                handle: row.get(3)?,
                server: row.get(4)?,
                is_default: row.get::<_, i32>(5)? != 0,
                avatar_url: row.get(6)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_used_at: row
                    .get::<_, Option<String>>(8)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        });

        match result {
            Ok(account) => Ok(Some(account)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete an account
    pub fn delete_account(&self, id: Uuid) -> Result<()> {
        self.conn.execute(
            "DELETE FROM accounts WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    /// Set an account as the default for its network
    pub fn set_default_account(&self, id: Uuid, _network: Network) -> Result<()> {
        // Unset ALL current defaults (only one default across all networks)
        self.conn
            .execute("UPDATE accounts SET is_default = 0", params![])?;

        // Set new default
        self.conn.execute(
            "UPDATE accounts SET is_default = 1 WHERE id = ?1",
            params![id.to_string()],
        )?;

        Ok(())
    }

    /// Update last used timestamp
    pub fn update_account_last_used(&self, id: Uuid) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts SET last_used_at = ?2 WHERE id = ?1",
            params![id.to_string(), Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    // ==================== Post Cache ====================

    /// Cache a post
    pub fn cache_post(&self, post: &Post) -> Result<()> {
        let network_str = format!("{:?}", post.network).to_lowercase();
        let media_json = serde_json::to_string(&post.media).unwrap_or_else(|_| "[]".to_string());

        self.conn.execute(
            r"INSERT OR REPLACE INTO post_cache 
               (id, network_id, network, author_handle, author_name, author_avatar, 
                content, content_raw, created_at, url, is_repost, repost_author,
                like_count, repost_count, reply_count, liked, reposted, reply_to_id,
                cid, uri, media_json, cached_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
            params![
                post.id.to_string(),
                post.network_id,
                network_str,
                post.author_handle,
                post.author_name,
                post.author_avatar,
                post.content,
                post.content_raw,
                post.created_at.to_rfc3339(),
                post.url,
                i32::from(post.is_repost),
                post.repost_author,
                post.like_count,
                post.repost_count,
                post.reply_count,
                i32::from(post.liked),
                i32::from(post.reposted),
                post.reply_to_id,
                post.cid,
                post.uri,
                media_json,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get cached posts for a network (most recent first)
    pub fn get_cached_posts(&self, network: Option<Network>, limit: usize) -> Result<Vec<Post>> {
        let sql = if let Some(net) = network {
            let network_str = format!("{net:?}").to_lowercase();
            format!(
                "SELECT id, network_id, network, author_handle, author_name, author_avatar,
                        content, content_raw, created_at, url, is_repost, repost_author,
                        like_count, repost_count, reply_count, liked, reposted, reply_to_id,
                        cid, uri, media_json
                 FROM post_cache WHERE network = '{network_str}' ORDER BY created_at DESC LIMIT {limit}"
            )
        } else {
            format!(
                "SELECT id, network_id, network, author_handle, author_name, author_avatar,
                        content, content_raw, created_at, url, is_repost, repost_author,
                        like_count, repost_count, reply_count, liked, reposted, reply_to_id,
                        cid, uri, media_json
                 FROM post_cache ORDER BY created_at DESC LIMIT {limit}"
            )
        };

        let mut stmt = self.conn.prepare(&sql)?;

        let posts = stmt.query_map([], |row| {
            let network_str: String = row.get(2)?;
            let network = Network::from_str(&network_str).unwrap_or_default();

            // Deserialize media from JSON
            let media_json: String = row
                .get::<_, Option<String>>(20)?
                .unwrap_or_else(|| "[]".to_string());
            let media: Vec<crate::models::MediaAttachment> =
                serde_json::from_str(&media_json).unwrap_or_default();

            Ok(Post {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                network_id: row.get(1)?,
                network,
                author_handle: row.get(3)?,
                author_name: row.get(4)?,
                author_avatar: row.get(5)?,
                content: row.get(6)?,
                content_raw: row.get(7)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .with_timezone(&Utc),
                url: row.get(9)?,
                is_repost: row.get::<_, i32>(10)? != 0,
                repost_author: row.get(11)?,
                like_count: row.get(12)?,
                repost_count: row.get(13)?,
                reply_count: row.get(14)?,
                liked: row.get::<_, i32>(15)? != 0,
                reposted: row.get::<_, i32>(16)? != 0,
                reply_to_id: row.get(17)?,
                media,
                cid: row.get(18)?,
                uri: row.get(19)?,
            })
        })?;

        posts.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Clear old cache entries
    pub fn clear_old_cache(&self, max_age_hours: u64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours as i64);
        let count = self.conn.execute(
            "DELETE FROM post_cache WHERE cached_at < ?1",
            params![cutoff.to_rfc3339()],
        )?;
        Ok(count)
    }

    // ==================== Scheduled Posts ====================

    /// Save a scheduled post
    pub fn save_scheduled_post(&self, post: &ScheduledPost) -> Result<()> {
        self.conn.execute(
            r"INSERT INTO scheduled_posts (id, content, networks, scheduled_for, status, error, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                post.id.to_string(),
                post.content,
                post.networks_str(),
                post.scheduled_for.to_rfc3339(),
                post.status.as_str(),
                post.error,
                post.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get all scheduled posts (sorted by scheduled time)
    pub fn get_scheduled_posts(&self) -> Result<Vec<ScheduledPost>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, networks, scheduled_for, status, error, created_at
             FROM scheduled_posts ORDER BY scheduled_for ASC",
        )?;

        let posts = stmt.query_map([], Self::row_to_scheduled_post)?;
        posts.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get pending scheduled posts that are due
    pub fn get_due_scheduled_posts(&self) -> Result<Vec<ScheduledPost>> {
        let now = Utc::now().to_rfc3339();
        let mut stmt = self.conn.prepare(
            "SELECT id, content, networks, scheduled_for, status, error, created_at
             FROM scheduled_posts 
             WHERE status = 'pending' AND scheduled_for <= ?1
             ORDER BY scheduled_for ASC",
        )?;

        let posts = stmt.query_map(params![now], Self::row_to_scheduled_post)?;
        posts.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get pending scheduled posts (not yet posted)
    pub fn get_pending_scheduled_posts(&self) -> Result<Vec<ScheduledPost>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, networks, scheduled_for, status, error, created_at
             FROM scheduled_posts 
             WHERE status = 'pending'
             ORDER BY scheduled_for ASC",
        )?;

        let posts = stmt.query_map([], Self::row_to_scheduled_post)?;
        posts.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Update scheduled post status
    pub fn update_scheduled_post_status(
        &self,
        id: Uuid,
        status: ScheduledPostStatus,
        error: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE scheduled_posts SET status = ?2, error = ?3 WHERE id = ?1",
            params![id.to_string(), status.as_str(), error],
        )?;
        Ok(())
    }

    /// Delete a scheduled post
    pub fn delete_scheduled_post(&self, id: Uuid) -> Result<()> {
        self.conn.execute(
            "DELETE FROM scheduled_posts WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    /// Cancel a scheduled post (sets status to cancelled)
    pub fn cancel_scheduled_post(&self, id: Uuid) -> Result<()> {
        self.update_scheduled_post_status(id, ScheduledPostStatus::Cancelled, None)
    }

    /// Helper to convert a row to `ScheduledPost`
    fn row_to_scheduled_post(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScheduledPost> {
        let networks_str: String = row.get(2)?;
        let status_str: String = row.get(4)?;

        Ok(ScheduledPost {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            content: row.get(1)?,
            networks: ScheduledPost::networks_from_str(&networks_str),
            scheduled_for: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                .unwrap()
                .with_timezone(&Utc),
            status: ScheduledPostStatus::from_str(&status_str).unwrap_or_default(),
            error: row.get(5)?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .unwrap()
                .with_timezone(&Utc),
        })
    }

    /// Clear old completed/failed/cancelled scheduled posts
    pub fn clear_old_scheduled_posts(&self, max_age_hours: u64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours as i64);
        let count = self.conn.execute(
            "DELETE FROM scheduled_posts WHERE status IN ('posted', 'failed', 'cancelled') AND created_at < ?1",
            params![cutoff.to_rfc3339()],
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_init() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sqlite");
        let _db = Database::open_path(&path).unwrap();
        // Should create without error
    }

    #[test]
    fn test_account_crud() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sqlite");
        let db = Database::open_path(&path).unwrap();

        // Create account
        let account = Account::new_mastodon("test", "https://mastodon.social", "Test User");
        db.insert_account(&account).unwrap();

        // Read accounts
        let accounts = db.get_accounts().unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].handle, "test");

        // Delete account
        db.delete_account(accounts[0].id).unwrap();
        let accounts = db.get_accounts().unwrap();
        assert!(accounts.is_empty());
    }
}
