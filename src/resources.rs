//! Read-only canonical resource inventory detectors.
//! @author codex

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{
    model::{CanonicalResource, ResourceParseFailure},
    platform_paths::{PlatformPathContext, ToolPathKind, tool_path_candidates},
};

#[derive(Clone, Debug, Default)]
pub struct InventoryScan {
    pub resources: Vec<CanonicalResource>,
    pub failures: Vec<ResourceParseFailure>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InventoryAuditExpectation {
    pub kind_counts: BTreeMap<String, usize>,
    pub scope_counts: BTreeMap<String, usize>,
    pub failure_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InventoryAuditReport {
    pub actual_kind_counts: BTreeMap<String, usize>,
    pub actual_scope_counts: BTreeMap<String, usize>,
    pub actual_failure_count: usize,
    pub mismatches: Vec<String>,
    pub passed: bool,
}

#[derive(Clone)]
struct McpSource {
    tool: &'static str,
    path: PathBuf,
    format: McpFormat,
    root_key: &'static str,
    scope: &'static str,
    project_path: Option<PathBuf>,
}

#[derive(Clone, Copy)]
enum McpFormat {
    Json,
    Toml,
}

#[derive(Clone, Debug)]
struct McpPayload {
    transport: String,
    command: Option<String>,
    args: Vec<String>,
    url: Option<String>,
    enabled: Option<bool>,
    env_keys: Vec<String>,
    env_fingerprints: BTreeMap<String, Value>,
    header_keys: Vec<String>,
    header_fingerprints: BTreeMap<String, Value>,
    sensitive_field_fingerprints: BTreeMap<String, Value>,
    redacted: bool,
}

struct InstructionSource {
    tool: &'static str,
    path: PathBuf,
    dialect: &'static str,
    scope: &'static str,
    project_path: Option<PathBuf>,
}

pub fn scan_inventory(home: &Path) -> InventoryScan {
    scan_inventory_with_kinds(home, None)
}

pub fn scan_inventory_with_kinds(home: &Path, kinds: Option<&[&str]>) -> InventoryScan {
    scan_inventory_with_project_roots(home, &[], kinds)
}

pub fn scan_inventory_with_project_roots(
    home: &Path,
    project_roots: &[PathBuf],
    kinds: Option<&[&str]>,
) -> InventoryScan {
    let now = Utc::now().to_rfc3339();
    let mut scan = InventoryScan::default();
    let mut by_id = BTreeMap::<String, CanonicalResource>::new();
    let project_roots = project_roots
        .iter()
        .filter(|path| path.is_dir())
        .cloned()
        .collect::<Vec<_>>();

    if should_scan_kind(kinds, "mcp") {
        for source in mcp_sources(home)
            .into_iter()
            .chain(project_mcp_sources(&project_roots))
        {
            if !source.path.exists() {
                continue;
            }
            match read_mcp_source(&source, &now) {
                Ok(resources) => {
                    for resource in resources {
                        merge_resource(&mut by_id, resource);
                    }
                }
                Err(reason) => scan.failures.push(ResourceParseFailure {
                    path: source.path.display().to_string(),
                    tool: source.tool.to_string(),
                    kind: Some("mcp".to_string()),
                    reason,
                    seen_at: now.clone(),
                }),
            }
        }
    }

    if should_scan_kind(kinds, "skill") {
        for result in read_skill_resources(home, &project_roots, &now) {
            match result {
                Ok(resource) => merge_resource(&mut by_id, resource),
                Err(failure) => scan.failures.push(failure),
            }
        }
    }

    if should_scan_kind(kinds, "instruction") {
        for result in read_instruction_resources(home, &project_roots, &now) {
            match result {
                Ok(resource) => merge_resource(&mut by_id, resource),
                Err(failure) => scan.failures.push(failure),
            }
        }
    }

    if should_scan_kind(kinds, "plugin") {
        for result in read_plugin_resources(home, &now) {
            match result {
                Ok(resource) => merge_resource(&mut by_id, resource),
                Err(failure) => scan.failures.push(failure),
            }
        }
    }

    if should_scan_kind(kinds, "subagent") {
        for result in read_subagent_resources(home, &project_roots, &now) {
            match result {
                Ok(resource) => merge_resource(&mut by_id, resource),
                Err(failure) => scan.failures.push(failure),
            }
        }
    }

    scan.resources = by_id.into_values().collect();
    scan
}

fn should_scan_kind(kinds: Option<&[&str]>, kind: &str) -> bool {
    kinds.is_none_or(|values| values.is_empty() || values.contains(&kind))
}

pub fn audit_inventory_fixture(
    inventory: &InventoryScan,
    expectation: &InventoryAuditExpectation,
) -> InventoryAuditReport {
    let actual_kind_counts = count_by(&inventory.resources, |resource| &resource.kind);
    let actual_scope_counts = count_by(&inventory.resources, |resource| &resource.scope);
    let actual_failure_count = inventory.failures.len();
    let mut mismatches = Vec::new();
    compare_counts(
        "kind",
        &expectation.kind_counts,
        &actual_kind_counts,
        &mut mismatches,
    );
    compare_counts(
        "scope",
        &expectation.scope_counts,
        &actual_scope_counts,
        &mut mismatches,
    );
    if expectation.failure_count != actual_failure_count {
        mismatches.push(format!(
            "failure count expected {}, got {}",
            expectation.failure_count, actual_failure_count
        ));
    }
    InventoryAuditReport {
        actual_kind_counts,
        actual_scope_counts,
        actual_failure_count,
        passed: mismatches.is_empty(),
        mismatches,
    }
}

fn count_by<'a>(
    resources: &'a [CanonicalResource],
    field: impl Fn(&'a CanonicalResource) -> &'a str,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for resource in resources {
        *counts.entry(field(resource).to_string()).or_insert(0) += 1;
    }
    counts
}

fn compare_counts(
    label: &str,
    expected: &BTreeMap<String, usize>,
    actual: &BTreeMap<String, usize>,
    mismatches: &mut Vec<String>,
) {
    for key in expected.keys().chain(actual.keys()) {
        let expected_value = expected.get(key).copied().unwrap_or(0);
        let actual_value = actual.get(key).copied().unwrap_or(0);
        if expected_value != actual_value {
            mismatches.push(format!(
                "{label} {key} expected {expected_value}, got {actual_value}"
            ));
        }
    }
}

fn mcp_sources(home: &Path) -> Vec<McpSource> {
    let context = PlatformPathContext::current_home_compatible(home);
    tool_path_candidates(&context)
        .into_iter()
        .filter(|candidate| candidate.scope == "user" && candidate.kind == ToolPathKind::McpConfig)
        .filter_map(|candidate| {
            let (format, root_key) = mcp_source_config(candidate.tool)?;
            Some(McpSource {
                tool: candidate.tool,
                path: candidate.path,
                format,
                root_key,
                scope: "user",
                project_path: None,
            })
        })
        .collect()
}

fn mcp_source_config(tool: &str) -> Option<(McpFormat, &'static str)> {
    match tool {
        "claude" | "gemini" | "cursor" => Some((McpFormat::Json, "mcpServers")),
        "vscode" => Some((McpFormat::Json, "servers")),
        "opencode" => Some((McpFormat::Json, "mcp")),
        "codex" => Some((McpFormat::Toml, "mcp_servers")),
        _ => None,
    }
}

fn project_mcp_sources(project_roots: &[PathBuf]) -> Vec<McpSource> {
    project_roots
        .iter()
        .flat_map(|project| {
            let context = PlatformPathContext::current_home_compatible_with_project(
                project,
                Some(project.clone()),
            );
            tool_path_candidates(&context)
                .into_iter()
                .filter(|candidate| {
                    candidate.scope == "project" && candidate.kind == ToolPathKind::ProjectMcpConfig
                })
                .filter_map(|candidate| {
                    let (format, root_key) = mcp_source_config(candidate.tool)?;
                    Some(McpSource {
                        tool: candidate.tool,
                        path: candidate.path,
                        format,
                        root_key,
                        scope: "project",
                        project_path: Some(project.clone()),
                    })
                })
        })
        .collect()
}

fn read_mcp_source(source: &McpSource, now: &str) -> Result<Vec<CanonicalResource>, String> {
    let content = std::fs::read_to_string(&source.path).map_err(|err| err.to_string())?;
    let servers = match source.format {
        McpFormat::Json => json_mcp_servers(&content, source.root_key)?,
        McpFormat::Toml => toml_mcp_servers(&content, source.root_key)?,
    };
    Ok(servers
        .into_iter()
        .filter_map(|(name, value)| mcp_resource(source, &name, &value, now))
        .collect())
}

fn json_mcp_servers(content: &str, root_key: &str) -> Result<Vec<(String, Value)>, String> {
    let value: Value = serde_json::from_str(content).map_err(|err| err.to_string())?;
    let Some(servers) = value.get(root_key).and_then(Value::as_object) else {
        return Ok(Vec::new());
    };
    Ok(servers
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect())
}

fn toml_mcp_servers(content: &str, root_key: &str) -> Result<Vec<(String, Value)>, String> {
    let value: toml::Value = toml::from_str(content).map_err(|err| err.to_string())?;
    let Some(servers) = value.get(root_key).and_then(toml::Value::as_table) else {
        return Ok(Vec::new());
    };
    Ok(servers
        .iter()
        .filter_map(|(name, value)| {
            serde_json::to_value(value)
                .ok()
                .map(|json| (name.clone(), json))
        })
        .collect())
}

fn mcp_resource(
    source: &McpSource,
    name: &str,
    value: &Value,
    now: &str,
) -> Option<CanonicalResource> {
    mcp_resource_with_provider(source, name, value, now, None)
}

fn mcp_resource_with_provider(
    source: &McpSource,
    name: &str,
    value: &Value,
    now: &str,
    provided_by_plugin: Option<&str>,
) -> Option<CanonicalResource> {
    let payload = mcp_payload(value)?;
    let payload_json = canonical_payload_json(&payload);
    let scope_identity = source
        .project_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| source.scope.to_string());
    let id = format!(
        "mcp:{}",
        sha256_8(&format!("mcp\n{name}\n{}\n{payload_json}", scope_identity))
    );
    let enabled_in = if payload.enabled == Some(false) {
        Vec::new()
    } else {
        vec![source.tool.to_string()]
    };
    Some(CanonicalResource {
        id,
        kind: "mcp".to_string(),
        name: name.to_string(),
        scope: source.scope.to_string(),
        origin_tool: source.tool.to_string(),
        origin_path: source.path.display().to_string(),
        origin_locator: Some(format!("{}.{}", source.root_key, name)),
        enabled_in,
        confidence: if payload.command.is_some() || payload.url.is_some() {
            1.0
        } else {
            0.6
        },
        redacted: payload.redacted,
        payload_json,
        provided_by_plugin: provided_by_plugin.map(ToOwned::to_owned),
        last_seen: now.to_string(),
    })
}

fn mcp_payload(value: &Value) -> Option<McpPayload> {
    let object = value.as_object()?;
    let (command, mut args) = command_and_args(object);
    let url = object
        .get("url")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let transport = object
        .get("type")
        .and_then(Value::as_str)
        .map(normalize_transport)
        .unwrap_or_else(|| {
            if url.is_some() {
                "http".to_string()
            } else {
                "stdio".to_string()
            }
        });
    if args.is_empty() {
        args = object
            .get("args")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(redact_arg)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }
    let enabled = object.get("enabled").and_then(Value::as_bool);
    let (env_keys, env_fingerprints, env_redacted) =
        redact_env(object.get("env").or_else(|| object.get("environment")));
    let (header_keys, header_fingerprints, header_redacted) = redact_headers(object.get("headers"));
    let (sensitive_field_fingerprints, sensitive_fields_redacted) = redact_sensitive_fields(object);
    let args_redacted = args.iter().any(|arg| arg.contains("<redacted:"));
    let command_redacted = command.is_some();
    Some(McpPayload {
        transport,
        command,
        args,
        url,
        enabled,
        env_keys,
        env_fingerprints,
        header_keys,
        header_fingerprints,
        sensitive_field_fingerprints,
        redacted: env_redacted
            || header_redacted
            || sensitive_fields_redacted
            || args_redacted
            || command_redacted,
    })
}

fn command_and_args(object: &serde_json::Map<String, Value>) -> (Option<String>, Vec<String>) {
    let Some(command_value) = object.get("command") else {
        return (None, Vec::new());
    };
    if let Some(command) = command_value.as_str() {
        return (Some(command.to_string()), Vec::new());
    }
    let Some(values) = command_value.as_array() else {
        return (None, Vec::new());
    };
    let mut parts = values.iter().filter_map(Value::as_str);
    let command = parts.next().map(ToOwned::to_owned);
    let args = parts.map(redact_arg).collect::<Vec<_>>();
    (command, args)
}

fn normalize_transport(value: &str) -> String {
    match value {
        "local" => "stdio".to_string(),
        "remote" => "http".to_string(),
        other => other.to_string(),
    }
}

fn canonical_payload_json(payload: &McpPayload) -> String {
    json!({
        "transport": payload.transport,
        "command": payload.command.as_ref().map(|command| command_metadata(command)),
        "args": payload.args,
        "url": payload.url,
        "enabled": payload.enabled,
        "env_keys": payload.env_keys,
        "env_fingerprints": payload.env_fingerprints,
        "header_keys": payload.header_keys,
        "header_fingerprints": payload.header_fingerprints,
        "sensitive_field_fingerprints": payload.sensitive_field_fingerprints,
    })
    .to_string()
}

fn command_metadata(command: &str) -> Value {
    let name = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(command);
    let display_name = if looks_sensitive(name) {
        format!("<redacted:{}>", sha256_8(name))
    } else {
        name.to_string()
    };
    json!({
        "name": display_name,
        "len": command.len(),
        "sha256_8": sha256_8(command),
    })
}

fn read_skill_resources(
    home: &Path,
    project_roots: &[PathBuf],
    now: &str,
) -> Vec<Result<CanonicalResource, ResourceParseFailure>> {
    let mut results = Vec::new();
    for tool in ["claude", "opencode"] {
        for root in user_tool_roots(home, tool, ToolPathKind::SkillDir) {
            results.extend(read_skill_resources_in_root(tool, root, "user", now));
        }
    }
    for project in project_roots {
        for tool in ["claude", "opencode"] {
            for root in project_tool_roots(project, tool, ToolPathKind::ProjectSkillDir) {
                results.extend(read_skill_resources_in_root(tool, root, "project", now));
            }
        }
    }
    results
}

fn read_skill_resources_in_root(
    tool: &'static str,
    root: PathBuf,
    scope: &'static str,
    now: &str,
) -> Vec<Result<CanonicalResource, ResourceParseFailure>> {
    if !root.exists() {
        return Vec::new();
    }
    let mut results = Vec::new();
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(err) => {
            return vec![Err(parse_failure(
                &root,
                tool,
                "skill",
                err.to_string(),
                now,
            ))];
        }
    };
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if !path.is_dir() || !path.join("SKILL.md").exists() {
                    continue;
                }
                results.push(skill_resource(tool, &path, scope, now));
            }
            Err(err) => results.push(Err(parse_failure(
                &root,
                tool,
                "skill",
                err.to_string(),
                now,
            ))),
        }
    }
    results
}

fn skill_resource(
    tool: &'static str,
    skill_dir: &Path,
    scope: &'static str,
    now: &str,
) -> Result<CanonicalResource, ResourceParseFailure> {
    let fallback_name = skill_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
    let manifest_path = skill_dir.join("SKILL.md");
    let manifest_content = fs::read_to_string(&manifest_path)
        .map_err(|err| parse_failure(&manifest_path, tool, "skill", err.to_string(), now))?;
    let frontmatter = frontmatter_map(&manifest_content);
    let declared_name = frontmatter
        .get("name")
        .filter(|value| !value.is_empty())
        .cloned();
    let name = if tool == "opencode" {
        declared_name.clone().unwrap_or(fallback_name)
    } else {
        fallback_name
    };
    let mut files = Vec::new();
    let mut aggregate = Vec::new();
    for entry in WalkDir::new(skill_dir).sort_by_file_name() {
        let entry =
            entry.map_err(|err| parse_failure(skill_dir, tool, "skill", err.to_string(), now))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let bytes = fs::read(path)
            .map_err(|err| parse_failure(path, tool, "skill", err.to_string(), now))?;
        let relative_path = path
            .strip_prefix(skill_dir)
            .unwrap_or(path)
            .display()
            .to_string();
        let content_hash = sha256_hex(&bytes);
        aggregate.extend_from_slice(relative_path.as_bytes());
        aggregate.extend_from_slice(content_hash.as_bytes());
        files.push(json!({
            "path": relative_path,
            "byte_count": bytes.len(),
            "content_hash": content_hash,
        }));
    }
    let payload_json = json!({
        "manifest": "SKILL.md",
        "frontmatter_keys": sorted_keys(&frontmatter),
        "frontmatter_metadata": skill_frontmatter_metadata(tool, &frontmatter),
        "file_count": files.len(),
        "byte_count": files.iter().filter_map(|file| file.get("byte_count").and_then(Value::as_u64)).sum::<u64>(),
        "content_hash": sha256_hex(&aggregate),
        "files": files,
    })
    .to_string();
    let confidence = skill_confidence(tool, &name, &declared_name);
    Ok(CanonicalResource {
        id: format!(
            "skill:{}",
            sha256_8(&format!(
                "skill\n{name}\n{scope}\n{}\n{payload_json}",
                skill_dir.display()
            ))
        ),
        kind: "skill".to_string(),
        name,
        scope: scope.to_string(),
        origin_tool: tool.to_string(),
        origin_path: skill_dir.display().to_string(),
        origin_locator: Some("SKILL.md".to_string()),
        enabled_in: vec![tool.to_string()],
        confidence,
        redacted: true,
        payload_json,
        provided_by_plugin: None,
        last_seen: now.to_string(),
    })
}

fn skill_confidence(tool: &str, name: &str, declared_name: &Option<String>) -> f64 {
    if tool != "opencode" {
        return 1.0;
    }
    match declared_name {
        Some(value) if value == name => 1.0,
        Some(_) => 0.7,
        None => 0.6,
    }
}

fn skill_frontmatter_metadata(tool: &str, frontmatter: &BTreeMap<String, String>) -> Value {
    if frontmatter.is_empty() {
        return json!({});
    }
    let description_fingerprint = frontmatter.get("description").map(|description| {
        json!({
            "len": description.len(),
            "sha256_8": sha256_8(description),
        })
    });
    match tool {
        "opencode" => json!({
            "schema": "opencode-skill-frontmatter-v1",
            "declared_name": frontmatter.get("name"),
            "description_fingerprint": description_fingerprint,
            "license": frontmatter.get("license"),
            "compatibility": frontmatter.get("compatibility"),
            "metadata_present": frontmatter.contains_key("metadata"),
        }),
        _ => json!({
            "schema": "skill-frontmatter-v1",
            "description_fingerprint": description_fingerprint,
        }),
    }
}

fn read_instruction_resources(
    home: &Path,
    project_roots: &[PathBuf],
    now: &str,
) -> Vec<Result<CanonicalResource, ResourceParseFailure>> {
    let (mut sources, mut failures) = match user_instruction_sources(home, now) {
        Ok(sources) => (sources, Vec::new()),
        Err(failure) => (Vec::new(), vec![Err(failure)]),
    };
    for project in project_roots {
        match project_instruction_sources(project, now) {
            Ok(project_sources) => sources.extend(project_sources),
            Err(failure) => failures.push(Err(failure)),
        }
    }
    let mut results = sources
        .into_iter()
        .filter(|source| source.path.exists())
        .map(|source| instruction_resource(&source, now))
        .collect::<Vec<_>>();
    results.extend(failures);
    results
}

fn user_instruction_sources(
    home: &Path,
    now: &str,
) -> Result<Vec<InstructionSource>, ResourceParseFailure> {
    let context = PlatformPathContext::current_home_compatible(home);
    let mut sources = Vec::new();
    for candidate in tool_path_candidates(&context)
        .into_iter()
        .filter(|candidate| candidate.scope == "user")
    {
        match candidate.kind {
            ToolPathKind::InstructionFile => {
                if let Some(dialect) = instruction_dialect(candidate.tool, &candidate.path) {
                    sources.push(InstructionSource {
                        tool: candidate.tool,
                        path: candidate.path,
                        dialect,
                        scope: "user",
                        project_path: None,
                    });
                }
            }
            ToolPathKind::InstructionDir if candidate.tool == "cursor" => {
                let cursor_rules = candidate.path;
                if !cursor_rules.exists() {
                    continue;
                }
                let entries = fs::read_dir(&cursor_rules).map_err(|err| {
                    parse_failure(&cursor_rules, "cursor", "instruction", err.to_string(), now)
                })?;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("mdc") {
                        sources.push(InstructionSource {
                            tool: "cursor",
                            path,
                            dialect: "cursor-rules",
                            scope: "user",
                            project_path: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Ok(sources)
}

fn instruction_dialect(tool: &str, path: &Path) -> Option<&'static str> {
    match tool {
        "claude" => Some("claude-md"),
        "codex" => Some("agents-md"),
        "opencode" => Some("agents-md"),
        "gemini" => Some("gemini-md"),
        "vscode" => Some("copilot"),
        "cursor" if path.file_name().and_then(|name| name.to_str()) == Some(".cursorrules") => {
            Some("cursor-rules-legacy")
        }
        _ => None,
    }
}

fn project_instruction_sources(
    project: &Path,
    now: &str,
) -> Result<Vec<InstructionSource>, ResourceParseFailure> {
    let mut sources = Vec::new();
    let project_path = project.to_path_buf();
    let context = PlatformPathContext::current_home_compatible_with_project(
        project,
        Some(project_path.clone()),
    );
    for candidate in tool_path_candidates(&context)
        .into_iter()
        .filter(|candidate| candidate.scope == "project")
    {
        match candidate.kind {
            ToolPathKind::ProjectInstructionFile => {
                if let Some(dialect) = instruction_dialect(candidate.tool, &candidate.path) {
                    sources.push(InstructionSource {
                        tool: candidate.tool,
                        path: candidate.path,
                        dialect,
                        scope: "project",
                        project_path: Some(project_path.clone()),
                    });
                }
            }
            ToolPathKind::ProjectInstructionDir if candidate.tool == "cursor" => {
                let cursor_rules = candidate.path;
                if !cursor_rules.exists() {
                    continue;
                }
                let entries = fs::read_dir(&cursor_rules).map_err(|err| {
                    parse_failure(&cursor_rules, "cursor", "instruction", err.to_string(), now)
                })?;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("mdc") {
                        sources.push(InstructionSource {
                            tool: "cursor",
                            path,
                            dialect: "cursor-rules",
                            scope: "project",
                            project_path: Some(project_path.clone()),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Ok(sources)
}

fn instruction_resource(
    source: &InstructionSource,
    now: &str,
) -> Result<CanonicalResource, ResourceParseFailure> {
    let content = fs::read_to_string(&source.path).map_err(|err| {
        parse_failure(
            &source.path,
            source.tool,
            "instruction",
            err.to_string(),
            now,
        )
    })?;
    let name = source
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("instruction")
        .to_string();
    let headings = markdown_headings(&content);
    let paragraph_hashes = paragraph_hashes(&content);
    let payload_json = json!({
        "dialect": source.dialect,
        "project_path": source.project_path.as_ref().map(|path| path.display().to_string()),
        "byte_count": content.len(),
        "content_hash": sha256_hex(content.as_bytes()),
        "headings": headings,
        "paragraph_hashes": paragraph_hashes,
        "frontmatter_keys": frontmatter_keys(&content),
        "frontmatter_metadata": frontmatter_metadata(&content, source.dialect),
    })
    .to_string();
    Ok(CanonicalResource {
        id: format!(
            "instruction:{}",
            sha256_8(&format!(
                "instruction\n{}\n{}",
                sha256_hex(
                    format!(
                        "{}\n{}\n{}",
                        source.scope,
                        source
                            .project_path
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_default(),
                        sha256_hex(content.as_bytes())
                    )
                    .as_bytes()
                ),
                source.dialect
            ))
        ),
        kind: "instruction".to_string(),
        name,
        scope: source.scope.to_string(),
        origin_tool: source.tool.to_string(),
        origin_path: source.path.display().to_string(),
        origin_locator: None,
        enabled_in: vec![source.tool.to_string()],
        confidence: if headings.is_empty() { 0.8 } else { 1.0 },
        redacted: true,
        payload_json,
        provided_by_plugin: None,
        last_seen: now.to_string(),
    })
}

fn markdown_headings(content: &str) -> Vec<Value> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let level = trimmed.chars().take_while(|ch| *ch == '#').count();
            if !(1..=6).contains(&level) || !trimmed[level..].starts_with(' ') {
                return None;
            }
            Some(json!({
                "level": level,
                "text": trimmed[level..].trim(),
            }))
        })
        .collect()
}

fn paragraph_hashes(content: &str) -> Vec<Value> {
    content
        .split("\n\n")
        .filter_map(|paragraph| {
            let normalized = paragraph
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#') && *line != "---")
                .collect::<Vec<_>>()
                .join("\n");
            if normalized.is_empty() || normalized.contains(':') && normalized.lines().count() == 1
            {
                return None;
            }
            Some(json!({
                "byte_count": normalized.len(),
                "sha256_8": sha256_8(&normalized),
            }))
        })
        .collect()
}

fn frontmatter_keys(content: &str) -> Vec<String> {
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return Vec::new();
    }
    let mut keys = Vec::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some((key, _)) = line.split_once(':') {
            keys.push(key.trim().to_string());
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

fn frontmatter_metadata(content: &str, dialect: &str) -> Value {
    let frontmatter = frontmatter_map(content);
    if frontmatter.is_empty() {
        return json!({});
    }
    match dialect {
        "cursor-rules" => cursor_rule_frontmatter_metadata(&frontmatter),
        _ => json!({
            "schema": "markdown-frontmatter-v1",
            "keys": sorted_keys(&frontmatter),
        }),
    }
}

fn cursor_rule_frontmatter_metadata(frontmatter: &BTreeMap<String, String>) -> Value {
    let description_fingerprint = frontmatter.get("description").map(|description| {
        json!({
            "len": description.len(),
            "sha256_8": sha256_8(description),
        })
    });
    let globs = frontmatter
        .get("globs")
        .map(|value| parse_list_value(Some(value)))
        .unwrap_or_default();
    let always_apply = frontmatter
        .get("alwaysApply")
        .or_else(|| frontmatter.get("always_apply"))
        .and_then(|value| parse_bool(value));
    json!({
        "schema": "cursor-rules-frontmatter-v1",
        "keys": sorted_keys(frontmatter),
        "description_fingerprint": description_fingerprint,
        "globs": globs,
        "always_apply": always_apply,
    })
}

pub fn render_cursor_rule_document(
    description: &str,
    globs: &[String],
    always_apply: bool,
    body: &str,
) -> Result<String, String> {
    let description = description.trim();
    if description.is_empty() {
        return Err("cursor rule description is required".to_string());
    }
    if globs.is_empty() {
        return Err("cursor rule globs are required".to_string());
    }
    let mut normalized_globs = Vec::new();
    for glob in globs {
        let glob = glob.trim();
        if glob.is_empty() || glob.contains('\n') || glob.contains('\r') {
            return Err("cursor rule globs must be non-empty single-line values".to_string());
        }
        normalized_globs.push(glob.to_string());
    }
    let body = body.trim();
    if body.is_empty() {
        return Err("cursor rule body is required".to_string());
    }
    let globs = normalized_globs
        .iter()
        .map(|glob| quoted_frontmatter_value(glob))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "---\ndescription: {}\nglobs: [{}]\nalwaysApply: {}\n---\n\n{}\n",
        quoted_frontmatter_value(description),
        globs,
        if always_apply { "true" } else { "false" },
        body
    ))
}

fn quoted_frontmatter_value(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn sorted_keys(values: &BTreeMap<String, String>) -> Vec<String> {
    let mut keys = values.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    keys
}

fn read_plugin_resources(
    home: &Path,
    now: &str,
) -> Vec<Result<CanonicalResource, ResourceParseFailure>> {
    let mut results = Vec::new();
    for root in user_tool_roots(home, "claude", ToolPathKind::PluginDir) {
        results.extend(read_plugin_resources_in_root(root, now));
    }
    results
}

fn read_plugin_resources_in_root(
    root: PathBuf,
    now: &str,
) -> Vec<Result<CanonicalResource, ResourceParseFailure>> {
    if !root.exists() {
        return Vec::new();
    }
    let mut results = Vec::new();
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(err) => {
            return vec![Err(parse_failure(
                &root,
                "claude",
                "plugin",
                err.to_string(),
                now,
            ))];
        }
    };
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_dir() {
                    match plugin_resource("claude", &path, now) {
                        Ok(plugin) => {
                            let plugin_name = plugin.name.clone();
                            results.push(Ok(plugin));
                            results.extend(plugin_mcp_resources(
                                "claude",
                                &path,
                                &plugin_name,
                                now,
                            ));
                            results.extend(plugin_subagent_resources(
                                "claude",
                                &path,
                                &plugin_name,
                                now,
                            ));
                        }
                        Err(failure) => results.push(Err(failure)),
                    }
                }
            }
            Err(err) => results.push(Err(parse_failure(
                &root,
                "claude",
                "plugin",
                err.to_string(),
                now,
            ))),
        }
    }
    results
}

fn plugin_mcp_resources(
    tool: &'static str,
    plugin_dir: &Path,
    plugin_name: &str,
    now: &str,
) -> Vec<Result<CanonicalResource, ResourceParseFailure>> {
    let root = plugin_dir.join("mcp");
    if !root.exists() {
        return Vec::new();
    }
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(err) => return vec![Err(parse_failure(&root, tool, "mcp", err.to_string(), now))],
    };
    let mut results = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                    continue;
                }
                let source = McpSource {
                    tool,
                    path: path.clone(),
                    format: McpFormat::Json,
                    root_key: "mcpServers",
                    scope: "user",
                    project_path: None,
                };
                match read_plugin_mcp_source(&source, plugin_name, now) {
                    Ok(resources) => results.extend(resources.into_iter().map(Ok)),
                    Err(reason) => {
                        results.push(Err(parse_failure(&path, tool, "mcp", reason, now)))
                    }
                }
            }
            Err(err) => results.push(Err(parse_failure(&root, tool, "mcp", err.to_string(), now))),
        }
    }
    results
}

fn read_plugin_mcp_source(
    source: &McpSource,
    plugin_name: &str,
    now: &str,
) -> Result<Vec<CanonicalResource>, String> {
    let content = std::fs::read_to_string(&source.path).map_err(|err| err.to_string())?;
    let servers = json_mcp_servers(&content, source.root_key)?;
    Ok(servers
        .into_iter()
        .filter_map(|(name, value)| {
            mcp_resource_with_provider(source, &name, &value, now, Some(plugin_name))
        })
        .collect())
}

fn plugin_subagent_resources(
    tool: &'static str,
    plugin_dir: &Path,
    plugin_name: &str,
    now: &str,
) -> Vec<Result<CanonicalResource, ResourceParseFailure>> {
    let root = plugin_dir.join("agents");
    if !root.exists() {
        return Vec::new();
    }
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(err) => {
            return vec![Err(parse_failure(
                &root,
                tool,
                "subagent",
                err.to_string(),
                now,
            ))];
        }
    };
    let mut results = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                    results.push(subagent_resource_with_provider(
                        tool,
                        &path,
                        "user",
                        now,
                        Some(plugin_name),
                    ));
                }
            }
            Err(err) => results.push(Err(parse_failure(
                &root,
                tool,
                "subagent",
                err.to_string(),
                now,
            ))),
        }
    }
    results
}

fn plugin_resource(
    tool: &'static str,
    plugin_dir: &Path,
    now: &str,
) -> Result<CanonicalResource, ResourceParseFailure> {
    let fallback_name = plugin_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
    let manifest_path = plugin_dir.join("plugin.json");
    let manifest = if manifest_path.exists() {
        let content = fs::read_to_string(&manifest_path)
            .map_err(|err| parse_failure(&manifest_path, tool, "plugin", err.to_string(), now))?;
        serde_json::from_str::<Value>(&content)
            .map_err(|err| parse_failure(&manifest_path, tool, "plugin", err.to_string(), now))?
    } else {
        json!({})
    };
    let name = manifest
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&fallback_name)
        .to_string();
    let (files, byte_count, content_hash) = file_inventory(plugin_dir, tool, "plugin", now)?;
    let payload_json = json!({
        "manifest": if manifest_path.exists() { Some("plugin.json") } else { None::<&str> },
        "version": manifest.get("version").and_then(Value::as_str),
        "marketplace": manifest.get("marketplace").and_then(Value::as_str).or_else(|| manifest.get("source").and_then(Value::as_str)),
        "component_counts": component_counts(plugin_dir),
        "file_count": files.len(),
        "byte_count": byte_count,
        "content_hash": content_hash,
        "files": files,
    })
    .to_string();
    Ok(CanonicalResource {
        id: format!(
            "plugin:{}",
            sha256_8(&format!("plugin\n{name}\n{payload_json}"))
        ),
        kind: "plugin".to_string(),
        name,
        scope: "user".to_string(),
        origin_tool: tool.to_string(),
        origin_path: plugin_dir.display().to_string(),
        origin_locator: manifest_path.exists().then(|| "plugin.json".to_string()),
        enabled_in: vec![tool.to_string()],
        confidence: if manifest_path.exists() { 1.0 } else { 0.75 },
        redacted: true,
        payload_json,
        provided_by_plugin: None,
        last_seen: now.to_string(),
    })
}

fn read_subagent_resources(
    home: &Path,
    project_roots: &[PathBuf],
    now: &str,
) -> Vec<Result<CanonicalResource, ResourceParseFailure>> {
    let mut results = Vec::new();
    for root in user_tool_roots(home, "claude", ToolPathKind::SubagentDir) {
        results.extend(read_subagent_resources_in_root("claude", root, "user", now));
    }
    for project in project_roots {
        for root in project_tool_roots(project, "claude", ToolPathKind::ProjectSubagentDir) {
            results.extend(read_subagent_resources_in_root(
                "claude", root, "project", now,
            ));
        }
    }
    results
}

fn user_tool_roots(home: &Path, tool: &str, kind: ToolPathKind) -> Vec<PathBuf> {
    let context = PlatformPathContext::current_home_compatible(home);
    tool_path_candidates(&context)
        .into_iter()
        .filter(|candidate| {
            candidate.scope == "user" && candidate.tool == tool && candidate.kind == kind
        })
        .map(|candidate| candidate.path)
        .collect()
}

fn project_tool_roots(project: &Path, tool: &str, kind: ToolPathKind) -> Vec<PathBuf> {
    let project = project.to_path_buf();
    let context =
        PlatformPathContext::current_home_compatible_with_project(&project, Some(project.clone()));
    tool_path_candidates(&context)
        .into_iter()
        .filter(|candidate| {
            candidate.scope == "project" && candidate.tool == tool && candidate.kind == kind
        })
        .map(|candidate| candidate.path)
        .collect()
}

fn read_subagent_resources_in_root(
    tool: &'static str,
    root: PathBuf,
    scope: &'static str,
    now: &str,
) -> Vec<Result<CanonicalResource, ResourceParseFailure>> {
    if !root.exists() {
        return Vec::new();
    }
    let mut results = Vec::new();
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(err) => {
            return vec![Err(parse_failure(
                &root,
                tool,
                "subagent",
                err.to_string(),
                now,
            ))];
        }
    };
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                    results.push(subagent_resource(tool, &path, scope, now));
                }
            }
            Err(err) => results.push(Err(parse_failure(
                &root,
                tool,
                "subagent",
                err.to_string(),
                now,
            ))),
        }
    }
    results
}

fn subagent_resource(
    tool: &'static str,
    path: &Path,
    scope: &'static str,
    now: &str,
) -> Result<CanonicalResource, ResourceParseFailure> {
    subagent_resource_with_provider(tool, path, scope, now, None)
}

fn subagent_resource_with_provider(
    tool: &'static str,
    path: &Path,
    scope: &'static str,
    now: &str,
    provided_by_plugin: Option<&str>,
) -> Result<CanonicalResource, ResourceParseFailure> {
    let content = fs::read_to_string(path)
        .map_err(|err| parse_failure(path, tool, "subagent", err.to_string(), now))?;
    let frontmatter = frontmatter_map(&content);
    let fallback_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("subagent")
        .to_string();
    let name = frontmatter
        .get("name")
        .filter(|value| !value.is_empty())
        .cloned()
        .unwrap_or(fallback_name);
    let body = markdown_body_without_frontmatter(&content);
    let mut frontmatter_keys = frontmatter.keys().cloned().collect::<Vec<_>>();
    frontmatter_keys.sort();
    let payload_json = json!({
        "model": frontmatter.get("model"),
        "allowed_tools": parse_list_value(frontmatter.get("allowed_tools").map(String::as_str)),
        "frontmatter_keys": frontmatter_keys,
        "byte_count": content.len(),
        "content_hash": sha256_hex(content.as_bytes()),
        "headings": markdown_headings(body),
        "body_hashes": paragraph_hashes(body),
    })
    .to_string();
    Ok(CanonicalResource {
        id: format!(
            "subagent:{}",
            sha256_8(&format!(
                "subagent\n{}\n{}",
                name,
                sha256_hex(format!("{scope}\n{}\n{content}", path.display()).as_bytes())
            ))
        ),
        kind: "subagent".to_string(),
        name,
        scope: scope.to_string(),
        origin_tool: tool.to_string(),
        origin_path: path.display().to_string(),
        origin_locator: Some("frontmatter".to_string()),
        enabled_in: vec![tool.to_string()],
        confidence: if frontmatter.contains_key("name") {
            1.0
        } else {
            0.75
        },
        redacted: true,
        payload_json,
        provided_by_plugin: provided_by_plugin.map(ToOwned::to_owned),
        last_seen: now.to_string(),
    })
}

fn file_inventory(
    root: &Path,
    tool: &str,
    kind: &str,
    now: &str,
) -> Result<(Vec<Value>, u64, String), ResourceParseFailure> {
    let mut files = Vec::new();
    let mut byte_count = 0_u64;
    let mut aggregate = Vec::new();
    for entry in WalkDir::new(root).sort_by_file_name() {
        let entry = entry.map_err(|err| parse_failure(root, tool, kind, err.to_string(), now))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let bytes =
            fs::read(path).map_err(|err| parse_failure(path, tool, kind, err.to_string(), now))?;
        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        let content_hash = sha256_hex(&bytes);
        byte_count += bytes.len() as u64;
        aggregate.extend_from_slice(relative_path.as_bytes());
        aggregate.extend_from_slice(content_hash.as_bytes());
        files.push(json!({
            "path_hash": sha256_8(&relative_path),
            "extension": path.extension().and_then(|extension| extension.to_str()),
            "depth": relative_path.split('/').count(),
            "byte_count": bytes.len(),
            "content_hash": content_hash,
        }));
    }
    Ok((files, byte_count, sha256_hex(&aggregate)))
}

fn component_counts(root: &Path) -> Value {
    json!({
        "commands": count_files(root.join("commands")),
        "agents": count_files(root.join("agents")),
        "hooks": count_files(root.join("hooks")),
        "mcp": count_files(root.join("mcp")),
        "skills": count_files(root.join("skills")),
    })
}

fn count_files(path: PathBuf) -> u64 {
    if !path.exists() {
        return 0;
    }
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .count() as u64
}

fn frontmatter_map(content: &str) -> BTreeMap<String, String> {
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return BTreeMap::new();
    }
    let mut values = BTreeMap::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            values.insert(
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            );
        }
    }
    values
}

fn markdown_body_without_frontmatter(content: &str) -> &str {
    let Some(rest) = content.strip_prefix("---\n") else {
        return content;
    };
    rest.split_once("\n---\n")
        .map(|(_, body)| body)
        .unwrap_or(content)
}

fn parse_list_value(value: Option<&str>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    let trimmed = value.trim().trim_start_matches('[').trim_end_matches(']');
    trimmed
        .split(',')
        .map(|item| item.trim().trim_matches('"').trim_matches('\''))
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn redact_env(value: Option<&Value>) -> (Vec<String>, BTreeMap<String, Value>, bool) {
    let Some(env) = value.and_then(Value::as_object) else {
        return (Vec::new(), BTreeMap::new(), false);
    };
    let mut keys = Vec::new();
    let mut fingerprints = BTreeMap::new();
    for (key, value) in env {
        keys.push(key.clone());
        let raw = value.as_str().unwrap_or("");
        fingerprints.insert(
            key.clone(),
            json!({
                "len": raw.len(),
                "prefix": safe_prefix(raw),
                "sha256_8": sha256_8(raw),
            }),
        );
    }
    keys.sort();
    (keys, fingerprints, true)
}

fn redact_headers(value: Option<&Value>) -> (Vec<String>, BTreeMap<String, Value>, bool) {
    let Some(headers) = value.and_then(Value::as_object) else {
        return (Vec::new(), BTreeMap::new(), false);
    };
    let mut keys = Vec::new();
    let mut fingerprints = BTreeMap::new();
    for (key, value) in headers {
        keys.push(key.clone());
        let raw = value.as_str().unwrap_or("");
        if looks_sensitive_key(key) || looks_sensitive(raw) {
            fingerprints.insert(key.clone(), secret_fingerprint(raw));
        }
    }
    keys.sort();
    let redacted = !fingerprints.is_empty();
    (keys, fingerprints, redacted)
}

fn redact_sensitive_fields(
    object: &serde_json::Map<String, Value>,
) -> (BTreeMap<String, Value>, bool) {
    let mut fingerprints = BTreeMap::new();
    for (key, value) in object {
        if matches!(key.as_str(), "env" | "headers") || !looks_sensitive_key(key) {
            continue;
        }
        let raw = value.as_str().unwrap_or("");
        fingerprints.insert(key.clone(), secret_fingerprint(raw));
    }
    let redacted = !fingerprints.is_empty();
    (fingerprints, redacted)
}

fn secret_fingerprint(raw: &str) -> Value {
    json!({
        "len": raw.len(),
        "sha256_8": sha256_8(raw),
    })
}

fn redact_arg(value: &str) -> String {
    if looks_sensitive(value) {
        format!("<redacted:{}>", sha256_8(value))
    } else {
        value.to_string()
    }
}

fn looks_sensitive_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    normalized.contains("apikey")
        || normalized.contains("authorization")
        || normalized.ends_with("token")
        || normalized.contains("accesstoken")
        || normalized.contains("secret")
}

fn looks_sensitive(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("api_key=")
        || lower.contains("apikey=")
        || lower.contains("token=")
        || lower.contains("authorization:")
        || lower.starts_with("bearer ")
        || value.starts_with("sk-")
        || value.starts_with("ghp_")
        || value.starts_with("github_pat_")
        || value.starts_with("glpat-")
        || value.starts_with("xoxb-")
        || value.starts_with("xoxp-")
        || looks_high_entropy_secret(value)
}

fn looks_high_entropy_secret(value: &str) -> bool {
    let compact = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        .collect::<String>();
    if compact.len() < 32 {
        return false;
    }
    let has_alpha = compact.chars().any(|ch| ch.is_ascii_alphabetic());
    let has_digit = compact.chars().any(|ch| ch.is_ascii_digit());
    let unique = compact
        .chars()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    has_alpha && has_digit && unique >= 12
}

fn safe_prefix(value: &str) -> String {
    value.chars().take(4).collect()
}

fn sha256_8(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn parse_failure(
    path: &Path,
    tool: &str,
    kind: &str,
    reason: String,
    now: &str,
) -> ResourceParseFailure {
    ResourceParseFailure {
        path: path.display().to_string(),
        tool: tool.to_string(),
        kind: Some(kind.to_string()),
        reason,
        seen_at: now.to_string(),
    }
}

fn merge_resource(
    resources: &mut BTreeMap<String, CanonicalResource>,
    resource: CanonicalResource,
) {
    if let Some(existing) = resources.get_mut(&resource.id) {
        for tool in resource.enabled_in {
            if !existing.enabled_in.contains(&tool) {
                existing.enabled_in.push(tool);
            }
        }
        existing.enabled_in.sort();
        if !existing
            .origin_tool
            .split(',')
            .any(|tool| tool == resource.origin_tool)
        {
            existing.origin_tool = format!("{},{}", existing.origin_tool, resource.origin_tool);
        }
        existing.confidence = existing.confidence.max(resource.confidence);
        existing.redacted = existing.redacted || resource.redacted;
        if existing.provided_by_plugin.is_none() {
            existing.provided_by_plugin = resource.provided_by_plugin;
        }
        existing.last_seen = resource.last_seen;
    } else {
        resources.insert(resource.id.clone(), resource);
    }
}

#[cfg(test)]
mod tests {

    fn opencode_config_dir(home: &std::path::Path) -> std::path::PathBuf {
        let ctx = crate::platform_paths::PlatformPathContext::current_home_compatible(home);
        match ctx.platform {
            crate::platform_paths::PlatformKind::Macos => home.join(".config/opencode"),
            _ => ctx.config_dir.join("opencode"),
        }
    }

    fn normalize_path(p: impl AsRef<std::path::Path>) -> String {
        p.as_ref().display().to_string().replace('\\', "/")
    }

    use std::{fs, path::PathBuf};

    use tempfile::tempdir;
    use walkdir::WalkDir;

    use super::*;

    #[test]
    fn mcp_sources_use_path_resolver_for_user_scope_paths() {
        let source = include_str!("resources.rs");

        assert!(source.contains("tool_path_candidates"));
        assert!(source.contains("ToolPathKind::McpConfig"));
        for hardcoded_join in [
            "path: home.join(\".claude.json\")",
            "path: home.join(\".codex/config.toml\")",
            "path: home.join(\".gemini/settings.json\")",
            "path: home.join(\".cursor/mcp.json\")",
        ] {
            assert!(
                !source.contains(hardcoded_join),
                "user-scope MCP source still hardcodes {hardcoded_join}"
            );
        }
    }

    #[test]
    fn project_mcp_sources_use_path_resolver_candidates() {
        let source = include_str!("resources.rs");

        assert!(source.contains("ToolPathKind::ProjectMcpConfig"));
        for hardcoded_join in [
            "path: project.join(\".mcp.json\")",
            "path: project.join(\".cursor/mcp.json\")",
        ] {
            assert!(
                !source.contains(hardcoded_join),
                "project MCP source still hardcodes {hardcoded_join}"
            );
        }
    }

    #[test]
    fn claude_user_ecosystem_roots_use_path_resolver() {
        let source = include_str!("resources.rs");

        assert!(source.contains("ToolPathKind::SkillDir"));
        assert!(source.contains("ToolPathKind::PluginDir"));
        assert!(source.contains("ToolPathKind::SubagentDir"));
        for root in [".claude/skills", ".claude/plugins", ".claude/agents"] {
            let hardcoded_join = format!("home.{}(\"{}\")", "join", root);
            assert!(
                !source.contains(&hardcoded_join),
                "Claude user ecosystem root still hardcodes {hardcoded_join}"
            );
        }
    }

    #[test]
    fn project_claude_ecosystem_roots_use_path_resolver() {
        let source = include_str!("resources.rs");
        let production_source = source.split("#[cfg(test)]").next().unwrap_or(source);

        assert!(production_source.contains("ToolPathKind::ProjectSkillDir"));
        assert!(production_source.contains("ToolPathKind::ProjectSubagentDir"));
        for hardcoded_join in [
            "project.join(\".claude/skills\")",
            "project.join(\".claude/agents\")",
        ] {
            assert!(
                !production_source.contains(hardcoded_join),
                "project Claude ecosystem root still hardcodes {hardcoded_join}"
            );
        }
    }

    #[test]
    fn user_instruction_sources_use_path_resolver() {
        let source = include_str!("resources.rs");

        assert!(source.contains("ToolPathKind::InstructionFile"));
        assert!(source.contains("ToolPathKind::InstructionDir"));
        for path in [
            ".claude/CLAUDE.md",
            ".codex/AGENTS.md",
            ".gemini/GEMINI.md",
            ".cursorrules",
            ".cursor/rules",
        ] {
            let hardcoded_join = format!("home.{}(\"{}\")", "join", path);
            assert!(
                !source.contains(&hardcoded_join),
                "user instruction source still hardcodes {hardcoded_join}"
            );
        }
    }

    #[test]
    fn project_instruction_sources_use_path_resolver() {
        let source = include_str!("resources.rs");
        let production_source = source.split("#[cfg(test)]").next().unwrap_or(source);

        assert!(production_source.contains("ToolPathKind::ProjectInstructionFile"));
        assert!(production_source.contains("ToolPathKind::ProjectInstructionDir"));
        for hardcoded_join in [
            "path: project.join(\"CLAUDE.md\")",
            "path: project.join(\"AGENTS.md\")",
            "path: project.join(\"GEMINI.md\")",
            "path: project.join(\".cursorrules\")",
            "project.join(\".cursor/rules\")",
        ] {
            assert!(
                !production_source.contains(hardcoded_join),
                "project instruction source still hardcodes {hardcoded_join}"
            );
        }
    }

    #[test]
    fn scans_json_and_toml_mcp_configs_with_redacted_env_values() {
        let home = tempdir().unwrap();
        fs::write(
            home.path().join(".claude.json"),
            r#"{"mcpServers":{"github":{"command":"npx","args":["-y","@modelcontextprotocol/server-github"],"env":{"GITHUB_TOKEN":"ghp_secret123"}}}}"#,
        )
        .unwrap();
        fs::create_dir_all(home.path().join(".codex")).unwrap();
        fs::write(
            home.path().join(".codex/config.toml"),
            r#"[mcp_servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "ghp_secret123" }
"#,
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        assert_eq!(inventory.failures.len(), 0);
        assert_eq!(inventory.resources.len(), 1);
        let resource = &inventory.resources[0];
        assert_eq!(resource.kind, "mcp");
        assert_eq!(resource.name, "github");
        assert_eq!(
            resource.enabled_in,
            vec!["claude".to_string(), "codex".to_string()]
        );
        assert!(resource.redacted);
        assert!(resource.payload_json.contains("GITHUB_TOKEN"));
        assert!(resource.payload_json.contains("sha256_8"));
        assert!(!resource.payload_json.contains("ghp_secret123"));
    }

    #[test]
    fn scans_project_vscode_mcp_servers_without_secret_values() {
        let home = tempdir().unwrap();
        let project = home.path().join("work/repo");
        fs::create_dir_all(project.join(".vscode")).unwrap();
        fs::write(
            project.join(".vscode/mcp.json"),
            serde_json::json!({
                "inputs": [
                    {
                        "type": "promptString",
                        "id": "api-token",
                        "description": "API token",
                        "password": true
                    }
                ],
                "servers": {
                    "context7": {
                        "type": "http",
                        "url": "https://mcp.context7.com/mcp",
                        "headers": {
                            "Authorization": "Bearer should-not-persist",
                            "Accept": "application/json"
                        }
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let inventory =
            scan_inventory_with_project_roots(home.path(), std::slice::from_ref(&project), None);

        let resource = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "mcp"
                    && resource.origin_tool == "vscode"
                    && resource.name == "context7"
            })
            .expect("VS Code workspace MCP server should be scanned");
        assert_eq!(resource.scope, "project");
        assert_eq!(
            normalize_path(resource.origin_path.clone()),
            normalize_path(project.join(".vscode").join("mcp.json"))
        );
        assert_eq!(resource.origin_locator.as_deref(), Some("servers.context7"));
        assert_eq!(resource.enabled_in, vec!["vscode".to_string()]);
        assert!(resource.redacted);
        assert!(resource.payload_json.contains("Authorization"));
        assert!(resource.payload_json.contains("Accept"));
        assert!(!resource.payload_json.contains("should-not-persist"));
    }

    #[test]
    fn scans_opencode_mcp_from_user_and_project_config_without_secret_values() {
        let home = tempdir().unwrap();
        fs::create_dir_all(opencode_config_dir(home.path())).unwrap();
        fs::write(
            opencode_config_dir(home.path()).join("opencode.json"),
            serde_json::json!({
                "$schema": "https://opencode.ai/config.json",
                "mcp": {
                    "local-docs": {
                        "type": "local",
                        "command": ["npx", "-y", "@upstash/context7-mcp", "--token", "sk-opencode-local-secret-1234567890abcdef"],
                        "environment": {
                            "CONTEXT7_API_KEY": "sk-opencode-env-secret-1234567890abcdef"
                        },
                        "enabled": true
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        let project = home.path().join("work/repo");
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("opencode.json"),
            serde_json::json!({
                "$schema": "https://opencode.ai/config.json",
                "mcp": {
                    "remote-docs": {
                        "type": "remote",
                        "url": "https://mcp.context7.com/mcp",
                        "headers": {
                            "Authorization": "Bearer sk-opencode-header-secret-1234567890abcdef",
                            "Accept": "application/json"
                        },
                        "oauth": false
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let inventory =
            scan_inventory_with_project_roots(home.path(), std::slice::from_ref(&project), None);

        let user_mcp = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "mcp"
                    && resource.origin_tool == "opencode"
                    && resource.name == "local-docs"
            })
            .expect("OpenCode user MCP should be scanned");
        assert_eq!(user_mcp.scope, "user");
        assert_eq!(user_mcp.origin_locator.as_deref(), Some("mcp.local-docs"));
        assert_eq!(user_mcp.enabled_in, vec!["opencode".to_string()]);
        assert!(user_mcp.redacted);
        assert!(user_mcp.payload_json.contains("\"transport\":\"stdio\""));
        assert!(user_mcp.payload_json.contains("\"name\":\"npx\""));
        assert!(user_mcp.payload_json.contains("CONTEXT7_API_KEY"));
        assert!(user_mcp.payload_json.contains("sha256_8"));
        assert!(!user_mcp.payload_json.contains("sk-opencode-local-secret"));
        assert!(!user_mcp.payload_json.contains("sk-opencode-env-secret"));

        let project_mcp = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "mcp"
                    && resource.origin_tool == "opencode"
                    && resource.name == "remote-docs"
            })
            .expect("OpenCode project MCP should be scanned");
        assert_eq!(project_mcp.scope, "project");
        assert_eq!(
            project_mcp.origin_path,
            project.join("opencode.json").display().to_string()
        );
        assert_eq!(
            project_mcp.origin_locator.as_deref(),
            Some("mcp.remote-docs")
        );
        assert!(project_mcp.payload_json.contains("\"transport\":\"http\""));
        assert!(project_mcp.payload_json.contains("Authorization"));
        assert!(project_mcp.payload_json.contains("Accept"));
        assert!(
            !project_mcp
                .payload_json
                .contains("sk-opencode-header-secret")
        );
        assert!(!project_mcp.payload_json.contains("application/json"));
    }

    #[test]
    fn scans_opencode_disabled_mcp_without_marking_it_enabled() {
        let home = tempdir().unwrap();
        fs::create_dir_all(opencode_config_dir(home.path())).unwrap();
        fs::write(
            opencode_config_dir(home.path()).join("opencode.json"),
            serde_json::json!({
                "$schema": "https://opencode.ai/config.json",
                "mcp": {
                    "disabled-docs": {
                        "type": "remote",
                        "url": "https://mcp.example.test/mcp",
                        "enabled": false
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        let resource = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "mcp"
                    && resource.origin_tool == "opencode"
                    && resource.name == "disabled-docs"
            })
            .expect("disabled OpenCode MCP config should still be inventoried");
        assert_eq!(resource.enabled_in, Vec::<String>::new());
        assert!(resource.payload_json.contains("\"enabled\":false"));
    }

    #[test]
    fn redacts_high_risk_mcp_args_before_payload_json() {
        let home = tempdir().unwrap();
        fs::write(
            home.path().join(".claude.json"),
            r#"{"mcpServers":{"slack":{"command":"npx","args":["--token","xoxb-1234567890-ABCDEFGHIJKLMN-opensesame"]}}}"#,
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        let resource = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "mcp" && resource.name == "slack")
            .unwrap();
        assert!(resource.redacted);
        assert!(resource.payload_json.contains("<redacted:"));
        assert!(!resource.payload_json.contains("xoxb-1234567890"));
        assert!(!resource.payload_json.contains("opensesame"));
    }

    #[test]
    fn fingerprints_mcp_headers_and_api_key_without_storing_secret_values() {
        let home = tempdir().unwrap();
        fs::write(
            home.path().join(".claude.json"),
            serde_json::json!({
                "mcpServers": {
                    "remote": {
                        "type": "sse",
                        "url": "https://example.test/mcp",
                        "headers": {
                            "Authorization": "Bearer sk-live-secret-header-1234567890abcdef",
                            "X-Api-Key": "ghp_header_secret_1234567890abcdef",
                            "Accept": "application/json"
                        },
                        "apiKey": "sk-live-top-level-api-key-1234567890abcdef"
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        let resource = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "mcp" && resource.name == "remote")
            .unwrap();
        assert!(resource.redacted);
        assert!(resource.payload_json.contains("header_keys"));
        assert!(resource.payload_json.contains("header_fingerprints"));
        assert!(
            resource
                .payload_json
                .contains("sensitive_field_fingerprints")
        );
        assert!(resource.payload_json.contains("Authorization"));
        assert!(resource.payload_json.contains("X-Api-Key"));
        assert!(resource.payload_json.contains("apiKey"));
        assert!(resource.payload_json.contains("sha256_8"));
        assert!(!resource.payload_json.contains("sk-live-secret-header"));
        assert!(!resource.payload_json.contains("ghp_header_secret"));
        assert!(!resource.payload_json.contains("sk-live-top-level-api-key"));
        assert!(!resource.payload_json.contains("application/json"));
    }

    #[test]
    fn summarizes_mcp_command_paths_without_persisting_raw_command() {
        let home = tempdir().unwrap();
        let command = home
            .path()
            .join("bin/sk-live-secret-like-command-1234567890abcdef")
            .display()
            .to_string();
        fs::write(
            home.path().join(".claude.json"),
            serde_json::json!({
                "mcpServers": {
                    "local": {
                        "command": command,
                        "args": ["server.js"]
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let inventory = scan_inventory(home.path());
        let resource = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "mcp" && resource.name == "local")
            .unwrap();

        assert!(resource.redacted);
        assert!(resource.payload_json.contains("sha256_8"));
        assert!(resource.payload_json.contains("<redacted:"));
        assert!(!resource.payload_json.contains(&command));
    }

    #[test]
    fn records_parse_failure_without_stopping_inventory_scan() {
        let home = tempdir().unwrap();
        fs::write(home.path().join(".claude.json"), "{not-json").unwrap();
        fs::create_dir_all(home.path().join(".gemini")).unwrap();
        fs::write(
            home.path().join(".gemini/settings.json"),
            r#"{"mcpServers":{"docs":{"url":"https://example.test/mcp","type":"sse"}}}"#,
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        assert_eq!(inventory.resources.len(), 1);
        assert_eq!(inventory.failures.len(), 1);
        assert_eq!(inventory.failures[0].tool, "claude");
        assert_eq!(inventory.failures[0].kind.as_deref(), Some("mcp"));
    }

    #[test]
    fn keeps_distinct_mcp_names_even_when_payload_matches() {
        let home = tempdir().unwrap();
        fs::write(
            home.path().join(".claude.json"),
            r#"{"mcpServers":{"github":{"command":"npx"},"docs":{"command":"npx"}}}"#,
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        assert_eq!(inventory.resources.len(), 2);
        let names = inventory
            .resources
            .iter()
            .map(|resource| resource.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"github"));
        assert!(names.contains(&"docs"));
    }

    #[test]
    fn scans_claude_skills_without_storing_file_contents() {
        let home = tempdir().unwrap();
        let skill_dir = home.path().join(".claude/skills/reviewer");
        fs::create_dir_all(skill_dir.join("references")).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "# Reviewer\n\nUse secret internal rubric.",
        )
        .unwrap();
        fs::write(
            skill_dir.join("references/checklist.md"),
            "Do not leak this checklist body.",
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        let skill = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "skill" && resource.name == "reviewer")
            .unwrap();
        assert_eq!(skill.scope, "user");
        assert_eq!(skill.enabled_in, vec!["claude".to_string()]);
        assert!(skill.payload_json.contains("SKILL.md"));
        assert!(skill.payload_json.contains("content_hash"));
        assert!(!skill.payload_json.contains("secret internal rubric"));
        assert!(!skill.payload_json.contains("checklist body"));
    }

    #[test]
    fn scans_opencode_skills_from_user_and_project_roots_without_body_text() {
        let home = tempdir().unwrap();
        let user_skill = opencode_config_dir(home.path()).join("skills/git-release");
        fs::create_dir_all(&user_skill).unwrap();
        fs::write(
            user_skill.join("SKILL.md"),
            r#"---
name: git-release
description: Prepare private release notes from internal context
license: MIT
compatibility: opencode
---
# Release

Never store this OpenCode user skill body.
"#,
        )
        .unwrap();

        let project = home.path().join("work/repo");
        let project_skill = project.join(".opencode/skills/repo-review");
        fs::create_dir_all(&project_skill).unwrap();
        fs::write(
            project_skill.join("SKILL.md"),
            r#"---
name: repo-review
description: Review private repository implementation details
---
# Review

Never store this OpenCode project skill body.
"#,
        )
        .unwrap();

        let inventory =
            scan_inventory_with_project_roots(home.path(), std::slice::from_ref(&project), None);

        let user_resource = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "skill"
                    && resource.origin_tool == "opencode"
                    && resource.name == "git-release"
            })
            .expect("OpenCode user skill should be scanned");
        assert_eq!(user_resource.scope, "user");
        assert_eq!(user_resource.enabled_in, vec!["opencode".to_string()]);
        assert_eq!(user_resource.origin_locator.as_deref(), Some("SKILL.md"));
        assert!(user_resource.redacted);
        assert!(user_resource.payload_json.contains("frontmatter_keys"));
        assert!(
            user_resource
                .payload_json
                .contains("description_fingerprint")
        );
        assert!(user_resource.payload_json.contains("compatibility"));
        assert!(!user_resource.payload_json.contains("Prepare private"));
        assert!(
            !user_resource
                .payload_json
                .contains("Never store this OpenCode user skill body")
        );

        let project_resource = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "skill"
                    && resource.origin_tool == "opencode"
                    && resource.name == "repo-review"
            })
            .expect("OpenCode project skill should be scanned");
        assert_eq!(project_resource.scope, "project");
        assert_eq!(project_resource.enabled_in, vec!["opencode".to_string()]);
        assert!(
            !project_resource
                .payload_json
                .contains("Review private repository")
        );
        assert!(
            !project_resource
                .payload_json
                .contains("Never store this OpenCode project skill body")
        );
    }

    #[test]
    fn scan_inventory_with_kinds_limits_detector_families() {
        let home = tempdir().unwrap();
        fs::write(
            home.path().join(".claude.json"),
            r#"{"mcpServers":{"github":{"command":"npx"}}}"#,
        )
        .unwrap();
        let skill_dir = home.path().join(".claude/skills/reviewer");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "# Reviewer\n\nReview implementation evidence.",
        )
        .unwrap();

        let inventory = scan_inventory_with_kinds(home.path(), Some(&["skill"]));

        assert_eq!(inventory.failures.len(), 0);
        assert_eq!(inventory.resources.len(), 1);
        assert_eq!(inventory.resources[0].kind, "skill");
    }

    #[test]
    fn scans_instruction_files_as_structure_fingerprints_only() {
        let home = tempdir().unwrap();
        fs::create_dir_all(home.path().join(".codex")).unwrap();
        fs::write(
            home.path().join(".codex/AGENTS.md"),
            "# Build Rules\n\nNever store this body sentence.\n\n## Tests\nRun real checks.",
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        let instruction = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "instruction" && resource.name == "AGENTS.md")
            .unwrap();
        assert_eq!(instruction.scope, "user");
        assert_eq!(instruction.enabled_in, vec!["codex".to_string()]);
        assert!(instruction.payload_json.contains("Build Rules"));
        assert!(instruction.payload_json.contains("paragraph_hashes"));
        assert!(instruction.payload_json.contains("byte_count"));
        assert!(
            !instruction
                .payload_json
                .contains("Never store this body sentence")
        );
        assert!(!instruction.payload_json.contains("Run real checks"));
    }

    #[test]
    fn parses_cursor_rule_frontmatter_without_storing_description_or_body() {
        let home = tempdir().unwrap();
        let rules_dir = home.path().join(".cursor/rules");
        fs::create_dir_all(&rules_dir).unwrap();
        fs::write(
            rules_dir.join("react.mdc"),
            r#"---
description: "Use React safely in payment pages"
globs: ["ui/**/*.tsx", "ui/**/*.ts"]
alwaysApply: false
---
# React Rules

Never store this Cursor rule body.
"#,
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        let instruction = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "instruction" && resource.name == "react.mdc")
            .unwrap();
        assert_eq!(instruction.origin_tool, "cursor");
        assert!(
            instruction
                .payload_json
                .contains("cursor-rules-frontmatter-v1")
        );
        assert!(instruction.payload_json.contains("always_apply"));
        assert!(instruction.payload_json.contains("ui/**/*.tsx"));
        assert!(instruction.payload_json.contains("description_fingerprint"));
        assert!(!instruction.payload_json.contains("Use React safely"));
        assert!(
            !instruction
                .payload_json
                .contains("Never store this Cursor rule body")
        );
    }

    #[test]
    fn renders_cursor_rule_frontmatter_that_scanner_can_parse_without_persisting_body() {
        let home = tempdir().unwrap();
        let rules_dir = home.path().join(".cursor/rules");
        fs::create_dir_all(&rules_dir).unwrap();
        let generated = render_cursor_rule_document(
            "Generated guidance for React routes",
            &["ui/**/*.tsx".to_string(), "ui/**/*.ts".to_string()],
            true,
            "# Generated Rule\n\nNever store this generated Cursor rule body.",
        )
        .unwrap();
        fs::write(rules_dir.join("generated.mdc"), generated).unwrap();

        let inventory = scan_inventory(home.path());

        let instruction = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "instruction" && resource.name == "generated.mdc")
            .unwrap();
        assert!(
            instruction
                .payload_json
                .contains("cursor-rules-frontmatter-v1")
        );
        assert!(instruction.payload_json.contains("ui/**/*.tsx"));
        assert!(instruction.payload_json.contains("\"always_apply\":true"));
        assert!(instruction.payload_json.contains("description_fingerprint"));
        assert!(!instruction.payload_json.contains("Generated guidance"));
        assert!(
            !instruction
                .payload_json
                .contains("Never store this generated")
        );
    }

    #[test]
    fn scans_claude_plugins_without_storing_file_contents() {
        let home = tempdir().unwrap();
        let plugin_dir = home.path().join(".claude/plugins/test-plugin");
        fs::create_dir_all(plugin_dir.join("commands")).unwrap();
        fs::create_dir_all(plugin_dir.join("agents")).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name":"test-plugin","version":"1.2.3","marketplace":"local"}"#,
        )
        .unwrap();
        fs::write(
            plugin_dir.join("commands/run.md"),
            "Execute this secret command body.",
        )
        .unwrap();
        fs::write(
            plugin_dir.join("agents/reviewer.md"),
            "Do not persist this agent body.",
        )
        .unwrap();
        fs::write(
            plugin_dir.join("commands/sk-live-secret-like-command-1234567890abcdef.md"),
            "Do not persist this filename either.",
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        let plugin = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "plugin" && resource.name == "test-plugin")
            .unwrap();
        assert_eq!(plugin.scope, "user");
        assert_eq!(plugin.enabled_in, vec!["claude".to_string()]);
        assert!(plugin.payload_json.contains("version"));
        assert!(plugin.payload_json.contains("component_counts"));
        assert!(plugin.payload_json.contains("content_hash"));
        assert!(plugin.payload_json.contains("path_hash"));
        assert!(!plugin.payload_json.contains("secret command body"));
        assert!(!plugin.payload_json.contains("agent body"));
        assert!(
            !plugin
                .payload_json
                .contains("sk-live-secret-like-command-1234567890abcdef")
        );
    }

    #[test]
    fn scans_plugin_provided_resources_with_provider_relationship() {
        let home = tempdir().unwrap();
        let plugin_dir = home.path().join(".claude/plugins/github-tools");
        fs::create_dir_all(plugin_dir.join("mcp")).unwrap();
        fs::create_dir_all(plugin_dir.join("agents")).unwrap();
        fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name":"github-tools","version":"2.0.0","marketplace":"local"}"#,
        )
        .unwrap();
        fs::write(
            plugin_dir.join("mcp/github.json"),
            r#"{"mcpServers":{"github":{"command":"npx","args":["--token","ghp_plugin_secret_1234567890"],"env":{"GITHUB_TOKEN":"ghp_plugin_env_secret"}}}}"#,
        )
        .unwrap();
        fs::write(
            plugin_dir.join("agents/reviewer.md"),
            "---\nname: plugin-reviewer\nmodel: inherit\nallowed_tools: [Read, Grep]\n---\nNever store this plugin agent body.",
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        let plugin = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "plugin" && resource.name == "github-tools")
            .unwrap();
        assert_eq!(plugin.provided_by_plugin, None);
        let plugin_mcp = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "mcp"
                    && resource.name == "github"
                    && resource.provided_by_plugin.as_deref() == Some("github-tools")
            })
            .unwrap();
        assert_eq!(plugin_mcp.scope, "user");
        assert_eq!(plugin_mcp.origin_tool, "claude");
        assert!(plugin_mcp.redacted);
        assert!(plugin_mcp.payload_json.contains("GITHUB_TOKEN"));
        assert!(!plugin_mcp.payload_json.contains("ghp_plugin_secret"));
        assert!(!plugin_mcp.payload_json.contains("ghp_plugin_env_secret"));

        let plugin_agent = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "subagent"
                    && resource.name == "plugin-reviewer"
                    && resource.provided_by_plugin.as_deref() == Some("github-tools")
            })
            .unwrap();
        assert_eq!(plugin_agent.scope, "user");
        assert!(plugin_agent.payload_json.contains("body_hashes"));
        assert!(
            !plugin_agent
                .payload_json
                .contains("Never store this plugin agent body")
        );
    }

    #[test]
    fn scans_claude_subagents_without_storing_body_text() {
        let home = tempdir().unwrap();
        let agents_dir = home.path().join(".claude/agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("reviewer.md"),
            r#"---
name: reviewer
model: claude-sonnet
allowed_tools: [Read, Grep]
---
# Reviewer

Never store this subagent instruction body.
"#,
        )
        .unwrap();

        let inventory = scan_inventory(home.path());

        let subagent = inventory
            .resources
            .iter()
            .find(|resource| resource.kind == "subagent" && resource.name == "reviewer")
            .unwrap();
        assert_eq!(subagent.scope, "user");
        assert_eq!(subagent.enabled_in, vec!["claude".to_string()]);
        assert!(subagent.payload_json.contains("claude-sonnet"));
        assert!(subagent.payload_json.contains("Read"));
        assert!(subagent.payload_json.contains("body_hashes"));
        assert!(
            !subagent
                .payload_json
                .contains("Never store this subagent instruction body")
        );
    }

    #[test]
    fn scans_project_level_resources_without_body_or_secret_values() {
        let home = tempdir().unwrap();
        let project = home.path().join("work/repo");
        fs::create_dir_all(project.join(".cursor")).unwrap();
        fs::write(
            project.join(".cursor/mcp.json"),
            r#"{"mcpServers":{"repo-db":{"command":"node","args":["server.js","sk-test-secret"],"env":{"DATABASE_URL":"postgres://secret"}}}}"#,
        )
        .unwrap();
        fs::write(
            project.join("AGENTS.md"),
            "# Repo Rules\n\nNever store this project instruction body.",
        )
        .unwrap();
        let skill_dir = project.join(".claude/skills/repo-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "# Repo Skill\n\nNever store this project skill body.",
        )
        .unwrap();
        let agent_dir = project.join(".claude/agents");
        fs::create_dir_all(&agent_dir).unwrap();
        fs::write(
            agent_dir.join("repo-reviewer.md"),
            "---\nname: repo-reviewer\nmodel: inherit\n---\nNever store this project agent body.",
        )
        .unwrap();

        let inventory =
            scan_inventory_with_project_roots(home.path(), std::slice::from_ref(&project), None);

        let project_resources = inventory
            .resources
            .iter()
            .filter(|resource| resource.scope == "project")
            .collect::<Vec<_>>();
        assert_eq!(project_resources.len(), 4);
        assert!(project_resources.iter().any(|resource| {
            resource.kind == "mcp" && resource.name == "repo-db" && resource.origin_tool == "cursor"
        }));
        assert!(
            project_resources
                .iter()
                .any(|resource| resource.kind == "instruction" && resource.name == "AGENTS.md")
        );
        assert!(
            project_resources
                .iter()
                .any(|resource| resource.kind == "skill" && resource.name == "repo-skill")
        );
        assert!(
            project_resources
                .iter()
                .any(|resource| resource.kind == "subagent" && resource.name == "repo-reviewer")
        );
        let serialized = serde_json::to_string(&project_resources).unwrap();
        assert!(!serialized.contains("postgres://secret"));
        assert!(!serialized.contains("sk-test-secret"));
        assert!(!serialized.contains("Never store this project instruction body."));
        assert!(!serialized.contains("Never store this project skill body."));
        assert!(!serialized.contains("Never store this project agent body."));
    }

    #[test]
    fn scans_project_vscode_copilot_instructions_without_body_text() {
        let home = tempdir().unwrap();
        let project = home.path().join("work/repo");
        fs::create_dir_all(project.join(".github")).unwrap();
        fs::write(
            project.join(".github/copilot-instructions.md"),
            "# Copilot Rules\n\nNever persist this Copilot instruction body.\n\n## Tests\nRun verified checks.",
        )
        .unwrap();

        let inventory =
            scan_inventory_with_project_roots(home.path(), std::slice::from_ref(&project), None);

        let instruction = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "instruction"
                    && resource.origin_tool == "vscode"
                    && resource.name == "copilot-instructions.md"
            })
            .expect("VS Code Copilot project instructions should be scanned");
        assert_eq!(instruction.scope, "project");
        assert_eq!(instruction.enabled_in, vec!["vscode".to_string()]);
        assert_eq!(
            normalize_path(instruction.origin_path.clone()),
            normalize_path(project.join(".github").join("copilot-instructions.md"))
        );
        assert!(instruction.payload_json.contains("Copilot Rules"));
        assert!(instruction.payload_json.contains("paragraph_hashes"));
        assert!(
            !instruction
                .payload_json
                .contains("Never persist this Copilot instruction body")
        );
        assert!(!instruction.payload_json.contains("Run verified checks"));
    }

    #[test]
    fn audits_inventory_fixture_counts_against_manual_expectations() {
        let home = tempdir().unwrap();
        fs::write(
            home.path().join(".claude.json"),
            r#"{"mcpServers":{"github":{"command":"npx","env":{"GITHUB_TOKEN":"ghp_secret123"}}}}"#,
        )
        .unwrap();
        fs::create_dir_all(home.path().join(".codex")).unwrap();
        fs::write(
            home.path().join(".codex/AGENTS.md"),
            "# User Agents\n\nUser body.",
        )
        .unwrap();
        let user_skill = home.path().join(".claude/skills/user-skill");
        fs::create_dir_all(&user_skill).unwrap();
        fs::write(user_skill.join("SKILL.md"), "# User Skill\n\nBody.").unwrap();
        fs::create_dir_all(home.path().join(".claude/plugins/plugin-one/mcp")).unwrap();
        fs::create_dir_all(home.path().join(".claude/plugins/plugin-one/agents")).unwrap();
        fs::write(
            home.path().join(".claude/plugins/plugin-one/plugin.json"),
            r#"{"name":"plugin-one"}"#,
        )
        .unwrap();
        fs::write(
            home.path()
                .join(".claude/plugins/plugin-one/mcp/plugin-docs.json"),
            r#"{"mcpServers":{"plugin-docs":{"command":"node","args":["server.js"]}}}"#,
        )
        .unwrap();
        fs::write(
            home.path()
                .join(".claude/plugins/plugin-one/agents/plugin-agent.md"),
            "---\nname: plugin-agent\n---\nBody.",
        )
        .unwrap();
        fs::create_dir_all(home.path().join(".claude/agents")).unwrap();
        fs::write(
            home.path().join(".claude/agents/user-agent.md"),
            "---\nname: user-agent\n---\nBody.",
        )
        .unwrap();
        fs::create_dir_all(home.path().join(".gemini")).unwrap();
        fs::write(home.path().join(".gemini/settings.json"), "{bad-json").unwrap();

        let project = home.path().join("work/repo");
        fs::create_dir_all(project.join(".cursor")).unwrap();
        fs::write(
            project.join(".cursor/mcp.json"),
            r#"{"mcpServers":{"repo-db":{"command":"node","args":["server.js"]}}}"#,
        )
        .unwrap();
        fs::write(
            project.join("AGENTS.md"),
            "# Project Agents\n\nProject body.",
        )
        .unwrap();
        let project_skill = project.join(".claude/skills/project-skill");
        fs::create_dir_all(&project_skill).unwrap();
        fs::write(project_skill.join("SKILL.md"), "# Project Skill\n\nBody.").unwrap();
        fs::create_dir_all(project.join(".claude/agents")).unwrap();
        fs::write(
            project.join(".claude/agents/project-agent.md"),
            "---\nname: project-agent\n---\nBody.",
        )
        .unwrap();

        let inventory = scan_inventory_with_project_roots(home.path(), &[project], None);
        let mut expected_kinds = BTreeMap::new();
        expected_kinds.insert("instruction".to_string(), 2);
        expected_kinds.insert("mcp".to_string(), 3);
        expected_kinds.insert("plugin".to_string(), 1);
        expected_kinds.insert("skill".to_string(), 2);
        expected_kinds.insert("subagent".to_string(), 3);
        let mut expected_scopes = BTreeMap::new();
        expected_scopes.insert("project".to_string(), 4);
        expected_scopes.insert("user".to_string(), 7);
        let expectation = InventoryAuditExpectation {
            kind_counts: expected_kinds,
            scope_counts: expected_scopes,
            failure_count: 1,
        };

        let report = audit_inventory_fixture(&inventory, &expectation);

        assert!(report.passed, "{:?}", report.mismatches);
        assert_eq!(report.actual_kind_counts, expectation.kind_counts);
        assert_eq!(report.actual_scope_counts, expectation.scope_counts);
        assert_eq!(report.actual_failure_count, 1);
    }

    #[test]
    fn audits_checked_in_redacted_inventory_fixture() {
        let home = checked_in_redacted_fixture_home();
        assert!(
            home.exists(),
            "checked-in redacted resource inventory fixture is missing"
        );
        assert_checked_in_fixture_is_sanitized(&home);

        let project = home.join("work/redacted-repo");
        let inventory =
            scan_inventory_with_project_roots(&home, std::slice::from_ref(&project), None);
        let mut expected_kinds = BTreeMap::new();
        expected_kinds.insert("mcp".to_string(), 6);
        expected_kinds.insert("instruction".to_string(), 4);
        expected_kinds.insert("skill".to_string(), 2);
        expected_kinds.insert("plugin".to_string(), 1);
        expected_kinds.insert("subagent".to_string(), 2);
        let mut expected_scopes = BTreeMap::new();
        expected_scopes.insert("user".to_string(), 11);
        expected_scopes.insert("project".to_string(), 4);

        let report = audit_inventory_fixture(
            &inventory,
            &InventoryAuditExpectation {
                kind_counts: expected_kinds,
                scope_counts: expected_scopes,
                failure_count: 0,
            },
        );

        assert!(
            report.passed,
            "fixture audit mismatches: {:?}",
            report.mismatches
        );
        let serialized = serde_json::to_string(&inventory.resources).unwrap();
        assert!(!serialized.contains("__WAPC_REDACTED_FIXTURE_TOKEN__"));
        assert!(!serialized.contains("Never store"));
        assert!(inventory.resources.iter().any(|resource| {
            resource.kind == "instruction" && resource.payload_json.contains("frontmatter_metadata")
        }));
        assert!(inventory.resources.iter().any(|resource| {
            resource.kind == "skill"
                && resource.origin_tool == "opencode"
                && resource
                    .payload_json
                    .contains("opencode-skill-frontmatter-v1")
        }));
        assert!(inventory.resources.iter().any(|resource| {
            resource.kind == "mcp"
                && resource.origin_tool == "opencode"
                && resource.name == "fixture-docs"
        }));
        let disabled = inventory
            .resources
            .iter()
            .find(|resource| {
                resource.kind == "mcp"
                    && resource.origin_tool == "opencode"
                    && resource.name == "disabled-docs"
            })
            .expect("disabled OpenCode MCP fixture should be inventoried");
        assert_eq!(disabled.enabled_in, Vec::<String>::new());
        assert!(disabled.payload_json.contains("\"enabled\":false"));
    }

    fn checked_in_redacted_fixture_home() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/resource_inventory/redacted-home")
    }

    fn assert_checked_in_fixture_is_sanitized(home: &std::path::Path) {
        let forbidden = [
            "sk-",
            "ghp_",
            "github_pat_",
            "xoxb-",
            "xoxp-",
            "/Users/",
            "secret-client",
        ];
        for entry in WalkDir::new(home).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let content = fs::read_to_string(entry.path()).unwrap();
            for pattern in forbidden {
                assert!(
                    !content.contains(pattern),
                    "fixture file {} contains forbidden pattern {pattern}",
                    entry.path().display()
                );
            }
        }
    }
}
