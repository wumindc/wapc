//! OpenCode passive usage collector.
//! @author codex

use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::model::{SourcePrecision, TokenUsage, ToolKind, UsageRecord};

pub fn parse_file(path: &Path) -> Result<Vec<UsageRecord>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read OpenCode file {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse OpenCode JSON {}", path.display()))?;
    let Some(tokens) = value.get("tokens") else {
        return Ok(Vec::new());
    };
    let token_usage = TokenUsage {
        input: usage_u64(tokens, "input"),
        output: usage_u64(tokens, "output"),
        cache_read: tokens
            .pointer("/cache/read")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_write: tokens
            .pointer("/cache/write")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        reasoning: usage_u64(tokens, "reasoning"),
        tool: 0,
    };
    if token_usage.total() == 0 {
        return Ok(Vec::new());
    }
    let event_id = string_at(&value, "/id").unwrap_or_else(|| "unknown".to_string());
    Ok(vec![UsageRecord {
        id: format!("opencode:{}:{}", path.display(), event_id),
        tool: ToolKind::OpenCode,
        source_path: path.display().to_string(),
        session_id: string_at(&value, "/sessionID"),
        timestamp: None,
        project_path: None,
        model: None,
        usage: token_usage,
        cost_usd: value.get("cost").and_then(Value::as_f64),
        precision: SourcePrecision::Exact,
    }])
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::model::{SourcePrecision, ToolKind};

    use super::*;

    #[test]
    fn parses_part_token_usage_from_storage_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("part.json");
        fs::write(
            &path,
            r#"{"id":"prt1","sessionID":"ses1","messageID":"msg1","type":"step-finish","cost":0.1234,"tokens":{"input":7,"output":8,"reasoning":9,"cache":{"read":10,"write":11}}}"#,
        )
        .unwrap();

        let records = parse_file(&path).unwrap();

        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.tool, ToolKind::OpenCode);
        assert_eq!(record.precision, SourcePrecision::Exact);
        assert_eq!(record.session_id.as_deref(), Some("ses1"));
        assert_eq!(record.usage.input, 7);
        assert_eq!(record.usage.output, 8);
        assert_eq!(record.usage.reasoning, 9);
        assert_eq!(record.usage.cache_read, 10);
        assert_eq!(record.usage.cache_write, 11);
        assert_eq!(record.cost_usd, Some(0.1234));
    }
}
