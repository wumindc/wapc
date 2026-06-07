//! Read-only AI coding tool registry detection.
//! @author codex

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;

use crate::{
    model::DetectedTool,
    platform_paths::{ToolPathCandidate, ToolPathKind, tool_registry_paths_for_home},
};

struct ToolDefinition {
    id: &'static str,
    display_name: &'static str,
    executable: &'static str,
}

const TOOL_DEFINITIONS: &[ToolDefinition] = &[
    ToolDefinition {
        id: "claude",
        display_name: "Claude Code",
        executable: "claude",
    },
    ToolDefinition {
        id: "codex",
        display_name: "Codex",
        executable: "codex",
    },
    ToolDefinition {
        id: "gemini",
        display_name: "Gemini CLI",
        executable: "gemini",
    },
    ToolDefinition {
        id: "opencode",
        display_name: "OpenCode",
        executable: "opencode",
    },
];

pub fn detect_tools(home: &Path) -> Vec<DetectedTool> {
    let detected_at = Utc::now().to_rfc3339();
    let path_candidates = tool_registry_paths_for_home(home);
    TOOL_DEFINITIONS
        .iter()
        .map(|definition| detect_tool(definition, &path_candidates, &detected_at))
        .collect()
}

fn detect_tool(
    definition: &ToolDefinition,
    path_candidates: &[ToolPathCandidate],
    detected_at: &str,
) -> DetectedTool {
    let config_dir = candidate_path(path_candidates, definition.id, ToolPathKind::ConfigDir);
    let data_dir = candidate_path(path_candidates, definition.id, ToolPathKind::DataDir);
    let executable_exists = executable_exists(definition.executable);
    let config_dir_exists = config_dir.as_ref().is_some_and(|path| path.exists());
    let data_dir_exists = data_dir.as_ref().is_some_and(|path| path.exists());
    let installed = config_dir_exists || data_dir_exists || executable_exists;
    let version = if executable_exists {
        detect_version(definition.executable)
    } else {
        None
    };

    DetectedTool {
        id: definition.id.to_string(),
        display_name: definition.display_name.to_string(),
        installed,
        version,
        config_dir: config_dir.map(display_path),
        data_dir: data_dir.map(display_path),
        config_dir_exists,
        data_dir_exists,
        last_detected_at: detected_at.to_string(),
    }
}

fn candidate_path(
    candidates: &[ToolPathCandidate],
    tool: &str,
    kind: ToolPathKind,
) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|candidate| candidate.tool == tool && candidate.kind == kind)
        .map(|candidate| candidate.path.clone())
}

fn executable_exists(executable: &str) -> bool {
    executable_path(executable).is_some()
}

fn detect_version(executable: &str) -> Option<String> {
    let mut child = Command::new(executable)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let started_at = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_string(&mut stdout);
                }
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                if !status.success() {
                    return Some("unknown".to_string());
                }
                let stdout = stdout.trim().to_string();
                let stderr = stderr.trim().to_string();
                let version = if stdout.is_empty() { stderr } else { stdout };
                return Some(if version.is_empty() {
                    "unknown".to_string()
                } else {
                    version
                });
            }
            Ok(None) if started_at.elapsed() > Duration::from_millis(800) => {
                let _ = child.kill();
                let _ = child.wait();
                return Some("unknown".to_string());
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => return Some("unknown".to_string()),
        }
    }
}

fn executable_path(executable: &str) -> Option<PathBuf> {
    let direct = Path::new(executable);
    if direct.components().count() > 1 && is_executable_file(direct) {
        return Some(direct.to_path_buf());
    }
    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .map(|dir| dir.join(executable))
        .find(|candidate| is_executable_file(candidate))
}

fn is_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

fn display_path(path: PathBuf) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn detects_codex_from_config_and_data_directories() {
        let home = tempdir().unwrap();
        fs::create_dir_all(home.path().join(".codex/sessions")).unwrap();
        fs::write(home.path().join(".codex/config.toml"), "").unwrap();

        let tools = detect_tools(home.path());
        let codex = tools.iter().find(|tool| tool.id == "codex").unwrap();

        assert!(codex.installed);
        assert!(codex.config_dir_exists);
        assert!(codex.data_dir_exists);
        assert_eq!(codex.display_name, "Codex");
        let expected_config_dir = home.path().join(".codex").display().to_string();
        assert_eq!(
            codex.config_dir.as_deref(),
            Some(expected_config_dir.as_str())
        );
    }

    #[test]
    fn registry_detector_uses_platform_path_resolver_for_tool_directories() {
        let source = std::fs::read_to_string("src/tool_registry.rs").unwrap();

        assert!(source.contains("tool_registry_paths_for_home"));
        assert!(source.contains("ToolPathKind::ConfigDir"));
        assert!(!source.contains("config_dir: \".codex\""));
        assert!(!source.contains("data_dir: \".gemini/tmp\""));
    }
}
