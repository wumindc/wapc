//! Finds local AI coding tool usage files and dispatches passive collectors.
//! @author codex

use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

use crate::{
    collectors::{claude, codex, gemini, opencode},
    model::UsageRecord,
};

pub fn scan_home(home: &Path) -> Result<Vec<UsageRecord>> {
    let mut records = Vec::new();
    scan_jsonl_tree(
        &home.join(".claude/projects"),
        claude::parse_file,
        &mut records,
    )?;
    scan_jsonl_tree(
        &home.join(".codex/sessions"),
        codex::parse_file,
        &mut records,
    )?;
    scan_jsonl_tree(
        &home.join(".codex/archived_sessions"),
        codex::parse_file,
        &mut records,
    )?;
    scan_json_tree(&home.join(".gemini/tmp"), gemini::parse_file, &mut records)?;
    scan_json_tree(
        &home.join(".local/share/opencode/storage"),
        opencode::parse_file,
        &mut records,
    )?;
    Ok(records)
}

pub fn audit_paths(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".claude/projects"),
        home.join(".codex/sessions"),
        home.join(".codex/archived_sessions"),
        home.join(".gemini/tmp"),
        home.join(".local/share/opencode/storage"),
    ]
}

fn scan_jsonl_tree(
    root: &Path,
    parser: fn(&Path) -> Result<Vec<UsageRecord>>,
    records: &mut Vec<UsageRecord>,
) -> Result<()> {
    scan_tree(root, "jsonl", parser, records)
}

fn scan_json_tree(
    root: &Path,
    parser: fn(&Path) -> Result<Vec<UsageRecord>>,
    records: &mut Vec<UsageRecord>,
) -> Result<()> {
    scan_tree(root, "json", parser, records)
}

fn scan_tree(
    root: &Path,
    extension: &str,
    parser: fn(&Path) -> Result<Vec<UsageRecord>>,
    records: &mut Vec<UsageRecord>,
) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some(extension) {
            continue;
        }
        if let Ok(mut parsed) = parser(path) {
            records.append(&mut parsed);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::model::ToolKind;

    use super::*;

    #[test]
    fn scans_known_tool_directories_under_home() {
        let home = tempdir().unwrap();
        let claude_dir = home.path().join(".claude/projects/proj");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("session.jsonl"),
            r#"{"type":"assistant","sessionId":"s1","uuid":"u1","timestamp":"2026-05-25T10:00:00Z","cwd":"/repo","message":{"role":"assistant","model":"claude","usage":{"input_tokens":1,"output_tokens":2}}}"#,
        )
        .unwrap();

        let records = scan_home(home.path()).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tool, ToolKind::Claude);
        assert_eq!(records[0].usage.total(), 3);
    }
}
