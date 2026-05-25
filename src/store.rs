//! SQLite persistence for normalized usage records.
//! @author codex

use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, params};
use serde::Serialize;

use crate::model::{TokenUsage, UsageRecord};

pub struct UsageStore {
    conn: Connection,
}

#[derive(Clone, Debug, Serialize)]
pub struct UsageSummary {
    pub name: String,
    pub records: u64,
    pub usage: TokenUsage,
    pub cost_usd: f64,
}

impl UsageStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn upsert_records(&self, records: &[UsageRecord]) -> Result<usize> {
        let mut changed = 0;
        for record in records {
            self.conn.execute(
                "INSERT INTO usage_records (
                    id, tool, source_path, session_id, ts, project_path, model,
                    input_tokens, output_tokens, cache_read_tokens, cache_write_tokens,
                    reasoning_tokens, tool_tokens, total_tokens, cost_usd, precision
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                ON CONFLICT(id) DO UPDATE SET
                    tool = excluded.tool,
                    source_path = excluded.source_path,
                    session_id = excluded.session_id,
                    ts = excluded.ts,
                    project_path = excluded.project_path,
                    model = excluded.model,
                    input_tokens = excluded.input_tokens,
                    output_tokens = excluded.output_tokens,
                    cache_read_tokens = excluded.cache_read_tokens,
                    cache_write_tokens = excluded.cache_write_tokens,
                    reasoning_tokens = excluded.reasoning_tokens,
                    tool_tokens = excluded.tool_tokens,
                    total_tokens = excluded.total_tokens,
                    cost_usd = excluded.cost_usd,
                    precision = excluded.precision",
                params![
                    record.id,
                    record.tool.as_str(),
                    record.source_path,
                    record.session_id,
                    record.timestamp.map(|dt| dt.to_rfc3339()),
                    record.project_path,
                    record.model,
                    record.usage.input as i64,
                    record.usage.output as i64,
                    record.usage.cache_read as i64,
                    record.usage.cache_write as i64,
                    record.usage.reasoning as i64,
                    record.usage.tool as i64,
                    record.usage.total() as i64,
                    record.cost_usd,
                    record.precision.as_str(),
                ],
            )?;
            changed += 1;
        }
        Ok(changed)
    }

    pub fn summary_by_tool(&self, tool: Option<&str>) -> Result<Vec<UsageSummary>> {
        self.summary_by_tool_filtered(tool, None)
    }

    pub fn summary_by_tool_filtered(
        &self,
        tool: Option<&str>,
        day_prefix: Option<&str>,
    ) -> Result<Vec<UsageSummary>> {
        self.summary_grouped("tool", tool, day_prefix)
    }

    pub fn summary_by_project_filtered(
        &self,
        project: Option<&str>,
        day_prefix: Option<&str>,
    ) -> Result<Vec<UsageSummary>> {
        self.summary_grouped("COALESCE(project_path, '(unknown)')", project, day_prefix)
    }

    fn summary_grouped(
        &self,
        group_expr: &str,
        name_filter: Option<&str>,
        day_prefix: Option<&str>,
    ) -> Result<Vec<UsageSummary>> {
        let select =
            format!("SELECT {group_expr}, COUNT(*), SUM(input_tokens), SUM(output_tokens), SUM(cache_read_tokens),
            SUM(cache_write_tokens), SUM(reasoning_tokens), SUM(tool_tokens), SUM(cost_usd)
            FROM usage_records");
        let group = format!(" GROUP BY {group_expr} ORDER BY SUM(total_tokens) DESC");
        let filter_column = if group_expr == "tool" {
            "tool"
        } else {
            "project_path"
        };
        let sql = match (name_filter, day_prefix) {
            (Some(_), Some(_)) => {
                format!("{select} WHERE {filter_column} = ?1 AND ts LIKE ?2{group}")
            }
            (Some(_), None) => format!("{select} WHERE {filter_column} = ?1{group}"),
            (None, Some(_)) => format!("{select} WHERE ts LIKE ?1{group}"),
            (None, None) => format!("{select}{group}"),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let day_like = day_prefix.map(|day| format!("{day}%"));
        let rows = match (name_filter, day_like.as_deref()) {
            (Some(name), Some(day)) => stmt.query_map((name, day), row_to_summary)?,
            (Some(name), None) => stmt.query_map([name], row_to_summary)?,
            (None, Some(day)) => stmt.query_map([day], row_to_summary)?,
            (None, None) => stmt.query_map([], row_to_summary)?,
        };
        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(row?);
        }
        Ok(summaries)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_records (
                id TEXT PRIMARY KEY,
                tool TEXT NOT NULL,
                source_path TEXT NOT NULL,
                session_id TEXT,
                ts TEXT,
                project_path TEXT,
                model TEXT,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                cache_read_tokens INTEGER NOT NULL,
                cache_write_tokens INTEGER NOT NULL,
                reasoning_tokens INTEGER NOT NULL,
                tool_tokens INTEGER NOT NULL,
                total_tokens INTEGER NOT NULL,
                cost_usd REAL,
                precision TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_usage_tool ON usage_records(tool);
            CREATE INDEX IF NOT EXISTS idx_usage_project ON usage_records(project_path);
            CREATE INDEX IF NOT EXISTS idx_usage_ts ON usage_records(ts);",
        )?;
        Ok(())
    }
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<UsageSummary> {
    Ok(UsageSummary {
        name: row.get(0)?,
        records: row.get::<_, i64>(1)? as u64,
        usage: TokenUsage {
            input: row.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
            output: row.get::<_, Option<i64>>(3)?.unwrap_or(0) as u64,
            cache_read: row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
            cache_write: row.get::<_, Option<i64>>(5)?.unwrap_or(0) as u64,
            reasoning: row.get::<_, Option<i64>>(6)?.unwrap_or(0) as u64,
            tool: row.get::<_, Option<i64>>(7)?.unwrap_or(0) as u64,
        },
        cost_usd: row.get::<_, Option<f64>>(8)?.unwrap_or(0.0),
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::model::{SourcePrecision, TokenUsage, ToolKind, UsageRecord};

    use super::*;

    #[test]
    fn upserts_records_and_summarizes_by_tool() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let record = UsageRecord {
            id: "r1".to_string(),
            tool: ToolKind::Claude,
            source_path: "/tmp/session.jsonl".to_string(),
            session_id: Some("s1".to_string()),
            timestamp: None,
            project_path: Some("/repo".to_string()),
            model: Some("claude-opus".to_string()),
            usage: TokenUsage {
                input: 1,
                output: 2,
                cache_read: 3,
                cache_write: 4,
                reasoning: 5,
                tool: 6,
            },
            cost_usd: Some(0.5),
            precision: SourcePrecision::Exact,
        };

        assert_eq!(
            store.upsert_records(std::slice::from_ref(&record)).unwrap(),
            1
        );
        assert_eq!(store.upsert_records(&[record]).unwrap(), 1);

        let summaries = store.summary_by_tool(None).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, "claude");
        assert_eq!(summaries[0].records, 1);
        assert_eq!(summaries[0].usage.total(), 21);
        assert_eq!(summaries[0].cost_usd, 0.5);
    }

    #[test]
    fn summarizes_by_project_path() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut left = sample_record("left", 10);
        left.project_path = Some("/repo-a".to_string());
        let mut right = sample_record("right", 20);
        right.project_path = Some("/repo-b".to_string());
        store.upsert_records(&[left, right]).unwrap();

        let summaries = store
            .summary_by_project_filtered(Some("/repo-b"), None)
            .unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, "/repo-b");
        assert_eq!(summaries[0].usage.input, 20);
    }

    #[test]
    fn filters_summary_by_utc_day_prefix() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut today = sample_record("today", 10);
        today.timestamp = Some("2026-05-25T01:00:00Z".parse().unwrap());
        let mut yesterday = sample_record("yesterday", 20);
        yesterday.timestamp = Some("2026-05-24T23:00:00Z".parse().unwrap());
        store.upsert_records(&[today, yesterday]).unwrap();

        let summaries = store
            .summary_by_tool_filtered(None, Some("2026-05-25"))
            .unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].usage.input, 10);
        assert_eq!(summaries[0].usage.total(), 10);
    }

    fn sample_record(id: &str, input: u64) -> UsageRecord {
        UsageRecord {
            id: id.to_string(),
            tool: ToolKind::Claude,
            source_path: "/tmp/session.jsonl".to_string(),
            session_id: Some("s1".to_string()),
            timestamp: None,
            project_path: Some("/repo".to_string()),
            model: Some("claude-opus".to_string()),
            usage: TokenUsage {
                input,
                ..TokenUsage::default()
            },
            cost_usd: None,
            precision: SourcePrecision::Exact,
        }
    }
}
