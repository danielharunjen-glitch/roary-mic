use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info, warn};
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_specta::Event;

/// Database migrations for transcription history.
/// Each migration is applied in order. The library tracks which migrations
/// have been applied using SQLite's user_version pragma.
///
/// Note: For users upgrading from tauri-plugin-sql, migrate_from_tauri_plugin_sql()
/// converts the old _sqlx_migrations table tracking to the user_version pragma,
/// ensuring migrations don't re-run on existing databases.
static MIGRATIONS: &[M] = &[
    M::up(
        "CREATE TABLE IF NOT EXISTS transcription_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            saved BOOLEAN NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            transcription_text TEXT NOT NULL
        );",
    ),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_processed_text TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_prompt TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_requested BOOLEAN NOT NULL DEFAULT 0;"),
    M::up(
        "CREATE TABLE IF NOT EXISTS corrections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            original_text TEXT NOT NULL,
            corrected_text TEXT NOT NULL,
            history_id INTEGER,
            created_at INTEGER NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1
        );
        CREATE INDEX IF NOT EXISTS idx_corrections_enabled_created
            ON corrections(enabled, created_at DESC);",
    ),
    // Add a `kind` column so the UI can split corrections (Whisper mistakes)
    // from references (phrase → expansion, e.g. "my email" → "you@example.com").
    // Both kinds share the same substitution engine.
    M::up("ALTER TABLE corrections ADD COLUMN kind TEXT NOT NULL DEFAULT 'correction';"),
];

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct PaginatedHistory {
    pub entries: Vec<HistoryEntry>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(tag = "action")]
pub enum HistoryUpdatePayload {
    #[serde(rename = "added")]
    Added { entry: HistoryEntry },
    #[serde(rename = "updated")]
    Updated { entry: HistoryEntry },
    #[serde(rename = "deleted")]
    Deleted { id: i64 },
    #[serde(rename = "toggled")]
    Toggled { id: i64 },
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct HistoryEntry {
    pub id: i64,
    pub file_name: String,
    pub timestamp: i64,
    pub saved: bool,
    pub title: String,
    pub transcription_text: String,
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
    pub post_process_requested: bool,
}

/// A user-authored correction: `original_text` in a future transcription will
/// be replaced with `corrected_text`. Created when the user edits a history
/// entry and the diff can be reduced to a single contiguous span.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Correction {
    pub id: i64,
    pub original_text: String,
    pub corrected_text: String,
    pub history_id: Option<i64>,
    pub created_at: i64,
    pub enabled: bool,
    /// `"correction"` (default) for Whisper-mistake fixes, `"reference"` for
    /// phrase expansions (say "my email" → insert actual email).
    pub kind: String,
}

/// Reduce a before/after pair to the minimal differing span by tokenizing on
/// whitespace and stripping the common prefix and suffix. Falls back to the
/// full strings when the diff is a pure insertion or deletion — that way every
/// edit produces a usable find/replace rule.
pub fn extract_correction_pair(original: &str, corrected: &str) -> Option<(String, String)> {
    let original = original.trim();
    let corrected = corrected.trim();

    if original == corrected || original.is_empty() || corrected.is_empty() {
        return None;
    }

    let orig: Vec<&str> = original.split_whitespace().collect();
    let corr: Vec<&str> = corrected.split_whitespace().collect();

    let prefix = orig
        .iter()
        .zip(corr.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let remaining_orig = orig.len() - prefix;
    let remaining_corr = corr.len() - prefix;

    let suffix = orig[prefix..]
        .iter()
        .rev()
        .zip(corr[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count()
        .min(remaining_orig)
        .min(remaining_corr);

    let orig_mid = &orig[prefix..orig.len() - suffix];
    let corr_mid = &corr[prefix..corr.len() - suffix];

    // Prefer a narrow single-span rule when available.
    if !orig_mid.is_empty() && !corr_mid.is_empty() {
        return Some((orig_mid.join(" "), corr_mid.join(" ")));
    }

    // Fallback: record the full-string pair. Matches the exact phrase on
    // future transcriptions but at least guarantees that an edit is never
    // silently lost.
    Some((original.to_string(), corrected.to_string()))
}

/// Apply enabled corrections to `text` in the order given. Matching is
/// case-insensitive so `halo → hello` also rewrites `Halo` at a sentence
/// start. Word boundaries are applied only at ends of the pattern that
/// start/end with a word character — so "OMW" matches as a word but a
/// pattern like "!!!" matches literally. Later corrections can see the
/// output of earlier ones.
pub fn apply_corrections(text: &str, corrections: &[Correction]) -> String {
    let mut result = text.to_string();
    for c in corrections
        .iter()
        .filter(|c| c.enabled && !c.original_text.is_empty())
    {
        let is_word_char = |ch: char| ch.is_alphanumeric() || ch == '_';
        let starts_word = c
            .original_text
            .chars()
            .next()
            .map(is_word_char)
            .unwrap_or(false);
        let ends_word = c
            .original_text
            .chars()
            .next_back()
            .map(is_word_char)
            .unwrap_or(false);
        let escaped = regex::escape(&c.original_text);
        let body = match (starts_word, ends_word) {
            (true, true) => format!(r"\b{}\b", escaped),
            (true, false) => format!(r"\b{}", escaped),
            (false, true) => format!(r"{}\b", escaped),
            (false, false) => escaped,
        };
        // `(?i)` flag → case-insensitive matching. Replacement text is kept
        // verbatim, so users get exactly what they typed in the edit.
        let pattern = format!("(?i){}", body);
        match Regex::new(&pattern) {
            Ok(re) => {
                // `NoExpand` treats the replacement as a literal string so
                // `$1`, `${name}` etc. don't get interpreted as capture
                // references. Without it, a user reference like
                // "price" -> "$100" would be silently mangled.
                result = re
                    .replace_all(&result, regex::NoExpand(c.corrected_text.as_str()))
                    .to_string();
            }
            Err(e) => {
                warn!("Skipping correction {} due to invalid regex: {}", c.id, e);
            }
        }
    }
    result
}

pub struct HistoryManager {
    app_handle: AppHandle,
    recordings_dir: PathBuf,
    db_path: PathBuf,
}

impl HistoryManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Create recordings directory in app data dir
        let app_data_dir = crate::portable::app_data_dir(app_handle)?;
        let recordings_dir = app_data_dir.join("recordings");
        let db_path = app_data_dir.join("history.db");

        // Ensure recordings directory exists
        if !recordings_dir.exists() {
            fs::create_dir_all(&recordings_dir)?;
            debug!("Created recordings directory: {:?}", recordings_dir);
        }

        let manager = Self {
            app_handle: app_handle.clone(),
            recordings_dir,
            db_path,
        };

        // Initialize database and run migrations synchronously
        manager.init_database()?;

        Ok(manager)
    }

    fn init_database(&self) -> Result<()> {
        info!("Initializing database at {:?}", self.db_path);

        let mut conn = Connection::open(&self.db_path)?;

        // Handle migration from tauri-plugin-sql to rusqlite_migration
        // tauri-plugin-sql used _sqlx_migrations table, rusqlite_migration uses user_version pragma
        self.migrate_from_tauri_plugin_sql(&conn)?;

        // Create migrations object and run to latest version
        let migrations = Migrations::new(MIGRATIONS.to_vec());

        // Validate migrations in debug builds
        #[cfg(debug_assertions)]
        migrations.validate().expect("Invalid migrations");

        // Get current version before migration
        let version_before: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        debug!("Database version before migration: {}", version_before);

        // Apply any pending migrations
        migrations.to_latest(&mut conn)?;

        // Get version after migration
        let version_after: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version_after > version_before {
            info!(
                "Database migrated from version {} to {}",
                version_before, version_after
            );
        } else {
            debug!("Database already at latest version {}", version_after);
        }

        Ok(())
    }

    /// Migrate from tauri-plugin-sql's migration tracking to rusqlite_migration's.
    /// tauri-plugin-sql used a _sqlx_migrations table, while rusqlite_migration uses
    /// SQLite's user_version pragma. This function checks if the old system was in use
    /// and sets the user_version accordingly so migrations don't re-run.
    fn migrate_from_tauri_plugin_sql(&self, conn: &Connection) -> Result<()> {
        // Check if the old _sqlx_migrations table exists
        let has_sqlx_migrations: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_sqlx_migrations {
            return Ok(());
        }

        // Check current user_version
        let current_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if current_version > 0 {
            // Already migrated to rusqlite_migration system
            return Ok(());
        }

        // Get the highest version from the old migrations table
        let old_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if old_version > 0 {
            info!(
                "Migrating from tauri-plugin-sql (version {}) to rusqlite_migration",
                old_version
            );

            // Set user_version to match the old migration state
            conn.pragma_update(None, "user_version", old_version)?;

            // Optionally drop the old migrations table (keeping it doesn't hurt)
            // conn.execute("DROP TABLE IF EXISTS _sqlx_migrations", [])?;

            info!(
                "Migration tracking converted: user_version set to {}",
                old_version
            );
        }

        Ok(())
    }

    fn get_connection(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    fn map_correction(row: &rusqlite::Row<'_>) -> rusqlite::Result<Correction> {
        Ok(Correction {
            id: row.get("id")?,
            original_text: row.get("original_text")?,
            corrected_text: row.get("corrected_text")?,
            history_id: row.get("history_id")?,
            created_at: row.get("created_at")?,
            enabled: row.get::<_, i64>("enabled")? != 0,
            kind: row
                .get::<_, Option<String>>("kind")?
                .unwrap_or_else(|| "correction".to_string()),
        })
    }

    fn map_history_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
        Ok(HistoryEntry {
            id: row.get("id")?,
            file_name: row.get("file_name")?,
            timestamp: row.get("timestamp")?,
            saved: row.get("saved")?,
            title: row.get("title")?,
            transcription_text: row.get("transcription_text")?,
            post_processed_text: row.get("post_processed_text")?,
            post_process_prompt: row.get("post_process_prompt")?,
            post_process_requested: row.get("post_process_requested")?,
        })
    }

    pub fn recordings_dir(&self) -> &std::path::Path {
        &self.recordings_dir
    }

    /// Save a new history entry to the database.
    /// The WAV file should already have been written to the recordings directory.
    pub fn save_entry(
        &self,
        file_name: String,
        transcription_text: String,
        post_process_requested: bool,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
    ) -> Result<HistoryEntry> {
        let timestamp = Utc::now().timestamp();
        let title = self.format_timestamp_title(timestamp);

        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &file_name,
                timestamp,
                false,
                &title,
                &transcription_text,
                &post_processed_text,
                &post_process_prompt,
                post_process_requested,
            ],
        )?;

        let entry = HistoryEntry {
            id: conn.last_insert_rowid(),
            file_name,
            timestamp,
            saved: false,
            title,
            transcription_text,
            post_processed_text,
            post_process_prompt,
            post_process_requested,
        };

        debug!("Saved history entry with id {}", entry.id);

        self.cleanup_old_entries()?;

        // Emit typed event for real-time frontend updates
        if let Err(e) = (HistoryUpdatePayload::Added {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    /// Update an existing history entry with new transcription results (used by retry).
    pub fn update_transcription(
        &self,
        id: i64,
        transcription_text: String,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
    ) -> Result<HistoryEntry> {
        let conn = self.get_connection()?;
        let updated = conn.execute(
            "UPDATE transcription_history
             SET transcription_text = ?1,
                 post_processed_text = ?2,
                 post_process_prompt = ?3
             WHERE id = ?4",
            params![
                transcription_text,
                post_processed_text,
                post_process_prompt,
                id
            ],
        )?;

        if updated == 0 {
            return Err(anyhow!("History entry {} not found", id));
        }

        let entry = conn
            .query_row(
                "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                 FROM transcription_history WHERE id = ?1",
                params![id],
                Self::map_history_entry,
            )?;

        debug!("Updated transcription for history entry {}", id);

        if let Err(e) = (HistoryUpdatePayload::Updated {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    pub fn cleanup_old_entries(&self) -> Result<()> {
        let retention_period = crate::settings::get_recording_retention_period(&self.app_handle);

        match retention_period {
            crate::settings::RecordingRetentionPeriod::Never => {
                // Don't delete anything
                return Ok(());
            }
            crate::settings::RecordingRetentionPeriod::PreserveLimit => {
                // Use the old count-based logic with history_limit
                let limit = crate::settings::get_history_limit(&self.app_handle);
                return self.cleanup_by_count(limit);
            }
            _ => {
                // Use time-based logic
                return self.cleanup_by_time(retention_period);
            }
        }
    }

    fn delete_entries_and_files(&self, entries: &[(i64, String)]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let conn = self.get_connection()?;
        let mut deleted_count = 0;

        for (id, file_name) in entries {
            // Delete database entry
            conn.execute(
                "DELETE FROM transcription_history WHERE id = ?1",
                params![id],
            )?;

            // Delete WAV file
            let file_path = self.recordings_dir.join(file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete WAV file {}: {}", file_name, e);
                } else {
                    debug!("Deleted old WAV file: {}", file_name);
                    deleted_count += 1;
                }
            }
        }

        Ok(deleted_count)
    }

    fn cleanup_by_count(&self, limit: usize) -> Result<()> {
        let conn = self.get_connection()?;

        // Get all entries that are not saved, ordered by timestamp desc
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        if entries.len() > limit {
            let entries_to_delete = &entries[limit..];
            let deleted_count = self.delete_entries_and_files(entries_to_delete)?;

            if deleted_count > 0 {
                debug!("Cleaned up {} old history entries by count", deleted_count);
            }
        }

        Ok(())
    }

    fn cleanup_by_time(
        &self,
        retention_period: crate::settings::RecordingRetentionPeriod,
    ) -> Result<()> {
        let conn = self.get_connection()?;

        // Calculate cutoff timestamp (current time minus retention period)
        let now = Utc::now().timestamp();
        let cutoff_timestamp = match retention_period {
            crate::settings::RecordingRetentionPeriod::Days3 => now - (3 * 24 * 60 * 60), // 3 days in seconds
            crate::settings::RecordingRetentionPeriod::Weeks2 => now - (2 * 7 * 24 * 60 * 60), // 2 weeks in seconds
            crate::settings::RecordingRetentionPeriod::Months3 => now - (3 * 30 * 24 * 60 * 60), // 3 months in seconds (approximate)
            _ => unreachable!("Should not reach here"),
        };

        // Get all unsaved entries older than the cutoff timestamp
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 AND timestamp < ?1",
        )?;

        let rows = stmt.query_map(params![cutoff_timestamp], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries_to_delete: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries_to_delete.push(row?);
        }

        let deleted_count = self.delete_entries_and_files(&entries_to_delete)?;

        if deleted_count > 0 {
            debug!(
                "Cleaned up {} old history entries based on retention period",
                deleted_count
            );
        }

        Ok(())
    }

    pub async fn get_history_entries(
        &self,
        cursor: Option<i64>,
        limit: Option<usize>,
    ) -> Result<PaginatedHistory> {
        let conn = self.get_connection()?;
        let limit = limit.map(|l| l.min(100));

        let mut entries: Vec<HistoryEntry> = match (cursor, limit) {
            (Some(cursor_id), Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt = conn.prepare(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history
                     WHERE id < ?1
                     ORDER BY id DESC
                     LIMIT ?2",
                )?;
                let result = stmt
                    .query_map(params![cursor_id, fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (None, Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt = conn.prepare(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history
                     ORDER BY id DESC
                     LIMIT ?1",
                )?;
                let result = stmt
                    .query_map(params![fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (_, None) => {
                let mut stmt = conn.prepare(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history
                     ORDER BY id DESC",
                )?;
                let result = stmt
                    .query_map([], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
        };

        let has_more = limit.is_some_and(|lim| entries.len() > lim);
        if has_more {
            entries.pop();
        }

        Ok(PaginatedHistory { entries, has_more })
    }

    #[cfg(test)]
    fn get_latest_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(
            "SELECT
                id,
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
             FROM transcription_history
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    /// Get the latest entry with non-empty transcription text.
    pub fn get_latest_completed_entry(&self) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        Self::get_latest_completed_entry_with_conn(&conn)
    }

    fn get_latest_completed_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(
            "SELECT
                id,
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
             FROM transcription_history
             WHERE transcription_text != ''
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    pub async fn toggle_saved_status(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get current saved status
        let current_saved: bool = conn.query_row(
            "SELECT saved FROM transcription_history WHERE id = ?1",
            params![id],
            |row| row.get("saved"),
        )?;

        let new_saved = !current_saved;

        conn.execute(
            "UPDATE transcription_history SET saved = ?1 WHERE id = ?2",
            params![new_saved, id],
        )?;

        debug!("Toggled saved status for entry {}: {}", id, new_saved);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Toggled { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    pub fn get_audio_file_path(&self, file_name: &str) -> PathBuf {
        self.recordings_dir.join(file_name)
    }

    pub async fn get_entry_by_id(&self, id: i64) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT
                id,
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
             FROM transcription_history
             WHERE id = ?1",
        )?;

        let entry = stmt.query_row([id], Self::map_history_entry).optional()?;

        Ok(entry)
    }

    pub async fn delete_entry(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get the entry to find the file name
        if let Some(entry) = self.get_entry_by_id(id).await? {
            // Delete the audio file first
            let file_path = self.get_audio_file_path(&entry.file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    // Continue with database deletion even if file deletion fails
                }
            }
        }

        // Delete from database
        conn.execute(
            "DELETE FROM transcription_history WHERE id = ?1",
            params![id],
        )?;

        debug!("Deleted history entry with id: {}", id);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Deleted { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    /// Update the effective text of a history entry (what the user meant it to
    /// say). The post-processed column is treated as the "final" text if it
    /// was set; otherwise the raw transcription is. The diff against the prior
    /// effective text is saved as a [`Correction`] if it reduces to a single
    /// contiguous span.
    pub async fn update_entry_text(&self, id: i64, new_text: String) -> Result<HistoryEntry> {
        let conn = self.get_connection()?;

        let (transcription_text, post_processed_text, post_process_requested) = conn.query_row(
            "SELECT transcription_text, post_processed_text, post_process_requested
             FROM transcription_history WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )?;

        let prior_text: String = post_processed_text
            .clone()
            .unwrap_or_else(|| transcription_text.clone());

        if prior_text == new_text {
            // No-op — return the existing entry.
            return conn
                .query_row(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history WHERE id = ?1",
                    params![id],
                    Self::map_history_entry,
                )
                .map_err(|e| anyhow!(e));
        }

        // Persist the user's correction for future application. When the entry
        // has no post_processed_text we write to transcription_text directly;
        // otherwise we write to post_processed_text so the raw transcription is
        // preserved alongside the user's intent.
        let updated = if post_processed_text.is_some() || post_process_requested {
            conn.execute(
                "UPDATE transcription_history
                 SET post_processed_text = ?1
                 WHERE id = ?2",
                params![new_text, id],
            )?
        } else {
            conn.execute(
                "UPDATE transcription_history
                 SET transcription_text = ?1
                 WHERE id = ?2",
                params![new_text, id],
            )?
        };

        if updated == 0 {
            return Err(anyhow!("History entry {} not found", id));
        }

        if let Some((orig, corr)) = extract_correction_pair(&prior_text, &new_text) {
            let ts = Utc::now().timestamp();
            conn.execute(
                "INSERT INTO corrections (original_text, corrected_text, history_id, created_at, enabled, kind)
                 VALUES (?1, ?2, ?3, ?4, 1, 'correction')",
                params![orig, corr, id, ts],
            )?;
            debug!(
                "Saved correction for history entry {}: '{}' -> '{}'",
                id, orig, corr
            );
        } else {
            debug!(
                "Edit on history entry {} did not reduce to a single span; no correction saved",
                id
            );
        }

        let entry = conn.query_row(
            "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
             FROM transcription_history WHERE id = ?1",
            params![id],
            Self::map_history_entry,
        )?;

        if let Err(e) = (HistoryUpdatePayload::Updated {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    /// List recent corrections (most recent first), capped at `limit` rows.
    /// Pass `Some(kind)` to filter to a specific kind ("correction" or
    /// "reference"); pass `None` to get every kind.
    pub fn list_corrections(&self, limit: usize, kind: Option<&str>) -> Result<Vec<Correction>> {
        let conn = self.get_connection()?;
        Self::list_corrections_with_conn(&conn, limit, kind)
    }

    fn list_corrections_with_conn(
        conn: &Connection,
        limit: usize,
        kind: Option<&str>,
    ) -> Result<Vec<Correction>> {
        let rows: Vec<Correction> = if let Some(k) = kind {
            let mut stmt = conn.prepare(
                "SELECT id, original_text, corrected_text, history_id, created_at, enabled, kind
                 FROM corrections
                 WHERE kind = ?1
                 ORDER BY created_at DESC
                 LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(params![k, limit as i64], Self::map_correction)?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, original_text, corrected_text, history_id, created_at, enabled, kind
                 FROM corrections
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )?;
            let rows = stmt
                .query_map(params![limit as i64], Self::map_correction)?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };
        Ok(rows)
    }

    /// Load every enabled correction. Used to apply them to new transcriptions
    /// before paste.
    pub fn get_active_corrections(&self) -> Result<Vec<Correction>> {
        let conn = self.get_connection()?;
        Self::get_active_corrections_with_conn(&conn)
    }

    fn get_active_corrections_with_conn(conn: &Connection) -> Result<Vec<Correction>> {
        let mut stmt = conn.prepare(
            "SELECT id, original_text, corrected_text, history_id, created_at, enabled, kind
             FROM corrections
             WHERE enabled = 1
             ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map([], Self::map_correction)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn set_correction_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let conn = self.get_connection()?;
        let updated = conn.execute(
            "UPDATE corrections SET enabled = ?1 WHERE id = ?2",
            params![if enabled { 1 } else { 0 }, id],
        )?;
        if updated == 0 {
            return Err(anyhow!("Correction {} not found", id));
        }
        Ok(())
    }

    pub fn delete_correction(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;
        let deleted = conn.execute("DELETE FROM corrections WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(anyhow!("Correction {} not found", id));
        }
        Ok(())
    }

    /// Insert a user-authored correction that isn't derived from an edit.
    /// `kind` should be "correction" (default) or "reference".
    pub fn insert_correction(
        &self,
        original_text: String,
        corrected_text: String,
        kind: Option<String>,
    ) -> Result<Correction> {
        let original_text = original_text.trim().to_string();
        let corrected_text = corrected_text.trim().to_string();
        let kind = kind.unwrap_or_else(|| "correction".to_string());
        if original_text.is_empty() {
            return Err(anyhow!("Original text must not be empty"));
        }
        if original_text == corrected_text {
            return Err(anyhow!("Original and corrected text must differ"));
        }
        if kind != "correction" && kind != "reference" {
            return Err(anyhow!("Unknown correction kind: {}", kind));
        }

        let conn = self.get_connection()?;
        let created_at = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO corrections (original_text, corrected_text, history_id, created_at, enabled, kind)
             VALUES (?1, ?2, NULL, ?3, 1, ?4)",
            params![&original_text, &corrected_text, created_at, &kind],
        )?;
        let id = conn.last_insert_rowid();

        Ok(Correction {
            id,
            original_text,
            corrected_text,
            history_id: None,
            created_at,
            enabled: true,
            kind,
        })
    }

    fn format_timestamp_title(&self, timestamp: i64) -> String {
        if let Some(utc_datetime) = DateTime::from_timestamp(timestamp, 0) {
            // Convert UTC to local timezone
            let local_datetime = utc_datetime.with_timezone(&Local);
            local_datetime.format("%B %e, %Y - %l:%M%p").to_string()
        } else {
            format!("Recording {}", timestamp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                saved BOOLEAN NOT NULL DEFAULT 0,
                title TEXT NOT NULL,
                transcription_text TEXT NOT NULL,
                post_processed_text TEXT,
                post_process_prompt TEXT,
                post_process_requested BOOLEAN NOT NULL DEFAULT 0
            );",
        )
        .expect("create transcription_history table");
        conn
    }

    fn insert_entry(conn: &Connection, timestamp: i64, text: &str, post_processed: Option<&str>) {
        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                format!("handy-{}.wav", timestamp),
                timestamp,
                false,
                format!("Recording {}", timestamp),
                text,
                post_processed,
                Option::<String>::None,
                false,
            ],
        )
        .expect("insert history entry");
    }

    #[test]
    fn get_latest_entry_returns_none_when_empty() {
        let conn = setup_conn();
        let entry = HistoryManager::get_latest_entry_with_conn(&conn).expect("fetch latest entry");
        assert!(entry.is_none());
    }

    #[test]
    fn get_latest_entry_returns_newest_entry() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "first", None);
        insert_entry(&conn, 200, "second", Some("processed"));

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch latest entry")
            .expect("entry exists");

        assert_eq!(entry.timestamp, 200);
        assert_eq!(entry.transcription_text, "second");
        assert_eq!(entry.post_processed_text.as_deref(), Some("processed"));
    }

    #[test]
    fn get_latest_completed_entry_skips_empty_entries() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "completed", None);
        insert_entry(&conn, 200, "", None);

        let entry = HistoryManager::get_latest_completed_entry_with_conn(&conn)
            .expect("fetch latest completed entry")
            .expect("completed entry exists");

        assert_eq!(entry.timestamp, 100);
        assert_eq!(entry.transcription_text, "completed");
    }

    fn correction(id: i64, original: &str, corrected: &str) -> Correction {
        Correction {
            id,
            original_text: original.to_string(),
            corrected_text: corrected.to_string(),
            history_id: None,
            created_at: 0,
            enabled: true,
            kind: "correction".to_string(),
        }
    }

    #[test]
    fn extract_correction_pair_identical_returns_none() {
        assert_eq!(extract_correction_pair("hello world", "hello world"), None);
    }

    #[test]
    fn extract_correction_pair_single_word_change() {
        assert_eq!(
            extract_correction_pair("halo world", "hello world"),
            Some(("halo".to_string(), "hello".to_string()))
        );
    }

    #[test]
    fn extract_correction_pair_change_in_middle() {
        assert_eq!(
            extract_correction_pair(
                "I will see you halo there friend",
                "I will see you hello there friend"
            ),
            Some(("halo".to_string(), "hello".to_string()))
        );
    }

    #[test]
    fn extract_correction_pair_multi_word_span() {
        assert_eq!(
            extract_correction_pair("omw soon", "on my way soon"),
            Some(("omw".to_string(), "on my way".to_string()))
        );
    }

    #[test]
    fn extract_correction_pair_pure_insertion_falls_back_to_full_pair() {
        // Insertion alone can't be narrowed — fall back to the full-string
        // pair so the edit isn't silently discarded.
        assert_eq!(
            extract_correction_pair("hello", "hello world"),
            Some(("hello".to_string(), "hello world".to_string()))
        );
    }

    #[test]
    fn extract_correction_pair_pure_deletion_falls_back_to_full_pair() {
        assert_eq!(
            extract_correction_pair("hello world", "hello"),
            Some(("hello world".to_string(), "hello".to_string()))
        );
    }

    #[test]
    fn extract_correction_pair_identical_after_trim_returns_none() {
        assert_eq!(extract_correction_pair("  hello  ", "hello"), None);
    }

    #[test]
    fn apply_corrections_empty_rules_passthrough() {
        let out = apply_corrections("hello world", &[]);
        assert_eq!(out, "hello world");
    }

    #[test]
    fn apply_corrections_word_boundary_prevents_partial_match() {
        let rules = vec![correction(1, "halo", "hello")];
        assert_eq!(apply_corrections("halos halo", &rules), "halos hello");
    }

    #[test]
    fn apply_corrections_is_case_insensitive() {
        let rules = vec![correction(1, "halo", "hello")];
        // Both "Halo" and "halo" should match regardless of case.
        assert_eq!(apply_corrections("Halo halo", &rules), "hello hello");
    }

    #[test]
    fn apply_corrections_multi_word_phrase() {
        let rules = vec![correction(1, "omw", "on my way")];
        assert_eq!(
            apply_corrections("omw to the store", &rules),
            "on my way to the store"
        );
    }

    #[test]
    fn apply_corrections_disabled_rule_skipped() {
        let mut rules = vec![correction(1, "halo", "hello")];
        rules[0].enabled = false;
        assert_eq!(apply_corrections("halo world", &rules), "halo world");
    }

    #[test]
    fn apply_corrections_ordered_application() {
        let rules = vec![correction(1, "halo", "hello"), correction(2, "hello", "hi")];
        // First rule turns "halo" into "hello"; second rule then rewrites to "hi".
        assert_eq!(apply_corrections("halo there", &rules), "hi there");
    }

    #[test]
    fn apply_corrections_empty_pattern_skipped() {
        let rules = vec![correction(1, "", "anything")];
        // Empty pattern would be pathological — the filter should drop it.
        assert_eq!(apply_corrections("halo", &rules), "halo");
    }

    #[test]
    fn apply_corrections_replacement_dollar_is_literal() {
        // References often expand to text containing `$` — currency ($100),
        // shell variables ($HOME), template placeholders (${name}). These must
        // not be interpreted as regex backreferences.
        let rules = vec![
            correction(1, "price", "$100"),
            correction(2, "home", "$HOME"),
            correction(3, "name", "${user_name}"),
        ];
        assert_eq!(
            apply_corrections("price at home, name me", &rules),
            "$100 at $HOME, ${user_name} me"
        );
    }

    fn setup_corrections_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE corrections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                original_text TEXT NOT NULL,
                corrected_text TEXT NOT NULL,
                history_id INTEGER,
                created_at INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                kind TEXT NOT NULL DEFAULT 'correction'
            );",
        )
        .expect("create corrections table");
        conn
    }

    fn insert_correction_row(
        conn: &Connection,
        original: &str,
        corrected: &str,
        created_at: i64,
        kind: &str,
    ) {
        conn.execute(
            "INSERT INTO corrections (original_text, corrected_text, history_id, created_at, enabled, kind)
             VALUES (?1, ?2, NULL, ?3, 1, ?4)",
            params![original, corrected, created_at, kind],
        )
        .expect("insert correction row");
    }

    #[test]
    fn list_corrections_filters_to_reference_kind() {
        let conn = setup_corrections_conn();
        // Mix kinds: two corrections and one reference.
        insert_correction_row(&conn, "halo", "hello", 100, "correction");
        insert_correction_row(&conn, "my email", "daniel@example.com", 200, "reference");
        insert_correction_row(&conn, "omw", "on my way", 300, "correction");

        let refs = HistoryManager::list_corrections_with_conn(&conn, 10, Some("reference"))
            .expect("list references");
        assert_eq!(refs.len(), 1, "expected exactly one reference row");
        assert_eq!(refs[0].original_text, "my email");
        assert_eq!(refs[0].corrected_text, "daniel@example.com");
        assert_eq!(refs[0].kind, "reference");

        let corrs = HistoryManager::list_corrections_with_conn(&conn, 10, Some("correction"))
            .expect("list corrections");
        assert_eq!(corrs.len(), 2, "expected two correction rows");
        // Sorted newest-first: "omw" (300) comes before "halo" (100).
        assert_eq!(corrs[0].original_text, "omw");
        assert_eq!(corrs[1].original_text, "halo");

        let all =
            HistoryManager::list_corrections_with_conn(&conn, 10, None).expect("list all kinds");
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn get_active_corrections_loads_kind_column() {
        let conn = setup_corrections_conn();
        insert_correction_row(&conn, "halo", "hello", 100, "correction");
        insert_correction_row(&conn, "my email", "daniel@example.com", 200, "reference");

        let active = HistoryManager::get_active_corrections_with_conn(&conn)
            .expect("get_active_corrections must succeed when kind column is present");
        assert_eq!(active.len(), 2);
        // Sorted oldest-first (ASC by created_at) to preserve application order.
        assert_eq!(active[0].original_text, "halo");
        assert_eq!(active[0].kind, "correction");
        assert_eq!(active[1].original_text, "my email");
        assert_eq!(active[1].kind, "reference");
    }
}
