//! Privacy audit report for local WAPC data handling.
//! @author codex

use std::path::Path;

use chrono::Utc;

use crate::{
    model::{PrivacyAuditReport, PrivacyAuditSource, PrivacyAuditTable},
    platform_paths::{PlatformPathContext, ToolPathKind, tool_path_candidates},
};

pub fn privacy_audit(home: &Path, db: &Path) -> PrivacyAuditReport {
    PrivacyAuditReport {
        generated_at: Utc::now().to_rfc3339(),
        local_only: true,
        db_path: db.display().to_string(),
        read_sources: read_sources(home),
        stored_tables: stored_tables(),
        forbidden_fields: forbidden_fields(),
        export_boundary: "Exports include persisted metadata summaries only; prompt, response, source code, tool output, and key material are excluded. Adapter capability declarations are read-only metadata and do not imply write, sync, injection, enable, or disable support. Cross-tool sync can write Claude, Codex, Gemini, and Cursor MCP target config files only after explicit confirmation. Backups may contain original tool configuration contents, including secrets already present in source files; the database stores backup paths and change metadata only, never backup content. Phase 4 apply_sync does not persist env values; sync history stores target metadata, strategy labels, and change ids only. sync preset JSON exports exclude env values and key material. Phase 5 resource templates store structure and source metadata only, include content fingerprints, and exclude secret values; template installs route through Sync Engine preview and confirmation before any target write. Phase 5 redacted team reports hash project paths and exclude source paths, session ids, project names, prompt bodies, and key material. Headless dashboard is disabled by default, binds only to 127.0.0.1, and serves read-only usage summaries only; it exposes no write, sync, import, or resource mutation endpoints. wapc://import deep links are preview-only until the user explicitly routes them through Sync Engine preview and confirmation; they reject raw env values, Authorization headers, and token-like secrets. Windows/Linux candidate paths are privacy-audit metadata only: they are unverified read-only candidates, carry no prompt/response/source body/key material, and keep writes unsupported until real platform fixtures and rollback e2e evidence exist.".to_string(),
    }
}

fn read_sources(home: &Path) -> Vec<PrivacyAuditSource> {
    let mut sources = current_tool_candidate_sources(home);
    sources.extend(vec![
        source(
            "Project AGENTS.md",
            "<project>/AGENTS.md",
            "project instruction structure fingerprinting",
        ),
        source(
            "Project CLAUDE.md",
            "<project>/CLAUDE.md",
            "project instruction structure fingerprinting",
        ),
        source(
            "Project GEMINI.md",
            "<project>/GEMINI.md",
            "project instruction structure fingerprinting",
        ),
        source(
            "Project Cursor rules",
            "<project>/.cursor/rules",
            "project instruction structure fingerprinting",
        ),
        source(
            "Project Claude MCP config",
            "<project>/.mcp.json",
            "read-only project resource inventory",
        ),
        source(
            "Project Cursor MCP config",
            "<project>/.cursor/mcp.json",
            "read-only project resource inventory",
        ),
        source(
            "Project Claude Code skills",
            "<project>/.claude/skills",
            "read-only project skill inventory",
        ),
        source(
            "Project Claude Code subagents",
            "<project>/.claude/agents",
            "project subagent metadata and structure fingerprinting",
        ),
        PrivacyAuditSource {
            name: "WAPC resource backups".to_string(),
            path: home.join(".wapc/backups").display().to_string(),
            purpose: "rollback source for confirmed Sync Engine writes".to_string(),
            reads_body: true,
            writes_source: false,
        },
    ]);
    sources.extend(cross_platform_candidate_sources());
    sources
}

fn current_tool_candidate_sources(home: &Path) -> Vec<PrivacyAuditSource> {
    let context = PlatformPathContext::current_home_compatible(home);
    let mut sources = Vec::new();
    for candidate in tool_path_candidates(&context)
        .into_iter()
        .filter(|candidate| candidate.scope == "user")
    {
        match candidate.kind {
            ToolPathKind::SessionData => sources.push(source_owned(
                current_session_source_name(&candidate),
                candidate.path,
                "usage metadata parsing",
                false,
                false,
            )),
            ToolPathKind::ConfigDir => sources.push(source_owned(
                format!("{} config directory", display_tool(candidate.tool)),
                candidate.path,
                "tool presence detection",
                false,
                false,
            )),
            ToolPathKind::McpConfig => {
                sources.push(source_owned(
                    format!("{} MCP config", display_tool(candidate.tool)),
                    candidate.path.clone(),
                    "read-only resource inventory",
                    false,
                    false,
                ));
                sources.push(source_owned(
                    format!("{} MCP sync target", display_tool(candidate.tool)),
                    candidate.path,
                    "cross-tool sync target write after explicit confirmation",
                    true,
                    true,
                ));
            }
            ToolPathKind::SkillDir
            | ToolPathKind::PluginDir
            | ToolPathKind::SubagentDir
            | ToolPathKind::InstructionFile
            | ToolPathKind::InstructionDir => {
                if let Some(source) = candidate_resource_source(&candidate) {
                    sources.push(source);
                }
            }
            ToolPathKind::DataDir
            | ToolPathKind::ProjectMcpConfig
            | ToolPathKind::ProjectSkillDir
            | ToolPathKind::ProjectSubagentDir
            | ToolPathKind::ProjectInstructionFile
            | ToolPathKind::ProjectInstructionDir => {}
        }
    }
    sources
}

fn candidate_resource_source(
    candidate: &crate::platform_paths::ToolPathCandidate,
) -> Option<PrivacyAuditSource> {
    let purpose = match candidate.kind {
        ToolPathKind::SkillDir => "read-only skill inventory",
        ToolPathKind::PluginDir => "read-only plugin inventory",
        ToolPathKind::SubagentDir => "subagent metadata and structure fingerprinting",
        ToolPathKind::InstructionFile | ToolPathKind::InstructionDir => {
            "instruction structure fingerprinting"
        }
        _ => return None,
    };
    Some(source_owned(
        candidate_resource_source_name(candidate)?,
        candidate.path.clone(),
        purpose,
        false,
        false,
    ))
}

fn candidate_resource_source_name(
    candidate: &crate::platform_paths::ToolPathCandidate,
) -> Option<String> {
    let path = candidate.path.display().to_string().replace('\\', "/");
    match (candidate.tool, candidate.kind) {
        ("claude", ToolPathKind::SkillDir) => Some("Claude Code skills".to_string()),
        ("claude", ToolPathKind::PluginDir) => Some("Claude Code plugins".to_string()),
        ("claude", ToolPathKind::SubagentDir) => Some("Claude Code subagents".to_string()),
        ("claude", ToolPathKind::InstructionFile) => {
            Some("Claude Code user instructions".to_string())
        }
        ("codex", ToolPathKind::InstructionFile) => Some("Codex user instructions".to_string()),
        ("opencode", ToolPathKind::InstructionFile) => {
            Some("OpenCode user instructions".to_string())
        }
        ("gemini", ToolPathKind::InstructionFile) => Some("Gemini user instructions".to_string()),
        ("cursor", ToolPathKind::InstructionFile) if path.ends_with(".cursorrules") => {
            Some("Cursor legacy rules".to_string())
        }
        ("cursor", ToolPathKind::InstructionDir) => Some("Cursor user rules".to_string()),
        _ => None,
    }
}

fn current_session_source_name(candidate: &crate::platform_paths::ToolPathCandidate) -> String {
    let path = candidate.path.display().to_string().replace('\\', "/");
    match candidate.tool {
        "claude" => "Claude Code sessions".to_string(),
        "codex" if path.ends_with(".codex/archived_sessions") => {
            "Codex archived sessions".to_string()
        }
        "codex" => "Codex sessions".to_string(),
        "gemini" => "Gemini CLI chats".to_string(),
        "opencode" => "OpenCode storage".to_string(),
        _ => format!("{} session data", display_tool(candidate.tool)),
    }
}

fn source_owned(
    name: String,
    path: impl AsRef<Path>,
    purpose: &str,
    reads_body: bool,
    writes_source: bool,
) -> PrivacyAuditSource {
    PrivacyAuditSource {
        name,
        path: path.as_ref().display().to_string(),
        purpose: purpose.to_string(),
        reads_body,
        writes_source,
    }
}

fn source(name: &str, path: impl AsRef<Path>, purpose: &str) -> PrivacyAuditSource {
    PrivacyAuditSource {
        name: name.to_string(),
        path: path.as_ref().display().to_string(),
        purpose: purpose.to_string(),
        reads_body: false,
        writes_source: false,
    }
}

fn cross_platform_candidate_sources() -> Vec<PrivacyAuditSource> {
    let contexts = [
        PlatformPathContext::windows(
            std::path::PathBuf::from(r"C:\Users\Example User"),
            std::path::PathBuf::from(r"C:\Users\Example User\AppData\Roaming"),
            std::path::PathBuf::from(r"C:\Users\Example User\AppData\Local"),
            Some(std::path::PathBuf::from(
                r"C:\Users\Example User\work\my project",
            )),
        ),
        PlatformPathContext::linux(
            std::path::PathBuf::from("/home/example user"),
            std::path::PathBuf::from("/home/example user/.config"),
            std::path::PathBuf::from("/home/example user/.local/share"),
            Some(std::path::PathBuf::from(
                "/home/example user/work/my project",
            )),
        ),
    ];
    contexts
        .iter()
        .flat_map(tool_path_candidates)
        .filter(|candidate| {
            candidate.kind == ToolPathKind::McpConfig
                || candidate.kind == ToolPathKind::ProjectMcpConfig
                || candidate.kind == ToolPathKind::ProjectSkillDir
                || candidate.kind == ToolPathKind::ProjectSubagentDir
                || candidate.kind == ToolPathKind::ProjectInstructionFile
                || candidate.kind == ToolPathKind::ProjectInstructionDir
                || candidate.kind == ToolPathKind::SessionData
        })
        .map(|candidate| PrivacyAuditSource {
            name: format!(
                "{} {} {} candidate",
                title_case(candidate.platform.as_str()),
                display_tool(candidate.tool),
                display_kind(candidate.kind)
            ),
            path: candidate.path.display().to_string(),
            purpose: format!(
                "read-only candidate; {}; write unsupported until real platform fixture and rollback e2e evidence exist",
                if candidate.verified {
                    "verified by local fixture"
                } else {
                    "unverified"
                }
            ),
            reads_body: false,
            writes_source: false,
        })
        .collect()
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn display_tool(tool: &str) -> &'static str {
    match tool {
        "claude" => "Claude",
        "codex" => "Codex",
        "gemini" => "Gemini",
        "opencode" => "OpenCode",
        "cursor" => "Cursor",
        _ => "Tool",
    }
}

fn display_kind(kind: ToolPathKind) -> &'static str {
    match kind {
        ToolPathKind::McpConfig | ToolPathKind::ProjectMcpConfig => "MCP config",
        ToolPathKind::SessionData => "session data",
        ToolPathKind::ConfigDir => "config directory",
        ToolPathKind::DataDir => "data directory",
        ToolPathKind::SkillDir | ToolPathKind::ProjectSkillDir => "skill directory",
        ToolPathKind::PluginDir => "plugin directory",
        ToolPathKind::SubagentDir | ToolPathKind::ProjectSubagentDir => "subagent directory",
        ToolPathKind::InstructionFile | ToolPathKind::ProjectInstructionFile => "instruction file",
        ToolPathKind::InstructionDir | ToolPathKind::ProjectInstructionDir => {
            "instruction directory"
        }
    }
}

fn stored_tables() -> Vec<PrivacyAuditTable> {
    vec![
        table(
            "usage_records",
            &[
                "id",
                "tool",
                "source_path",
                "session_id",
                "session metadata only",
                "ts",
                "project_path",
                "model",
                "message count metadata",
                "token buckets",
                "cost_usd",
                "precision",
                "cost_source",
            ],
        ),
        table(
            "tools",
            &[
                "id",
                "display_name",
                "installed",
                "version",
                "config_dir",
                "data_dir",
                "directory existence",
                "last_detected_at",
            ],
        ),
        table(
            "source_health",
            &[
                "tool",
                "source_glob",
                "exists",
                "readable_files",
                "parsed_records",
                "failed_files",
                "latest_event_ts",
                "checked_at",
            ],
        ),
        table(
            "pricing_rules",
            &[
                "model_match",
                "match_kind",
                "provider",
                "currency",
                "token bucket prices",
                "source",
                "updated_at",
            ],
        ),
        table(
            "project_aliases",
            &["canonical_path", "alias", "updated_at"],
        ),
        table(
            "resources",
            &[
                "id",
                "kind",
                "name",
                "scope",
                "origin metadata",
                "enabled_in",
                "confidence",
                "redacted",
                "payload_json redacted",
                "env key names",
                "env value fingerprints",
                "file inventory",
                "content fingerprints",
                "heading labels",
                "paragraph hashes",
                "component counts",
                "frontmatter metadata",
                "provided_by_plugin",
                "last_seen",
            ],
        ),
        table(
            "resource_templates",
            &[
                "id",
                "name",
                "kind",
                "scope",
                "description",
                "source",
                "content_fingerprint",
                "required_env_keys",
                "payload_json structure only",
                "updated_at",
                "no secret values",
            ],
        ),
        table(
            "resource_parse_failures",
            &["path", "tool", "kind", "reason", "seen_at"],
        ),
        table(
            "resource_changes",
            &[
                "change_id",
                "sync_id",
                "tool",
                "resource_id",
                "kind",
                "op",
                "target_path",
                "backup_path",
                "status",
                "reverts_change_id",
                "created_at",
                "change metadata",
            ],
        ),
        table(
            "sync_operations",
            &[
                "sync_id",
                "source_resource_id",
                "targets_json metadata",
                "allow_cross_scope",
                "env_strategy",
                "created_at",
            ],
        ),
        table(
            "sync_presets",
            &[
                "id",
                "name",
                "resources_json resource ids",
                "targets_json metadata",
                "updated_at",
                "no env values",
                "no key material",
            ],
        ),
        table(
            "resource_backups",
            &[
                "backup_path",
                "tool",
                "original_path",
                "change_id",
                "created_at",
            ],
        ),
        table(
            "file_fingerprints",
            &[
                "tool",
                "path",
                "fingerprint",
                "observed_at",
                "content hash only",
            ],
        ),
    ]
}

fn table(name: &str, fields: &[&str]) -> PrivacyAuditTable {
    PrivacyAuditTable {
        name: name.to_string(),
        fields: fields.iter().map(|field| (*field).to_string()).collect(),
    }
}

fn forbidden_fields() -> Vec<String> {
    [
        "prompt",
        "response",
        "session prompt text",
        "session response text",
        "session message body",
        "message content",
        "source code",
        "tool output",
        "api_key",
        "secret",
        "token value",
        "mcp env value",
        "mcp env raw value",
        "sync env value",
        "sync preset env value",
        "sync preset key material",
        "sync history raw target secret",
        "redacted report raw project path",
        "redacted report session id",
        "redacted report project name",
        "headless dashboard write endpoint",
        "headless dashboard non-loopback bind",
        "headless dashboard sync endpoint",
        "deep link raw secret",
        "deep link write without preview",
        "template raw secret",
        "template install write without preview",
        "platform fixture prompt body",
        "platform fixture response body",
        "platform fixture source body",
        "platform fixture secret value",
        "mcp secret argument",
        "instruction body",
        "skill file content",
        "plugin file content",
        "subagent body",
        "backup content in database",
    ]
    .iter()
    .map(|field| (*field).to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn privacy_audit_uses_path_resolver_for_current_tool_path_sources() {
        let source = include_str!("privacy.rs");

        assert!(source.contains("current_tool_candidate_sources"));
        for root in [
            ".claude/projects",
            ".codex/sessions",
            ".codex/archived_sessions",
            ".gemini/tmp",
            ".local/share/opencode/storage",
            ".claude",
            ".codex",
            ".gemini",
            ".config/opencode",
            ".claude.json",
            ".codex/config.toml",
            ".gemini/settings.json",
            ".cursor/mcp.json",
        ] {
            let hardcoded_join = format!("home.{}(\"{}\")", "join", root);
            assert!(
                !source.contains(&hardcoded_join),
                "privacy audit still hardcodes {hardcoded_join}"
            );
        }
    }

    #[test]
    fn privacy_audit_uses_path_resolver_for_user_resource_path_sources() {
        let source = include_str!("privacy.rs");

        assert!(source.contains("candidate_resource_source"));
        for path in [
            ".claude/skills",
            ".claude/CLAUDE.md",
            ".codex/AGENTS.md",
            ".gemini/GEMINI.md",
            ".cursor/rules",
            ".cursorrules",
            ".claude/plugins",
            ".claude/agents",
        ] {
            let hardcoded_join = format!("home.{}(\"{}\")", "join", path);
            assert!(
                !source.contains(&hardcoded_join),
                "privacy audit user resource source still hardcodes {hardcoded_join}"
            );
        }
    }

    #[test]
    fn privacy_audit_covers_phase_one_sources_tables_and_forbidden_fields() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);

        assert!(report.local_only);
        assert_eq!(report.db_path, db.display().to_string());
        assert!(
            report
                .read_sources
                .iter()
                .any(|source| source.path.ends_with(".claude/projects") && !source.writes_source)
        );
        assert!(
            report
                .stored_tables
                .iter()
                .any(|table| table.name == "pricing_rules")
        );
        assert!(
            report
                .stored_tables
                .iter()
                .any(|table| table.name == "project_aliases")
        );
        assert!(
            report
                .stored_tables
                .iter()
                .any(|table| table.name == "resources")
        );
        assert!(
            report
                .stored_tables
                .iter()
                .any(|table| table.name == "resource_parse_failures")
        );
        assert!(
            report
                .read_sources
                .iter()
                .any(|source| source.path.ends_with(".codex/config.toml") && !source.reads_body)
        );
        assert!(
            report
                .read_sources
                .iter()
                .any(|source| source.path.ends_with(".claude/skills") && !source.reads_body)
        );
        assert!(
            report
                .read_sources
                .iter()
                .any(|source| source.path.ends_with(".codex/AGENTS.md") && !source.reads_body)
        );
        assert!(
            report
                .read_sources
                .iter()
                .any(|source| source.path.ends_with(".claude/plugins") && !source.reads_body)
        );
        assert!(
            report
                .read_sources
                .iter()
                .any(|source| source.path.ends_with(".claude/agents") && !source.reads_body)
        );
        assert!(report.forbidden_fields.contains(&"prompt".to_string()));
        assert!(report.forbidden_fields.contains(&"api_key".to_string()));
        assert!(
            report
                .forbidden_fields
                .contains(&"mcp env raw value".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"instruction body".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"skill file content".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"plugin file content".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"subagent body".to_string())
        );
        assert!(report.export_boundary.contains("metadata"));
    }

    #[test]
    fn privacy_audit_names_phase_two_resource_and_session_field_boundaries() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);
        let resources = report
            .stored_tables
            .iter()
            .find(|table| table.name == "resources")
            .unwrap();

        assert!(resources.fields.contains(&"env key names".to_string()));
        assert!(
            resources
                .fields
                .contains(&"env value fingerprints".to_string())
        );
        assert!(resources.fields.contains(&"file inventory".to_string()));
        assert!(
            resources
                .fields
                .contains(&"content fingerprints".to_string())
        );
        assert!(resources.fields.contains(&"heading labels".to_string()));
        assert!(resources.fields.contains(&"paragraph hashes".to_string()));
        assert!(resources.fields.contains(&"component counts".to_string()));
        assert!(
            resources
                .fields
                .contains(&"frontmatter metadata".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"session prompt text".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"session response text".to_string())
        );
    }

    #[test]
    fn privacy_audit_names_project_level_resource_scan_boundaries() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);

        assert!(
            report
                .read_sources
                .iter()
                .any(|source| source.path == "<project>/AGENTS.md" && !source.reads_body)
        );
        assert!(
            report
                .read_sources
                .iter()
                .any(|source| source.path == "<project>/.cursor/mcp.json" && !source.writes_source)
        );
        assert!(
            report
                .read_sources
                .iter()
                .any(|source| source.path == "<project>/.claude/agents" && !source.reads_body)
        );
    }

    #[test]
    fn privacy_audit_names_cross_platform_candidate_path_boundaries() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);
        let windows_codex = report
            .read_sources
            .iter()
            .find(|source| source.name == "Windows Codex MCP config candidate")
            .expect("privacy audit should name Windows Codex MCP candidate path");
        let linux_gemini = report
            .read_sources
            .iter()
            .find(|source| source.name == "Linux Gemini MCP config candidate")
            .expect("privacy audit should name Linux Gemini MCP candidate path");
        let joined = serde_json::to_string(&report).unwrap();

        assert!(
            windows_codex
                .path
                .contains(r"C:\Users\Example User\.codex\config.toml")
        );
        assert!(windows_codex.purpose.contains("read-only candidate"));
        assert!(windows_codex.purpose.contains("unverified"));
        assert!(windows_codex.purpose.contains("write unsupported"));
        assert!(!windows_codex.reads_body);
        assert!(!windows_codex.writes_source);
        assert!(
            linux_gemini
                .path
                .contains("/home/example user/.gemini/settings.json")
        );
        assert!(linux_gemini.purpose.contains("read-only candidate"));
        assert!(linux_gemini.purpose.contains("unverified"));
        assert!(!linux_gemini.reads_body);
        assert!(!linux_gemini.writes_source);
        assert!(
            report
                .export_boundary
                .contains("Windows/Linux candidate paths")
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"platform fixture prompt body".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"platform fixture secret value".to_string())
        );
        assert!(!joined.contains("sk-live"));
        assert!(!joined.contains("prompt body fixture"));
    }

    #[test]
    fn privacy_audit_names_adapter_and_session_metadata_boundaries() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);
        let usage_records = report
            .stored_tables
            .iter()
            .find(|table| table.name == "usage_records")
            .unwrap();

        assert!(
            usage_records
                .fields
                .contains(&"session metadata only".to_string())
        );
        assert!(
            usage_records
                .fields
                .contains(&"message count metadata".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"session message body".to_string())
        );
        assert!(
            report
                .export_boundary
                .contains("Adapter capability declarations are read-only metadata")
        );
    }

    #[test]
    fn privacy_audit_names_phase_three_backup_and_change_boundaries() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);

        assert!(report.read_sources.iter().any(|source| {
            source.path.ends_with(".wapc/backups")
                && source.purpose.contains("rollback")
                && source.reads_body
                && !source.writes_source
        }));
        assert!(report.stored_tables.iter().any(|table| {
            table.name == "resource_changes"
                && table.fields.contains(&"change metadata".to_string())
                && table.fields.contains(&"backup_path".to_string())
        }));
        assert!(report.stored_tables.iter().any(|table| {
            table.name == "resource_backups"
                && table.fields.contains(&"backup_path".to_string())
                && table.fields.contains(&"original_path".to_string())
        }));
        assert!(report.stored_tables.iter().any(|table| {
            table.name == "file_fingerprints"
                && table.fields.contains(&"content hash only".to_string())
        }));
        assert!(
            report
                .export_boundary
                .contains("Backups may contain original tool configuration contents")
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"backup content in database".to_string())
        );
    }

    #[test]
    fn privacy_audit_names_phase_four_sync_metadata_and_env_boundaries() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);

        assert!(report.stored_tables.iter().any(|table| {
            table.name == "sync_operations"
                && table.fields.contains(&"targets_json metadata".to_string())
                && table.fields.contains(&"env_strategy".to_string())
        }));
        assert!(report.stored_tables.iter().any(|table| {
            table.name == "resource_changes" && table.fields.contains(&"sync_id".to_string())
        }));
        assert!(
            report
                .export_boundary
                .contains("apply_sync does not persist env values")
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"sync env value".to_string())
        );
    }

    #[test]
    fn privacy_audit_names_phase_four_sync_preset_export_and_target_write_boundaries() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);

        assert!(report.read_sources.iter().any(|source| {
            source.path.ends_with(".codex/config.toml")
                && source.writes_source
                && source.purpose.contains("cross-tool sync target write")
        }));
        assert!(report.read_sources.iter().any(|source| {
            source.path.ends_with(".cursor/mcp.json")
                && source.writes_source
                && source.purpose.contains("cross-tool sync target write")
        }));
        assert!(report.stored_tables.iter().any(|table| {
            table.name == "sync_presets"
                && table
                    .fields
                    .contains(&"resources_json resource ids".to_string())
                && table.fields.contains(&"targets_json metadata".to_string())
                && table.fields.contains(&"no env values".to_string())
        }));
        assert!(
            report
                .export_boundary
                .contains("sync preset JSON exports exclude env values and key material")
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"sync preset env value".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"sync preset key material".to_string())
        );
    }

    #[test]
    fn privacy_audit_names_phase_five_redacted_team_report_boundary() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);

        assert!(
            report
                .export_boundary
                .contains("redacted team reports hash project paths")
        );
        assert!(report.export_boundary.contains(
            "exclude source paths, session ids, project names, prompt bodies, and key material"
        ));
        assert!(
            report
                .forbidden_fields
                .contains(&"redacted report raw project path".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"redacted report session id".to_string())
        );
    }

    #[test]
    fn privacy_audit_names_phase_five_headless_dashboard_boundary() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);

        assert!(
            report
                .export_boundary
                .contains("Headless dashboard is disabled by default, binds only to 127.0.0.1")
        );
        assert!(
            report
                .export_boundary
                .contains("serves read-only usage summaries only")
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"headless dashboard write endpoint".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"headless dashboard non-loopback bind".to_string())
        );
    }

    #[test]
    fn privacy_audit_names_phase_five_deep_link_import_boundary() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);

        assert!(
            report
                .export_boundary
                .contains("wapc://import deep links are preview-only")
        );
        assert!(
            report
                .export_boundary
                .contains("reject raw env values, Authorization headers, and token-like secrets")
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"deep link raw secret".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"deep link write without preview".to_string())
        );
    }

    #[test]
    fn privacy_audit_names_phase_five_resource_template_boundary() {
        let home = tempdir().unwrap();
        let db = home.path().join(".wapc/wapc.db");

        let report = privacy_audit(home.path(), &db);

        assert!(report.stored_tables.iter().any(|table| {
            table.name == "resource_templates"
                && table.fields.contains(&"content_fingerprint".to_string())
                && table.fields.contains(&"no secret values".to_string())
        }));
        assert!(
            report
                .export_boundary
                .contains("resource templates store structure and source metadata only")
        );
        assert!(
            report
                .export_boundary
                .contains("template installs route through Sync Engine preview")
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"template raw secret".to_string())
        );
        assert!(
            report
                .forbidden_fields
                .contains(&"template install write without preview".to_string())
        );
    }
}
