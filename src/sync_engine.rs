//! Safe resource write pipeline.
//! @author codex

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use toml::Value as TomlValue;

use crate::{
    model::{
        ApplyChangeRequest, ApplyChangeResult, ResourceBackup, ResourceChangeLog,
        ResourceChangeRequest, WritePlan, WritePlanRisk,
    },
    store::UsageStore,
};

const MAX_BACKUPS_PER_TOOL: usize = 10;

pub fn plan_resource_change(
    _home: &Path,
    store: &UsageStore,
    request: ResourceChangeRequest,
) -> Result<WritePlan> {
    if request.kind != "mcp" || request.op != "disable" {
        bail!(
            "unsupported resource change: kind={} op={}",
            request.kind,
            request.op
        );
    }
    enforce_resource_write_boundary(store, &request)?;
    let target = PathBuf::from(&request.target_path);
    let before = fs::read_to_string(&target)
        .with_context(|| format!("failed to read target file {}", target.display()))?;
    let after = disable_mcp_json(&before, &request.resource_name)?;
    let before_fingerprint = sha256_hex(before.as_bytes());
    let after_fingerprint = sha256_hex(after.as_bytes());
    let now = Utc::now().to_rfc3339();
    store.record_file_fingerprint(
        &request.tool,
        &request.target_path,
        &before_fingerprint,
        &now,
    )?;

    Ok(WritePlan {
        plan_id: plan_id(&request, &before_fingerprint, &after_fingerprint, &now),
        tool: request.tool,
        kind: request.kind,
        op: request.op,
        resource_id: request.resource_id,
        resource_name: request.resource_name,
        target_path: request.target_path,
        target_scope: None,
        target_project_path: None,
        before_fingerprint,
        after_fingerprint,
        diff: line_diff(&before, &after),
        preview_before: before,
        preview_after: after,
        requires_backup: true,
        risks: vec![WritePlanRisk {
            code: "external_config_write".to_string(),
            message: "Will modify a local tool configuration file after backup.".to_string(),
            severity: "medium".to_string(),
        }],
        created_at: now,
    })
}

pub fn apply_resource_change(
    home: &Path,
    store: &UsageStore,
    request: ApplyChangeRequest,
) -> Result<ApplyChangeResult> {
    let plan = request.plan;
    let sync_id = request.sync_id;
    let target = PathBuf::from(&plan.target_path);
    let current = fs::read(&target)
        .with_context(|| format!("failed to read target file {}", target.display()))?;
    let current_fingerprint = sha256_hex(&current);
    if current_fingerprint == plan.after_fingerprint {
        return Ok(ApplyChangeResult {
            change_id: plan.plan_id,
            backup_path: None,
            status: "noop".to_string(),
        });
    }
    if current_fingerprint != plan.before_fingerprint && !request.confirm_drift {
        bail!("drift detected for {}", plan.target_path);
    }

    let now = Utc::now().to_rfc3339();
    let change_id = change_id(&plan, &now);
    let backup_path = backup_target(home, &plan, &change_id, &now)?;
    let write_result = atomic_write(&target, plan.preview_after.as_bytes())
        .and_then(|_| verify_plan(&target, &plan, request.force_verify_failure));

    match write_result {
        Ok(()) => {
            store.insert_resource_backup(&ResourceBackup {
                backup_path: backup_path.display().to_string(),
                tool: plan.tool.clone(),
                original_path: plan.target_path.clone(),
                change_id: Some(change_id.clone()),
                created_at: now.clone(),
            })?;
            rotate_tool_backups(home, store, &plan.tool, MAX_BACKUPS_PER_TOOL)?;
            store.insert_resource_change(&ResourceChangeLog {
                change_id: change_id.clone(),
                sync_id: sync_id.clone(),
                tool: plan.tool.clone(),
                resource_id: plan.resource_id.clone(),
                kind: plan.kind.clone(),
                op: plan.op.clone(),
                target_path: plan.target_path.clone(),
                backup_path: Some(backup_path.display().to_string()),
                status: "committed".to_string(),
                reverts_change_id: None,
                created_at: now.clone(),
            })?;
            store.record_file_fingerprint(
                &plan.tool,
                &plan.target_path,
                &plan.after_fingerprint,
                &now,
            )?;
            Ok(ApplyChangeResult {
                change_id,
                backup_path: Some(backup_path.display().to_string()),
                status: "committed".to_string(),
            })
        }
        Err(err) => {
            restore_backup(&backup_path, &target)?;
            store.insert_resource_change(&ResourceChangeLog {
                change_id: change_id.clone(),
                sync_id,
                tool: plan.tool.clone(),
                resource_id: plan.resource_id.clone(),
                kind: plan.kind.clone(),
                op: plan.op.clone(),
                target_path: plan.target_path.clone(),
                backup_path: Some(backup_path.display().to_string()),
                status: "failed".to_string(),
                reverts_change_id: None,
                created_at: now,
            })?;
            Err(err)
        }
    }
}

pub fn rollback_resource_change(
    home: &Path,
    store: &UsageStore,
    change_id: &str,
) -> Result<ApplyChangeResult> {
    let original = store
        .get_resource_change(change_id)?
        .with_context(|| format!("resource change not found: {change_id}"))?;
    if original.status != "committed" {
        bail!("resource change {change_id} is not rollbackable");
    }
    if original.op == "rollback" || original.reverts_change_id.is_some() {
        bail!("resource change {change_id} is not rollbackable");
    }
    let source_backup = original
        .backup_path
        .as_ref()
        .context("resource change has no backup path")?;
    let target = PathBuf::from(&original.target_path);
    let backup_bytes = fs::read(source_backup)
        .with_context(|| format!("failed to read backup file {source_backup}"))?;

    let now = Utc::now().to_rfc3339();
    let rollback_id = rollback_change_id(&original, &now);
    let rollback_backup_path = backup_file(
        home,
        &original.tool,
        &original.target_path,
        &rollback_id,
        &now,
    )?;
    let write_result = atomic_write(&target, &backup_bytes).and_then(|_| {
        let current = fs::read(&target)?;
        if sha256_hex(&current) != sha256_hex(&backup_bytes) {
            bail!("rollback verify failed for {}", target.display());
        }
        Ok(())
    });

    match write_result {
        Ok(()) => {
            store.insert_resource_backup(&ResourceBackup {
                backup_path: rollback_backup_path.display().to_string(),
                tool: original.tool.clone(),
                original_path: original.target_path.clone(),
                change_id: Some(rollback_id.clone()),
                created_at: now.clone(),
            })?;
            rotate_tool_backups(home, store, &original.tool, MAX_BACKUPS_PER_TOOL)?;
            store.insert_resource_change(&ResourceChangeLog {
                change_id: rollback_id.clone(),
                sync_id: original.sync_id.clone(),
                tool: original.tool.clone(),
                resource_id: original.resource_id.clone(),
                kind: original.kind.clone(),
                op: "rollback".to_string(),
                target_path: original.target_path.clone(),
                backup_path: Some(rollback_backup_path.display().to_string()),
                status: "committed".to_string(),
                reverts_change_id: Some(original.change_id.clone()),
                created_at: now.clone(),
            })?;
            store.update_resource_change_status(&original.change_id, "rolledback")?;
            store.record_file_fingerprint(
                &original.tool,
                &original.target_path,
                &sha256_hex(&backup_bytes),
                &now,
            )?;
            Ok(ApplyChangeResult {
                change_id: rollback_id,
                backup_path: Some(rollback_backup_path.display().to_string()),
                status: "committed".to_string(),
            })
        }
        Err(err) => {
            restore_backup(&rollback_backup_path, &target)?;
            store.insert_resource_change(&ResourceChangeLog {
                change_id: rollback_id.clone(),
                sync_id: original.sync_id.clone(),
                tool: original.tool.clone(),
                resource_id: original.resource_id.clone(),
                kind: original.kind.clone(),
                op: "rollback".to_string(),
                target_path: original.target_path.clone(),
                backup_path: Some(rollback_backup_path.display().to_string()),
                status: "failed".to_string(),
                reverts_change_id: Some(original.change_id.clone()),
                created_at: now,
            })?;
            Err(err)
        }
    }
}

fn enforce_resource_write_boundary(
    store: &UsageStore,
    request: &ResourceChangeRequest,
) -> Result<()> {
    let Some(resource_id) = request.resource_id.as_deref() else {
        return Ok(());
    };
    let Some(resource) = store.get_resource(resource_id)? else {
        return Ok(());
    };
    if resource.scope == "enterprise" || resource.scope == "managed" {
        bail!(
            "enterprise or managed resource remains read-only: {}",
            resource.id
        );
    }
    if resource.provided_by_plugin.is_some() {
        bail!(
            "plugin-provided resource remains read-only: {}",
            resource.id
        );
    }
    if resource.kind != request.kind {
        bail!(
            "resource kind mismatch: stored={} requested={}",
            resource.kind,
            request.kind
        );
    }
    if resource.origin_path != request.target_path {
        bail!(
            "resource target path mismatch: stored={} requested={}",
            resource.origin_path,
            request.target_path
        );
    }
    Ok(())
}

fn disable_mcp_json(content: &str, resource_name: &str) -> Result<String> {
    let mut value: Value = serde_json::from_str(content)?;
    let servers = value
        .get_mut("mcpServers")
        .and_then(Value::as_object_mut)
        .context("target JSON does not contain mcpServers object")?;
    if servers.remove(resource_name).is_none() {
        bail!("MCP resource not found: {resource_name}");
    }
    Ok(format!("{}\n", serde_json::to_string_pretty(&value)?))
}

fn verify_plan(target: &Path, plan: &WritePlan, force_failure: bool) -> Result<()> {
    if force_failure {
        bail!("verify failed by test hook");
    }
    let current = fs::read(target)?;
    let current_fingerprint = sha256_hex(&current);
    if current_fingerprint != plan.after_fingerprint {
        bail!("verify failed for {}", target.display());
    }
    let content = String::from_utf8(current)?;
    let exists = mcp_resource_exists(&content, plan)?;
    match plan.op.as_str() {
        "disable" if exists => bail!("verify failed: MCP resource still present"),
        "sync" if !exists => bail!("verify failed: MCP resource is missing after sync"),
        _ => {}
    }
    Ok(())
}

fn mcp_resource_exists(content: &str, plan: &WritePlan) -> Result<bool> {
    if plan.tool == "codex" || plan.target_path.ends_with(".toml") {
        let value: TomlValue = toml::from_str(content)?;
        return Ok(value
            .get("mcp_servers")
            .and_then(TomlValue::as_table)
            .is_some_and(|servers| servers.contains_key(&plan.resource_name)));
    }
    let value: Value = serde_json::from_str(content)?;
    Ok(value
        .get("mcpServers")
        .and_then(Value::as_object)
        .is_some_and(|servers| servers.contains_key(&plan.resource_name)))
}

fn backup_target(home: &Path, plan: &WritePlan, change_id: &str, now: &str) -> Result<PathBuf> {
    backup_file(home, &plan.tool, &plan.target_path, change_id, now)
}

fn backup_file(
    home: &Path,
    tool: &str,
    target_path: &str,
    change_id: &str,
    now: &str,
) -> Result<PathBuf> {
    let file_name = Path::new(target_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("target");
    let timestamp = now
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    let dir = home
        .join(".wapc/backups")
        .join(tool)
        .join(format!("{timestamp}-{change_id}"));
    create_private_dir_all(&dir)?;
    let backup_path = dir.join(file_name);
    fs::copy(target_path, &backup_path)?;
    set_owner_only_file_permissions(&backup_path)?;
    Ok(backup_path)
}

fn rotate_tool_backups(
    home: &Path,
    store: &UsageStore,
    tool: &str,
    max_backups: usize,
) -> Result<()> {
    if max_backups == 0 {
        return Ok(());
    }
    let root = home.join(".wapc/backups").join(tool);
    if !root.exists() {
        return Ok(());
    }

    let mut dirs = fs::read_dir(&root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()?;
            Some((modified, path))
        })
        .collect::<Vec<_>>();
    dirs.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));

    for (_, dir) in dirs.into_iter().skip(max_backups) {
        let backup_paths = fs::read_dir(&dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        fs::remove_dir_all(&dir)?;
        for backup_path in backup_paths {
            store.delete_resource_backup(&backup_path)?;
        }
    }
    Ok(())
}

fn atomic_write(target: &Path, bytes: &[u8]) -> Result<()> {
    let parent = target
        .parent()
        .context("target path has no parent directory")?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".{}.wapc-tmp",
        target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("write")
    ));
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temp, target)?;
    if let Ok(parent_file) = File::open(parent) {
        let _ = parent_file.sync_all();
    }
    Ok(())
}

fn restore_backup(backup_path: &Path, target: &Path) -> Result<()> {
    let bytes = fs::read(backup_path)?;
    atomic_write(target, &bytes)
}

fn line_diff(before: &str, after: &str) -> String {
    let mut diff = Vec::new();
    let before_lines = before.lines().collect::<Vec<_>>();
    let after_lines = after.lines().collect::<Vec<_>>();
    for line in &before_lines {
        if !after_lines.contains(line) {
            diff.push(format!("-{line}"));
        }
    }
    for line in &after_lines {
        if !before_lines.contains(line) {
            diff.push(format!("+{line}"));
        }
    }
    diff.join("\n")
}

fn plan_id(
    request: &ResourceChangeRequest,
    before_fingerprint: &str,
    after_fingerprint: &str,
    now: &str,
) -> String {
    format!(
        "plan:{}",
        sha256_8(&format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            request.tool,
            request.kind,
            request.op,
            request.target_path,
            before_fingerprint,
            after_fingerprint
        )) + &sha256_8(now)
    )
}

fn change_id(plan: &WritePlan, now: &str) -> String {
    format!(
        "chg:{}",
        sha256_8(&format!(
            "{}\n{}\n{}\n{}\n{}",
            plan.plan_id, plan.target_path, plan.before_fingerprint, plan.after_fingerprint, now
        ))
    )
}

fn rollback_change_id(change: &ResourceChangeLog, now: &str) -> String {
    format!(
        "chg:{}",
        sha256_8(&format!(
            "rollback\n{}\n{}\n{}\n{}",
            change.change_id, change.target_path, change.status, now
        ))
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn sha256_8(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(unix)]
fn create_private_dir_all(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn create_private_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Sync Engine behavior tests.
    //! @author codex

    use std::fs;

    use tempfile::tempdir;

    use crate::{
        model::{ApplyChangeRequest, CanonicalResource, ResourceChangeRequest},
        store::UsageStore,
        sync_engine::{apply_resource_change, plan_resource_change, rollback_resource_change},
    };

    fn write_mcp_config(path: &std::path::Path) {
        fs::write(
            path,
            r#"{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_TOKEN": "ghp_do_not_log" }
    },
    "docs": {
      "url": "https://example.test/mcp",
      "type": "sse"
    }
  }
}"#,
        )
        .unwrap();
    }

    fn disable_github_request(path: &std::path::Path) -> ResourceChangeRequest {
        ResourceChangeRequest {
            tool: "claude".to_string(),
            kind: "mcp".to_string(),
            op: "disable".to_string(),
            resource_id: Some("mcp:github".to_string()),
            target_path: path.display().to_string(),
            resource_name: "github".to_string(),
        }
    }

    fn enterprise_resource(path: &std::path::Path) -> CanonicalResource {
        CanonicalResource {
            id: "mcp:enterprise:claude:github".to_string(),
            kind: "mcp".to_string(),
            name: "github".to_string(),
            scope: "enterprise".to_string(),
            origin_tool: "claude".to_string(),
            origin_path: path.display().to_string(),
            origin_locator: Some("mcpServers.github".to_string()),
            enabled_in: vec!["claude".to_string()],
            confidence: 1.0,
            redacted: true,
            payload_json: r#"{"transport":"stdio","command":"npx","args":[],"env_keys":["GITHUB_TOKEN"],"env_fingerprints":{}}"#.to_string(),
            provided_by_plugin: None,
            last_seen: "2026-06-06T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn plan_disable_mcp_json_entry_returns_diff_without_writing_file() {
        let home = tempdir().unwrap();
        let config = home.path().join(".claude.json");
        write_mcp_config(&config);
        let before = fs::read_to_string(&config).unwrap();
        let db = home.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();

        let plan = plan_resource_change(home.path(), &store, disable_github_request(&config))
            .expect("plan should be generated");

        assert_eq!(fs::read_to_string(&config).unwrap(), before);
        assert_eq!(plan.tool, "claude");
        assert_eq!(plan.kind, "mcp");
        assert_eq!(plan.op, "disable");
        assert_eq!(plan.target_path, config.display().to_string());
        assert!(plan.diff.contains("-    \"github\""));
        assert!(plan.preview_after.contains("\"docs\""));
        assert!(plan.requires_backup);
        assert!(!plan.before_fingerprint.is_empty());
        assert!(!plan.after_fingerprint.is_empty());
    }

    #[test]
    fn plan_resource_change_rejects_enterprise_resources_before_file_write_preview() {
        let home = tempdir().unwrap();
        let config = home.path().join(".claude.json");
        write_mcp_config(&config);
        let before = fs::read_to_string(&config).unwrap();
        let db = home.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[enterprise_resource(&config)])
            .unwrap();
        let mut request = disable_github_request(&config);
        request.resource_id = Some("mcp:enterprise:claude:github".to_string());

        let error = plan_resource_change(home.path(), &store, request)
            .expect_err("enterprise resources must remain read-only");

        assert!(error.to_string().contains("enterprise"));
        assert_eq!(fs::read_to_string(&config).unwrap(), before);
        assert!(store.list_resource_changes(None).unwrap().is_empty());
    }

    #[test]
    fn apply_disable_mcp_json_entry_backs_up_writes_verifies_and_commits_change() {
        let home = tempdir().unwrap();
        let config = home.path().join(".claude.json");
        write_mcp_config(&config);
        let db = home.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let plan = plan_resource_change(home.path(), &store, disable_github_request(&config))
            .expect("plan should be generated");

        let result = apply_resource_change(
            home.path(),
            &store,
            ApplyChangeRequest {
                plan,
                confirm_drift: false,
                sync_id: None,
                force_verify_failure: false,
            },
        )
        .expect("apply should commit");

        let updated = fs::read_to_string(&config).unwrap();
        assert!(!updated.contains("\"github\""));
        assert!(updated.contains("\"docs\""));
        assert!(result.backup_path.is_some());
        assert_eq!(store.list_resource_changes(None).unwrap().len(), 1);
        assert_eq!(store.list_resource_backups(None).unwrap().len(), 1);
    }

    #[test]
    fn apply_disable_mcp_json_entry_is_idempotent_for_the_same_plan() {
        let home = tempdir().unwrap();
        let config = home.path().join(".claude.json");
        write_mcp_config(&config);
        let db = home.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let plan = plan_resource_change(home.path(), &store, disable_github_request(&config))
            .expect("plan should be generated");
        let expected_after = plan.preview_after.clone();

        apply_resource_change(
            home.path(),
            &store,
            ApplyChangeRequest {
                plan: plan.clone(),
                confirm_drift: false,
                sync_id: None,
                force_verify_failure: false,
            },
        )
        .expect("first apply should commit");
        let noop = apply_resource_change(
            home.path(),
            &store,
            ApplyChangeRequest {
                plan,
                confirm_drift: false,
                sync_id: None,
                force_verify_failure: false,
            },
        )
        .expect("second apply of the same plan should be a no-op");

        assert_eq!(noop.status, "noop");
        assert!(noop.backup_path.is_none());
        assert_eq!(fs::read_to_string(&config).unwrap(), expected_after);
        assert_eq!(store.list_resource_changes(None).unwrap().len(), 1);
        assert_eq!(store.list_resource_backups(None).unwrap().len(), 1);
    }

    #[test]
    fn apply_disable_mcp_json_entry_blocks_unconfirmed_drift_without_writing() {
        let home = tempdir().unwrap();
        let config = home.path().join(".claude.json");
        write_mcp_config(&config);
        let db = home.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let plan = plan_resource_change(home.path(), &store, disable_github_request(&config))
            .expect("plan should be generated");
        fs::write(&config, r#"{"mcpServers":{"github":{"command":"node"}}}"#).unwrap();
        let drifted = fs::read_to_string(&config).unwrap();

        let error = apply_resource_change(
            home.path(),
            &store,
            ApplyChangeRequest {
                plan,
                confirm_drift: false,
                sync_id: None,
                force_verify_failure: false,
            },
        )
        .expect_err("drift should require explicit confirmation");

        assert!(error.to_string().contains("drift"));
        assert_eq!(fs::read_to_string(&config).unwrap(), drifted);
        assert!(store.list_resource_changes(None).unwrap().is_empty());
    }

    #[test]
    fn apply_disable_mcp_json_entry_rolls_back_when_verify_fails() {
        let home = tempdir().unwrap();
        let config = home.path().join(".claude.json");
        write_mcp_config(&config);
        let original = fs::read_to_string(&config).unwrap();
        let db = home.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let plan = plan_resource_change(home.path(), &store, disable_github_request(&config))
            .expect("plan should be generated");

        let error = apply_resource_change(
            home.path(),
            &store,
            ApplyChangeRequest {
                plan,
                confirm_drift: false,
                sync_id: None,
                force_verify_failure: true,
            },
        )
        .expect_err("verify failure should rollback");

        assert!(error.to_string().contains("verify"));
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, "failed");
    }

    #[test]
    fn apply_disable_mcp_json_entry_rotates_old_tool_backups() {
        let home = tempdir().unwrap();
        let config = home.path().join(".claude.json");
        let db = home.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();

        for _ in 0..=super::MAX_BACKUPS_PER_TOOL {
            write_mcp_config(&config);
            let plan = plan_resource_change(home.path(), &store, disable_github_request(&config))
                .expect("plan should be generated");
            apply_resource_change(
                home.path(),
                &store,
                ApplyChangeRequest {
                    plan,
                    confirm_drift: false,
                    sync_id: None,
                    force_verify_failure: false,
                },
            )
            .expect("apply should commit");
        }

        let backups = store.list_resource_backups(Some("claude")).unwrap();
        assert_eq!(backups.len(), super::MAX_BACKUPS_PER_TOOL);
        let backup_root = home.path().join(".wapc/backups/claude");
        let backup_dirs = fs::read_dir(backup_root)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .count();
        assert_eq!(backup_dirs, super::MAX_BACKUPS_PER_TOOL);
    }

    #[test]
    fn rollback_resource_change_restores_backup_and_records_revert_change() {
        let home = tempdir().unwrap();
        let config = home.path().join(".claude.json");
        write_mcp_config(&config);
        let original = fs::read_to_string(&config).unwrap();
        let db = home.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let plan = plan_resource_change(home.path(), &store, disable_github_request(&config))
            .expect("plan should be generated");
        let result = apply_resource_change(
            home.path(),
            &store,
            ApplyChangeRequest {
                plan,
                confirm_drift: false,
                sync_id: None,
                force_verify_failure: false,
            },
        )
        .expect("apply should commit");

        let rollback = rollback_resource_change(home.path(), &store, &result.change_id)
            .expect("rollback should restore backup");

        assert_eq!(fs::read_to_string(&config).unwrap(), original);
        assert_eq!(rollback.status, "committed");
        let changes = store.list_resource_changes(None).unwrap();
        assert!(changes.iter().any(|change| {
            change.reverts_change_id.as_deref() == Some(result.change_id.as_str())
                && change.op == "rollback"
        }));
        let original_change = changes
            .iter()
            .find(|change| change.change_id == result.change_id)
            .expect("original change should remain logged");
        assert_eq!(original_change.status, "rolledback");
    }

    #[test]
    fn rollback_resource_change_rejects_rollback_records() {
        let home = tempdir().unwrap();
        let config = home.path().join(".claude.json");
        write_mcp_config(&config);
        let original = fs::read_to_string(&config).unwrap();
        let db = home.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let plan = plan_resource_change(home.path(), &store, disable_github_request(&config))
            .expect("plan should be generated");
        let result = apply_resource_change(
            home.path(),
            &store,
            ApplyChangeRequest {
                plan,
                confirm_drift: false,
                sync_id: Some("sync:phase4".to_string()),
                force_verify_failure: false,
            },
        )
        .expect("apply should commit");
        let rollback = rollback_resource_change(home.path(), &store, &result.change_id)
            .expect("rollback should restore backup");

        let error = rollback_resource_change(home.path(), &store, &rollback.change_id)
            .expect_err("rollback records must not be rollbackable");

        assert!(error.to_string().contains("not rollbackable"));
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(changes.len(), 2);
        assert!(
            changes
                .iter()
                .all(|change| { change.sync_id.as_deref() == Some("sync:phase4") })
        );
    }
}
