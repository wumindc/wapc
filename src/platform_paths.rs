//! Platform-aware WAPC path resolution.
//! @author codex

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformKind {
    Macos,
    Windows,
    Linux,
}

impl PlatformKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolPathKind {
    ConfigDir,
    DataDir,
    McpConfig,
    SessionData,
    SkillDir,
    PluginDir,
    SubagentDir,
    InstructionFile,
    InstructionDir,
    ProjectMcpConfig,
    ProjectSkillDir,
    ProjectSubagentDir,
    ProjectInstructionFile,
    ProjectInstructionDir,
}

impl ToolPathKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfigDir => "config_dir",
            Self::DataDir => "data_dir",
            Self::McpConfig => "mcp_config",
            Self::SessionData => "session_data",
            Self::SkillDir => "skill_dir",
            Self::PluginDir => "plugin_dir",
            Self::SubagentDir => "subagent_dir",
            Self::InstructionFile => "instruction_file",
            Self::InstructionDir => "instruction_dir",
            Self::ProjectMcpConfig => "project_mcp_config",
            Self::ProjectSkillDir => "project_skill_dir",
            Self::ProjectSubagentDir => "project_subagent_dir",
            Self::ProjectInstructionFile => "project_instruction_file",
            Self::ProjectInstructionDir => "project_instruction_dir",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformPathContext {
    pub platform: PlatformKind,
    pub home_dir: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub project_root: Option<PathBuf>,
}

impl PlatformPathContext {
    pub fn macos(home_dir: PathBuf, project_root: Option<PathBuf>) -> Self {
        Self {
            config_dir: home_dir.join("Library/Application Support"),
            data_dir: home_dir.join("Library/Application Support"),
            home_dir,
            platform: PlatformKind::Macos,
            project_root,
        }
    }

    pub fn windows(
        home_dir: PathBuf,
        app_data_dir: PathBuf,
        local_app_data_dir: PathBuf,
        project_root: Option<PathBuf>,
    ) -> Self {
        Self {
            home_dir,
            config_dir: app_data_dir,
            data_dir: local_app_data_dir,
            platform: PlatformKind::Windows,
            project_root,
        }
    }

    pub fn linux(
        home_dir: PathBuf,
        xdg_config_dir: PathBuf,
        xdg_data_dir: PathBuf,
        project_root: Option<PathBuf>,
    ) -> Self {
        Self {
            home_dir,
            config_dir: xdg_config_dir,
            data_dir: xdg_data_dir,
            platform: PlatformKind::Linux,
            project_root,
        }
    }

    pub fn current_home_compatible(home_dir: impl AsRef<Path>) -> Self {
        let home_dir = home_dir.as_ref().to_path_buf();
        Self::current_home_compatible_with_project(home_dir, None)
    }

    pub fn current_home_compatible_with_project(
        home_dir: impl AsRef<Path>,
        project_root: Option<PathBuf>,
    ) -> Self {
        let home_dir = home_dir.as_ref().to_path_buf();
        match std::env::consts::OS {
            "windows" => {
                let app_data = std::env::var_os("APPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home_dir.join("AppData/Roaming"));
                let local_app_data = std::env::var_os("LOCALAPPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home_dir.join("AppData/Local"));
                Self::windows(home_dir, app_data, local_app_data, project_root)
            }
            "linux" => {
                let config = std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home_dir.join(".config"));
                let data = std::env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home_dir.join(".local/share"));
                Self::linux(home_dir, config, data, project_root)
            }
            _ => Self::macos(home_dir, project_root),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolPathCandidate {
    pub tool: &'static str,
    pub platform: PlatformKind,
    pub scope: &'static str,
    pub kind: ToolPathKind,
    pub path: PathBuf,
    pub verified: bool,
    pub read_only: bool,
    pub write_supported: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ToolPathVerificationRecord {
    pub tool: String,
    pub platform: String,
    pub scope: String,
    pub kind: String,
    pub path: String,
    pub candidate_verified: bool,
    pub exists: bool,
    pub is_file: bool,
    pub is_dir: bool,
    pub read_only: bool,
    pub write_supported: bool,
}

pub fn verify_tool_path_candidates(
    context: &PlatformPathContext,
) -> Vec<ToolPathVerificationRecord> {
    tool_path_candidates(context)
        .into_iter()
        .map(|candidate| verify_tool_path_candidate(context, candidate))
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WapcPaths {
    pub home_dir: PathBuf,
    pub app_dir: PathBuf,
    pub db_path: PathBuf,
    pub backups_dir: PathBuf,
    pub settings_path: PathBuf,
}

impl WapcPaths {
    pub fn from_platform_home() -> Result<Self> {
        let home = dirs_next::home_dir().context("cannot resolve home directory")?;
        Ok(Self::from_home(home))
    }

    pub fn from_home(home: impl AsRef<Path>) -> Self {
        let home_dir = home.as_ref().to_path_buf();
        let app_dir = home_dir.join(".wapc");
        Self {
            home_dir,
            db_path: app_dir.join("wapc.db"),
            backups_dir: app_dir.join("backups"),
            settings_path: app_dir.join("settings.json"),
            app_dir,
        }
    }
}

pub fn tool_registry_paths_for_home(home: &Path) -> Vec<ToolPathCandidate> {
    tool_path_candidates(&PlatformPathContext::current_home_compatible(home))
        .into_iter()
        .filter(|candidate| {
            candidate.scope == "user"
                && (candidate.kind == ToolPathKind::ConfigDir
                    || candidate.kind == ToolPathKind::DataDir)
        })
        .collect()
}

pub fn tool_path_candidates(context: &PlatformPathContext) -> Vec<ToolPathCandidate> {
    let mut candidates = Vec::new();
    push_user_tool_candidates(context, &mut candidates);
    if let Some(project_root) = &context.project_root {
        push_project_candidates(context.platform, project_root, &mut candidates);
    }
    candidates
}

fn verify_tool_path_candidate(
    context: &PlatformPathContext,
    candidate: ToolPathCandidate,
) -> ToolPathVerificationRecord {
    let metadata = fs::metadata(&candidate.path).ok();
    ToolPathVerificationRecord {
        tool: candidate.tool.to_string(),
        platform: candidate.platform.as_str().to_string(),
        scope: candidate.scope.to_string(),
        kind: candidate.kind.as_str().to_string(),
        path: display_path_with_aliases(&candidate.path, context),
        candidate_verified: candidate.verified,
        exists: metadata.is_some(),
        is_file: metadata.as_ref().is_some_and(fs::Metadata::is_file),
        is_dir: metadata.as_ref().is_some_and(fs::Metadata::is_dir),
        read_only: candidate.read_only,
        write_supported: candidate.write_supported,
    }
}

fn display_path_with_aliases(path: &Path, context: &PlatformPathContext) -> String {
    if let Some(project_root) = &context.project_root
        && let Ok(relative) = path.strip_prefix(project_root)
    {
        if relative.as_os_str().is_empty() {
            return "<project>".to_string();
        }
        return format!("<project>/{}", relative.display());
    }
    if let Ok(relative) = path.strip_prefix(&context.home_dir) {
        if relative.as_os_str().is_empty() {
            return "~".to_string();
        }
        return format!("~/{}", relative.display());
    }
    path.display().to_string()
}

fn push_user_tool_candidates(
    context: &PlatformPathContext,
    candidates: &mut Vec<ToolPathCandidate>,
) {
    let verified = context.platform == PlatformKind::Macos;
    push_candidate(
        candidates,
        context,
        "claude",
        ToolPathKind::ConfigDir,
        join_candidate_path(context.platform, &context.home_dir, ".claude"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "claude",
        ToolPathKind::DataDir,
        join_candidate_path(context.platform, &context.home_dir, ".claude/projects"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "claude",
        ToolPathKind::SessionData,
        join_candidate_path(context.platform, &context.home_dir, ".claude/projects"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "claude",
        ToolPathKind::SkillDir,
        join_candidate_path(context.platform, &context.home_dir, ".claude/skills"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "claude",
        ToolPathKind::PluginDir,
        join_candidate_path(context.platform, &context.home_dir, ".claude/plugins"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "claude",
        ToolPathKind::SubagentDir,
        join_candidate_path(context.platform, &context.home_dir, ".claude/agents"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "claude",
        ToolPathKind::InstructionFile,
        join_candidate_path(context.platform, &context.home_dir, ".claude/CLAUDE.md"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "claude",
        ToolPathKind::McpConfig,
        join_candidate_path(context.platform, &context.home_dir, ".claude.json"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "codex",
        ToolPathKind::ConfigDir,
        join_candidate_path(context.platform, &context.home_dir, ".codex"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "codex",
        ToolPathKind::DataDir,
        join_candidate_path(context.platform, &context.home_dir, ".codex/sessions"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "codex",
        ToolPathKind::SessionData,
        join_candidate_path(context.platform, &context.home_dir, ".codex/sessions"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "codex",
        ToolPathKind::SessionData,
        join_candidate_path(
            context.platform,
            &context.home_dir,
            ".codex/archived_sessions",
        ),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "codex",
        ToolPathKind::McpConfig,
        join_candidate_path(context.platform, &context.home_dir, ".codex/config.toml"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "codex",
        ToolPathKind::InstructionFile,
        join_candidate_path(context.platform, &context.home_dir, ".codex/AGENTS.md"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "gemini",
        ToolPathKind::ConfigDir,
        join_candidate_path(context.platform, &context.home_dir, ".gemini"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "gemini",
        ToolPathKind::DataDir,
        join_candidate_path(context.platform, &context.home_dir, ".gemini/tmp"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "gemini",
        ToolPathKind::SessionData,
        join_candidate_path(context.platform, &context.home_dir, ".gemini/tmp"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "gemini",
        ToolPathKind::McpConfig,
        join_candidate_path(context.platform, &context.home_dir, ".gemini/settings.json"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "gemini",
        ToolPathKind::InstructionFile,
        join_candidate_path(context.platform, &context.home_dir, ".gemini/GEMINI.md"),
        verified,
    );
    let opencode_config = match context.platform {
        PlatformKind::Linux => {
            join_candidate_path(context.platform, &context.config_dir, "opencode")
        }
        PlatformKind::Windows => {
            join_candidate_path(context.platform, &context.config_dir, "opencode")
        }
        PlatformKind::Macos => {
            join_candidate_path(context.platform, &context.home_dir, ".config/opencode")
        }
    };
    let opencode_data = match context.platform {
        PlatformKind::Linux => {
            join_candidate_path(context.platform, &context.data_dir, "opencode/storage")
        }
        PlatformKind::Windows => {
            join_candidate_path(context.platform, &context.data_dir, "opencode/storage")
        }
        PlatformKind::Macos => join_candidate_path(
            context.platform,
            &context.home_dir,
            ".local/share/opencode/storage",
        ),
    };
    push_candidate(
        candidates,
        context,
        "opencode",
        ToolPathKind::ConfigDir,
        opencode_config.clone(),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "opencode",
        ToolPathKind::DataDir,
        opencode_data.clone(),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "opencode",
        ToolPathKind::SessionData,
        opencode_data,
        verified,
    );
    push_candidate(
        candidates,
        context,
        "opencode",
        ToolPathKind::McpConfig,
        join_candidate_path(context.platform, &opencode_config, "opencode.json"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "opencode",
        ToolPathKind::InstructionFile,
        join_candidate_path(context.platform, &opencode_config, "AGENTS.md"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "opencode",
        ToolPathKind::SkillDir,
        join_candidate_path(context.platform, &opencode_config, "skills"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "cursor",
        ToolPathKind::McpConfig,
        join_candidate_path(context.platform, &context.home_dir, ".cursor/mcp.json"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "cursor",
        ToolPathKind::InstructionFile,
        join_candidate_path(context.platform, &context.home_dir, ".cursorrules"),
        verified,
    );
    push_candidate(
        candidates,
        context,
        "cursor",
        ToolPathKind::InstructionDir,
        join_candidate_path(context.platform, &context.home_dir, ".cursor/rules"),
        verified,
    );
}

fn push_project_candidates(
    platform: PlatformKind,
    project_root: &Path,
    candidates: &mut Vec<ToolPathCandidate>,
) {
    for (tool, path) in [
        (
            "claude",
            join_candidate_path(platform, project_root, ".mcp.json"),
        ),
        (
            "cursor",
            join_candidate_path(platform, project_root, ".cursor/mcp.json"),
        ),
        (
            "vscode",
            join_candidate_path(platform, project_root, ".vscode/mcp.json"),
        ),
        (
            "opencode",
            join_candidate_path(platform, project_root, "opencode.json"),
        ),
    ] {
        candidates.push(ToolPathCandidate {
            tool,
            platform,
            scope: "project",
            kind: ToolPathKind::ProjectMcpConfig,
            path,
            verified: platform == PlatformKind::Macos,
            read_only: true,
            write_supported: false,
        });
    }
    for (tool, kind, path) in [
        (
            "claude",
            ToolPathKind::ProjectSkillDir,
            join_candidate_path(platform, project_root, ".claude/skills"),
        ),
        (
            "opencode",
            ToolPathKind::ProjectSkillDir,
            join_candidate_path(platform, project_root, ".opencode/skills"),
        ),
        (
            "claude",
            ToolPathKind::ProjectSubagentDir,
            join_candidate_path(platform, project_root, ".claude/agents"),
        ),
    ] {
        candidates.push(ToolPathCandidate {
            tool,
            platform,
            scope: "project",
            kind,
            path,
            verified: platform == PlatformKind::Macos,
            read_only: true,
            write_supported: false,
        });
    }
    for (tool, path) in [
        (
            "claude",
            join_candidate_path(platform, project_root, "CLAUDE.md"),
        ),
        (
            "codex",
            join_candidate_path(platform, project_root, "AGENTS.md"),
        ),
        (
            "opencode",
            join_candidate_path(platform, project_root, "AGENTS.md"),
        ),
        (
            "vscode",
            join_candidate_path(platform, project_root, ".github/copilot-instructions.md"),
        ),
        (
            "gemini",
            join_candidate_path(platform, project_root, "GEMINI.md"),
        ),
        (
            "cursor",
            join_candidate_path(platform, project_root, ".cursorrules"),
        ),
    ] {
        candidates.push(ToolPathCandidate {
            tool,
            platform,
            scope: "project",
            kind: ToolPathKind::ProjectInstructionFile,
            path,
            verified: platform == PlatformKind::Macos,
            read_only: true,
            write_supported: false,
        });
    }
    candidates.push(ToolPathCandidate {
        tool: "cursor",
        platform,
        scope: "project",
        kind: ToolPathKind::ProjectInstructionDir,
        path: join_candidate_path(platform, project_root, ".cursor/rules"),
        verified: platform == PlatformKind::Macos,
        read_only: true,
        write_supported: false,
    });
}

fn push_candidate(
    candidates: &mut Vec<ToolPathCandidate>,
    context: &PlatformPathContext,
    tool: &'static str,
    kind: ToolPathKind,
    path: PathBuf,
    verified: bool,
) {
    candidates.push(ToolPathCandidate {
        tool,
        platform: context.platform,
        scope: "user",
        kind,
        path,
        verified,
        read_only: true,
        write_supported: false,
    });
}

fn join_candidate_path(platform: PlatformKind, base: &Path, relative: &str) -> PathBuf {
    if platform == PlatformKind::Windows {
        let mut value = base.display().to_string();
        if !value.ends_with('\\') && !value.ends_with('/') {
            value.push('\\');
        }
        value.push_str(&relative.replace('/', "\\"));
        return PathBuf::from(value);
    }
    base.join(relative)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolves_wapc_paths_from_explicit_home_without_touching_filesystem() {
        let home = PathBuf::from("/Users/example");

        let paths = WapcPaths::from_home(&home);

        assert_eq!(paths.home_dir, home);
        assert_eq!(paths.app_dir, PathBuf::from("/Users/example/.wapc"));
        assert_eq!(paths.db_path, PathBuf::from("/Users/example/.wapc/wapc.db"));
        assert_eq!(
            paths.backups_dir,
            PathBuf::from("/Users/example/.wapc/backups")
        );
        assert_eq!(
            paths.settings_path,
            PathBuf::from("/Users/example/.wapc/settings.json")
        );
    }

    #[test]
    fn resolves_cross_platform_tool_path_candidates_without_touching_filesystem() {
        let macos = PlatformPathContext::macos(
            PathBuf::from("/Users/Example User"),
            Some(PathBuf::from("/Users/Example User/work/my project")),
        );
        let windows = PlatformPathContext::windows(
            PathBuf::from(r"C:\Users\Example User"),
            PathBuf::from(r"C:\Users\Example User\AppData\Roaming"),
            PathBuf::from(r"C:\Users\Example User\AppData\Local"),
            Some(PathBuf::from(r"C:\Users\Example User\work\my project")),
        );
        let linux = PlatformPathContext::linux(
            PathBuf::from("/home/example user"),
            PathBuf::from("/home/example user/.config"),
            PathBuf::from("/home/example user/.local/share"),
            Some(PathBuf::from("/home/example user/work/my project")),
        );

        let macos_candidates = tool_path_candidates(&macos);
        let windows_candidates = tool_path_candidates(&windows);
        let linux_candidates = tool_path_candidates(&linux);

        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "codex"
                && candidate.kind == ToolPathKind::McpConfig
                && candidate.path == std::path::Path::new("/Users/Example User/.codex/config.toml")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(windows_candidates.iter().any(|candidate| {
            candidate.tool == "gemini"
                && candidate.kind == ToolPathKind::McpConfig
                && candidate.path
                    == std::path::Path::new(r"C:\Users\Example User\.gemini\settings.json")
                && !candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(linux_candidates.iter().any(|candidate| {
            candidate.tool == "opencode"
                && candidate.kind == ToolPathKind::ConfigDir
                && candidate.path == std::path::Path::new("/home/example user/.config/opencode")
                && !candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(linux_candidates.iter().any(|candidate| {
            candidate.tool == "codex"
                && candidate.kind == ToolPathKind::McpConfig
                && candidate.path == std::path::Path::new("/home/example user/.codex/config.toml")
                && !candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(linux_candidates.iter().any(|candidate| {
            candidate.tool == "gemini"
                && candidate.kind == ToolPathKind::McpConfig
                && candidate.path
                    == std::path::Path::new("/home/example user/.gemini/settings.json")
                && !candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "codex"
                && candidate.kind == ToolPathKind::SessionData
                && candidate.path
                    == std::path::Path::new("/Users/Example User/.codex/archived_sessions")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "claude"
                && candidate.kind == ToolPathKind::SkillDir
                && candidate.path == std::path::Path::new("/Users/Example User/.claude/skills")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "claude"
                && candidate.kind == ToolPathKind::PluginDir
                && candidate.path == std::path::Path::new("/Users/Example User/.claude/plugins")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "claude"
                && candidate.kind == ToolPathKind::SubagentDir
                && candidate.path == std::path::Path::new("/Users/Example User/.claude/agents")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "claude"
                && candidate.kind == ToolPathKind::InstructionFile
                && candidate.path == std::path::Path::new("/Users/Example User/.claude/CLAUDE.md")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "cursor"
                && candidate.kind == ToolPathKind::InstructionDir
                && candidate.path == std::path::Path::new("/Users/Example User/.cursor/rules")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(linux_candidates.iter().any(|candidate| {
            candidate.tool == "codex"
                && candidate.kind == ToolPathKind::InstructionFile
                && candidate.path == std::path::Path::new("/home/example user/.codex/AGENTS.md")
                && !candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(linux_candidates.iter().any(|candidate| {
            candidate.tool == "opencode"
                && candidate.kind == ToolPathKind::SessionData
                && candidate.path
                    == std::path::Path::new("/home/example user/.local/share/opencode/storage")
                && !candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(windows_candidates.iter().any(|candidate| {
            candidate.tool == "cursor"
                && candidate.scope == "project"
                && candidate.path
                    == std::path::Path::new(
                        r"C:\Users\Example User\work\my project\.cursor\mcp.json",
                    )
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "vscode"
                && candidate.scope == "project"
                && candidate.kind == ToolPathKind::ProjectMcpConfig
                && candidate.path
                    == std::path::Path::new("/Users/Example User/work/my project/.vscode/mcp.json")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "claude"
                && candidate.scope == "project"
                && candidate.kind == ToolPathKind::ProjectSkillDir
                && candidate.path
                    == std::path::Path::new("/Users/Example User/work/my project/.claude/skills")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(windows_candidates.iter().any(|candidate| {
            candidate.tool == "claude"
                && candidate.scope == "project"
                && candidate.kind == ToolPathKind::ProjectSubagentDir
                && candidate.path
                    == std::path::Path::new(r"C:\Users\Example User\work\my project\.claude\agents")
                && !candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "codex"
                && candidate.scope == "project"
                && candidate.kind == ToolPathKind::ProjectInstructionFile
                && candidate.path
                    == std::path::Path::new("/Users/Example User/work/my project/AGENTS.md")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "opencode"
                && candidate.kind == ToolPathKind::McpConfig
                && candidate.path
                    == std::path::Path::new("/Users/Example User/.config/opencode/opencode.json")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "opencode"
                && candidate.kind == ToolPathKind::InstructionFile
                && candidate.path
                    == std::path::Path::new("/Users/Example User/.config/opencode/AGENTS.md")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "opencode"
                && candidate.kind == ToolPathKind::SkillDir
                && candidate.path
                    == std::path::Path::new("/Users/Example User/.config/opencode/skills")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "opencode"
                && candidate.scope == "project"
                && candidate.kind == ToolPathKind::ProjectInstructionFile
                && candidate.path
                    == std::path::Path::new("/Users/Example User/work/my project/AGENTS.md")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "vscode"
                && candidate.scope == "project"
                && candidate.kind == ToolPathKind::ProjectInstructionFile
                && candidate.path
                    == std::path::Path::new(
                        "/Users/Example User/work/my project/.github/copilot-instructions.md",
                    )
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "opencode"
                && candidate.scope == "project"
                && candidate.kind == ToolPathKind::ProjectMcpConfig
                && candidate.path
                    == std::path::Path::new("/Users/Example User/work/my project/opencode.json")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(macos_candidates.iter().any(|candidate| {
            candidate.tool == "opencode"
                && candidate.scope == "project"
                && candidate.kind == ToolPathKind::ProjectSkillDir
                && candidate.path
                    == std::path::Path::new("/Users/Example User/work/my project/.opencode/skills")
                && candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
        assert!(windows_candidates.iter().any(|candidate| {
            candidate.tool == "cursor"
                && candidate.scope == "project"
                && candidate.kind == ToolPathKind::ProjectInstructionDir
                && candidate.path
                    == std::path::Path::new(r"C:\Users\Example User\work\my project\.cursor\rules")
                && !candidate.verified
                && candidate.read_only
                && !candidate.write_supported
        }));
    }

    #[test]
    fn verifies_macos_candidates_from_filesystem_without_reading_contents() {
        let temp = tempdir().unwrap();
        let home = temp.path().join("home");
        let project = temp.path().join("workspace/my project");

        fs::create_dir_all(home.join(".codex")).unwrap();
        fs::create_dir_all(home.join(".gemini")).unwrap();
        fs::create_dir_all(home.join(".cursor/rules")).unwrap();
        fs::create_dir_all(project.join(".cursor")).unwrap();
        fs::write(home.join(".claude.json"), "SHOULD_NOT_BE_READ_SECRET").unwrap();
        fs::write(home.join(".codex/config.toml"), "SHOULD_NOT_BE_READ_SECRET").unwrap();
        fs::write(
            home.join(".gemini/settings.json"),
            "SHOULD_NOT_BE_READ_SECRET",
        )
        .unwrap();
        fs::write(home.join(".cursor/mcp.json"), "SHOULD_NOT_BE_READ_SECRET").unwrap();
        fs::write(
            project.join(".cursor/mcp.json"),
            "SHOULD_NOT_BE_READ_SECRET",
        )
        .unwrap();
        fs::write(project.join("AGENTS.md"), "SHOULD_NOT_BE_READ_SECRET").unwrap();

        let context = PlatformPathContext::macos(home.clone(), Some(project));
        let records = verify_tool_path_candidates(&context);
        let serialized = format!("{records:?}");

        assert!(!serialized.contains("SHOULD_NOT_BE_READ_SECRET"));
        assert!(records.iter().all(|record| {
            record.platform == "macos"
                && record.candidate_verified
                && record.read_only
                && !record.write_supported
                && (record.path.starts_with("~/") || record.path.starts_with("<project>/"))
        }));
        assert!(records.iter().any(|record| {
            record.tool == "codex"
                && record.kind == "mcp_config"
                && record.path == "~/.codex/config.toml"
                && record.exists
                && record.is_file
                && !record.is_dir
        }));
        assert!(records.iter().any(|record| {
            record.tool == "cursor"
                && record.kind == "instruction_dir"
                && record.path == "~/.cursor/rules"
                && record.exists
                && !record.is_file
                && record.is_dir
        }));
        assert!(records.iter().any(|record| {
            record.tool == "cursor"
                && record.scope == "project"
                && record.kind == "project_mcp_config"
                && record.path == "<project>/.cursor/mcp.json"
                && record.exists
                && record.is_file
        }));
        assert!(records.iter().any(|record| {
            record.tool == "claude"
                && record.kind == "skill_dir"
                && record.path == "~/.claude/skills"
                && !record.exists
                && !record.is_file
                && !record.is_dir
        }));
    }
}
