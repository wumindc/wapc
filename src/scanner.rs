//! Finds local AI coding tool usage files and dispatches passive collectors.
//! @author codex

use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

use crate::{
    collectors::{claude, codex, gemini, opencode},
    model::{SourceHealth, UsageRecord},
    platform_paths::{PlatformPathContext, ToolPathCandidate, ToolPathKind, tool_path_candidates},
};

struct SourceDefinition {
    tool: &'static str,
    source_glob: &'static str,
    candidate_suffix: &'static str,
    extension: &'static str,
    parser: fn(&Path) -> Result<Vec<UsageRecord>>,
}

struct SessionSource<'a> {
    definition: &'a SourceDefinition,
    root: PathBuf,
}

const SOURCE_DEFINITIONS: &[SourceDefinition] = &[
    SourceDefinition {
        tool: "claude",
        source_glob: "~/.claude/projects/**/*.jsonl",
        candidate_suffix: ".claude/projects",
        extension: "jsonl",
        parser: claude::parse_file,
    },
    SourceDefinition {
        tool: "codex",
        source_glob: "~/.codex/sessions/**/*.jsonl",
        candidate_suffix: ".codex/sessions",
        extension: "jsonl",
        parser: codex::parse_file,
    },
    SourceDefinition {
        tool: "codex",
        source_glob: "~/.codex/archived_sessions/**/*.jsonl",
        candidate_suffix: ".codex/archived_sessions",
        extension: "jsonl",
        parser: codex::parse_file,
    },
    SourceDefinition {
        tool: "gemini",
        source_glob: "~/.gemini/tmp/**/*.json",
        candidate_suffix: ".gemini/tmp",
        extension: "json",
        parser: gemini::parse_file,
    },
    SourceDefinition {
        tool: "opencode",
        source_glob: "~/.local/share/opencode/storage/**/*.json",
        candidate_suffix: "opencode/storage",
        extension: "json",
        parser: opencode::parse_file,
    },
];

pub fn scan_home(home: &Path) -> Result<Vec<UsageRecord>> {
    let mut records = Vec::new();
    for source in session_sources(home) {
        scan_tree(
            &source.root,
            source.definition.extension,
            source.definition.parser,
            &mut records,
        )?;
    }
    Ok(records)
}

pub fn audit_paths(home: &Path) -> Vec<PathBuf> {
    session_sources(home)
        .into_iter()
        .map(|source| source.root)
        .collect()
}

pub fn source_health(home: &Path) -> Result<Vec<SourceHealth>> {
    let checked_at = chrono::Utc::now().to_rfc3339();
    session_sources(home)
        .into_iter()
        .map(|source| scan_source_health(&source, &checked_at))
        .collect()
}

fn session_sources(home: &Path) -> Vec<SessionSource<'static>> {
    let context = PlatformPathContext::current_home_compatible(home);
    let candidates = tool_path_candidates(&context);
    SOURCE_DEFINITIONS
        .iter()
        .filter_map(|definition| {
            candidates
                .iter()
                .find(|candidate| session_candidate_matches(definition, candidate))
                .map(|candidate| SessionSource {
                    definition,
                    root: candidate.path.clone(),
                })
        })
        .collect()
}

fn session_candidate_matches(definition: &SourceDefinition, candidate: &ToolPathCandidate) -> bool {
    if candidate.kind != ToolPathKind::SessionData || candidate.tool != definition.tool {
        return false;
    }
    normalized_path(&candidate.path).ends_with(definition.candidate_suffix)
}

fn normalized_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn scan_source_health(source: &SessionSource<'_>, checked_at: &str) -> Result<SourceHealth> {
    let definition = source.definition;
    let root = &source.root;
    if !root.exists() {
        return Ok(SourceHealth {
            tool: definition.tool.to_string(),
            source_glob: definition.source_glob.to_string(),
            exists: false,
            readable_files: 0,
            parsed_records: 0,
            failed_files: 0,
            latest_event_ts: None,
            checked_at: checked_at.to_string(),
        });
    }

    let mut readable_files = 0;
    let mut parsed_records = 0;
    let mut failed_files = 0;
    let mut latest_event_ts = None;

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                failed_files += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some(definition.extension) {
            continue;
        }
        readable_files += 1;
        match (definition.parser)(path) {
            Ok(records) => {
                parsed_records += records.len() as u64;
                for record in records {
                    if let Some(timestamp) = record.timestamp {
                        let value = timestamp.to_rfc3339();
                        if latest_event_ts
                            .as_deref()
                            .is_none_or(|current| value.as_str() > current)
                        {
                            latest_event_ts = Some(value);
                        }
                    }
                }
            }
            Err(_) => failed_files += 1,
        }
    }

    Ok(SourceHealth {
        tool: definition.tool.to_string(),
        source_glob: definition.source_glob.to_string(),
        exists: true,
        readable_files,
        parsed_records,
        failed_files,
        latest_event_ts,
        checked_at: checked_at.to_string(),
    })
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
    fn session_scanner_uses_path_resolver_for_source_roots() {
        let source = include_str!("scanner.rs");

        assert!(source.contains("tool_path_candidates"));
        assert!(source.contains("ToolPathKind::SessionData"));
        for root in [
            ".claude/projects",
            ".codex/sessions",
            ".codex/archived_sessions",
            ".gemini/tmp",
            ".local/share/opencode/storage",
        ] {
            let scan_join = format!("&home.{}(\"{}\")", "join", root);
            let audit_join = format!("home.{}(\"{}\")", "join", root);
            assert!(
                !source.contains(&scan_join),
                "scan_home still hardcodes {scan_join}"
            );
            assert!(
                !source.contains(&audit_join),
                "audit_paths still hardcodes {audit_join}"
            );
        }
    }

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

    #[test]
    fn source_health_counts_parse_successes_and_failures() {
        let home = tempdir().unwrap();
        let claude_dir = home.path().join(".claude/projects/proj");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("good.jsonl"),
            r#"{"type":"assistant","sessionId":"s1","uuid":"u1","timestamp":"2026-05-25T10:00:00Z","cwd":"/repo","message":{"role":"assistant","model":"claude","usage":{"input_tokens":1,"output_tokens":2}}}"#,
        )
        .unwrap();
        fs::write(claude_dir.join("bad.jsonl"), "{not-json").unwrap();

        let health = source_health(home.path()).unwrap();
        let claude = health
            .iter()
            .find(|source| source.tool == "claude")
            .unwrap();

        assert!(claude.exists);
        assert_eq!(claude.readable_files, 2);
        assert_eq!(claude.parsed_records, 1);
        assert_eq!(claude.failed_files, 1);
        assert_eq!(
            claude.latest_event_ts.as_deref(),
            Some("2026-05-25T10:00:00+00:00")
        );
    }
}
