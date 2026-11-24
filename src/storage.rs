use crate::collector::{ScanSummary, UsageEvent};
use rusqlite::{params, Connection, Result};
use std::path::Path;

const SCHEMA_VERSION: i64 = 1;

pub struct UsageStore {
    connection: Connection,
}

impl UsageStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let connection = Connection::open(path)?;
        let store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    pub fn insert_scan(&self, summary: &ScanSummary) -> Result<usize> {
        let transaction = self.connection.unchecked_transaction()?;
        let mut statement = transaction.prepare(
            "INSERT OR IGNORE INTO usage_events (
                event_key, provider, project, session_id, message_id, recorded_at,
                model, input_tokens, output_tokens, cache_read_tokens,
                cache_creation_tokens, source_file
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )?;
        let mut inserted = 0;

        for event in &summary.events {
            inserted += statement.execute(params![
                event_key(event),
                event.source,
                event.project,
                event.session_id,
                event.message_id,
                event.recorded_at,
                event.model,
                event.input_tokens,
                event.output_tokens,
                event.cache_read_tokens,
                event.cache_creation_tokens,
                event.source_file.to_string_lossy(),
            ])?;
        }

        drop(statement);
        transaction.commit()?;
        Ok(inserted)
    }

    pub fn stats(&self) -> Result<Stats> {
        let totals = self.connection.query_row(
            "SELECT COUNT(*), COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0),
                    COALESCE(SUM(cache_read_tokens), 0), COALESCE(SUM(cache_creation_tokens), 0)
             FROM usage_events",
            [],
            |row| {
                Ok(Totals {
                    events: row.get(0)?,
                    input_tokens: row.get(1)?,
                    output_tokens: row.get(2)?,
                    cache_read_tokens: row.get(3)?,
                    cache_creation_tokens: row.get(4)?,
                })
            },
        )?;

        let mut by_date = self.connection.prepare(
            "SELECT COALESCE(substr(recorded_at, 1, 10), 'unknown'), COUNT(*),
                    COALESCE(SUM(input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens), 0)
             FROM usage_events GROUP BY 1 ORDER BY 1",
        )?;
        let dates = by_date
            .query_map([], |row| Ok(Bucket { name: row.get(0)?, events: row.get(1)?, tokens: row.get(2)? }))?
            .collect::<Result<Vec<_>>>()?;

        let mut by_model = self.connection.prepare(
            "SELECT COALESCE(model, 'unknown'), COUNT(*),
                    COALESCE(SUM(input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens), 0)
             FROM usage_events GROUP BY 1 ORDER BY 3 DESC",
        )?;
        let models = by_model
            .query_map([], |row| Ok(Bucket { name: row.get(0)?, events: row.get(1)?, tokens: row.get(2)? }))?
            .collect::<Result<Vec<_>>>()?;

        let mut by_project = self.connection.prepare(
            "SELECT COALESCE(project, 'unknown'), COUNT(*),
                    COALESCE(SUM(input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens), 0)
             FROM usage_events GROUP BY 1 ORDER BY 3 DESC",
        )?;
        let projects = by_project
            .query_map([], |row| Ok(Bucket { name: row.get(0)?, events: row.get(1)?, tokens: row.get(2)? }))?
            .collect::<Result<Vec<_>>>()?;

        Ok(Stats { totals, dates, models, projects })
    }

    fn migrate(&self) -> Result<()> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;

        let current: Option<i64> = self.connection.query_row(
            "SELECT MAX(version) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        if current.unwrap_or(0) < SCHEMA_VERSION {
            self.connection.execute_batch(
                "CREATE TABLE IF NOT EXISTS usage_events (
                    id INTEGER PRIMARY KEY,
                    event_key TEXT NOT NULL UNIQUE,
                    provider TEXT NOT NULL,
                    project TEXT,
                    session_id TEXT NOT NULL,
                    message_id TEXT,
                    recorded_at TEXT,
                    model TEXT,
                    input_tokens INTEGER NOT NULL DEFAULT 0,
                    output_tokens INTEGER NOT NULL DEFAULT 0,
                    cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                    cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                    source_file TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_usage_events_recorded_at ON usage_events(recorded_at);
                CREATE INDEX IF NOT EXISTS idx_usage_events_model ON usage_events(model);
                CREATE INDEX IF NOT EXISTS idx_usage_events_project ON usage_events(project);
                INSERT INTO schema_migrations(version) VALUES (1);",
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Totals {
    pub events: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bucket {
    pub name: String,
    pub events: i64,
    pub tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    pub totals: Totals,
    pub dates: Vec<Bucket>,
    pub models: Vec<Bucket>,
    pub projects: Vec<Bucket>,
}

fn event_key(event: &UsageEvent) -> String {
    if let Some(message_id) = &event.message_id {
        return format!("{}:{}:{}", event.source, event.session_id, message_id);
    }

    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        event.source,
        event.source_file.display(),
        event.recorded_at.as_deref().unwrap_or_default(),
        event.model.as_deref().unwrap_or_default(),
        event.input_tokens,
        event.output_tokens,
        event.cache_read_tokens + event.cache_creation_tokens,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn event(id: &str) -> UsageEvent {
        UsageEvent {
            source: "claude-code",
            project: Some("project".to_owned()),
            session_id: "session".to_owned(),
            message_id: Some(id.to_owned()),
            recorded_at: Some("2025-01-01T00:00:00Z".to_owned()),
            model: Some("claude-sonnet".to_owned()),
            input_tokens: 10,
            output_tokens: 5,
            cache_read_tokens: 20,
            cache_creation_tokens: 2,
            source_file: PathBuf::from("session.jsonl"),
        }
    }

    #[test]
    fn insert_is_idempotent() {
        let store = UsageStore::open(Path::new(":memory:")).unwrap();
        let summary = ScanSummary { files: 1, malformed_lines: 0, events: vec![event("msg-1")] };

        assert_eq!(store.insert_scan(&summary).unwrap(), 1);
        assert_eq!(store.insert_scan(&summary).unwrap(), 0);
        assert_eq!(store.stats().unwrap().totals.events, 1);
    }
}
