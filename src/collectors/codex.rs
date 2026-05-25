//! Codex passive usage collector.
//! @author codex

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::model::{SourcePrecision, TokenUsage, ToolKind, UsageRecord};

pub fn parse_file(path: &Path) -> Result<Vec<UsageRecord>> {
    let file = File::open(path).with_context(|| format!("open Codex file {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("read Codex line {}", index + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line)
            .with_context(|| format!("parse Codex JSON line {}", index + 1))?;
        let Some(usage) = value.pointer("/payload/info/last_token_usage") else {
            continue;
        };
        let token_usage = TokenUsage {
            input: usage_u64(usage, "input_tokens"),
            output: usage_u64(usage, "output_tokens"),
            cache_read: usage_u64(usage, "cached_input_tokens"),
            cache_write: 0,
            reasoning: usage_u64(usage, "reasoning_output_tokens"),
            tool: 0,
        };
        if token_usage.total() == 0 {
            continue;
        }

        let event_id = string_at(&value, "/payload/id")
            .or_else(|| string_at(&value, "/payload/session_id"))
            .unwrap_or_else(|| format!("line-{}", index + 1));
        records.push(UsageRecord {
            id: format!("codex:{}:{}", path.display(), event_id),
            tool: ToolKind::Codex,
            source_path: path.display().to_string(),
            session_id: string_at(&value, "/payload/id"),
            timestamp: parse_timestamp(string_at(&value, "/timestamp").as_deref()),
            project_path: string_at(&value, "/payload/cwd"),
            model: string_at(&value, "/payload/model")
                .or_else(|| string_at(&value, "/payload/collaboration_mode/settings/model")),
            usage: token_usage,
            cost_usd: None,
            precision: SourcePrecision::Exact,
        });
    }

    Ok(records)
}

fn usage_u64(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}

fn string_at(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn parse_timestamp(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::model::{SourcePrecision, ToolKind};

    use super::*;

    #[test]
    fn parses_last_token_usage_from_rollout_events() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rollout.jsonl");
        fs::write(
            &path,
            r#"{"type":"response_item","timestamp":"2026-05-25T10:00:00Z","payload":{"id":"evt1","type":"token_usage","cwd":"/repo","info":{"model_context_window":272000,"last_token_usage":{"input_tokens":101,"output_tokens":202,"cached_input_tokens":303,"reasoning_output_tokens":404,"total_tokens":1010},"total_token_usage":{"input_tokens":999,"output_tokens":999,"cached_input_tokens":999,"reasoning_output_tokens":999,"total_tokens":3996}}}}"#,
        )
        .unwrap();

        let records = parse_file(&path).unwrap();

        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.tool, ToolKind::Codex);
        assert_eq!(record.precision, SourcePrecision::Exact);
        assert_eq!(record.project_path.as_deref(), Some("/repo"));
        assert_eq!(record.usage.input, 101);
        assert_eq!(record.usage.output, 202);
        assert_eq!(record.usage.cache_read, 303);
        assert_eq!(record.usage.reasoning, 404);
        assert_eq!(record.usage.total(), 1010);
    }
}
