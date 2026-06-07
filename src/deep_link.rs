//! Safe preview parser for wapc:// resource import links.
//! @author codex

use std::path::Path;

use anyhow::{Result, bail};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    cross_sync,
    model::{
        CanonicalResource, DeepLinkImportPreview, PlanDeepLinkImportRequest, PlanSyncRequest,
        PlanSyncResult,
    },
    store::UsageStore,
};

/// Plan a deep-link import through the sync engine without persisting the imported resource.
/// @author codex
pub fn plan_deep_link_import(
    _home: &Path,
    store: &UsageStore,
    request: PlanDeepLinkImportRequest,
) -> Result<PlanSyncResult> {
    let preview = preview_deep_link_import(&request.url)?;
    let source = preview.resource;
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

pub fn preview_deep_link_import(link: &str) -> Result<DeepLinkImportPreview> {
    let query = link
        .strip_prefix("wapc://import?")
        .ok_or_else(|| anyhow::anyhow!("unsupported deep link: expected wapc://import"))?;
    let source = query_param(query, "source")?;
    let resource_json = query_param(query, "resource")?;
    validate_source(&source)?;
    let input = serde_json::from_str::<DeepLinkResourceInput>(&resource_json)?;
    validate_resource_input(&input)?;
    let payload_json = serde_json::to_string(&input.payload)?;
    reject_secret_values(&input.payload)?;
    let content_fingerprint = stable_hash_16(&format!(
        "{}:{}:{}:{}",
        input.kind, input.name, input.scope, payload_json
    ));
    let risks = source_risks(&source);
    Ok(DeepLinkImportPreview {
        schema: "wapc.deep_link_import_preview.v1".to_string(),
        source: source.clone(),
        content_fingerprint: content_fingerprint.clone(),
        resource: CanonicalResource {
            id: format!(
                "deep-link:{}:{}:{}",
                input.kind, input.name, content_fingerprint
            ),
            kind: input.kind,
            name: input.name,
            scope: input.scope,
            origin_tool: "deep-link".to_string(),
            origin_path: source,
            origin_locator: Some("wapc://import".to_string()),
            enabled_in: Vec::new(),
            confidence: 0.7,
            redacted: payload_mentions_secret_placeholders(&input.payload),
            payload_json,
            provided_by_plugin: None,
            last_seen: Utc::now().to_rfc3339(),
        },
        risks,
    })
}

fn validate_source(source: &str) -> Result<()> {
    if source.trim().is_empty() {
        bail!("deep link source is required");
    }
    if source.chars().any(char::is_control) {
        bail!("deep link source must not contain control characters");
    }
    if contains_raw_secret(source) {
        bail!("deep link source must not contain raw secret values");
    }
    Ok(())
}

#[derive(Deserialize)]
struct DeepLinkResourceInput {
    kind: String,
    name: String,
    scope: String,
    payload: Value,
}

fn validate_resource_input(input: &DeepLinkResourceInput) -> Result<()> {
    if !matches!(
        input.kind.as_str(),
        "mcp" | "skill" | "plugin" | "instruction" | "subagent"
    ) {
        bail!("unsupported deep link resource kind: {}", input.kind);
    }
    if !matches!(input.scope.as_str(), "user" | "project") {
        bail!("unsupported deep link resource scope: {}", input.scope);
    }
    if input.name.trim().is_empty() {
        bail!("deep link resource name is required");
    }
    if !input.payload.is_object() {
        bail!("deep link resource payload must be a JSON object");
    }
    Ok(())
}

fn query_param(query: &str, name: &str) -> Result<String> {
    let mut found = None;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();
        if key == name {
            if found.is_some() {
                bail!("duplicate deep link query parameter `{name}`");
            }
            found = Some(percent_decode(value)?);
        }
    }
    found.ok_or_else(|| anyhow::anyhow!("deep link query parameter `{name}` is required"))
}

fn percent_decode(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                if index + 2 >= bytes.len() {
                    bail!("malformed percent encoding in deep link query parameter");
                }
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3])?;
                let decoded = u8::from_str_radix(hex, 16).map_err(|_| {
                    anyhow::anyhow!("malformed percent encoding in deep link query parameter")
                })?;
                output.push(decoded);
                index += 3;
            }
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    Ok(String::from_utf8(output)?)
}

fn reject_secret_values(value: &Value) -> Result<()> {
    reject_secret_values_at(value, "")
}

fn reject_secret_values_at(value: &Value, key_path: &str) -> Result<()> {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                let next_path = if key_path.is_empty() {
                    key.to_string()
                } else {
                    format!("{key_path}.{key}")
                };
                if key.eq_ignore_ascii_case("env") && nested.is_object() {
                    reject_raw_env_values(nested)?;
                }
                if sensitive_key(key) {
                    reject_sensitive_key_value(key, nested)?;
                }
                reject_secret_values_at(nested, &next_path)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                reject_secret_values_at(item, key_path)?;
            }
        }
        Value::String(text) if contains_raw_secret(text) => {
            bail!("deep link resource contains a raw secret value");
        }
        _ => {}
    }
    Ok(())
}

fn reject_raw_env_values(value: &Value) -> Result<()> {
    let Some(map) = value.as_object() else {
        return Ok(());
    };
    for env_value in map.values() {
        if !matches!(env_value, Value::String(text) if placeholder_value(text)) {
            bail!("deep link env values must be placeholders, not raw secrets");
        }
    }
    Ok(())
}

fn reject_sensitive_key_value(key: &str, value: &Value) -> Result<()> {
    match value {
        Value::String(text) if placeholder_value(text) => Ok(()),
        Value::Array(_) if matches!(key, "env_keys" | "required_env_keys") => Ok(()),
        Value::Object(_) if key == "env_fingerprints" => Ok(()),
        Value::Null => Ok(()),
        _ => bail!("deep link resource contains a sensitive `{key}` value"),
    }
}

fn sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "authorization" | "api_key" | "apikey" | "secret"
    ) || (lower.contains("token") && lower != "env_keys" && lower != "required_env_keys")
}

fn contains_raw_secret(value: &str) -> bool {
    let mut current = value.to_string();
    for _ in 0..3 {
        let lower = current.to_ascii_lowercase();
        if contains_raw_secret_marker(&lower) {
            return true;
        }
        let Ok(decoded) = percent_decode(&current) else {
            return false;
        };
        if decoded == current {
            return false;
        }
        current = decoded;
    }
    let lower = current.to_ascii_lowercase();
    contains_raw_secret_marker(&lower)
}

fn contains_raw_secret_marker(lower: &str) -> bool {
    lower.contains("ghp_")
        || lower.contains("github_pat_")
        || lower.contains("bearer ")
        || contains_encoded_bearer_separator(lower)
        || lower
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
            .any(|part| part.starts_with("sk-"))
}

fn contains_encoded_bearer_separator(value: &str) -> bool {
    [
        "bearer%20",
        "bearer+",
        "bearer%09",
        "bearer%0a",
        "bearer%0d",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

fn placeholder_value(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty() || (trimmed.starts_with("<") && trimmed.ends_with(">"))
}

fn payload_mentions_secret_placeholders(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            matches!(key.as_str(), "env" | "env_keys" | "required_env_keys")
                || payload_mentions_secret_placeholders(value)
        }),
        Value::Array(items) => items.iter().any(payload_mentions_secret_placeholders),
        Value::String(text) => placeholder_value(text),
        _ => false,
    }
}

fn source_risks(source: &str) -> Vec<String> {
    if source.starts_with("https://") {
        Vec::new()
    } else {
        vec!["source is not https; review origin before syncing".to_string()]
    }
}

fn stable_hash_16(value: &str) -> String {
    let digest = Sha256::digest(format!("wapc-deep-link-import-v1:{value}").as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::{
        model::{PlanDeepLinkImportRequest, SyncTarget},
        store::UsageStore,
    };

    #[test]
    fn previews_wapc_import_link_as_canonical_resource_without_writing() {
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/docs-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp","env_keys":["DOCS_TOKEN"],"env_fingerprints":{}}}"#
            )
        );

        let preview = preview_deep_link_import(&link).unwrap();

        assert_eq!(preview.schema, "wapc.deep_link_import_preview.v1");
        assert_eq!(preview.source, "https://example.test/templates/docs-mcp");
        assert_eq!(preview.resource.kind, "mcp");
        assert_eq!(preview.resource.name, "docs");
        assert_eq!(preview.resource.scope, "user");
        assert_eq!(preview.resource.origin_tool, "deep-link");
        assert_eq!(
            preview.resource.origin_path,
            "https://example.test/templates/docs-mcp"
        );
        assert_eq!(
            preview.resource.origin_locator.as_deref(),
            Some("wapc://import")
        );
        assert_eq!(preview.resource.enabled_in, Vec::<String>::new());
        assert!(preview.resource.redacted);
        assert_eq!(preview.content_fingerprint.len(), 16);
        assert!(preview.risks.is_empty());
        assert!(preview.resource.payload_json.contains("DOCS_TOKEN"));
        assert!(!preview.resource.payload_json.contains("sk-test-secret"));
    }

    #[test]
    fn rejects_wapc_import_link_with_raw_env_secret() {
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/github-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"github","scope":"user","payload":{"transport":"stdio","command":"npx","env":{"GITHUB_TOKEN":"ghp_secret1234567890"}}}"#
            )
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("secret"));
    }

    #[test]
    fn plans_deep_link_import_without_persisting_imported_resource() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-core-plan-deep-link-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let target_path = dir.join(".gemini/settings.json");
        fs::create_dir_all(target_path.parent().unwrap()).unwrap();
        fs::write(&target_path, r#"{"mcpServers":{}}"#).unwrap();
        let before = fs::read_to_string(&target_path).unwrap();
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/docs-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp","env_keys":["DOCS_TOKEN"],"env_fingerprints":{}}}"#
            )
        );

        let result = plan_deep_link_import(
            &dir,
            &store,
            PlanDeepLinkImportRequest {
                url: link,
                targets: vec![SyncTarget {
                    tool: "gemini".to_string(),
                    scope: "user".to_string(),
                    project_path: None,
                    target_path: target_path.display().to_string(),
                    format: "json".to_string(),
                }],
                allow_cross_scope: false,
                env_strategy: "manual".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets[0].status, "planned");
        assert!(result.source_resource_id.starts_with("deep-link:mcp:docs:"));
        assert!(
            result.targets[0]
                .plan
                .as_ref()
                .unwrap()
                .preview_after
                .contains("<WAPC_MANUAL_ENV:DOCS_TOKEN>")
        );
        assert_eq!(fs::read_to_string(&target_path).unwrap(), before);
        assert!(
            store
                .list_resources(None, None, None, None)
                .unwrap()
                .is_empty()
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn warns_about_non_https_deep_link_source() {
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("http://example.test/templates/docs-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp"}}"#
            )
        );

        let preview = preview_deep_link_import(&link).unwrap();

        assert_eq!(preview.risks.len(), 1);
        assert!(preview.risks[0].contains("not https"));
    }

    #[test]
    fn rejects_authorization_header_values() {
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/private-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"private","scope":"user","payload":{"headers":{"Authorization":"Bearer live-token"}}}"#
            )
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("Authorization"));
    }

    #[test]
    fn rejects_non_wapc_import_links() {
        let error = preview_deep_link_import("https://example.test/import").unwrap_err();

        assert!(error.to_string().contains("wapc://import"));
    }

    #[test]
    fn rejects_duplicate_deep_link_query_parameters() {
        let safe_resource = r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp"}}"#;
        let link = format!(
            "wapc://import?source={}&resource={}&resource={}",
            percent_encode("https://example.test/templates/docs-mcp"),
            percent_encode(safe_resource),
            percent_encode(safe_resource)
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("duplicate"));
        assert!(error.to_string().contains("resource"));
    }

    #[test]
    fn rejects_malformed_deep_link_percent_encoding() {
        let safe_resource = r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp"}}"#;
        let link = format!(
            "wapc://import?resource={}&source=https://example.test/templates/%",
            percent_encode(safe_resource)
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("percent"));
    }

    #[test]
    fn rejects_deep_link_source_with_control_characters() {
        let safe_resource = r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp"}}"#;
        let link = format!(
            "wapc://import?resource={}&source=https://example.test/templates/docs%0Aevil",
            percent_encode(safe_resource)
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("source"));
        assert!(error.to_string().contains("control"));
    }

    #[test]
    fn rejects_deep_link_source_with_raw_token() {
        let safe_resource = r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp"}}"#;
        let source = "https://example.test/templates/docs?access_token=ghp_secret1234567890";
        let link = format!(
            "wapc://import?resource={}&source={}",
            percent_encode(safe_resource),
            percent_encode(source)
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("source"));
        assert!(error.to_string().contains("secret"));
    }

    #[test]
    fn rejects_deep_link_source_url_with_url_encoded_bearer_tab_token() {
        let safe_resource = r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp"}}"#;
        let source = "https://example.test/templates/docs?authorization=Bearer%09live-token";
        let link = format!(
            "wapc://import?resource={}&source={}",
            percent_encode(safe_resource),
            percent_encode(source)
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("source"));
        assert!(error.to_string().contains("secret"));
    }

    #[test]
    fn rejects_deep_link_payload_url_with_raw_token() {
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/docs-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp?access_token=ghp_secret1234567890"}}"#
            )
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("secret"));
    }

    #[test]
    fn rejects_deep_link_payload_url_with_percent_encoded_token_prefix() {
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/docs-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp?access_token=ghp%5Fsecret1234567890"}}"#
            )
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("secret"));
    }

    #[test]
    fn rejects_deep_link_payload_url_with_double_encoded_token_prefix() {
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/docs-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp?access_token=ghp%255Fsecret1234567890"}}"#
            )
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("secret"));
    }

    #[test]
    fn rejects_deep_link_payload_url_with_url_encoded_bearer_token() {
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/docs-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp?authorization=Bearer%20live-token"}}"#
            )
        );

        let error = preview_deep_link_import(&link).unwrap_err();

        assert!(error.to_string().contains("secret"));
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
