//! Resource template library tests and implementation.
//! @author codex

use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    cross_sync,
    model::{
        CanonicalResource, PlanSyncRequest, PlanSyncResult, PlanTemplateSyncRequest,
        ResourceTemplate,
    },
    store::UsageStore,
};

pub fn built_in_resource_templates() -> Vec<ResourceTemplate> {
    vec![context7_template()]
}

pub fn seed_builtin_resource_templates(store: &UsageStore) -> Result<usize> {
    store.upsert_resource_templates(&built_in_resource_templates())
}

pub fn plan_template_sync(
    _home: &Path,
    store: &UsageStore,
    request: PlanTemplateSyncRequest,
) -> Result<PlanSyncResult> {
    let template = store
        .get_resource_template(&request.template_id)?
        .with_context(|| format!("resource template not found: {}", request.template_id))?;
    let source = canonical_resource_from_template(&template);
    cross_sync::plan_sync_from_resource(
        store,
        source.clone(),
        PlanSyncRequest {
            resource_id: source.id,
            targets: request.targets,
            allow_cross_scope: request.allow_cross_scope,
            env_strategy: request.env_strategy,
        },
    )
}

pub fn canonical_resource_from_template(template: &ResourceTemplate) -> CanonicalResource {
    CanonicalResource {
        id: format!("template:{}:{}", template.id, template.content_fingerprint),
        kind: template.kind.clone(),
        name: template_name(template),
        scope: template.scope.clone(),
        origin_tool: "template-library".to_string(),
        origin_path: template.source.clone(),
        origin_locator: Some(format!("resource_templates.{}", template.id)),
        enabled_in: Vec::new(),
        confidence: 0.8,
        redacted: !template.required_env_keys.is_empty(),
        payload_json: template.payload_json.clone(),
        provided_by_plugin: None,
        last_seen: Utc::now().to_rfc3339(),
    }
}

fn context7_template() -> ResourceTemplate {
    let payload = json!({
        "transport": "stdio",
        "command": "npx",
        "args": ["-y", "@upstash/context7-mcp"],
        "env_keys": ["CONTEXT7_API_KEY"],
        "env_fingerprints": {},
    });
    let payload_json = serde_json::to_string(&payload).unwrap();
    ResourceTemplate {
        id: "builtin:context7-mcp".to_string(),
        name: "Context7 MCP".to_string(),
        kind: "mcp".to_string(),
        scope: "user".to_string(),
        description: "Context7 MCP template for up-to-date library documentation.".to_string(),
        source: "https://context7.com/docs/resources/all-clients".to_string(),
        content_fingerprint: stable_hash_16(&format!("context7\nmcp\nuser\n{payload_json}")),
        required_env_keys: vec!["CONTEXT7_API_KEY".to_string()],
        payload_json,
        updated_at: "2026-06-06T00:00:00Z".to_string(),
    }
}

fn template_name(template: &ResourceTemplate) -> String {
    template
        .id
        .rsplit_once(':')
        .map(|(_, name)| name.trim_end_matches("-mcp").to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| template.name.to_ascii_lowercase().replace(' ', "-"))
}

fn stable_hash_16(value: &str) -> String {
    let digest = Sha256::digest(format!("wapc-resource-template-v1:{value}").as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        model::{PlanTemplateSyncRequest, SyncTarget},
        store::UsageStore,
        template_library::{
            built_in_resource_templates, plan_template_sync, seed_builtin_resource_templates,
        },
    };

    #[test]
    fn seeds_builtin_templates_with_source_fingerprint_and_env_keys() {
        let dir = tempfile::tempdir().unwrap();
        let store = UsageStore::open(&dir.path().join("wapc.db")).unwrap();

        let built_ins = built_in_resource_templates();
        assert!(!built_ins.is_empty());
        assert!(
            built_ins
                .iter()
                .any(|template| template.id == "builtin:context7-mcp")
        );

        let seeded = seed_builtin_resource_templates(&store).unwrap();
        let templates = store.list_resource_templates().unwrap();
        let docs = templates
            .iter()
            .find(|template| template.id == "builtin:context7-mcp")
            .unwrap();

        assert_eq!(seeded, built_ins.len());
        assert_eq!(docs.kind, "mcp");
        assert_eq!(docs.scope, "user");
        assert!(docs.source.starts_with("https://"));
        assert_eq!(docs.content_fingerprint.len(), 16);
        assert_eq!(docs.required_env_keys, vec!["CONTEXT7_API_KEY".to_string()]);
        assert!(docs.description.contains("MCP"));
        assert!(!docs.payload_json.contains("sk-"));
        assert!(!docs.payload_json.contains("ghp_"));
    }

    #[test]
    fn plans_template_sync_without_persisting_template_as_resource() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let target_path = home.join("claude.json");
        fs::write(&target_path, r#"{"mcpServers":{}}"#).unwrap();
        let store = UsageStore::open(&dir.path().join("wapc.db")).unwrap();
        seed_builtin_resource_templates(&store).unwrap();

        let result = plan_template_sync(
            &home,
            &store,
            PlanTemplateSyncRequest {
                template_id: "builtin:context7-mcp".to_string(),
                targets: vec![SyncTarget {
                    tool: "claude".to_string(),
                    scope: "user".to_string(),
                    project_path: None,
                    target_path: target_path.to_string_lossy().to_string(),
                    format: "json".to_string(),
                }],
                allow_cross_scope: false,
                env_strategy: "manual".to_string(),
            },
        )
        .unwrap();

        let target = &result.targets[0];
        let plan = target.plan.as_ref().unwrap();

        assert_eq!(
            result.source_resource_id.len(),
            "template:builtin:context7-mcp:".len() + 16
        );
        assert_eq!(target.status, "planned");
        assert_eq!(
            target.required_env_keys,
            vec!["CONTEXT7_API_KEY".to_string()]
        );
        assert_eq!(
            plan.resource_id.as_deref(),
            Some(result.source_resource_id.as_str())
        );
        assert!(plan.preview_after.contains("\"context7\""));
        assert!(plan.preview_after.contains("@upstash/context7-mcp"));
        assert!(
            plan.preview_after
                .contains("<WAPC_MANUAL_ENV:CONTEXT7_API_KEY>")
        );
        assert!(!plan.preview_after.contains("real-token"));
        assert!(
            store
                .list_resources(None, None, None, None)
                .unwrap()
                .is_empty()
        );
    }
}
