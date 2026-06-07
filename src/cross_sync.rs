//! Cross-tool sync planning engine.
//! @author codex

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use toml::Value as TomlValue;

use crate::{
    deep_link,
    model::{
        ApplyChangeRequest, ApplySyncRequest, ApplySyncResult, ApplySyncTargetResult,
        CanonicalResource, PlanSyncRequest, PlanSyncResult, ResourceChangeLog, SyncOperation,
        SyncTarget, SyncTargetPlan, WritePlan, WritePlanRisk,
    },
    store::UsageStore,
    sync_engine, template_library,
};

pub fn plan_sync(
    _home: &Path,
    store: &UsageStore,
    request: PlanSyncRequest,
) -> Result<PlanSyncResult> {
    let source = store
        .get_resource(&request.resource_id)?
        .with_context(|| format!("source resource not found: {}", request.resource_id))?;
    plan_sync_from_resource(store, source, request)
}

pub fn plan_sync_from_resource(
    store: &UsageStore,
    source: CanonicalResource,
    request: PlanSyncRequest,
) -> Result<PlanSyncResult> {
    if source.kind != "mcp" && source.kind != "instruction" {
        bail!("unsupported sync source kind: {}", source.kind);
    }
    let created_at = Utc::now().to_rfc3339();
    let targets = request
        .targets
        .iter()
        .map(|target| plan_target(store, &source, target, &request, &created_at))
        .collect::<Vec<_>>();

    Ok(PlanSyncResult {
        source_resource_id: request.resource_id,
        created_at,
        targets,
    })
}

pub fn apply_sync(
    home: &Path,
    store: &UsageStore,
    request: ApplySyncRequest,
) -> Result<ApplySyncResult> {
    enforce_non_empty_apply_request(&request)?;
    validate_apply_env_strategy_value(&request)?;
    enforce_single_source_apply_request(&request)?;
    validate_apply_env_values(&request)?;
    validate_apply_env_strategy_placeholders(&request)?;
    let created_at = Utc::now().to_rfc3339();
    let sync_id = sync_id(&request.plans, &created_at);
    let env_strategy = apply_env_strategy_label(&request).to_string();
    let source_resource_id = request
        .plans
        .iter()
        .find_map(|plan| plan.resource_id.clone());
    let targets_json = serde_json::to_string(
        &request
            .plans
            .iter()
            .map(|plan| {
                json!({
                    "plan_id": plan.plan_id,
                    "tool": plan.tool,
                    "kind": plan.kind,
                    "op": plan.op,
                    "scope": plan.target_scope,
                    "project_path": plan.target_project_path,
                    "target_path": plan.target_path,
                })
            })
            .collect::<Vec<_>>(),
    )?;
    store.insert_sync_operation(&SyncOperation {
        sync_id: sync_id.clone(),
        source_resource_id,
        targets_json,
        allow_cross_scope: request.allow_cross_scope,
        env_strategy,
        created_at,
    })?;

    let mut changes = Vec::new();
    for plan in request.plans {
        let plan_id = plan.plan_id.clone();
        let tool = plan.tool.clone();
        let resource_id = plan.resource_id.clone();
        let kind = plan.kind.clone();
        let op = plan.op.clone();
        let target_path = plan.target_path.clone();
        match validate_apply_plan_source(
            store,
            &plan,
            request.allow_cross_scope,
            request.deep_link_url.as_deref(),
        )
        .and_then(|_| materialize_env_placeholders(plan, &request.env_values))
        .and_then(|plan| {
            sync_engine::apply_resource_change(
                home,
                store,
                ApplyChangeRequest {
                    plan,
                    confirm_drift: request.confirm_drift,
                    sync_id: Some(sync_id.clone()),
                    force_verify_failure: false,
                },
            )
        }) {
            Ok(result) => {
                let is_noop = result.status == "noop";
                changes.push(ApplySyncTargetResult {
                    plan_id,
                    target_path,
                    status: result.status,
                    change_id: if is_noop {
                        None
                    } else {
                        Some(result.change_id)
                    },
                    backup_path: result.backup_path,
                    reason: if is_noop {
                        Some(
                            "target already matches planned state; no change was written"
                                .to_string(),
                        )
                    } else {
                        None
                    },
                });
            }
            Err(err) => {
                let failure_change_id = ensure_failed_change_record(
                    store,
                    &sync_id,
                    FailedChangeMetadata {
                        plan_id: &plan_id,
                        tool: &tool,
                        resource_id: resource_id.as_deref(),
                        kind: &kind,
                        op: &op,
                        target_path: &target_path,
                    },
                )?;
                changes.push(ApplySyncTargetResult {
                    plan_id,
                    target_path,
                    status: "failed".to_string(),
                    change_id: Some(failure_change_id),
                    backup_path: None,
                    reason: Some(err.to_string()),
                });
            }
        }
    }

    Ok(ApplySyncResult { sync_id, changes })
}

fn enforce_non_empty_apply_request(request: &ApplySyncRequest) -> Result<()> {
    if request.plans.is_empty() {
        bail!("apply_sync plans must contain at least one plan");
    }
    Ok(())
}

fn apply_env_strategy_label(request: &ApplySyncRequest) -> &str {
    let strategy = request
        .env_strategy
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("none");
    if strategy == "none" && request.plans.iter().any(plan_preview_contains_skipped_env) {
        "skip"
    } else {
        strategy
    }
}

fn validate_apply_env_strategy_value(request: &ApplySyncRequest) -> Result<()> {
    let strategy = apply_env_strategy_label(request);
    if !matches!(strategy, "none" | "reuse" | "manual" | "skip") {
        bail!("unsupported env_strategy for apply_sync: {strategy}");
    }
    Ok(())
}

fn plan_preview_contains_skipped_env(plan: &WritePlan) -> bool {
    json_plan_preview_contains_skipped_env(plan) || toml_plan_preview_contains_skipped_env(plan)
}

fn json_plan_preview_contains_skipped_env(plan: &WritePlan) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(&plan.preview_after) else {
        return false;
    };
    value
        .get("mcpServers")
        .and_then(Value::as_object)
        .and_then(|servers| servers.get(&plan.resource_name))
        .and_then(|entry| entry.get("env"))
        .and_then(Value::as_object)
        .map(|env| env.values().any(|value| value.as_str() == Some("")))
        .unwrap_or(false)
}

fn toml_plan_preview_contains_skipped_env(plan: &WritePlan) -> bool {
    let Ok(value) = toml::from_str::<TomlValue>(&plan.preview_after) else {
        return false;
    };
    value
        .get("mcp_servers")
        .and_then(TomlValue::as_table)
        .and_then(|servers| servers.get(&plan.resource_name))
        .and_then(|entry| entry.get("env"))
        .and_then(TomlValue::as_table)
        .map(|env| env.values().any(|value| value.as_str() == Some("")))
        .unwrap_or(false)
}

fn validate_apply_env_values(request: &ApplySyncRequest) -> Result<()> {
    if request.env_values.is_empty() {
        return Ok(());
    }
    let expected = request
        .plans
        .iter()
        .flat_map(|plan| manual_env_keys_in_preview(&plan.preview_after))
        .collect::<BTreeSet<_>>();
    let unexpected = request
        .env_values
        .keys()
        .filter(|key| !expected.contains(*key))
        .cloned()
        .collect::<Vec<_>>();
    if !unexpected.is_empty() {
        bail!(
            "unexpected env_values keys for apply_sync: {}",
            unexpected.join(", ")
        );
    }
    Ok(())
}

fn validate_apply_env_strategy_placeholders(request: &ApplySyncRequest) -> Result<()> {
    let strategy = apply_env_strategy_label(request);
    let manual_keys = request
        .plans
        .iter()
        .flat_map(|plan| manual_env_keys_in_preview(&plan.preview_after))
        .collect::<BTreeSet<_>>();
    let reuse_keys = request
        .plans
        .iter()
        .flat_map(|plan| reuse_env_keys_in_preview(&plan.preview_after))
        .collect::<BTreeSet<_>>();
    if !manual_keys.is_empty() && strategy != "manual" {
        bail!("env_strategy must be manual for manual env placeholders");
    }
    if !reuse_keys.is_empty() && strategy != "reuse" {
        bail!("env_strategy must be reuse for reuse env placeholders");
    }
    Ok(())
}

fn manual_env_keys_in_preview(preview: &str) -> BTreeSet<String> {
    env_keys_in_preview(preview, "<WAPC_MANUAL_ENV:")
}

fn reuse_env_keys_in_preview(preview: &str) -> BTreeSet<String> {
    env_keys_in_preview(preview, "<WAPC_REUSE_ENV:")
}

fn env_keys_in_preview(preview: &str, prefix: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let mut rest = preview;
    while let Some(start) = rest.find(prefix) {
        rest = &rest[start + prefix.len()..];
        let Some(end) = rest.find('>') else {
            break;
        };
        let key = rest[..end].trim();
        if !key.is_empty() {
            keys.insert(key.to_string());
        }
        rest = &rest[end + 1..];
    }
    keys
}

fn enforce_single_source_apply_request(request: &ApplySyncRequest) -> Result<()> {
    let source_ids = request
        .plans
        .iter()
        .filter_map(|plan| plan.resource_id.as_deref())
        .filter(|resource_id| !resource_id.trim().is_empty())
        .collect::<BTreeSet<_>>();
    if source_ids.len() > 1 {
        bail!(
            "apply_sync requires a single source resource per operation, got {}",
            source_ids.len()
        );
    }
    Ok(())
}

fn validate_apply_plan_source(
    store: &UsageStore,
    plan: &WritePlan,
    allow_cross_scope: bool,
    deep_link_url: Option<&str>,
) -> Result<()> {
    if plan.kind != "mcp" || plan.op != "sync" {
        bail!(
            "apply_sync only accepts mcp sync plans, got kind={} op={}",
            plan.kind,
            plan.op
        );
    }
    validate_plan_fingerprints(plan)?;
    validate_plan_diff_self_consistency(plan)?;
    validate_plan_confirmation_metadata(plan)?;
    validate_plan_target_observed_fingerprint(store, plan)?;
    validate_plan_target_metadata(plan)?;
    let resource_id = plan
        .resource_id
        .as_deref()
        .context("apply_sync plan requires resource_id")?;
    if let Some(rest) = resource_id.strip_prefix("template:") {
        let (template_id, fingerprint) = rest
            .rsplit_once(':')
            .context("invalid template resource_id")?;
        let template = store
            .get_resource_template(template_id)?
            .with_context(|| format!("template source not found: {template_id}"))?;
        if template.content_fingerprint != fingerprint {
            bail!("template source fingerprint mismatch: {template_id}");
        }
        if template.kind != plan.kind {
            bail!(
                "template kind mismatch: stored={} requested={}",
                template.kind,
                plan.kind
            );
        }
        let source = template_library::canonical_resource_from_template(&template);
        if source.name != plan.resource_name {
            bail!(
                "template resource name mismatch: stored={} requested={}",
                source.name,
                plan.resource_name
            );
        }
        validate_plan_entry_matches_source_payload(plan, &source.payload_json)?;
        validate_plan_scope_authorization(&source.scope, plan, allow_cross_scope)?;
        validate_plan_only_changes_selected_mcp(plan)?;
        validate_plan_id_self_consistency(plan)?;
        return Ok(());
    }
    if resource_id.starts_with("deep-link:") {
        let link = deep_link_url
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .context("apply_sync deep-link plan requires deep_link_url")?;
        let source = deep_link::preview_deep_link_import(link)?.resource;
        if source.id != resource_id {
            bail!(
                "deep-link source id mismatch: planned={} provided={}",
                resource_id,
                source.id
            );
        }
        if source.kind != plan.kind {
            bail!(
                "deep-link source kind mismatch: planned={} provided={}",
                plan.kind,
                source.kind
            );
        }
        if source.name != plan.resource_name {
            bail!(
                "deep-link source name mismatch: planned={} provided={}",
                plan.resource_name,
                source.name
            );
        }
        validate_plan_entry_matches_source_payload(plan, &source.payload_json)?;
        validate_plan_scope_authorization(&source.scope, plan, allow_cross_scope)?;
        validate_plan_only_changes_selected_mcp(plan)?;
        validate_plan_id_self_consistency(plan)?;
        return Ok(());
    }
    let source = store
        .get_resource(resource_id)?
        .with_context(|| format!("source resource not found: {resource_id}"))?;
    if source.kind != plan.kind {
        bail!(
            "source resource kind mismatch: stored={} requested={}",
            source.kind,
            plan.kind
        );
    }
    if source.name != plan.resource_name {
        bail!(
            "source resource name mismatch: stored={} requested={}",
            source.name,
            plan.resource_name
        );
    }
    if source.provided_by_plugin.is_some() {
        bail!("plugin-provided source remains read-only: {resource_id}");
    }
    validate_plan_entry_matches_source_payload(plan, &source.payload_json)?;
    validate_plan_scope_authorization(&source.scope, plan, allow_cross_scope)?;
    validate_plan_only_changes_selected_mcp(plan)?;
    validate_plan_id_self_consistency(plan)?;
    Ok(())
}

fn validate_plan_confirmation_metadata(plan: &WritePlan) -> Result<()> {
    if !plan.requires_backup {
        bail!("plan backup requirement does not match cross-tool sync policy");
    }
    let expected_risks = cross_tool_write_risks();
    if plan.risks != expected_risks {
        bail!("plan risk metadata does not match cross-tool sync policy");
    }
    Ok(())
}

fn validate_plan_diff_self_consistency(plan: &WritePlan) -> Result<()> {
    let expected = line_diff(&plan.preview_before, &plan.preview_after);
    if expected != plan.diff {
        bail!("plan diff does not match preview_before and preview_after");
    }
    Ok(())
}

fn validate_plan_id_self_consistency(plan: &WritePlan) -> Result<()> {
    let resource_id = plan
        .resource_id
        .as_deref()
        .context("apply_sync plan requires resource_id")?;
    let expected = sync_plan_id_from_parts(SyncPlanIdParts {
        source_id: resource_id,
        tool: &plan.tool,
        target_path: &plan.target_path,
        target_scope: plan.target_scope.as_deref(),
        target_project_path: plan.target_project_path.as_deref(),
        before_fingerprint: &plan.before_fingerprint,
        after_fingerprint: &plan.after_fingerprint,
        created_at: &plan.created_at,
    });
    if expected != plan.plan_id {
        bail!("plan_id does not match planned source, target, and fingerprints");
    }
    Ok(())
}

fn validate_plan_target_observed_fingerprint(store: &UsageStore, plan: &WritePlan) -> Result<()> {
    let Some(observed) = store.get_file_fingerprint(&plan.tool, &plan.target_path)? else {
        bail!("target fingerprint was not recorded during plan_sync");
    };
    if observed != plan.before_fingerprint && observed != plan.after_fingerprint {
        bail!("target fingerprint does not match the planned target");
    }
    Ok(())
}

fn validate_plan_fingerprints(plan: &WritePlan) -> Result<()> {
    let before = sha256_hex(plan.preview_before.as_bytes());
    if before != plan.before_fingerprint {
        bail!("plan before_fingerprint does not match preview_before");
    }
    let after = sha256_hex(plan.preview_after.as_bytes());
    if after != plan.after_fingerprint {
        bail!("plan after_fingerprint does not match preview_after");
    }
    Ok(())
}

fn validate_plan_target_metadata(plan: &WritePlan) -> Result<()> {
    let Some(scope) = plan
        .target_scope
        .as_deref()
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
    else {
        if plan
            .target_project_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .is_some()
        {
            bail!("target_project_path requires target_scope metadata");
        }
        return Ok(());
    };
    if scope == "enterprise" || scope == "managed" {
        bail!("enterprise targets are read-only");
    }
    let project_path = plan
        .target_project_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty());
    if scope == "project" {
        let Some(project_path) = project_path else {
            bail!("project target metadata requires target_project_path");
        };
        if !Path::new(&plan.target_path).starts_with(Path::new(project_path)) {
            bail!("project target path must be under target_project_path");
        }
        return Ok(());
    }
    if project_path.is_some() {
        bail!("target_project_path is only valid for project target metadata");
    }
    Ok(())
}

fn validate_plan_scope_authorization(
    source_scope: &str,
    plan: &WritePlan,
    allow_cross_scope: bool,
) -> Result<()> {
    let Some(target_scope) = plan
        .target_scope
        .as_deref()
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
    else {
        return Ok(());
    };
    if source_scope != target_scope && !allow_cross_scope {
        bail!("cross-scope sync requires explicit allow_cross_scope at apply time");
    }
    Ok(())
}

fn validate_plan_entry_matches_source_payload(plan: &WritePlan, payload_json: &str) -> Result<()> {
    let payload: Value = serde_json::from_str(payload_json)?;
    if is_toml_plan(plan) {
        let after: TomlValue = toml::from_str(&plan.preview_after)?;
        let selected = after
            .get("mcp_servers")
            .and_then(TomlValue::as_table)
            .and_then(|servers| servers.get(&plan.resource_name))
            .with_context(|| format!("planned MCP resource is missing: {}", plan.resource_name))?;
        validate_toml_entry_matches_source_payload(selected, &payload)?;
    } else {
        let after: Value = serde_json::from_str(&plan.preview_after)?;
        let selected = after
            .get("mcpServers")
            .and_then(Value::as_object)
            .and_then(|servers| servers.get(&plan.resource_name))
            .with_context(|| format!("planned MCP resource is missing: {}", plan.resource_name))?;
        validate_json_entry_matches_source_payload(selected, &payload)?;
    }
    Ok(())
}

fn validate_json_entry_matches_source_payload(entry: &Value, payload: &Value) -> Result<()> {
    validate_json_env_matches_source_payload(entry, payload)?;
    validate_json_entry_fields(entry, payload)?;
    if let Some(url) = payload.get("url").and_then(Value::as_str) {
        if entry.get("url").and_then(Value::as_str) != Some(url) {
            bail!("planned MCP entry does not match source MCP payload");
        }
        let transport = payload
            .get("transport")
            .and_then(Value::as_str)
            .unwrap_or("http");
        if entry.get("type").and_then(Value::as_str) != Some(transport) {
            bail!("planned MCP entry does not match source MCP payload");
        }
        return Ok(());
    }
    if let Some(command) = payload.get("command").and_then(Value::as_str) {
        if entry.get("command").and_then(Value::as_str) != Some(command) {
            bail!("planned MCP entry does not match source MCP payload");
        }
        let expected_args = payload
            .get("args")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        let actual_args = entry
            .get("args")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        if actual_args != expected_args {
            bail!("planned MCP entry does not match source MCP payload");
        }
    }
    Ok(())
}

fn validate_toml_entry_matches_source_payload(entry: &TomlValue, payload: &Value) -> Result<()> {
    validate_toml_env_matches_source_payload(entry, payload)?;
    validate_toml_entry_fields(entry, payload)?;
    if let Some(url) = payload.get("url").and_then(Value::as_str) {
        if entry.get("url").and_then(TomlValue::as_str) != Some(url) {
            bail!("planned MCP entry does not match source MCP payload");
        }
        let transport = payload
            .get("transport")
            .and_then(Value::as_str)
            .unwrap_or("http");
        if entry.get("type").and_then(TomlValue::as_str) != Some(transport) {
            bail!("planned MCP entry does not match source MCP payload");
        }
        return Ok(());
    }
    if let Some(command) = payload.get("command").and_then(Value::as_str) {
        if entry.get("command").and_then(TomlValue::as_str) != Some(command) {
            bail!("planned MCP entry does not match source MCP payload");
        }
        let expected_args = payload
            .get("args")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        let actual_args = entry
            .get("args")
            .and_then(TomlValue::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(TomlValue::as_str)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if actual_args != expected_args {
            bail!("planned MCP entry does not match source MCP payload");
        }
    }
    Ok(())
}

fn expected_entry_fields(payload: &Value) -> BTreeSet<String> {
    let mut fields = BTreeSet::new();
    if payload.get("url").and_then(Value::as_str).is_some() {
        fields.insert("url".to_string());
        fields.insert("type".to_string());
    } else if payload.get("command").and_then(Value::as_str).is_some() {
        fields.insert("command".to_string());
        fields.insert("args".to_string());
    }
    if !source_env_keys(payload).is_empty() {
        fields.insert("env".to_string());
    }
    fields
}

fn validate_json_entry_fields(entry: &Value, payload: &Value) -> Result<()> {
    let expected = expected_entry_fields(payload);
    let actual = entry
        .as_object()
        .context("planned MCP entry must be an object")?
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual != expected {
        bail!("planned MCP entry contains unexpected MCP entry fields");
    }
    Ok(())
}

fn validate_toml_entry_fields(entry: &TomlValue, payload: &Value) -> Result<()> {
    let expected = expected_entry_fields(payload);
    let actual = entry
        .as_table()
        .context("planned MCP entry must be a table")?
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual != expected {
        bail!("planned MCP entry contains unexpected MCP entry fields");
    }
    Ok(())
}

fn source_env_keys(payload: &Value) -> BTreeSet<String> {
    payload
        .get("env_keys")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default()
}

fn validate_json_env_matches_source_payload(entry: &Value, payload: &Value) -> Result<()> {
    let expected_keys = source_env_keys(payload);
    let env = entry.get("env").and_then(Value::as_object);
    let actual_keys = env
        .map(|values| values.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    if actual_keys != expected_keys {
        bail!("planned MCP env keys do not match source MCP payload");
    }
    if let Some(env) = env {
        for key in &expected_keys {
            let value = env.get(key).and_then(Value::as_str).unwrap_or_default();
            validate_plan_env_placeholder(key, value)?;
        }
    }
    Ok(())
}

fn validate_toml_env_matches_source_payload(entry: &TomlValue, payload: &Value) -> Result<()> {
    let expected_keys = source_env_keys(payload);
    let env = entry.get("env").and_then(TomlValue::as_table);
    let actual_keys = env
        .map(|values| values.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    if actual_keys != expected_keys {
        bail!("planned MCP env keys do not match source MCP payload");
    }
    if let Some(env) = env {
        for key in &expected_keys {
            let value = env.get(key).and_then(TomlValue::as_str).unwrap_or_default();
            validate_plan_env_placeholder(key, value)?;
        }
    }
    Ok(())
}

fn validate_plan_env_placeholder(key: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value == format!("<WAPC_MANUAL_ENV:{key}>")
        || value == format!("<WAPC_REUSE_ENV:{key}>")
    {
        return Ok(());
    }
    bail!("planned MCP env values must remain placeholders before apply")
}

fn validate_plan_only_changes_selected_mcp(plan: &WritePlan) -> Result<()> {
    if is_toml_plan(plan) {
        validate_toml_plan_only_changes_selected_mcp(plan)
    } else {
        validate_json_plan_only_changes_selected_mcp(plan)
    }
}

fn validate_json_plan_only_changes_selected_mcp(plan: &WritePlan) -> Result<()> {
    let mut expected: Value = serde_json::from_str(&plan.preview_before)?;
    let after: Value = serde_json::from_str(&plan.preview_after)?;
    let selected_entry = after
        .get("mcpServers")
        .and_then(Value::as_object)
        .and_then(|servers| servers.get(&plan.resource_name))
        .cloned()
        .with_context(|| format!("planned MCP resource is missing: {}", plan.resource_name))?;
    let expected_object = expected
        .as_object_mut()
        .context("target JSON must be an object")?;
    let expected_servers = expected_object
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("target mcpServers must be an object")?;
    expected_servers.insert(plan.resource_name.clone(), selected_entry);

    if canonical_json_value(&expected)? != canonical_json_value(&after)? {
        bail!("apply_sync plans may only modify the selected MCP resource");
    }
    Ok(())
}

fn validate_toml_plan_only_changes_selected_mcp(plan: &WritePlan) -> Result<()> {
    let mut expected: TomlValue = toml::from_str(&plan.preview_before)?;
    let after: TomlValue = toml::from_str(&plan.preview_after)?;
    let selected_entry = after
        .get("mcp_servers")
        .and_then(TomlValue::as_table)
        .and_then(|servers| servers.get(&plan.resource_name))
        .cloned()
        .with_context(|| format!("planned MCP resource is missing: {}", plan.resource_name))?;
    let expected_root = expected
        .as_table_mut()
        .context("target TOML must be a table")?;
    let expected_servers = expected_root
        .entry("mcp_servers".to_string())
        .or_insert_with(|| TomlValue::Table(toml::map::Map::new()))
        .as_table_mut()
        .context("target mcp_servers must be a table")?;
    expected_servers.insert(plan.resource_name.clone(), selected_entry);

    if canonical_toml_value(&expected)? != canonical_toml_value(&after)? {
        bail!("apply_sync plans may only modify the selected MCP resource");
    }
    Ok(())
}

fn materialize_env_placeholders(
    mut plan: WritePlan,
    env_values: &BTreeMap<String, String>,
) -> Result<WritePlan> {
    if !plan.preview_after.contains("<WAPC_") {
        return Ok(plan);
    }
    let after = if is_toml_plan(&plan) {
        let mut value: TomlValue = toml::from_str(&plan.preview_after)?;
        replace_toml_env_placeholders(&mut value, env_values, &plan)?;
        canonical_toml_value(&value)?
    } else {
        let mut value: Value = serde_json::from_str(&plan.preview_after)?;
        replace_env_placeholders(&mut value, env_values, &plan)?;
        canonical_json_value(&value)?
    };
    plan.after_fingerprint = sha256_hex(after.as_bytes());
    plan.diff = line_diff(&plan.preview_before, &after);
    plan.preview_after = after;
    Ok(plan)
}

#[cfg(test)]
fn canonical_json_preview(content: &str) -> Result<String> {
    let value: Value = serde_json::from_str(content)?;
    canonical_json_value(&value)
}

fn canonical_json_value(value: &Value) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(value)?))
}

#[cfg(test)]
fn canonical_toml_preview(content: &str) -> Result<String> {
    let value: TomlValue = toml::from_str(content)?;
    canonical_toml_value(&value)
}

fn canonical_toml_value(value: &TomlValue) -> Result<String> {
    Ok(toml::to_string_pretty(value)?)
}

fn replace_env_placeholders(
    value: &mut Value,
    env_values: &BTreeMap<String, String>,
    plan: &WritePlan,
) -> Result<()> {
    match value {
        Value::String(text) if text.starts_with("<WAPC_MANUAL_ENV:") => {
            let key = placeholder_key(text, "<WAPC_MANUAL_ENV:")?;
            let value = env_values
                .get(key)
                .with_context(|| format!("missing manual env value for {key}"))?;
            *text = value.clone();
        }
        Value::String(text) if text.starts_with("<WAPC_REUSE_ENV:") => {
            let key = placeholder_key(text, "<WAPC_REUSE_ENV:")?;
            let value = existing_target_env_value(plan, key)?;
            *text = value;
        }
        Value::Array(values) => {
            for item in values {
                replace_env_placeholders(item, env_values, plan)?;
            }
        }
        Value::Object(object) => {
            for item in object.values_mut() {
                replace_env_placeholders(item, env_values, plan)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn replace_toml_env_placeholders(
    value: &mut TomlValue,
    env_values: &BTreeMap<String, String>,
    plan: &WritePlan,
) -> Result<()> {
    match value {
        TomlValue::String(text) if text.starts_with("<WAPC_MANUAL_ENV:") => {
            let key = placeholder_key(text, "<WAPC_MANUAL_ENV:")?;
            let value = env_values
                .get(key)
                .with_context(|| format!("missing manual env value for {key}"))?;
            *text = value.clone();
        }
        TomlValue::String(text) if text.starts_with("<WAPC_REUSE_ENV:") => {
            let key = placeholder_key(text, "<WAPC_REUSE_ENV:")?;
            let value = existing_target_env_value(plan, key)?;
            *text = value;
        }
        TomlValue::Array(values) => {
            for item in values {
                replace_toml_env_placeholders(item, env_values, plan)?;
            }
        }
        TomlValue::Table(table) => {
            for (_, item) in table.iter_mut() {
                replace_toml_env_placeholders(item, env_values, plan)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn placeholder_key<'a>(value: &'a str, prefix: &str) -> Result<&'a str> {
    value
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix('>'))
        .context("invalid env placeholder")
}

fn existing_target_env_value(plan: &WritePlan, key: &str) -> Result<String> {
    let content = std::fs::read_to_string(&plan.target_path)?;
    if is_toml_plan(plan) {
        let value: TomlValue = toml::from_str(&content)?;
        return value
            .get("mcp_servers")
            .and_then(TomlValue::as_table)
            .and_then(|servers| servers.get(&plan.resource_name))
            .and_then(|entry| entry.get("env"))
            .and_then(TomlValue::as_table)
            .and_then(|env| env.get(key))
            .and_then(TomlValue::as_str)
            .map(ToOwned::to_owned)
            .with_context(|| format!("target env value for {key} is unavailable for reuse"));
    }
    let value: Value = serde_json::from_str(&content)?;
    value
        .get("mcpServers")
        .and_then(Value::as_object)
        .and_then(|servers| servers.get(&plan.resource_name))
        .and_then(|entry| entry.get("env"))
        .and_then(Value::as_object)
        .and_then(|env| env.get(key))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .with_context(|| format!("target env value for {key} is unavailable for reuse"))
}

fn is_toml_plan(plan: &WritePlan) -> bool {
    plan.tool == "codex" || plan.target_path.ends_with(".toml")
}

fn ensure_failed_change_record(
    store: &UsageStore,
    sync_id: &str,
    metadata: FailedChangeMetadata<'_>,
) -> Result<String> {
    let existing = store
        .list_resource_changes(None)?
        .into_iter()
        .find(|change| {
            change.sync_id.as_deref() == Some(sync_id) && change.target_path == metadata.target_path
        });
    if let Some(change) = existing {
        return Ok(change.change_id);
    }
    let now = Utc::now().to_rfc3339();
    let change_id = format!(
        "chg:{}",
        sha256_8(&format!(
            "failed\n{}\n{}\n{}\n{}",
            sync_id, metadata.plan_id, metadata.target_path, now
        ))
    );
    store.insert_resource_change(&ResourceChangeLog {
        change_id: change_id.clone(),
        sync_id: Some(sync_id.to_string()),
        tool: metadata.tool.to_string(),
        resource_id: metadata.resource_id.map(ToOwned::to_owned),
        kind: metadata.kind.to_string(),
        op: metadata.op.to_string(),
        target_path: metadata.target_path.to_string(),
        backup_path: None,
        status: "failed".to_string(),
        reverts_change_id: None,
        created_at: now,
    })?;
    Ok(change_id)
}

struct FailedChangeMetadata<'a> {
    plan_id: &'a str,
    tool: &'a str,
    resource_id: Option<&'a str>,
    kind: &'a str,
    op: &'a str,
    target_path: &'a str,
}

fn plan_target(
    store: &UsageStore,
    source: &CanonicalResource,
    target: &SyncTarget,
    request: &PlanSyncRequest,
    created_at: &str,
) -> SyncTargetPlan {
    if target.scope == "enterprise" || target.scope == "managed" {
        return target_error(target, "unsupported", "enterprise targets are read-only");
    }
    if source.scope != target.scope && !request.allow_cross_scope {
        return target_error(
            target,
            "unsupported",
            "cross-scope sync requires explicit allow_cross_scope",
        );
    }
    if let Err(reason) = validate_project_target_path(target) {
        return target_error(target, "unsupported", reason.as_str());
    }

    if source.kind == "instruction" {
        let planned = plan_instruction_target(store, source, target, request, created_at);
        match planned {
            Ok(success) => {
                return SyncTargetPlan {
                    target: target.clone(),
                    status: "planned".to_string(),
                    reason: None,
                    required_env_keys: success.required_env_keys,
                    plan: Some(success.plan),
                }
            }
            Err(TargetPlanError::Unsupported(reason)) => {
                return target_error(target, "unsupported", reason.as_str())
            }
            Err(TargetPlanError::RequiresInput(keys, reason)) => {
                return SyncTargetPlan {
                    target: target.clone(),
                    status: "requires_input".to_string(),
                    reason: Some(reason),
                    required_env_keys: keys,
                    plan: None,
                }
            }
        }
    }

    let planned = match (target.tool.as_str(), target.format.as_str()) {
        ("codex", "toml") => plan_toml_mcp_target(store, source, target, request, created_at),
        ("claude" | "gemini" | "cursor", "json") => {
            plan_json_mcp_target(store, source, target, request, created_at)
        }
        _ => Err(TargetPlanError::Unsupported(format!(
            "target format/tool is unsupported for Phase 4 sync: tool={} format={}",
            target.tool, target.format
        ))),
    };

    match planned {
        Ok(success) => SyncTargetPlan {
            target: target.clone(),
            status: "planned".to_string(),
            reason: None,
            required_env_keys: success.required_env_keys,
            plan: Some(success.plan),
        },
        Err(TargetPlanError::RequiresInput(keys, reason)) => SyncTargetPlan {
            target: target.clone(),
            status: "requires_input".to_string(),
            reason: Some(reason),
            required_env_keys: keys,
            plan: None,
        },
        Err(TargetPlanError::Unsupported(reason)) => {
            target_error(target, "unsupported", reason.as_str())
        }
    }
}

fn validate_project_target_path(target: &SyncTarget) -> std::result::Result<(), String> {
    if target.scope != "project" {
        return Ok(());
    }
    let Some(project_path) = target
        .project_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return Err("project targets require explicit project_path".to_string());
    };
    if !Path::new(&target.target_path).starts_with(Path::new(project_path)) {
        return Err("project target path must be under project_path".to_string());
    }
    Ok(())
}

fn plan_json_mcp_target(
    store: &UsageStore,
    source: &CanonicalResource,
    target: &SyncTarget,
    request: &PlanSyncRequest,
    created_at: &str,
) -> std::result::Result<TargetPlanSuccess, TargetPlanError> {
    if !["reuse", "manual", "skip"].contains(&request.env_strategy.as_str()) {
        return Err(TargetPlanError::Unsupported(format!(
            "unsupported env strategy: {}",
            request.env_strategy
        )));
    }
    let payload = serde_json::from_str::<Value>(&source.payload_json)
        .map_err(|err| TargetPlanError::Unsupported(format!("invalid source payload: {err}")))?;
    let env_keys = payload
        .get("env_keys")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let before = std::fs::read_to_string(&target.target_path).map_err(|err| {
        TargetPlanError::Unsupported(format!("failed to read target file: {err}"))
    })?;
    let mut value = serde_json::from_str::<Value>(&before)
        .map_err(|err| TargetPlanError::Unsupported(format!("invalid target JSON: {err}")))?;
    let env_entries = env_entries_for_plan(
        &value,
        &source.name,
        &env_keys,
        request.env_strategy.as_str(),
    )?;
    let entry = json_mcp_entry(&payload, &env_entries)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| TargetPlanError::Unsupported("target JSON must be an object".to_string()))?;
    let servers = object
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| {
            TargetPlanError::Unsupported("target mcpServers must be an object".to_string())
        })?;
    servers.insert(source.name.clone(), entry);

    let after = canonical_json_value(&value)
        .map_err(|err| TargetPlanError::Unsupported(err.to_string()))?;
    let before_fingerprint = sha256_hex(before.as_bytes());
    let after_fingerprint = sha256_hex(after.as_bytes());
    store
        .record_file_fingerprint(
            &target.tool,
            &target.target_path,
            &before_fingerprint,
            created_at,
        )
        .map_err(|err| TargetPlanError::Unsupported(err.to_string()))?;

    Ok(TargetPlanSuccess {
        required_env_keys: env_keys,
        plan: WritePlan {
            plan_id: sync_plan_id(
                source,
                target,
                &before_fingerprint,
                &after_fingerprint,
                created_at,
            ),
            tool: target.tool.clone(),
            kind: source.kind.clone(),
            op: "sync".to_string(),
            resource_id: Some(source.id.clone()),
            resource_name: source.name.clone(),
            target_path: target.target_path.clone(),
            target_scope: Some(target.scope.clone()),
            target_project_path: target.project_path.clone(),
            before_fingerprint,
            after_fingerprint,
            diff: line_diff(&before, &after),
            preview_before: before,
            preview_after: after,
            requires_backup: true,
            risks: cross_tool_write_risks(),
            created_at: created_at.to_string(),
        },
    })
}

fn plan_toml_mcp_target(
    store: &UsageStore,
    source: &CanonicalResource,
    target: &SyncTarget,
    request: &PlanSyncRequest,
    created_at: &str,
) -> std::result::Result<TargetPlanSuccess, TargetPlanError> {
    if !["reuse", "manual", "skip"].contains(&request.env_strategy.as_str()) {
        return Err(TargetPlanError::Unsupported(format!(
            "unsupported env strategy: {}",
            request.env_strategy
        )));
    }
    let payload = serde_json::from_str::<Value>(&source.payload_json)
        .map_err(|err| TargetPlanError::Unsupported(format!("invalid source payload: {err}")))?;
    let env_keys = payload
        .get("env_keys")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let before = std::fs::read_to_string(&target.target_path).map_err(|err| {
        TargetPlanError::Unsupported(format!("failed to read target file: {err}"))
    })?;
    let mut value = toml::from_str::<TomlValue>(&before)
        .map_err(|err| TargetPlanError::Unsupported(format!("invalid target TOML: {err}")))?;
    let env_entries = toml_env_entries_for_plan(
        &value,
        &source.name,
        &env_keys,
        request.env_strategy.as_str(),
    )?;
    let entry = toml_mcp_entry(&payload, &env_entries)?;
    let root = value
        .as_table_mut()
        .ok_or_else(|| TargetPlanError::Unsupported("target TOML must be a table".to_string()))?;
    let servers = root
        .entry("mcp_servers".to_string())
        .or_insert_with(|| TomlValue::Table(toml::map::Map::new()))
        .as_table_mut()
        .ok_or_else(|| {
            TargetPlanError::Unsupported("target mcp_servers must be a table".to_string())
        })?;
    servers.insert(source.name.clone(), entry);

    let after = canonical_toml_value(&value)
        .map_err(|err| TargetPlanError::Unsupported(err.to_string()))?;
    let before_fingerprint = sha256_hex(before.as_bytes());
    let after_fingerprint = sha256_hex(after.as_bytes());
    store
        .record_file_fingerprint(
            &target.tool,
            &target.target_path,
            &before_fingerprint,
            created_at,
        )
        .map_err(|err| TargetPlanError::Unsupported(err.to_string()))?;

    Ok(TargetPlanSuccess {
        required_env_keys: env_keys,
        plan: WritePlan {
            plan_id: sync_plan_id(
                source,
                target,
                &before_fingerprint,
                &after_fingerprint,
                created_at,
            ),
            tool: target.tool.clone(),
            kind: source.kind.clone(),
            op: "sync".to_string(),
            resource_id: Some(source.id.clone()),
            resource_name: source.name.clone(),
            target_path: target.target_path.clone(),
            target_scope: Some(target.scope.clone()),
            target_project_path: target.project_path.clone(),
            before_fingerprint,
            after_fingerprint,
            diff: line_diff(&before, &after),
            preview_before: before,
            preview_after: after,
            requires_backup: true,
            risks: cross_tool_write_risks(),
            created_at: created_at.to_string(),
        },
    })
}

fn cross_tool_write_risks() -> Vec<WritePlanRisk> {
    vec![WritePlanRisk {
        code: "cross_tool_config_write".to_string(),
        message: "Will modify a local target tool MCP configuration after confirmation."
            .to_string(),
        severity: "high".to_string(),
    }]
}

fn plan_instruction_target(
    _store: &UsageStore,
    source: &CanonicalResource,
    target: &SyncTarget,
    _request: &PlanSyncRequest,
    created_at: &str,
) -> std::result::Result<TargetPlanSuccess, TargetPlanError> {
    let source_path = Path::new(&source.origin_path);
    let after_content = std::fs::read_to_string(source_path).map_err(|err| {
        TargetPlanError::Unsupported(format!(
            "Failed to read original source instruction file: {err}"
        ))
    })?;

    let target_path = Path::new(&target.target_path);
    let before_content = if target_path.exists() {
        std::fs::read_to_string(target_path).map_err(|err| {
            TargetPlanError::Unsupported(format!(
                "Failed to read existing target instruction file: {err}"
            ))
        })?
    } else {
        "".to_string()
    };

    let before_fingerprint = sha256_hex(before_content.as_bytes());
    let after_fingerprint = sha256_hex(after_content.as_bytes());

    let plan_id = sync_plan_id_from_parts(SyncPlanIdParts {
        source_id: &source.id,
        tool: &target.tool,
        target_path: &target.target_path,
        target_scope: Some(&target.scope),
        target_project_path: target.project_path.as_deref(),
        before_fingerprint: &before_fingerprint,
        after_fingerprint: &after_fingerprint,
        created_at,
    });

    let plan = WritePlan {
        plan_id,
        tool: target.tool.clone(),
        kind: source.kind.clone(),
        op: "sync".to_string(),
        resource_id: Some(source.id.clone()),
        resource_name: source.name.clone(),
        target_path: target.target_path.clone(),
        target_scope: Some(target.scope.clone()),
        target_project_path: target.project_path.clone(),
        before_fingerprint,
        after_fingerprint,
        diff: line_diff(&before_content, &after_content),
        preview_before: before_content,
        preview_after: after_content,
        requires_backup: true,
        risks: cross_tool_write_risks(),
        created_at: created_at.to_string(),
    };

    Ok(TargetPlanSuccess {
        plan,
        required_env_keys: vec![],
    })
}

struct TargetPlanSuccess {
    plan: WritePlan,
    required_env_keys: Vec<String>,
}

fn env_entries_for_plan(
    target_value: &Value,
    resource_name: &str,
    env_keys: &[String],
    env_strategy: &str,
) -> std::result::Result<BTreeMap<String, String>, TargetPlanError> {
    let mut entries = BTreeMap::new();
    for key in env_keys {
        let value = match env_strategy {
            "manual" => format!("<WAPC_MANUAL_ENV:{key}>"),
            "skip" => String::new(),
            "reuse" => {
                let existing = target_value
                    .get("mcpServers")
                    .and_then(Value::as_object)
                    .and_then(|servers| servers.get(resource_name))
                    .and_then(|entry| entry.get("env"))
                    .and_then(Value::as_object)
                    .and_then(|env| env.get(key))
                    .and_then(Value::as_str);
                if existing.is_none() {
                    return Err(TargetPlanError::RequiresInput(
                        env_keys.to_vec(),
                        "target does not contain existing env values for reuse".to_string(),
                    ));
                }
                format!("<WAPC_REUSE_ENV:{key}>")
            }
            _ => {
                return Err(TargetPlanError::Unsupported(format!(
                    "unsupported env strategy: {env_strategy}"
                )));
            }
        };
        entries.insert(key.clone(), value);
    }
    Ok(entries)
}

fn toml_env_entries_for_plan(
    target_value: &TomlValue,
    resource_name: &str,
    env_keys: &[String],
    env_strategy: &str,
) -> std::result::Result<BTreeMap<String, String>, TargetPlanError> {
    let mut entries = BTreeMap::new();
    for key in env_keys {
        let value = match env_strategy {
            "manual" => format!("<WAPC_MANUAL_ENV:{key}>"),
            "skip" => String::new(),
            "reuse" => {
                let existing = target_value
                    .get("mcp_servers")
                    .and_then(TomlValue::as_table)
                    .and_then(|servers| servers.get(resource_name))
                    .and_then(|entry| entry.get("env"))
                    .and_then(TomlValue::as_table)
                    .and_then(|env| env.get(key))
                    .and_then(TomlValue::as_str);
                if existing.is_none() {
                    return Err(TargetPlanError::RequiresInput(
                        env_keys.to_vec(),
                        "target does not contain existing env values for reuse".to_string(),
                    ));
                }
                format!("<WAPC_REUSE_ENV:{key}>")
            }
            _ => {
                return Err(TargetPlanError::Unsupported(format!(
                    "unsupported env strategy: {env_strategy}"
                )));
            }
        };
        entries.insert(key.clone(), value);
    }
    Ok(entries)
}

fn json_mcp_entry(
    payload: &Value,
    env_entries: &BTreeMap<String, String>,
) -> std::result::Result<Value, TargetPlanError> {
    let url = payload.get("url").and_then(Value::as_str);
    if let Some(url) = url {
        let transport = payload
            .get("transport")
            .and_then(Value::as_str)
            .unwrap_or("http");
        let mut entry = json!({
            "url": url,
            "type": transport,
        });
        if !env_entries.is_empty() {
            entry["env"] = json!(env_entries);
        }
        return Ok(entry);
    }
    if let Some(command) = payload.get("command").and_then(Value::as_str) {
        let args = payload
            .get("args")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut entry = json!({
            "command": command,
            "args": args,
        });
        if !env_entries.is_empty() {
            entry["env"] = json!(env_entries);
        }
        return Ok(entry);
    }
    Err(TargetPlanError::Unsupported(
        "redacted command MCP resources cannot be reconstructed into a target config yet"
            .to_string(),
    ))
}

fn toml_mcp_entry(
    payload: &Value,
    env_entries: &BTreeMap<String, String>,
) -> std::result::Result<TomlValue, TargetPlanError> {
    let Some(url) = payload.get("url").and_then(Value::as_str) else {
        if let Some(command) = payload.get("command").and_then(Value::as_str) {
            let args = payload
                .get("args")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|value| TomlValue::String(value.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let mut entry = toml::map::Map::new();
            entry.insert(
                "command".to_string(),
                TomlValue::String(command.to_string()),
            );
            entry.insert("args".to_string(), TomlValue::Array(args));
            if !env_entries.is_empty() {
                entry.insert(
                    "env".to_string(),
                    TomlValue::Table(
                        env_entries
                            .iter()
                            .map(|(key, value)| (key.clone(), TomlValue::String(value.clone())))
                            .collect(),
                    ),
                );
            }
            return Ok(TomlValue::Table(entry));
        }
        return Err(TargetPlanError::Unsupported(
            "redacted command MCP resources cannot be reconstructed into a target config yet"
                .to_string(),
        ));
    };
    let transport = payload
        .get("transport")
        .and_then(Value::as_str)
        .unwrap_or("http");
    let mut entry = toml::map::Map::new();
    entry.insert("url".to_string(), TomlValue::String(url.to_string()));
    entry.insert("type".to_string(), TomlValue::String(transport.to_string()));
    if !env_entries.is_empty() {
        entry.insert(
            "env".to_string(),
            TomlValue::Table(
                env_entries
                    .iter()
                    .map(|(key, value)| (key.clone(), TomlValue::String(value.clone())))
                    .collect(),
            ),
        );
    }
    Ok(TomlValue::Table(entry))
}

fn target_error(target: &SyncTarget, status: &str, reason: &str) -> SyncTargetPlan {
    SyncTargetPlan {
        target: target.clone(),
        status: status.to_string(),
        reason: Some(reason.to_string()),
        required_env_keys: Vec::new(),
        plan: None,
    }
}

enum TargetPlanError {
    Unsupported(String),
    RequiresInput(Vec<String>, String),
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

fn sync_plan_id(
    source: &CanonicalResource,
    target: &SyncTarget,
    before_fingerprint: &str,
    after_fingerprint: &str,
    created_at: &str,
) -> String {
    sync_plan_id_from_parts(SyncPlanIdParts {
        source_id: &source.id,
        tool: &target.tool,
        target_path: &target.target_path,
        target_scope: Some(&target.scope),
        target_project_path: target.project_path.as_deref(),
        before_fingerprint,
        after_fingerprint,
        created_at,
    })
}

struct SyncPlanIdParts<'a> {
    source_id: &'a str,
    tool: &'a str,
    target_path: &'a str,
    target_scope: Option<&'a str>,
    target_project_path: Option<&'a str>,
    before_fingerprint: &'a str,
    after_fingerprint: &'a str,
    created_at: &'a str,
}

fn sync_plan_id_from_parts(parts: SyncPlanIdParts<'_>) -> String {
    if parts.target_scope.is_none() && parts.target_project_path.is_none() {
        return legacy_sync_plan_id_from_parts(
            parts.source_id,
            parts.tool,
            parts.target_path,
            parts.before_fingerprint,
            parts.after_fingerprint,
            parts.created_at,
        );
    }
    format!(
        "sync-plan:{}{}",
        sha256_8(&format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            parts.source_id,
            parts.tool,
            parts.target_path,
            parts.target_scope.unwrap_or(""),
            parts.target_project_path.unwrap_or(""),
            parts.before_fingerprint
        )),
        sha256_8(&format!(
            "{}\n{}",
            parts.after_fingerprint, parts.created_at
        ))
    )
}

fn legacy_sync_plan_id_from_parts(
    source_id: &str,
    tool: &str,
    target_path: &str,
    before_fingerprint: &str,
    after_fingerprint: &str,
    created_at: &str,
) -> String {
    format!(
        "sync-plan:{}{}",
        sha256_8(&format!(
            "{}\n{}\n{}\n{}",
            source_id, tool, target_path, before_fingerprint
        )),
        sha256_8(&format!("{after_fingerprint}\n{created_at}"))
    )
}

fn sync_id(plans: &[WritePlan], created_at: &str) -> String {
    let mut source = String::new();
    for plan in plans {
        source.push_str(&plan.plan_id);
        source.push('\n');
        source.push_str(&plan.target_path);
        source.push('\n');
    }
    source.push_str(created_at);
    format!("sync:{}", sha256_8(&source))
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

#[cfg(test)]
mod tests {
    //! Phase 4 cross-tool sync plan behavior tests.
    //! @author codex

    use std::{collections::BTreeMap, fs};

    use tempfile::tempdir;

    use crate::{
        cross_sync::{
            apply_sync, canonical_json_preview, canonical_toml_preview, plan_sync,
            plan_sync_from_resource,
        },
        deep_link,
        model::{
            ApplySyncRequest, CanonicalResource, PlanSyncRequest, PlanTemplateSyncRequest,
            SyncTarget,
        },
        store::UsageStore,
        template_library::{plan_template_sync, seed_builtin_resource_templates},
    };

    fn mcp_resource(id: &str, scope: &str, payload_json: &str) -> CanonicalResource {
        CanonicalResource {
            id: id.to_string(),
            kind: "mcp".to_string(),
            name: "docs".to_string(),
            scope: scope.to_string(),
            origin_tool: "claude".to_string(),
            origin_path: "/tmp/source.json".to_string(),
            origin_locator: Some("mcpServers.docs".to_string()),
            enabled_in: vec!["claude".to_string()],
            confidence: 1.0,
            redacted: false,
            payload_json: payload_json.to_string(),
            provided_by_plugin: None,
            last_seen: "2026-06-06T00:00:00Z".to_string(),
        }
    }

    fn url_payload() -> &'static str {
        r#"{"transport":"http","command":null,"args":[],"url":"https://example.test/mcp","env_keys":[],"env_fingerprints":{}}"#
    }

    fn env_payload() -> &'static str {
        r#"{"transport":"http","command":null,"args":[],"url":"https://example.test/mcp","env_keys":["GITHUB_TOKEN"],"env_fingerprints":{"GITHUB_TOKEN":{"sha256_8":"abc12345"}}}"#
    }

    fn target(tool: &str, scope: &str, path: &std::path::Path, format: &str) -> SyncTarget {
        SyncTarget {
            tool: tool.to_string(),
            scope: scope.to_string(),
            project_path: None,
            target_path: path.display().to_string(),
            format: format.to_string(),
        }
    }

    fn project_target(
        tool: &str,
        project_path: &std::path::Path,
        path: &std::path::Path,
    ) -> SyncTarget {
        SyncTarget {
            tool: tool.to_string(),
            scope: "project".to_string(),
            project_path: Some(project_path.display().to_string()),
            target_path: path.display().to_string(),
            format: "json".to_string(),
        }
    }

    #[test]
    fn plan_sync_json_mcp_generates_one_plan_per_supported_target_without_writing() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let source = mcp_resource("mcp:docs", "user", url_payload());
        store.upsert_resources(&[source]).unwrap();
        let claude = dir.path().join(".claude.json");
        let gemini = dir.path().join(".gemini/settings.json");
        fs::create_dir_all(gemini.parent().unwrap()).unwrap();
        fs::write(
            &claude,
            r#"{"mcpServers":{"old":{"url":"https://old.test"}}}"#,
        )
        .unwrap();
        fs::write(&gemini, r#"{"mcpServers":{}}"#).unwrap();
        let before_claude = fs::read_to_string(&claude).unwrap();
        let before_gemini = fs::read_to_string(&gemini).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![
                    target("claude", "user", &claude, "json"),
                    target("gemini", "user", &gemini, "json"),
                ],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets.len(), 2);
        assert!(
            result
                .targets
                .iter()
                .all(|target| target.status == "planned")
        );
        assert!(result.targets.iter().all(|target| target.plan.is_some()));
        assert!(result.targets.iter().all(|target| target.reason.is_none()));
        assert!(result.targets.iter().all(|target| {
            target
                .plan
                .as_ref()
                .unwrap()
                .preview_after
                .contains("\"docs\"")
        }));
        assert_eq!(fs::read_to_string(&claude).unwrap(), before_claude);
        assert_eq!(fs::read_to_string(&gemini).unwrap(), before_gemini);
    }

    #[test]
    fn plan_sync_generates_codex_toml_plan_and_apply_writes_mcp_server() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let codex = dir.path().join(".codex/config.toml");
        fs::create_dir_all(codex.parent().unwrap()).unwrap();
        fs::write(&codex, "[mcp_servers]\n").unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("codex", "user", &codex, "toml")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets[0].status, "planned");
        let plan = result.targets[0].plan.clone().unwrap();
        assert!(plan.preview_after.contains("[mcp_servers.docs]"));
        assert!(
            plan.preview_after
                .contains("url = \"https://example.test/mcp\"")
        );
        assert_eq!(fs::read_to_string(&codex).unwrap(), "[mcp_servers]\n");

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "committed");
        let updated = fs::read_to_string(&codex).unwrap();
        assert!(updated.contains("[mcp_servers.docs]"));
        assert!(updated.contains("url = \"https://example.test/mcp\""));
    }

    #[test]
    fn apply_sync_rejects_forged_plan_without_source_resource_id() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let claude = dir.path().join(".claude.json");
        fs::write(&claude, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &claude, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.resource_id = None;

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("resource_id")
        );
        assert_eq!(fs::read_to_string(&claude).unwrap(), r#"{"mcpServers":{}}"#);
    }

    #[test]
    fn apply_sync_rejects_mixed_source_resources_before_writing_or_history() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let docs = mcp_resource("mcp:docs", "user", url_payload());
        let mut github = mcp_resource("mcp:github", "user", url_payload());
        github.name = "github".to_string();
        store.upsert_resources(&[docs, github]).unwrap();
        let claude = dir.path().join(".claude.json");
        let cursor = dir.path().join(".cursor/mcp.json");
        fs::create_dir_all(cursor.parent().unwrap()).unwrap();
        fs::write(&claude, r#"{"mcpServers":{}}"#).unwrap();
        fs::write(&cursor, r#"{"mcpServers":{}}"#).unwrap();

        let docs_plan = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &claude, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap()
        .targets[0]
            .plan
            .clone()
            .unwrap();
        let github_plan = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:github".to_string(),
                targets: vec![target("cursor", "user", &cursor, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap()
        .targets[0]
            .plan
            .clone()
            .unwrap();

        let error = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![docs_plan, github_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("single source resource"));
        assert_eq!(fs::read_to_string(&claude).unwrap(), r#"{"mcpServers":{}}"#);
        assert_eq!(fs::read_to_string(&cursor).unwrap(), r#"{"mcpServers":{}}"#);
        assert!(store.list_sync_operations().unwrap().is_empty());
        assert!(store.list_resource_changes(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_template_plan_name_mismatch_before_backup_or_write() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        seed_builtin_resource_templates(&store).unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_template_sync(
            dir.path(),
            &store,
            PlanTemplateSyncRequest {
                template_id: "builtin:context7-mcp".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "skip".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.resource_name = "not-context7".to_string();

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("skip".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("template resource name mismatch")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, "failed");
        assert!(changes[0].backup_path.is_none());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_that_injects_unrelated_mcp_entry() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        let mut after: serde_json::Value =
            serde_json::from_str(&forged_plan.preview_after).unwrap();
        after["mcpServers"]["evil"] = serde_json::json!({
            "url": "https://evil.example.test/mcp",
            "type": "http"
        });
        forged_plan.preview_after = super::canonical_json_value(&after).unwrap();
        forged_plan.after_fingerprint = super::sha256_hex(forged_plan.preview_after.as_bytes());
        forged_plan.diff =
            super::line_diff(&forged_plan.preview_before, &forged_plan.preview_after);

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("only modify the selected MCP resource")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_that_mutates_selected_mcp_endpoint() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        let mut after: serde_json::Value =
            serde_json::from_str(&forged_plan.preview_after).unwrap();
        after["mcpServers"]["docs"]["url"] = serde_json::json!("https://evil.example.test/mcp");
        forged_plan.preview_after = super::canonical_json_value(&after).unwrap();
        forged_plan.after_fingerprint = super::sha256_hex(forged_plan.preview_after.as_bytes());
        forged_plan.diff =
            super::line_diff(&forged_plan.preview_before, &forged_plan.preview_after);

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("does not match source MCP payload")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_that_mutates_selected_mcp_transport() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        let mut after: serde_json::Value =
            serde_json::from_str(&forged_plan.preview_after).unwrap();
        after["mcpServers"]["docs"]["type"] = serde_json::json!("sse");
        forged_plan.preview_after = super::canonical_json_value(&after).unwrap();
        forged_plan.after_fingerprint = super::sha256_hex(forged_plan.preview_after.as_bytes());
        forged_plan.diff =
            super::line_diff(&forged_plan.preview_before, &forged_plan.preview_after);

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("does not match source MCP payload")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_with_raw_env_value_in_preview() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", env_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "manual".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        let mut after: serde_json::Value =
            serde_json::from_str(&forged_plan.preview_after).unwrap();
        after["mcpServers"]["docs"]["env"]["GITHUB_TOKEN"] =
            serde_json::json!("ghp_raw_secret_should_not_be_in_preview");
        forged_plan.preview_after = super::canonical_json_value(&after).unwrap();
        forged_plan.after_fingerprint = super::sha256_hex(forged_plan.preview_after.as_bytes());
        forged_plan.diff =
            super::line_diff(&forged_plan.preview_before, &forged_plan.preview_after);

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("manual".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("env values must remain placeholders")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_with_unrelated_selected_mcp_field() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        let mut after: serde_json::Value =
            serde_json::from_str(&forged_plan.preview_after).unwrap();
        after["mcpServers"]["docs"]["headers"] = serde_json::json!({
            "Authorization": "Bearer raw-secret-in-forged-plan"
        });
        forged_plan.preview_after = super::canonical_json_value(&after).unwrap();
        forged_plan.after_fingerprint = super::sha256_hex(forged_plan.preview_after.as_bytes());
        forged_plan.diff =
            super::line_diff(&forged_plan.preview_before, &forged_plan.preview_after);

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("unexpected MCP entry fields")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_with_inconsistent_after_fingerprint_before_backup() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.after_fingerprint = "not-the-preview-after-sha256".to_string();

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("fingerprint")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].backup_path.is_none());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_retargeted_to_unplanned_file_before_backup() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let planned_file = dir.path().join(".claude.json");
        let unplanned_file = dir.path().join(".cursor/mcp.json");
        fs::create_dir_all(unplanned_file.parent().unwrap()).unwrap();
        fs::write(&planned_file, r#"{"mcpServers":{}}"#).unwrap();
        fs::write(&unplanned_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &planned_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.target_path = unplanned_file.display().to_string();

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("target fingerprint")
        );
        assert_eq!(
            fs::read_to_string(&planned_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert_eq!(
            fs::read_to_string(&unplanned_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].backup_path.is_none());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_with_forged_plan_id_before_backup() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.plan_id = "sync-plan:forged-audit-id".to_string();

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("plan_id")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].backup_path.is_none());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_with_forged_diff_before_backup() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.diff = "+nothing to see here".to_string();

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("diff")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].backup_path.is_none());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_with_forged_backup_requirement_before_backup() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.requires_backup = false;

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("backup")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].backup_path.is_none());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_with_forged_risk_metadata_before_backup() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.risks.clear();

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("risk")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].backup_path.is_none());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn sync_preview_formatting_is_idempotent_for_json_and_toml_targets() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let claude = dir.path().join(".claude.json");
        let codex = dir.path().join(".codex/config.toml");
        fs::create_dir_all(codex.parent().unwrap()).unwrap();
        fs::write(&claude, r#"{"mcpServers":{}}"#).unwrap();
        fs::write(&codex, "[mcp_servers]\n").unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![
                    target("claude", "user", &claude, "json"),
                    target("codex", "user", &codex, "toml"),
                ],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        let json_plan = result.targets[0].plan.as_ref().unwrap();
        let toml_plan = result.targets[1].plan.as_ref().unwrap();
        assert_eq!(
            json_plan.preview_after,
            canonical_json_preview(&json_plan.preview_after).unwrap()
        );
        assert_eq!(
            toml_plan.preview_after,
            canonical_toml_preview(&toml_plan.preview_after).unwrap()
        );
    }

    #[test]
    fn plan_sync_reports_unsupported_targets_without_fake_plans() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".unsupported/config.yml");
        fs::create_dir_all(target_file.parent().unwrap()).unwrap();
        fs::write(&target_file, "mcpServers: {}\n").unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("unknown", "user", &target_file, "yaml")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets[0].status, "unsupported");
        assert!(result.targets[0].plan.is_none());
        assert!(
            result.targets[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("format")
        );
    }

    #[test]
    fn plan_sync_rejects_cross_scope_by_default() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "project", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets[0].status, "unsupported");
        assert!(result.targets[0].plan.is_none());
        assert!(
            result.targets[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("cross-scope")
        );
    }

    #[test]
    fn plan_sync_rejects_project_target_without_explicit_project_path() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join("repo/.cursor/mcp.json");
        fs::create_dir_all(target_file.parent().unwrap()).unwrap();
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("cursor", "project", &target_file, "json")],
                allow_cross_scope: true,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets[0].status, "unsupported");
        assert!(result.targets[0].plan.is_none());
        assert!(
            result.targets[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("project_path")
        );
    }

    #[test]
    fn plan_sync_accepts_project_target_with_explicit_project_path_and_apply_writes_it() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let project_dir = dir.path().join("repo");
        let target_file = project_dir.join(".cursor/mcp.json");
        fs::create_dir_all(target_file.parent().unwrap()).unwrap();
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let before = fs::read_to_string(&target_file).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![project_target("cursor", &project_dir, &target_file)],
                allow_cross_scope: true,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets[0].status, "planned");
        let plan = result.targets[0].plan.clone().unwrap();
        assert!(plan.preview_after.contains("\"docs\""));
        assert_eq!(fs::read_to_string(&target_file).unwrap(), before);

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![plan],
                confirm_drift: false,
                allow_cross_scope: true,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "committed");
        assert!(
            fs::read_to_string(&target_file)
                .unwrap()
                .contains("\"docs\"")
        );
        let operations = store.list_sync_operations().unwrap();
        let targets: Vec<serde_json::Value> =
            serde_json::from_str(&operations[0].targets_json).unwrap();
        assert_eq!(targets[0]["scope"], "project");
        assert_eq!(
            targets[0]["project_path"],
            project_dir.display().to_string()
        );
    }

    #[test]
    fn apply_sync_rejects_plan_with_forged_target_scope_metadata_before_history_write() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let project_dir = dir.path().join("repo");
        let target_file = project_dir.join(".cursor/mcp.json");
        fs::create_dir_all(target_file.parent().unwrap()).unwrap();
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![project_target("cursor", &project_dir, &target_file)],
                allow_cross_scope: true,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.target_scope = Some("user".to_string());
        forged_plan.target_project_path = None;

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: true,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("plan_id")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_plan_with_self_consistent_but_false_project_path_metadata() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let project_dir = dir.path().join("repo");
        let false_project_dir = dir.path().join("other");
        let target_file = project_dir.join(".cursor/mcp.json");
        fs::create_dir_all(target_file.parent().unwrap()).unwrap();
        fs::create_dir_all(&false_project_dir).unwrap();
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![project_target("cursor", &project_dir, &target_file)],
                allow_cross_scope: true,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let mut forged_plan = result.targets[0].plan.clone().unwrap();
        forged_plan.target_project_path = Some(false_project_dir.display().to_string());
        forged_plan.plan_id = super::sync_plan_id_from_parts(super::SyncPlanIdParts {
            source_id: forged_plan.resource_id.as_deref().unwrap(),
            tool: &forged_plan.tool,
            target_path: &forged_plan.target_path,
            target_scope: forged_plan.target_scope.as_deref(),
            target_project_path: forged_plan.target_project_path.as_deref(),
            before_fingerprint: &forged_plan.before_fingerprint,
            after_fingerprint: &forged_plan.after_fingerprint,
            created_at: &forged_plan.created_at,
        });

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: true,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("project_path")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn plan_sync_rejects_project_target_outside_project_path() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let project_dir = dir.path().join("repo");
        let outside_target = dir.path().join("other/.cursor/mcp.json");
        fs::create_dir_all(outside_target.parent().unwrap()).unwrap();
        fs::write(&outside_target, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![project_target("cursor", &project_dir, &outside_target)],
                allow_cross_scope: true,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets[0].status, "unsupported");
        assert!(result.targets[0].plan.is_none());
        assert!(
            result.targets[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("under project_path")
        );
    }

    #[test]
    fn apply_sync_rejects_cross_scope_plan_when_apply_authorization_is_false() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let project_dir = dir.path().join("repo");
        let target_file = project_dir.join(".cursor/mcp.json");
        fs::create_dir_all(target_file.parent().unwrap()).unwrap();
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![project_target("cursor", &project_dir, &target_file)],
                allow_cross_scope: true,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let plan = result.targets[0].plan.clone().unwrap();

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "failed");
        assert!(
            applied.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("cross-scope")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_allows_enterprise_source_as_read_only_sync_source() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "enterprise", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: true,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        assert_eq!(result.targets[0].status, "planned");

        let applied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![result.targets[0].plan.clone().unwrap()],
                confirm_drift: false,
                allow_cross_scope: true,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(applied.changes[0].status, "committed");
        assert!(
            fs::read_to_string(&target_file)
                .unwrap()
                .contains("\"docs\"")
        );
    }

    #[test]
    fn plan_sync_requires_manual_env_when_target_lacks_existing_value() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", env_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();

        let result = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets[0].status, "requires_input");
        assert_eq!(
            result.targets[0].required_env_keys,
            vec!["GITHUB_TOKEN".to_string()]
        );
        assert!(result.targets[0].plan.is_none());
    }

    #[test]
    fn apply_sync_commits_successful_targets_and_isolates_drift_failures() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let claude = dir.path().join(".claude.json");
        let gemini = dir.path().join(".gemini/settings.json");
        fs::create_dir_all(gemini.parent().unwrap()).unwrap();
        fs::write(&claude, r#"{"mcpServers":{}}"#).unwrap();
        fs::write(&gemini, r#"{"mcpServers":{}}"#).unwrap();
        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![
                    target("claude", "user", &claude, "json"),
                    target("gemini", "user", &gemini, "json"),
                ],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let plans = planned
            .targets
            .into_iter()
            .map(|target| target.plan.unwrap())
            .collect::<Vec<_>>();
        fs::write(
            &gemini,
            r#"{"mcpServers":{"manual":{"url":"https://changed.test"}}}"#,
        )
        .unwrap();

        let result = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans,
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: None,
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(result.changes.len(), 2);
        assert!(
            result
                .changes
                .iter()
                .any(|change| change.status == "committed" && change.target_path == path(&claude))
        );
        assert!(
            result
                .changes
                .iter()
                .any(|change| change.status == "failed" && change.target_path == path(&gemini))
        );
        assert!(fs::read_to_string(&claude).unwrap().contains("\"docs\""));
        assert!(fs::read_to_string(&gemini).unwrap().contains("\"manual\""));
        let persisted = store.list_resource_changes(None).unwrap();
        assert_eq!(persisted.len(), 2);
        assert!(
            persisted
                .iter()
                .all(|change| change.sync_id.as_deref() == Some(result.sync_id.as_str()))
        );
        assert!(persisted.iter().any(|change| change.status == "failed"));
    }

    #[test]
    fn apply_sync_records_sync_operation_and_change_sync_ids() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        let result = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![planned.targets[0].plan.clone().unwrap()],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: None,
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        let operations = store.list_sync_operations().unwrap();
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].sync_id, result.sync_id);
        assert_eq!(
            operations[0].source_resource_id.as_deref(),
            Some("mcp:docs")
        );
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].sync_id.as_deref(), Some(result.sync_id.as_str()));
    }

    #[test]
    fn apply_sync_reports_reapplied_targets_as_noop_without_fake_change_ids() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        let plan = planned.targets[0].plan.clone().unwrap();

        apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![plan.clone()],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: None,
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .expect("first sync apply should commit");
        let reapplied = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: None,
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .expect("second sync apply should report no-op");

        assert_eq!(reapplied.changes.len(), 1);
        assert_eq!(reapplied.changes[0].status, "noop");
        assert!(reapplied.changes[0].change_id.is_none());
        assert!(reapplied.changes[0].backup_path.is_none());
        assert_eq!(
            reapplied.changes[0].reason.as_deref(),
            Some("target already matches planned state; no change was written")
        );
        assert_eq!(store.list_resource_changes(None).unwrap().len(), 1);
        assert_eq!(store.list_resource_backups(None).unwrap().len(), 1);
    }

    #[test]
    fn apply_sync_rejects_empty_plan_request_before_history() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();

        let error = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: Vec::new(),
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: None,
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("plans"));
        assert!(store.list_sync_operations().unwrap().is_empty());
        assert!(store.list_resource_changes(None).unwrap().is_empty());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_unknown_env_strategy_before_history_or_write() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        let error = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![planned.targets[0].plan.clone().unwrap()],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("side-channel".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("env_strategy"));
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_sync_operations().unwrap().is_empty());
        assert!(store.list_resource_changes(None).unwrap().is_empty());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_persists_explicit_cross_scope_authorization() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "project", url_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: true,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();
        assert_eq!(planned.targets[0].status, "planned");

        let result = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![planned.targets[0].plan.clone().unwrap()],
                confirm_drift: false,
                allow_cross_scope: true,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        let operations = store.list_sync_operations().unwrap();
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].sync_id, result.sync_id);
        assert!(operations[0].allow_cross_scope);
    }

    #[test]
    fn env_strategy_plan_sync_reuses_existing_env_without_exposing_secret_in_preview() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", env_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(
            &target_file,
            r#"{"mcpServers":{"docs":{"url":"https://old.test/mcp","type":"http","env":{"GITHUB_TOKEN":"target-secret"}}}}"#,
        )
        .unwrap();

        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        let target = &planned.targets[0];
        assert_eq!(target.status, "planned");
        assert_eq!(target.required_env_keys, vec!["GITHUB_TOKEN".to_string()]);
        let preview_after = &target.plan.as_ref().unwrap().preview_after;
        assert!(preview_after.contains("<WAPC_REUSE_ENV:GITHUB_TOKEN>"));
        assert!(!preview_after.contains("target-secret"));
    }

    #[test]
    fn env_strategy_apply_sync_manual_env_uses_memory_value_without_persisting_secret() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", env_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "manual".to_string(),
            },
        )
        .unwrap();

        let result = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![planned.targets[0].plan.clone().unwrap()],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("manual".to_string()),
                env_values: BTreeMap::from([(
                    "GITHUB_TOKEN".to_string(),
                    "manual-secret".to_string(),
                )]),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(result.changes[0].status, "committed");
        assert!(
            fs::read_to_string(&target_file)
                .unwrap()
                .contains("manual-secret")
        );
        let operations = store.list_sync_operations().unwrap();
        let changes = store.list_resource_changes(None).unwrap();
        assert_eq!(operations[0].env_strategy, "manual");
        assert!(!operations[0].targets_json.contains("manual-secret"));
        assert!(!format!("{changes:?}").contains("manual-secret"));
    }

    #[test]
    fn apply_sync_commits_deep_link_plan_when_url_revalidates_source() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let link = deep_link_url_for_name("docs");
        let preview = deep_link::preview_deep_link_import(&link).unwrap();
        let source = preview.resource;
        let planned = plan_sync_from_resource(
            &store,
            source.clone(),
            PlanSyncRequest {
                resource_id: source.id.clone(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "manual".to_string(),
            },
        )
        .unwrap();

        let result = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![planned.targets[0].plan.clone().unwrap()],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("manual".to_string()),
                env_values: BTreeMap::from([(
                    "DOCS_TOKEN".to_string(),
                    "manual-secret".to_string(),
                )]),
                deep_link_url: Some(link),
            },
        )
        .unwrap();

        assert_eq!(result.changes[0].status, "committed");
        assert!(
            fs::read_to_string(&target_file)
                .unwrap()
                .contains("manual-secret")
        );
        assert!(
            store
                .list_resources(None, None, None, None)
                .unwrap()
                .is_empty()
        );
        let operations = store.list_sync_operations().unwrap();
        assert_eq!(
            operations[0].source_resource_id.as_deref(),
            Some(source.id.as_str())
        );
        assert!(!operations[0].targets_json.contains("manual-secret"));
    }

    #[test]
    fn apply_sync_rejects_substituted_deep_link_url_before_target_write() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let link = deep_link_url_for_name("docs");
        let preview = deep_link::preview_deep_link_import(&link).unwrap();
        let source = preview.resource;
        let planned = plan_sync_from_resource(
            &store,
            source.clone(),
            PlanSyncRequest {
                resource_id: source.id,
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "manual".to_string(),
            },
        )
        .unwrap();

        let result = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![planned.targets[0].plan.clone().unwrap()],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("manual".to_string()),
                env_values: BTreeMap::from([(
                    "DOCS_TOKEN".to_string(),
                    "manual-secret".to_string(),
                )]),
                deep_link_url: Some(deep_link_url_for_name("other-docs")),
            },
        )
        .unwrap();

        assert_eq!(result.changes[0].status, "failed");
        assert!(
            result.changes[0]
                .reason
                .as_deref()
                .unwrap()
                .contains("deep-link source")
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
    }

    #[test]
    fn apply_sync_rejects_unexpected_manual_env_values_before_history_or_write() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", env_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "manual".to_string(),
            },
        )
        .unwrap();

        let error = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![planned.targets[0].plan.clone().unwrap()],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("manual".to_string()),
                env_values: BTreeMap::from([
                    ("GITHUB_TOKEN".to_string(), "manual-secret".to_string()),
                    ("UNUSED_TOKEN".to_string(), "extra-secret".to_string()),
                ]),
                deep_link_url: None,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("unexpected env_values"));
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"mcpServers":{}}"#
        );
        assert!(store.list_sync_operations().unwrap().is_empty());
        assert!(store.list_resource_changes(None).unwrap().is_empty());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn apply_sync_rejects_env_strategy_placeholder_mismatch_before_history_or_write() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", env_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(
            &target_file,
            r#"{"mcpServers":{"docs":{"url":"https://old.test/mcp","type":"http","env":{"GITHUB_TOKEN":"target-secret"}}}}"#,
        )
        .unwrap();
        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        let mut forged_plan = planned.targets[0].plan.clone().unwrap();
        forged_plan.preview_after = forged_plan.preview_after.replace(
            "<WAPC_REUSE_ENV:GITHUB_TOKEN>",
            "<WAPC_MANUAL_ENV:GITHUB_TOKEN>",
        );
        forged_plan.after_fingerprint = super::sha256_hex(forged_plan.preview_after.as_bytes());
        forged_plan.diff =
            super::line_diff(&forged_plan.preview_before, &forged_plan.preview_after);
        forged_plan.plan_id = super::sync_plan_id_from_parts(super::SyncPlanIdParts {
            source_id: forged_plan.resource_id.as_deref().unwrap(),
            tool: &forged_plan.tool,
            target_path: &forged_plan.target_path,
            target_scope: forged_plan.target_scope.as_deref(),
            target_project_path: forged_plan.target_project_path.as_deref(),
            before_fingerprint: &forged_plan.before_fingerprint,
            after_fingerprint: &forged_plan.after_fingerprint,
            created_at: &forged_plan.created_at,
        });

        let error = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![forged_plan],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: Some("reuse".to_string()),
                env_values: BTreeMap::from([(
                    "GITHUB_TOKEN".to_string(),
                    "manual-secret".to_string(),
                )]),
                deep_link_url: None,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("env_strategy"));
        let content = fs::read_to_string(&target_file).unwrap();
        assert!(content.contains("target-secret"));
        assert!(!content.contains("manual-secret"));
        assert!(store.list_sync_operations().unwrap().is_empty());
        assert!(store.list_resource_changes(None).unwrap().is_empty());
        assert!(store.list_resource_backups(None).unwrap().is_empty());
    }

    #[test]
    fn env_strategy_apply_sync_skip_env_writes_empty_placeholder() {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[mcp_resource("mcp:docs", "user", env_payload())])
            .unwrap();
        let target_file = dir.path().join(".claude.json");
        fs::write(&target_file, r#"{"mcpServers":{}}"#).unwrap();
        let planned = plan_sync(
            dir.path(),
            &store,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![target("claude", "user", &target_file, "json")],
                allow_cross_scope: false,
                env_strategy: "skip".to_string(),
            },
        )
        .unwrap();

        let result = apply_sync(
            dir.path(),
            &store,
            ApplySyncRequest {
                plans: vec![planned.targets[0].plan.clone().unwrap()],
                confirm_drift: false,
                allow_cross_scope: false,
                env_strategy: None,
                env_values: BTreeMap::new(),
                deep_link_url: None,
            },
        )
        .unwrap();

        assert_eq!(result.changes[0].status, "committed");
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(target_file).unwrap()).unwrap();
        assert_eq!(
            value["mcpServers"]["docs"]["env"]["GITHUB_TOKEN"],
            serde_json::Value::String(String::new())
        );
        let operations = store.list_sync_operations().unwrap();
        assert_eq!(operations[0].env_strategy, "skip");
    }

    fn path(path: &std::path::Path) -> String {
        path.display().to_string()
    }

    fn deep_link_url_for_name(name: &str) -> String {
        format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/docs-mcp"),
            percent_encode(&format!(
                r#"{{"kind":"mcp","name":"{name}","scope":"user","payload":{{"transport":"http","url":"https://example.test/mcp","env_keys":["DOCS_TOKEN"],"env_fingerprints":{{}}}}}}"#
            ))
        )
    }

    fn percent_encode(value: &str) -> String {
        value
            .bytes()
            .map(|byte| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (byte as char).to_string()
                }
                other => format!("%{other:02X}"),
            })
            .collect()
    }
}
