//! Shared usage model for normalized AI coding tool token records.
//! @author codex

use std::collections::BTreeMap;
use std::ops::Add;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub reasoning: u64,
    pub tool: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write + self.reasoning + self.tool
    }
}

impl Add for TokenUsage {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            input: self.input + rhs.input,
            output: self.output + rhs.output,
            cache_read: self.cache_read + rhs.cache_read,
            cache_write: self.cache_write + rhs.cache_write,
            reasoning: self.reasoning + rhs.reasoning,
            tool: self.tool + rhs.tool,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SourcePrecision {
    Exact,
    Computed,
    Estimated,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ToolKind {
    Claude,
    Codex,
    Gemini,
    OpenCode,
}

impl ToolKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::OpenCode => "opencode",
        }
    }
}

impl FromStr for ToolKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "gemini" => Ok(Self::Gemini),
            "opencode" => Ok(Self::OpenCode),
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

impl SourcePrecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Computed => "computed",
            Self::Estimated => "estimated",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub tool: ToolKind,
    pub source_path: String,
    pub session_id: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub project_path: Option<String>,
    pub model: Option<String>,
    pub usage: TokenUsage,
    pub cost_usd: Option<f64>,
    pub precision: SourcePrecision,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DetectedTool {
    pub id: String,
    pub display_name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub config_dir: Option<String>,
    pub data_dir: Option<String>,
    pub config_dir_exists: bool,
    pub data_dir_exists: bool,
    pub last_detected_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceHealth {
    pub tool: String,
    pub source_glob: String,
    pub exists: bool,
    pub readable_files: u64,
    pub parsed_records: u64,
    pub failed_files: u64,
    pub latest_event_ts: Option<String>,
    pub checked_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PricingRule {
    pub id: Option<i64>,
    pub model_match: String,
    pub match_kind: String,
    pub provider: Option<String>,
    pub currency: String,
    pub price_input: Option<f64>,
    pub price_output: Option<f64>,
    pub price_cache_read: Option<f64>,
    pub price_cache_write: Option<f64>,
    pub price_reasoning: Option<f64>,
    pub price_tool: Option<f64>,
    pub source: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CostRecomputeResult {
    pub updated: u64,
    pub exact_matches: u64,
    pub prefix_matches: u64,
    pub no_matches: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectAlias {
    pub canonical_path: String,
    pub alias: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub canonical_path: String,
    pub display_name: String,
    pub alias: Option<String>,
    pub original_paths: Vec<String>,
    pub tools: Vec<String>,
    pub records: u64,
    pub usage: TokenUsage,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExportReportRequest {
    pub view: String,
    pub format: String,
    pub path: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub include_fixture: bool,
    #[serde(default)]
    pub include_project_aliases: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExportReportResult {
    pub path: String,
    pub bytes_written: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BackupRequest {
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BackupResult {
    pub success: bool,
    pub path: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrivacyAuditReport {
    pub generated_at: String,
    pub local_only: bool,
    pub db_path: String,
    pub read_sources: Vec<PrivacyAuditSource>,
    pub stored_tables: Vec<PrivacyAuditTable>,
    pub forbidden_fields: Vec<String>,
    pub export_boundary: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrivacyAuditSource {
    pub name: String,
    pub path: String,
    pub purpose: String,
    pub reads_body: bool,
    pub writes_source: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrivacyAuditTable {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanonicalResource {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub scope: String,
    pub origin_tool: String,
    pub origin_path: String,
    pub origin_locator: Option<String>,
    pub enabled_in: Vec<String>,
    pub confidence: f64,
    pub redacted: bool,
    pub payload_json: String,
    pub provided_by_plugin: Option<String>,
    pub last_seen: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceGuideSection {
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceGuide {
    pub id: String,
    pub tool: Option<String>,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub sections: Vec<ResourceGuideSection>,
    pub risks: Vec<String>,
    pub unsupported_actions: Vec<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceTemplate {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub scope: String,
    pub description: String,
    pub source: String,
    pub content_fingerprint: String,
    pub required_env_keys: Vec<String>,
    pub payload_json: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeepLinkImportPreview {
    pub schema: String,
    pub source: String,
    pub content_fingerprint: String,
    pub resource: CanonicalResource,
    pub risks: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceParseFailure {
    pub path: String,
    pub tool: String,
    pub kind: Option<String>,
    pub reason: String,
    pub seen_at: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct InventoryScanResult {
    pub scanned: u64,
    pub upserted: u64,
    pub failures: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdapterCapability {
    pub tool: String,
    pub display_name: String,
    pub resource_kinds: Vec<String>,
    pub scopes: Vec<String>,
    pub transports: Vec<String>,
    pub read_only: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: String,
    pub tool: String,
    pub project_path: Option<String>,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub records: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
    pub source_paths: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceChangeRequest {
    pub tool: String,
    pub kind: String,
    pub op: String,
    pub resource_id: Option<String>,
    pub target_path: String,
    pub resource_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WritePlanRisk {
    pub code: String,
    pub message: String,
    pub severity: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WritePlan {
    pub plan_id: String,
    pub tool: String,
    pub kind: String,
    pub op: String,
    pub resource_id: Option<String>,
    pub resource_name: String,
    pub target_path: String,
    #[serde(default)]
    pub target_scope: Option<String>,
    #[serde(default)]
    pub target_project_path: Option<String>,
    pub before_fingerprint: String,
    pub after_fingerprint: String,
    pub diff: String,
    pub preview_before: String,
    pub preview_after: String,
    pub requires_backup: bool,
    pub risks: Vec<WritePlanRisk>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplyChangeRequest {
    pub plan: WritePlan,
    pub confirm_drift: bool,
    #[serde(default)]
    pub sync_id: Option<String>,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub force_verify_failure: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplyChangeResult {
    pub change_id: String,
    pub backup_path: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceChangeLog {
    pub change_id: String,
    pub sync_id: Option<String>,
    pub tool: String,
    pub resource_id: Option<String>,
    pub kind: String,
    pub op: String,
    pub target_path: String,
    pub backup_path: Option<String>,
    pub status: String,
    pub reverts_change_id: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceBackup {
    pub backup_path: String,
    pub tool: String,
    pub original_path: String,
    pub change_id: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncTarget {
    pub tool: String,
    pub scope: String,
    pub project_path: Option<String>,
    pub target_path: String,
    pub format: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanSyncRequest {
    pub resource_id: String,
    pub targets: Vec<SyncTarget>,
    pub allow_cross_scope: bool,
    pub env_strategy: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanTemplateSyncRequest {
    pub template_id: String,
    pub targets: Vec<SyncTarget>,
    pub allow_cross_scope: bool,
    pub env_strategy: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanDeepLinkImportRequest {
    pub url: String,
    pub targets: Vec<SyncTarget>,
    pub allow_cross_scope: bool,
    pub env_strategy: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncTargetPlan {
    pub target: SyncTarget,
    pub status: String,
    pub reason: Option<String>,
    pub required_env_keys: Vec<String>,
    pub plan: Option<WritePlan>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanSyncResult {
    pub source_resource_id: String,
    pub created_at: String,
    pub targets: Vec<SyncTargetPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplySyncRequest {
    pub plans: Vec<WritePlan>,
    pub confirm_drift: bool,
    #[serde(default)]
    pub allow_cross_scope: bool,
    #[serde(default)]
    pub env_strategy: Option<String>,
    #[serde(default)]
    pub env_values: BTreeMap<String, String>,
    #[serde(default)]
    pub deep_link_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplySyncTargetResult {
    pub plan_id: String,
    pub target_path: String,
    pub status: String,
    pub change_id: Option<String>,
    pub backup_path: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplySyncResult {
    pub sync_id: String,
    pub changes: Vec<ApplySyncTargetResult>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncOperation {
    pub sync_id: String,
    pub source_resource_id: Option<String>,
    pub targets_json: String,
    pub allow_cross_scope: bool,
    pub env_strategy: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncPreset {
    pub id: String,
    pub name: String,
    pub resources_json: String,
    pub targets_json: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AutoScanConfig {
    pub enabled: bool,
    pub interval_minutes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_usage_total_includes_all_billable_buckets() {
        let usage = TokenUsage {
            input: 10,
            output: 20,
            cache_read: 30,
            cache_write: 40,
            reasoning: 50,
            tool: 60,
        };

        assert_eq!(usage.total(), 210);
    }

    #[test]
    fn token_usage_adds_two_records_bucket_by_bucket() {
        let left = TokenUsage {
            input: 1,
            output: 2,
            cache_read: 3,
            cache_write: 4,
            reasoning: 5,
            tool: 6,
        };
        let right = TokenUsage {
            input: 10,
            output: 20,
            cache_read: 30,
            cache_write: 40,
            reasoning: 50,
            tool: 60,
        };

        assert_eq!(
            left + right,
            TokenUsage {
                input: 11,
                output: 22,
                cache_read: 33,
                cache_write: 44,
                reasoning: 55,
                tool: 66,
            }
        );
    }

    #[test]
    fn detected_tool_serializes_registry_fields() {
        let tool = DetectedTool {
            id: "codex".to_string(),
            display_name: "Codex".to_string(),
            installed: true,
            version: Some("1.0.0".to_string()),
            config_dir: Some("/Users/test/.codex".to_string()),
            data_dir: Some("/Users/test/.codex/sessions".to_string()),
            config_dir_exists: true,
            data_dir_exists: true,
            last_detected_at: "2026-06-05T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(tool).unwrap();

        assert_eq!(json["id"], "codex");
        assert_eq!(json["installed"], true);
        assert_eq!(json["config_dir_exists"], true);
        assert_eq!(json["data_dir_exists"], true);
    }

    #[test]
    fn source_health_serializes_doctor_counts() {
        let health = SourceHealth {
            tool: "claude".to_string(),
            source_glob: "~/.claude/projects/**/*.jsonl".to_string(),
            exists: true,
            readable_files: 2,
            parsed_records: 3,
            failed_files: 1,
            latest_event_ts: Some("2026-06-05T01:00:00Z".to_string()),
            checked_at: "2026-06-05T02:00:00Z".to_string(),
        };

        let json = serde_json::to_value(health).unwrap();

        assert_eq!(json["tool"], "claude");
        assert_eq!(json["readable_files"], 2);
        assert_eq!(json["parsed_records"], 3);
        assert_eq!(json["failed_files"], 1);
    }

    #[test]
    fn pricing_rule_serializes_all_token_bucket_prices() {
        let rule = PricingRule {
            id: Some(7),
            model_match: "claude-".to_string(),
            match_kind: "prefix".to_string(),
            provider: None,
            currency: "USD".to_string(),
            price_input: Some(3.0),
            price_output: Some(15.0),
            price_cache_read: Some(0.3),
            price_cache_write: Some(3.75),
            price_reasoning: Some(2.0),
            price_tool: Some(1.0),
            source: "user".to_string(),
            updated_at: "2026-06-05T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(rule).unwrap();

        assert_eq!(json["model_match"], "claude-");
        assert_eq!(json["match_kind"], "prefix");
        assert_eq!(json["price_output"], 15.0);
        assert_eq!(json["source"], "user");
    }

    #[test]
    fn project_summary_serializes_alias_and_origin_paths() {
        let summary = ProjectSummary {
            canonical_path: "/Users/test/work/repo".to_string(),
            display_name: "Repo Alias".to_string(),
            alias: Some("Repo Alias".to_string()),
            original_paths: vec![
                "/Users/test/work/repo".to_string(),
                "/Users/test/work/repo/".to_string(),
            ],
            tools: vec!["claude".to_string(), "codex".to_string()],
            records: 2,
            usage: TokenUsage {
                input: 10,
                output: 20,
                ..TokenUsage::default()
            },
            cost_usd: 0.25,
        };

        let json = serde_json::to_value(summary).unwrap();

        assert_eq!(json["canonical_path"], "/Users/test/work/repo");
        assert_eq!(json["display_name"], "Repo Alias");
        assert_eq!(json["original_paths"].as_array().unwrap().len(), 2);
        assert_eq!(json["tools"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn export_report_request_serializes_view_format_and_path() {
        let request = ExportReportRequest {
            view: "projects".to_string(),
            format: "markdown".to_string(),
            path: "/tmp/wapc-projects.md".to_string(),
            from: Some("2026-06-05T00:00:00Z".to_string()),
            to: None,
            include_fixture: true,
            include_project_aliases: false,
        };

        let json = serde_json::to_value(request).unwrap();

        assert_eq!(json["view"], "projects");
        assert_eq!(json["format"], "markdown");
        assert_eq!(json["path"], "/tmp/wapc-projects.md");
        assert_eq!(json["from"], "2026-06-05T00:00:00Z");
        assert_eq!(json["to"], serde_json::Value::Null);
        assert_eq!(json["include_fixture"], true);
    }

    #[test]
    fn privacy_audit_report_serializes_sources_tables_and_forbidden_fields() {
        let report = PrivacyAuditReport {
            generated_at: "2026-06-05T00:00:00Z".to_string(),
            local_only: true,
            db_path: "/Users/test/.wapc/wapc.db".to_string(),
            read_sources: vec![PrivacyAuditSource {
                name: "Claude Code sessions".to_string(),
                path: "~/.claude/projects".to_string(),
                purpose: "usage metadata parsing".to_string(),
                reads_body: false,
                writes_source: false,
            }],
            stored_tables: vec![PrivacyAuditTable {
                name: "usage_records".to_string(),
                fields: vec!["tool".to_string(), "total_tokens".to_string()],
            }],
            forbidden_fields: vec!["prompt".to_string(), "response".to_string()],
            export_boundary: "metadata only".to_string(),
        };

        let json = serde_json::to_value(report).unwrap();

        assert_eq!(json["local_only"], true);
        assert_eq!(json["read_sources"][0]["reads_body"], false);
        assert_eq!(json["stored_tables"][0]["name"], "usage_records");
        assert_eq!(json["forbidden_fields"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn canonical_mcp_resource_serializes_redacted_payload() {
        let resource = CanonicalResource {
            id: "mcp:github".to_string(),
            kind: "mcp".to_string(),
            name: "github".to_string(),
            scope: "user".to_string(),
            origin_tool: "claude".to_string(),
            origin_path: "/Users/test/.claude.json".to_string(),
            origin_locator: Some("mcpServers.github".to_string()),
            enabled_in: vec!["claude".to_string()],
            confidence: 1.0,
            redacted: true,
            payload_json: serde_json::json!({
                "transport": "stdio",
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-github"],
                "env_keys": ["GITHUB_TOKEN"],
                "env_fingerprints": {
                    "GITHUB_TOKEN": { "len": 8, "prefix": "ghp_", "sha256_8": "abcd1234" }
                }
            })
            .to_string(),
            provided_by_plugin: None,
            last_seen: "2026-06-05T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(resource).unwrap();

        assert_eq!(json["kind"], "mcp");
        assert_eq!(json["redacted"], true);
        assert!(json["payload_json"].as_str().unwrap().contains("sha256_8"));
        assert!(
            !json["payload_json"]
                .as_str()
                .unwrap()
                .contains("secret-token")
        );
    }

    #[test]
    fn adapter_capability_serializes_read_only_resources() {
        let capability = AdapterCapability {
            tool: "claude".to_string(),
            display_name: "Claude Code".to_string(),
            resource_kinds: vec!["mcp".to_string(), "skill".to_string()],
            scopes: vec!["user".to_string()],
            transports: vec!["stdio".to_string()],
            read_only: true,
            notes: vec!["No writes in Phase 2".to_string()],
        };

        let json = serde_json::to_value(capability).unwrap();

        assert_eq!(json["tool"], "claude");
        assert_eq!(json["read_only"], true);
        assert_eq!(json["resource_kinds"][0], "mcp");
    }

    #[test]
    fn session_meta_serializes_without_body_fields() {
        let session = SessionMeta {
            session_id: "s1".to_string(),
            tool: "codex".to_string(),
            project_path: Some("/repo".to_string()),
            first_ts: Some("2026-06-06T01:00:00Z".to_string()),
            last_ts: Some("2026-06-06T02:00:00Z".to_string()),
            records: 2,
            total_tokens: 42,
            cost_usd: 0.5,
            source_paths: vec!["/tmp/session.jsonl".to_string()],
        };

        let text = serde_json::to_string(&session).unwrap();

        assert!(text.contains("session_id"));
        assert!(!text.contains("prompt"));
        assert!(!text.contains("response"));
        assert!(!text.contains("message"));
    }

    #[test]
    fn resource_guide_serializes_safe_usage_sections() {
        let guide = ResourceGuide {
            id: "guide:claude:mcp".to_string(),
            tool: Some("claude".to_string()),
            kind: "mcp".to_string(),
            title: "Claude Code MCP 使用说明".to_string(),
            summary: "说明如何识别和安全管理 Claude Code MCP。".to_string(),
            sections: vec![ResourceGuideSection {
                title: "安全提醒".to_string(),
                body: "所有写入必须经过预览、备份、校验和回滚链路。".to_string(),
            }],
            risks: vec!["备份可能包含目标配置中原有密钥".to_string()],
            unsupported_actions: vec!["enterprise 范围资源不允许写入".to_string()],
            updated_at: "2026-06-06T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(guide).unwrap();

        assert_eq!(json["id"], "guide:claude:mcp");
        assert_eq!(json["kind"], "mcp");
        assert!(
            json["sections"][0]["body"]
                .as_str()
                .unwrap()
                .contains("回滚")
        );
        assert!(
            !json["sections"][0]["body"]
                .as_str()
                .unwrap()
                .contains("secret-token")
        );
    }
}
