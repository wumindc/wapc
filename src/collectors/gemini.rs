//! Gemini CLI passive usage collector.
//! @author codex

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::model::{SourcePrecision, TokenUsage, ToolKind, UsageRecord};

pub fn parse_file(path: &Path) -> Result<Vec<UsageRecord>> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read Gemini file {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse Gemini JSON {}", path.display()))?;
    let session_id = string_at(&value, "/sessionId");
    let messages = value.pointer("/messages").and_then(Value::as_array);
    let mut records = Vec::new();

    for (index, message) in messages.into_iter().flatten().enumerate() {
        let Some(tokens) = message.get("tokens") else {
            continue;
        };
        let token_usage = TokenUsage {
            input: usage_u64(tokens, "input"),
            output: usage_u64(tokens, "output"),
            cache_read: usage_u64(tokens, "cached"),
            cache_write: 0,
            reasoning: usage_u64(tokens, "thoughts"),
            tool: usage_u64(tokens, "tool"),
        };
        if token_usage.total() == 0 {
            continue;
        }
        let event_id =
            string_at(message, "/id").unwrap_or_else(|| format!("message-{}", index + 1));
        records.push(UsageRecord {
            id: format!("gemini:{}:{}", path.display(), event_id),
            tool: ToolKind::Gemini,
            source_path: path.display().to_string(),
            session_id: session_id.clone(),
            timestamp: parse_timestamp(string_at(message, "/timestamp").as_deref())
                .or_else(|| parse_timestamp(string_at(&value, "/startTime").as_deref())),
            project_path: None,
            model: string_at(message, "/model"),
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
    fn parses_message_token_blocks_from_chat_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("session.json");
        fs::write(
            &path,
            r#"{"sessionId":"gs1","startTime":"2026-05-25T10:00:00Z","projectHash":"abc","messages":[{"id":"m1","type":"user","content":"secret"},{"id":"m2","type":"assistant","model":"gemini-2.5-pro","timestamp":"2026-05-25T10:00:01Z","tokens":{"input":10,"output":20,"cached":30,"thoughts":40,"tool":50,"total":150},"content":"secret-response"}]}"#,
        )
        .unwrap();

        let records = parse_file(&path).unwrap();

        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.tool, ToolKind::Gemini);
        assert_eq!(record.precision, SourcePrecision::Exact);
        assert_eq!(record.session_id.as_deref(), Some("gs1"));
        assert_eq!(record.model.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(record.usage.input, 10);
        assert_eq!(record.usage.output, 20);
        assert_eq!(record.usage.cache_read, 30);
        assert_eq!(record.usage.reasoning, 40);
        assert_eq!(record.usage.tool, 50);
        assert!(!serde_json::to_string(record).unwrap().contains("secret"));
    }
}
