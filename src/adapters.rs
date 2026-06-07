//! Read-only adapter capability declarations for supported tools.
//! @author codex

use crate::model::AdapterCapability;

pub fn adapter_capabilities() -> Vec<AdapterCapability> {
    vec![
        capability(
            "claude",
            "Claude Code",
            &["mcp", "skill", "instruction", "plugin", "subagent"],
            &["user", "project"],
            &["stdio", "http", "sse"],
            &[
                "Reads ~/.claude.json, ~/.claude/skills, ~/.claude/CLAUDE.md, ~/.claude/plugins, ~/.claude/agents, and <project>/.mcp.json, <project>/CLAUDE.md, <project>/.claude/skills, <project>/.claude/agents.",
                "Phase 2 is read-only and stores only metadata, fingerprints, and redacted payloads.",
            ],
        ),
        capability(
            "codex",
            "Codex",
            &["mcp", "instruction"],
            &["user", "project"],
            &["stdio", "http"],
            &[
                "Reads ~/.codex/config.toml, ~/.codex/AGENTS.md, and <project>/AGENTS.md.",
                "Phase 2 is read-only; TOML MCP fields are normalized into canonical redacted metadata and fingerprints.",
            ],
        ),
        capability(
            "gemini",
            "Gemini CLI",
            &["mcp", "instruction"],
            &["user", "project"],
            &["stdio", "http", "sse"],
            &[
                "Reads ~/.gemini/settings.json, ~/.gemini/GEMINI.md, and <project>/GEMINI.md.",
                "Phase 2 is read-only; remote MCP transport field names remain best-effort metadata until per-version verification.",
            ],
        ),
        capability(
            "opencode",
            "OpenCode",
            &["mcp", "instruction", "skill"],
            &["user", "project"],
            &["stdio", "http"],
            &[
                "Reads ~/.config/opencode/opencode.json, <project>/opencode.json, ~/.config/opencode/AGENTS.md, <project>/AGENTS.md, ~/.config/opencode/skills/<name>/SKILL.md, and <project>/.opencode/skills/<name>/SKILL.md.",
                "Phase 2 is read-only and stores only redacted MCP metadata, instruction fingerprints, skill file metadata, and frontmatter fingerprints; OpenCode MCP runtime auth/OAuth state, skill install/sync/write, permission strategy, and rollback remain unsupported until separately verified.",
            ],
        ),
        capability(
            "cursor",
            "Cursor",
            &["mcp", "instruction"],
            &["user", "project"],
            &["stdio", "http", "sse"],
            &[
                "Reads ~/.cursor/mcp.json, ~/.cursor/rules/*.mdc, ~/.cursorrules, <project>/.cursor/mcp.json, <project>/.cursor/rules/*.mdc, and <project>/.cursorrules.",
                "Phase 2 project-level detection is read-only and stores only redacted metadata and fingerprints from known local project roots.",
            ],
        ),
        capability(
            "vscode",
            "VS Code Copilot",
            &["mcp", "instruction"],
            &["project"],
            &["stdio", "http", "sse"],
            &[
                "Reads project .vscode/mcp.json and .github/copilot-instructions.md from explicit project roots only.",
                "Phase 2 is read-only and stores only redacted MCP metadata plus instruction fingerprints; user profile paths, OAuth/header runtime behavior, and writes remain unsupported until separately verified.",
            ],
        ),
    ]
}

fn capability(
    tool: &str,
    display_name: &str,
    resource_kinds: &[&str],
    scopes: &[&str],
    transports: &[&str],
    notes: &[&str],
) -> AdapterCapability {
    AdapterCapability {
        tool: tool.to_string(),
        display_name: display_name.to_string(),
        resource_kinds: resource_kinds
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        scopes: scopes.iter().map(|value| (*value).to_string()).collect(),
        transports: transports
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        read_only: true,
        notes: notes.iter().map(|value| (*value).to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_current_phase_two_read_only_capabilities() {
        let capabilities = adapter_capabilities();

        assert_eq!(capabilities.len(), 6);
        assert!(capabilities.iter().all(|capability| capability.read_only));
        let claude = capabilities
            .iter()
            .find(|capability| capability.tool == "claude")
            .unwrap();
        assert!(claude.resource_kinds.contains(&"subagent".to_string()));
        assert!(claude.scopes.contains(&"user".to_string()));
        assert!(claude.scopes.contains(&"project".to_string()));
        let cursor = capabilities
            .iter()
            .find(|capability| capability.tool == "cursor")
            .unwrap();
        assert!(cursor.scopes.contains(&"project".to_string()));
        assert!(cursor.notes.iter().any(|note| note.contains("<project>")));
        let vscode = capabilities
            .iter()
            .find(|capability| capability.tool == "vscode")
            .unwrap();
        assert_eq!(vscode.resource_kinds, vec!["mcp", "instruction"]);
        assert_eq!(vscode.scopes, vec!["project"]);
        assert!(
            vscode
                .notes
                .iter()
                .any(|note| note.contains(".vscode/mcp.json"))
        );
        assert!(
            vscode
                .notes
                .iter()
                .any(|note| note.contains(".github/copilot-instructions.md"))
        );
        let opencode = capabilities
            .iter()
            .find(|capability| capability.tool == "opencode")
            .unwrap();
        assert_eq!(opencode.resource_kinds, vec!["mcp", "instruction", "skill"]);
        assert_eq!(opencode.scopes, vec!["user", "project"]);
        assert!(
            opencode
                .notes
                .iter()
                .any(|note| note.contains("opencode.json"))
        );
        assert!(
            opencode
                .notes
                .iter()
                .any(|note| note.contains("~/.config/opencode/AGENTS.md"))
        );
        assert!(
            opencode
                .notes
                .iter()
                .any(|note| note.contains(".opencode/skills"))
        );
    }

    #[test]
    fn adapter_capability_notes_exclude_write_sync_promises() {
        for capability in adapter_capabilities() {
            let notes = capability.notes.join(" ").to_ascii_lowercase();
            assert!(
                notes.contains("read-only"),
                "{} notes must say read-only",
                capability.tool
            );
            assert!(
                notes.contains("metadata")
                    || notes.contains("redacted")
                    || notes.contains("fingerprint"),
                "{} notes must describe safe persisted payload boundaries",
                capability.tool
            );
            assert!(!notes.contains("will write"));
            assert!(!notes.contains("will sync"));
            assert!(!notes.contains("enable or disable"));
        }
    }
}
