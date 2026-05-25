//! Claude Code passive usage collector.
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
    let file = File::open(path).with_context(|| format!("open Claude file {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("read Claude line {}", index + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line)
            .with_context(|| format!("parse Claude JSON line {}", index + 1))?;
        let Some(usage) = value.pointer("/message/usage") else {
            continue;
        };
        let token_usage = TokenUsage {
            input: usage_u64(usage, "input_tokens"),
            output: usage_u64(usage, "output_tokens"),
            cache_read: usage_u64(usage, "cache_read_input_tokens"),
            cache_write: usage_u64(usage, "cache_creation_input_tokens")
                + usage
                    .pointer("/cache_creation/ephemeral_5m_input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                + usage
                    .pointer("/cache_creation/ephemeral_1h_input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
            reasoning: 0,
            tool: 0,
        };
        if token_usage.total() == 0 {
            continue;
        }

        let session_id = string_at(&value, "/sessionId");
        let event_id = string_at(&value, "/requestId")
            .or_else(|| string_at(&value, "/uuid"))
            .unwrap_or_else(|| format!("line-{}", index + 1));
        records.push(UsageRecord {
            id: format!("claude:{}:{}", path.display(), event_id),
            tool: ToolKind::Claude,
            source_path: path.display().to_string(),
            session_id,
            timestamp: parse_timestamp(string_at(&value, "/timestamp").as_deref()),
            project_path: string_at(&value, "/cwd"),
            model: string_at(&value, "/message/model"),
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
    fn parses_assistant_usage_from_jsonl_without_text_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        fs::write(
            &path,
            r#"{"type":"assistant","sessionId":"s1","uuid":"u1","requestId":"r1","timestamp":"2026-05-25T10:00:00Z","cwd":"/repo","message":{"role":"assistant","model":"claude-opus-4-7","content":[{"type":"text","text":"secret"}],"usage":{"input_tokens":11,"output_tokens":22,"cache_read_input_tokens":33,"cache_creation_input_tokens":44}}}"#,
        )
        .unwrap();

        let records = parse_file(&path).unwrap();

        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.tool, ToolKind::Claude);
        assert_eq!(record.precision, SourcePrecision::Exact);
        assert_eq!(record.session_id.as_deref(), Some("s1"));
        assert_eq!(record.project_path.as_deref(), Some("/repo"));
        assert_eq!(record.model.as_deref(), Some("claude-opus-4-7"));
        assert_eq!(record.usage.input, 11);
        assert_eq!(record.usage.output, 22);
        assert_eq!(record.usage.cache_read, 33);
        assert_eq!(record.usage.cache_write, 44);
        assert!(!serde_json::to_string(record).unwrap().contains("secret"));
    }
}
