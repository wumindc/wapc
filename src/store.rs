//! SQLite persistence for normalized usage records.
//! @author codex

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use rusqlite::{Connection, params, params_from_iter};
use serde::Serialize;
use serde_json::Value;

use crate::model::{
    CanonicalResource, CostRecomputeResult, DetectedTool, PricingRule, ProjectAlias,
    ProjectSummary, ResourceBackup, ResourceChangeLog, ResourceParseFailure, ResourceTemplate,
    SessionMeta, SourceHealth, SyncOperation, SyncPreset, TokenUsage, UsageRecord,
};

pub struct UsageStore {
    conn: Connection,
}

#[derive(Clone, Debug, Serialize)]
pub struct UsageSummary {
    pub name: String,
    pub records: u64,
    pub usage: TokenUsage,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectModelSummary {
    pub canonical_path: String,
    pub tool: String,
    pub model: String,
    pub records: u64,
    pub usage: TokenUsage,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct DailyToolSummary {
    pub day: String,
    pub tool: String,
    pub total_tokens: u64,
}

impl UsageStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn backup_database(&self, target_path: &Path) -> Result<()> {
        if target_path.exists() {
            std::fs::remove_file(target_path)?;
        }
        self.conn.execute("VACUUM INTO ?1", params![target_path.to_string_lossy()])?;
        Ok(())
    }

    pub fn upsert_records(&self, records: &[UsageRecord]) -> Result<usize> {
        let mut changed = 0;
        for record in records {
            self.conn.execute(
                "INSERT INTO usage_records (
                    id, tool, source_path, session_id, ts, project_path, model,
                    input_tokens, output_tokens, cache_read_tokens, cache_write_tokens,
                    reasoning_tokens, tool_tokens, total_tokens, cost_usd, precision, cost_source
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                ON CONFLICT(id) DO UPDATE SET
                    tool = excluded.tool,
                    source_path = excluded.source_path,
                    session_id = excluded.session_id,
                    ts = excluded.ts,
                    project_path = excluded.project_path,
                    model = excluded.model,
                    input_tokens = excluded.input_tokens,
                    output_tokens = excluded.output_tokens,
                    cache_read_tokens = excluded.cache_read_tokens,
                    cache_write_tokens = excluded.cache_write_tokens,
                    reasoning_tokens = excluded.reasoning_tokens,
                    tool_tokens = excluded.tool_tokens,
                    total_tokens = excluded.total_tokens,
                    cost_usd = excluded.cost_usd,
                    precision = excluded.precision,
                    cost_source = excluded.cost_source",
                params![
                    record.id,
                    record.tool.as_str(),
                    record.source_path,
                    record.session_id,
                    record.timestamp.map(|dt| dt.to_rfc3339()),
                    record.project_path,
                    record.model,
                    record.usage.input as i64,
                    record.usage.output as i64,
                    record.usage.cache_read as i64,
                    record.usage.cache_write as i64,
                    record.usage.reasoning as i64,
                    record.usage.tool as i64,
                    record.usage.total() as i64,
                    record.cost_usd,
                    record.precision.as_str(),
                    if record.cost_usd.is_some() {
                        "imported"
                    } else {
                        "none"
                    },
                ],
            )?;
            changed += 1;
        }
        Ok(changed)
    }

    pub fn list_pricing_rules(&self) -> Result<Vec<PricingRule>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, model_match, match_kind, provider, currency, price_input,
                price_output, price_cache_read, price_cache_write, price_reasoning,
                price_tool, source, updated_at
            FROM pricing_rules
            ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], row_to_pricing_rule)?;
        let mut rules = Vec::new();
        for row in rows {
            rules.push(row?);
        }
        Ok(rules)
    }

    pub fn upsert_pricing_rule(&self, rule: &PricingRule) -> Result<PricingRule> {
        match rule.id {
            Some(id) => {
                self.conn.execute(
                    "UPDATE pricing_rules SET
                        model_match = ?1,
                        match_kind = ?2,
                        provider = ?3,
                        currency = ?4,
                        price_input = ?5,
                        price_output = ?6,
                        price_cache_read = ?7,
                        price_cache_write = ?8,
                        price_reasoning = ?9,
                        price_tool = ?10,
                        source = ?11,
                        updated_at = ?12
                    WHERE id = ?13",
                    params![
                        rule.model_match,
                        rule.match_kind,
                        rule.provider,
                        rule.currency,
                        rule.price_input,
                        rule.price_output,
                        rule.price_cache_read,
                        rule.price_cache_write,
                        rule.price_reasoning,
                        rule.price_tool,
                        rule.source,
                        rule.updated_at,
                        id,
                    ],
                )?;
                Ok(PricingRule {
                    id: Some(id),
                    ..rule.clone()
                })
            }
            None => {
                self.conn.execute(
                    "INSERT INTO pricing_rules (
                        model_match, match_kind, provider, currency, price_input,
                        price_output, price_cache_read, price_cache_write, price_reasoning,
                        price_tool, source, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        rule.model_match,
                        rule.match_kind,
                        rule.provider,
                        rule.currency,
                        rule.price_input,
                        rule.price_output,
                        rule.price_cache_read,
                        rule.price_cache_write,
                        rule.price_reasoning,
                        rule.price_tool,
                        rule.source,
                        rule.updated_at,
                    ],
                )?;
                Ok(PricingRule {
                    id: Some(self.conn.last_insert_rowid()),
                    ..rule.clone()
                })
            }
        }
    }

    pub fn delete_pricing_rule(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM pricing_rules WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn recompute_costs(&self) -> Result<CostRecomputeResult> {
        let rules = self.list_pricing_rules()?;
        let mut rows = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT id, model, input_tokens, output_tokens, cache_read_tokens,
                cache_write_tokens, reasoning_tokens, tool_tokens
            FROM usage_records",
        )?;
        let mapped = stmt.query_map([], |row| {
            Ok(UsageCostRow {
                id: row.get(0)?,
                model: row.get(1)?,
                usage: TokenUsage {
                    input: row.get::<_, i64>(2)? as u64,
                    output: row.get::<_, i64>(3)? as u64,
                    cache_read: row.get::<_, i64>(4)? as u64,
                    cache_write: row.get::<_, i64>(5)? as u64,
                    reasoning: row.get::<_, i64>(6)? as u64,
                    tool: row.get::<_, i64>(7)? as u64,
                },
            })
        })?;
        for row in mapped {
            rows.push(row?);
        }
        drop(stmt);

        let mut result = CostRecomputeResult {
            updated: 0,
            exact_matches: 0,
            prefix_matches: 0,
            no_matches: 0,
        };
        for row in rows {
            let matched = row
                .model
                .as_deref()
                .and_then(|model| best_pricing_rule(model, &rules));
            let (cost, source) = match matched {
                Some(rule) => {
                    let cost = calculate_cost_usd(&row.usage, rule);
                    if rule.match_kind == "exact" {
                        result.exact_matches += 1;
                        (Some(cost), "exact")
                    } else {
                        result.prefix_matches += 1;
                        (Some(cost), "prefix")
                    }
                }
                None => {
                    result.no_matches += 1;
                    (None, "none")
                }
            };
            self.conn.execute(
                "UPDATE usage_records SET cost_usd = ?1, cost_source = ?2 WHERE id = ?3",
                params![cost, source, row.id],
            )?;
            result.updated += 1;
        }
        Ok(result)
    }

    pub fn record_cost(&self, id: &str) -> Result<(Option<f64>, String)> {
        let mut stmt = self
            .conn
            .prepare("SELECT cost_usd, cost_source FROM usage_records WHERE id = ?1")?;
        let value = stmt.query_row(params![id], |row| {
            Ok((row.get::<_, Option<f64>>(0)?, row.get(1)?))
        })?;
        Ok(value)
    }

    pub fn list_project_aliases(&self) -> Result<Vec<ProjectAlias>> {
        let mut stmt = self.conn.prepare(
            "SELECT canonical_path, alias, updated_at
            FROM project_aliases
            ORDER BY canonical_path ASC",
        )?;
        let rows = stmt.query_map([], row_to_project_alias)?;
        let mut aliases = Vec::new();
        for row in rows {
            aliases.push(row?);
        }
        Ok(aliases)
    }

    pub fn set_project_alias(&self, alias: &ProjectAlias) -> Result<ProjectAlias> {
        let canonical_path = normalize_project_path(&alias.canonical_path);
        self.conn.execute(
            "INSERT INTO project_aliases (canonical_path, alias, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(canonical_path) DO UPDATE SET
                alias = excluded.alias,
                updated_at = excluded.updated_at",
            params![canonical_path, alias.alias, alias.updated_at],
        )?;
        Ok(ProjectAlias {
            canonical_path,
            alias: alias.alias.clone(),
            updated_at: alias.updated_at.clone(),
        })
    }

    pub fn project_summaries(&self) -> Result<Vec<ProjectSummary>> {
        self.project_summaries_in_window(None, None)
    }

    pub fn project_summaries_in_window(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<ProjectSummary>> {
        let aliases = self
            .list_project_aliases()?
            .into_iter()
            .map(|alias| (alias.canonical_path, alias.alias))
            .collect::<HashMap<_, _>>();
        let (where_clause, params) = usage_time_window_clause(from, to);
        let sql = format!(
            "SELECT tool, project_path, COUNT(*), SUM(input_tokens), SUM(output_tokens),
                SUM(cache_read_tokens), SUM(cache_write_tokens), SUM(reasoning_tokens),
                SUM(tool_tokens), SUM(cost_usd)
            FROM usage_records{where_clause}
            GROUP BY tool, project_path"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
            Ok(ProjectAggregateRow {
                tool: row.get(0)?,
                project_path: row.get(1)?,
                records: row.get::<_, i64>(2)? as u64,
                usage: TokenUsage {
                    input: row.get::<_, Option<i64>>(3)?.unwrap_or(0) as u64,
                    output: row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
                    cache_read: row.get::<_, Option<i64>>(5)?.unwrap_or(0) as u64,
                    cache_write: row.get::<_, Option<i64>>(6)?.unwrap_or(0) as u64,
                    reasoning: row.get::<_, Option<i64>>(7)?.unwrap_or(0) as u64,
                    tool: row.get::<_, Option<i64>>(8)?.unwrap_or(0) as u64,
                },
                cost_usd: row.get::<_, Option<f64>>(9)?.unwrap_or(0.0),
            })
        })?;
        let mut groups = HashMap::<String, ProjectAccumulator>::new();
        for row in rows {
            let row = row?;
            let canonical_path = row
                .project_path
                .as_deref()
                .map(normalize_project_path)
                .unwrap_or_else(|| "(unknown)".to_string());
            let entry = groups
                .entry(canonical_path.clone())
                .or_insert_with(|| ProjectAccumulator::new(canonical_path));
            entry.records += row.records;
            entry.usage = entry.usage.clone() + row.usage;
            entry.cost_usd += row.cost_usd;
            entry.tools.insert(row.tool);
            if let Some(path) = row.project_path {
                entry.original_paths.insert(path);
            }
        }

        let mut summaries = groups
            .into_values()
            .map(|group| {
                let alias = aliases.get(&group.canonical_path).cloned();
                let display_name = alias
                    .clone()
                    .unwrap_or_else(|| display_project_path(&group.canonical_path));
                ProjectSummary {
                    canonical_path: group.canonical_path,
                    display_name,
                    alias,
                    original_paths: group.original_paths.into_iter().collect(),
                    tools: group.tools.into_iter().collect(),
                    records: group.records,
                    usage: group.usage,
                    cost_usd: group.cost_usd,
                }
            })
            .collect::<Vec<_>>();
        summaries.sort_by_key(|summary| std::cmp::Reverse(summary.usage.total()));
        Ok(summaries)
    }

    pub fn project_model_summaries(&self) -> Result<Vec<ProjectModelSummary>> {
        self.project_model_summaries_in_window(None, None)
    }

    pub fn project_model_summaries_in_window(
        &self,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<ProjectModelSummary>> {
        let (where_clause, params) = usage_time_window_clause(from, to);
        let sql = format!(
            "SELECT project_path, tool, COALESCE(model, 'unknown'), COUNT(*),
                SUM(input_tokens), SUM(output_tokens), SUM(cache_read_tokens),
                SUM(cache_write_tokens), SUM(reasoning_tokens), SUM(tool_tokens), SUM(cost_usd)
            FROM usage_records{where_clause}
            GROUP BY project_path, tool, COALESCE(model, 'unknown')
            ORDER BY project_path ASC, tool ASC, COALESCE(model, 'unknown') ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
            let project_path = row.get::<_, Option<String>>(0)?;
            Ok(ProjectModelSummary {
                canonical_path: project_path
                    .as_deref()
                    .map(normalize_project_path)
                    .unwrap_or_else(|| "(unknown)".to_string()),
                tool: row.get(1)?,
                model: row.get(2)?,
                records: row.get::<_, i64>(3)? as u64,
                usage: TokenUsage {
                    input: row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
                    output: row.get::<_, Option<i64>>(5)?.unwrap_or(0) as u64,
                    cache_read: row.get::<_, Option<i64>>(6)?.unwrap_or(0) as u64,
                    cache_write: row.get::<_, Option<i64>>(7)?.unwrap_or(0) as u64,
                    reasoning: row.get::<_, Option<i64>>(8)?.unwrap_or(0) as u64,
                    tool: row.get::<_, Option<i64>>(9)?.unwrap_or(0) as u64,
                },
                cost_usd: row.get::<_, Option<f64>>(10)?.unwrap_or(0.0),
            })
        })?;
        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(row?);
        }
        Ok(summaries)
    }

    pub fn raw_project_paths(&self) -> Result<Vec<Option<String>>> {
        let mut stmt = self
            .conn
            .prepare("SELECT project_path FROM usage_records ORDER BY project_path ASC")?;
        let rows = stmt.query_map([], |row| row.get::<_, Option<String>>(0))?;
        let mut paths = Vec::new();
        for row in rows {
            paths.push(row?);
        }
        Ok(paths)
    }

    /// Return the total number of indexed usage records from DB without scanning files.
    /// @author Claude Sonnet 4.6 (Thinking)
    pub fn count_records(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM usage_records", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn upsert_tools(&self, tools: &[DetectedTool]) -> Result<usize> {
        let mut changed = 0;
        for tool in tools {
            self.conn.execute(
                "INSERT INTO tools (
                    id, display_name, installed, version, config_dir, data_dir,
                    config_dir_exists, data_dir_exists, last_detected_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    display_name = excluded.display_name,
                    installed = excluded.installed,
                    version = excluded.version,
                    config_dir = excluded.config_dir,
                    data_dir = excluded.data_dir,
                    config_dir_exists = excluded.config_dir_exists,
                    data_dir_exists = excluded.data_dir_exists,
                    last_detected_at = excluded.last_detected_at",
                params![
                    tool.id,
                    tool.display_name,
                    tool.installed as i64,
                    tool.version,
                    tool.config_dir,
                    tool.data_dir,
                    tool.config_dir_exists as i64,
                    tool.data_dir_exists as i64,
                    tool.last_detected_at,
                ],
            )?;
            changed += 1;
        }
        Ok(changed)
    }

    pub fn list_tools(&self) -> Result<Vec<DetectedTool>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, display_name, installed, version, config_dir, data_dir,
                config_dir_exists, data_dir_exists, last_detected_at
            FROM tools
            ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], row_to_detected_tool)?;
        let mut tools = Vec::new();
        for row in rows {
            tools.push(row?);
        }
        Ok(tools)
    }

    pub fn insert_source_health(&self, sources: &[SourceHealth]) -> Result<usize> {
        let mut changed = 0;
        for source in sources {
            self.conn.execute(
                "INSERT INTO source_health (
                    tool, source_glob, exists_flag, readable_files, parsed_records,
                    failed_files, latest_event_ts, checked_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    source.tool,
                    source.source_glob,
                    source.exists as i64,
                    source.readable_files as i64,
                    source.parsed_records as i64,
                    source.failed_files as i64,
                    source.latest_event_ts,
                    source.checked_at,
                ],
            )?;
            changed += 1;
        }
        Ok(changed)
    }

    pub fn latest_source_health(&self) -> Result<Vec<SourceHealth>> {
        let mut stmt = self.conn.prepare(
            "SELECT sh.tool, sh.source_glob, sh.exists_flag, sh.readable_files,
                sh.parsed_records, sh.failed_files, sh.latest_event_ts, sh.checked_at
            FROM source_health sh
            INNER JOIN (
                SELECT tool, source_glob, MAX(checked_at) AS checked_at
                FROM source_health
                GROUP BY tool, source_glob
            ) latest
            ON sh.tool = latest.tool
                AND sh.source_glob = latest.source_glob
                AND sh.checked_at = latest.checked_at
            ORDER BY sh.tool ASC, sh.source_glob ASC",
        )?;
        let rows = stmt.query_map([], row_to_source_health)?;
        let mut sources = Vec::new();
        for row in rows {
            sources.push(row?);
        }
        Ok(sources)
    }

    pub fn upsert_resources(&self, resources: &[CanonicalResource]) -> Result<usize> {
        let mut changed = 0;
        for resource in resources {
            ensure_resource_payload_is_safe(resource)?;
            self.conn.execute(
                "INSERT INTO resources (
                    id, kind, name, scope, origin_tool, origin_path, origin_locator,
                    enabled_in_json, confidence, redacted, payload_json, provided_by_plugin,
                    last_seen
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                ON CONFLICT(id) DO UPDATE SET
                    kind = excluded.kind,
                    name = excluded.name,
                    scope = excluded.scope,
                    origin_tool = excluded.origin_tool,
                    origin_path = excluded.origin_path,
                    origin_locator = excluded.origin_locator,
                    enabled_in_json = excluded.enabled_in_json,
                    confidence = excluded.confidence,
                    redacted = excluded.redacted,
                    payload_json = excluded.payload_json,
                    provided_by_plugin = excluded.provided_by_plugin,
                    last_seen = excluded.last_seen",
                params![
                    resource.id,
                    resource.kind,
                    resource.name,
                    resource.scope,
                    resource.origin_tool,
                    resource.origin_path,
                    resource.origin_locator,
                    serde_json::to_string(&resource.enabled_in)?,
                    resource.confidence,
                    resource.redacted as i64,
                    resource.payload_json,
                    resource.provided_by_plugin,
                    resource.last_seen,
                ],
            )?;
            changed += 1;
        }
        Ok(changed)
    }

    pub fn list_resources(
        &self,
        kind: Option<&str>,
        tool: Option<&str>,
        scope: Option<&str>,
        query: Option<&str>,
    ) -> Result<Vec<CanonicalResource>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, name, scope, origin_tool, origin_path, origin_locator,
                enabled_in_json, confidence, redacted, payload_json, provided_by_plugin,
                last_seen
            FROM resources
            ORDER BY kind ASC, name ASC, id ASC",
        )?;
        let rows = stmt.query_map([], row_to_resource)?;
        let query = query.map(str::to_ascii_lowercase);
        let mut resources = Vec::new();
        for row in rows {
            let resource = row?;
            if kind.is_some_and(|value| resource.kind != value) {
                continue;
            }
            if scope.is_some_and(|value| resource.scope != value) {
                continue;
            }
            if tool.is_some_and(|value| !resource.enabled_in.iter().any(|tool| tool == value)) {
                continue;
            }
            if let Some(query) = &query {
                let haystack = format!(
                    "{}\n{}\n{}\n{}",
                    resource.name, resource.kind, resource.scope, resource.origin_tool
                )
                .to_ascii_lowercase();
                if !haystack.contains(query) {
                    continue;
                }
            }
            resources.push(resource);
        }
        Ok(resources)
    }

    pub fn get_resource(&self, id: &str) -> Result<Option<CanonicalResource>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, name, scope, origin_tool, origin_path, origin_locator,
                enabled_in_json, confidence, redacted, payload_json, provided_by_plugin,
                last_seen
            FROM resources
            WHERE id = ?1",
        )?;
        match stmt.query_row(params![id], row_to_resource) {
            Ok(resource) => Ok(Some(resource)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn upsert_resource_templates(&self, templates: &[ResourceTemplate]) -> Result<usize> {
        let mut changed = 0;
        for template in templates {
            ensure_resource_template_is_safe(template)?;
            self.conn.execute(
                "INSERT INTO resource_templates (
                    id, name, kind, scope, description, source, content_fingerprint,
                    required_env_keys_json, payload_json, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    kind = excluded.kind,
                    scope = excluded.scope,
                    description = excluded.description,
                    source = excluded.source,
                    content_fingerprint = excluded.content_fingerprint,
                    required_env_keys_json = excluded.required_env_keys_json,
                    payload_json = excluded.payload_json,
                    updated_at = excluded.updated_at",
                params![
                    template.id,
                    template.name,
                    template.kind,
                    template.scope,
                    template.description,
                    template.source,
                    template.content_fingerprint,
                    serde_json::to_string(&template.required_env_keys)?,
                    template.payload_json,
                    template.updated_at,
                ],
            )?;
            changed += 1;
        }
        Ok(changed)
    }

    pub fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, scope, description, source, content_fingerprint,
                required_env_keys_json, payload_json, updated_at
            FROM resource_templates
            ORDER BY kind ASC, name ASC, id ASC",
        )?;
        let rows = stmt.query_map([], row_to_resource_template)?;
        let mut templates = Vec::new();
        for row in rows {
            templates.push(row?);
        }
        Ok(templates)
    }

    pub fn get_resource_template(&self, id: &str) -> Result<Option<ResourceTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, scope, description, source, content_fingerprint,
                required_env_keys_json, payload_json, updated_at
            FROM resource_templates
            WHERE id = ?1",
        )?;
        match stmt.query_row(params![id], row_to_resource_template) {
            Ok(template) => Ok(Some(template)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn insert_resource_parse_failures(
        &self,
        failures: &[ResourceParseFailure],
    ) -> Result<usize> {
        let mut changed = 0;
        for failure in failures {
            self.conn.execute(
                "INSERT INTO resource_parse_failures (path, tool, kind, reason, seen_at)
                VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    failure.path,
                    failure.tool,
                    failure.kind,
                    failure.reason,
                    failure.seen_at,
                ],
            )?;
            changed += 1;
        }
        Ok(changed)
    }

    pub fn list_resource_parse_failures(&self) -> Result<Vec<ResourceParseFailure>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, tool, kind, reason, seen_at
            FROM resource_parse_failures
            ORDER BY seen_at DESC, tool ASC, path ASC",
        )?;
        let rows = stmt.query_map([], row_to_resource_parse_failure)?;
        let mut failures = Vec::new();
        for row in rows {
            failures.push(row?);
        }
        Ok(failures)
    }

    pub fn record_file_fingerprint(
        &self,
        tool: &str,
        path: &str,
        fingerprint: &str,
        observed_at: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO file_fingerprints (tool, path, fingerprint, observed_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(tool, path) DO UPDATE SET
                fingerprint = excluded.fingerprint,
                observed_at = excluded.observed_at",
            params![tool, path, fingerprint, observed_at],
        )?;
        Ok(())
    }

    pub fn get_file_fingerprint(&self, tool: &str, path: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT fingerprint FROM file_fingerprints WHERE tool = ?1 AND path = ?2")?;
        match stmt.query_row(params![tool, path], |row| row.get(0)) {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn insert_resource_change(&self, change: &ResourceChangeLog) -> Result<()> {
        self.conn.execute(
            "INSERT INTO resource_changes (
                change_id, sync_id, tool, resource_id, kind, op, target_path, backup_path,
                status, reverts_change_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                change.change_id,
                change.sync_id,
                change.tool,
                change.resource_id,
                change.kind,
                change.op,
                change.target_path,
                change.backup_path,
                change.status,
                change.reverts_change_id,
                change.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_resource_change(&self, change_id: &str) -> Result<Option<ResourceChangeLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT change_id, sync_id, tool, resource_id, kind, op, target_path, backup_path,
                status, reverts_change_id, created_at
            FROM resource_changes
            WHERE change_id = ?1",
        )?;
        match stmt.query_row(params![change_id], row_to_resource_change) {
            Ok(change) => Ok(Some(change)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn update_resource_change_status(&self, change_id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE resource_changes SET status = ?1 WHERE change_id = ?2",
            params![status, change_id],
        )?;
        Ok(())
    }

    pub fn list_resource_changes(&self, tool: Option<&str>) -> Result<Vec<ResourceChangeLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT change_id, sync_id, tool, resource_id, kind, op, target_path, backup_path,
                status, reverts_change_id, created_at
            FROM resource_changes
            ORDER BY created_at DESC, change_id DESC",
        )?;
        let rows = stmt.query_map([], row_to_resource_change)?;
        let mut changes = Vec::new();
        for row in rows {
            let change = row?;
            if tool.is_some_and(|value| change.tool != value) {
                continue;
            }
            changes.push(change);
        }
        Ok(changes)
    }

    pub fn insert_sync_operation(&self, operation: &SyncOperation) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_operations (
                sync_id, source_resource_id, targets_json, allow_cross_scope, env_strategy,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                operation.sync_id,
                operation.source_resource_id,
                operation.targets_json,
                operation.allow_cross_scope,
                operation.env_strategy,
                operation.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_sync_operations(&self) -> Result<Vec<SyncOperation>> {
        let mut stmt = self.conn.prepare(
            "SELECT sync_id, source_resource_id, targets_json, allow_cross_scope, env_strategy,
                created_at
            FROM sync_operations
            ORDER BY created_at DESC, sync_id DESC",
        )?;
        let rows = stmt.query_map([], row_to_sync_operation)?;
        let mut operations = Vec::new();
        for row in rows {
            operations.push(row?);
        }
        Ok(operations)
    }

    pub fn save_sync_preset(&self, preset: &SyncPreset) -> Result<SyncPreset> {
        ensure_sync_preset_is_safe(preset)?;
        self.conn.execute(
            "INSERT INTO sync_presets (
                id, name, resources_json, targets_json, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                resources_json = excluded.resources_json,
                targets_json = excluded.targets_json,
                updated_at = excluded.updated_at",
            params![
                preset.id,
                preset.name,
                preset.resources_json,
                preset.targets_json,
                preset.updated_at,
            ],
        )?;
        Ok(preset.clone())
    }

    pub fn list_sync_presets(&self) -> Result<Vec<SyncPreset>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, resources_json, targets_json, updated_at
            FROM sync_presets
            ORDER BY updated_at DESC, name ASC, id ASC",
        )?;
        let rows = stmt.query_map([], row_to_sync_preset)?;
        let mut presets = Vec::new();
        for row in rows {
            presets.push(row?);
        }
        Ok(presets)
    }

    pub fn delete_sync_preset(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM sync_presets WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn insert_resource_backup(&self, backup: &ResourceBackup) -> Result<()> {
        self.conn.execute(
            "INSERT INTO resource_backups (
                backup_path, tool, original_path, change_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                backup.backup_path,
                backup.tool,
                backup.original_path,
                backup.change_id,
                backup.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete_resource_backup(&self, backup_path: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM resource_backups WHERE backup_path = ?1",
            params![backup_path],
        )?;
        Ok(())
    }

    pub fn list_resource_backups(&self, tool: Option<&str>) -> Result<Vec<ResourceBackup>> {
        let mut stmt = self.conn.prepare(
            "SELECT backup_path, tool, original_path, change_id, created_at
            FROM resource_backups
            ORDER BY created_at DESC, backup_path DESC",
        )?;
        let rows = stmt.query_map([], row_to_resource_backup)?;
        let mut backups = Vec::new();
        for row in rows {
            let backup = row?;
            if tool.is_some_and(|value| backup.tool != value) {
                continue;
            }
            backups.push(backup);
        }
        Ok(backups)
    }

    pub fn list_sessions(
        &self,
        tool: Option<&str>,
        project: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
        query: Option<&str>,
    ) -> Result<Vec<SessionMeta>> {
        let mut stmt = self.conn.prepare(
            "SELECT tool, session_id, project_path, MIN(ts), MAX(ts), COUNT(*),
                SUM(total_tokens), SUM(cost_usd), GROUP_CONCAT(DISTINCT source_path)
            FROM usage_records
            WHERE session_id IS NOT NULL
            GROUP BY tool, session_id, project_path
            ORDER BY MAX(ts) DESC, tool ASC, session_id ASC",
        )?;
        let rows = stmt.query_map([], row_to_session_meta)?;
        let query = query.map(str::to_ascii_lowercase);
        let mut sessions = Vec::new();
        for row in rows {
            let session = row?;
            if tool.is_some_and(|value| session.tool != value) {
                continue;
            }
            if project.is_some_and(|value| session.project_path.as_deref() != Some(value)) {
                continue;
            }
            if from.is_some_and(|value| {
                session
                    .last_ts
                    .as_deref()
                    .is_some_and(|last_ts| last_ts < value)
            }) {
                continue;
            }
            if to.is_some_and(|value| {
                session
                    .first_ts
                    .as_deref()
                    .is_some_and(|first_ts| first_ts > value)
            }) {
                continue;
            }
            if let Some(query) = &query {
                let haystack = format!(
                    "{}\n{}\n{}",
                    session.session_id,
                    session.tool,
                    session.project_path.as_deref().unwrap_or("")
                )
                .to_ascii_lowercase();
                if !haystack.contains(query) {
                    continue;
                }
            }
            sessions.push(session);
        }
        Ok(sessions)
    }

    pub fn summary_by_tool(&self, tool: Option<&str>) -> Result<Vec<UsageSummary>> {
        self.summary_by_tool_filtered(tool, None)
    }

    pub fn summary_by_tool_filtered(
        &self,
        tool: Option<&str>,
        day_prefix: Option<&str>,
    ) -> Result<Vec<UsageSummary>> {
        self.summary_grouped("tool", tool, day_prefix)
    }

    pub fn summary_by_project_filtered(
        &self,
        project: Option<&str>,
        day_prefix: Option<&str>,
    ) -> Result<Vec<UsageSummary>> {
        self.summary_grouped("COALESCE(project_path, '(unknown)')", project, day_prefix)
    }

    pub fn summary_by_day(&self) -> Result<Vec<UsageSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT substr(ts, 1, 10) AS day, COUNT(*), SUM(input_tokens), SUM(output_tokens), SUM(cache_read_tokens),
            SUM(cache_write_tokens), SUM(reasoning_tokens), SUM(tool_tokens), SUM(cost_usd)
            FROM usage_records
            WHERE ts IS NOT NULL
            GROUP BY day
            ORDER BY day DESC"
        )?;
        let rows = stmt.query_map([], row_to_summary)?;
        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(row?);
        }
        Ok(summaries)
    }

    pub fn daily_tool_totals(&self, days: &[String]) -> Result<Vec<DailyToolSummary>> {
        if days.is_empty() {
            return Ok(Vec::new());
        }
        let day_set = days.iter().map(String::as_str).collect::<HashSet<_>>();
        let mut stmt = self.conn.prepare(
            "SELECT substr(ts, 1, 10) AS day, tool, SUM(total_tokens)
            FROM usage_records
            WHERE ts IS NOT NULL
            GROUP BY day, tool
            ORDER BY day ASC, SUM(total_tokens) DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DailyToolSummary {
                day: row.get(0)?,
                tool: row.get(1)?,
                total_tokens: row.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
            })
        })?;
        let mut summaries = Vec::new();
        for row in rows {
            let summary = row?;
            if day_set.contains(summary.day.as_str()) {
                summaries.push(summary);
            }
        }
        Ok(summaries)
    }

    fn summary_grouped(
        &self,
        group_expr: &str,
        name_filter: Option<&str>,
        day_prefix: Option<&str>,
    ) -> Result<Vec<UsageSummary>> {
        let select =
            format!("SELECT {group_expr}, COUNT(*), SUM(input_tokens), SUM(output_tokens), SUM(cache_read_tokens),
            SUM(cache_write_tokens), SUM(reasoning_tokens), SUM(tool_tokens), SUM(cost_usd)
            FROM usage_records");
        let group = format!(" GROUP BY {group_expr} ORDER BY SUM(total_tokens) DESC");
        let filter_column = if group_expr == "tool" {
            "tool"
        } else {
            "project_path"
        };
        let sql = match (name_filter, day_prefix) {
            (Some(_), Some(_)) => {
                format!("{select} WHERE {filter_column} = ?1 AND ts LIKE ?2{group}")
            }
            (Some(_), None) => format!("{select} WHERE {filter_column} = ?1{group}"),
            (None, Some(_)) => format!("{select} WHERE ts LIKE ?1{group}"),
            (None, None) => format!("{select}{group}"),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let day_like = day_prefix.map(|day| format!("{day}%"));
        let rows = match (name_filter, day_like.as_deref()) {
            (Some(name), Some(day)) => stmt.query_map((name, day), row_to_summary)?,
            (Some(name), None) => stmt.query_map([name], row_to_summary)?,
            (None, Some(day)) => stmt.query_map([day], row_to_summary)?,
            (None, None) => stmt.query_map([], row_to_summary)?,
        };
        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(row?);
        }
        Ok(summaries)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_records (
                id TEXT PRIMARY KEY,
                tool TEXT NOT NULL,
                source_path TEXT NOT NULL,
                session_id TEXT,
                ts TEXT,
                project_path TEXT,
                model TEXT,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                cache_read_tokens INTEGER NOT NULL,
                cache_write_tokens INTEGER NOT NULL,
                reasoning_tokens INTEGER NOT NULL,
                tool_tokens INTEGER NOT NULL,
                total_tokens INTEGER NOT NULL,
                cost_usd REAL,
                precision TEXT NOT NULL,
                cost_source TEXT NOT NULL DEFAULT 'none'
            );
            CREATE INDEX IF NOT EXISTS idx_usage_tool ON usage_records(tool);
            CREATE INDEX IF NOT EXISTS idx_usage_project ON usage_records(project_path);
            CREATE INDEX IF NOT EXISTS idx_usage_ts ON usage_records(ts);
            CREATE TABLE IF NOT EXISTS tools (
                id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                installed INTEGER NOT NULL,
                version TEXT,
                config_dir TEXT,
                data_dir TEXT,
                config_dir_exists INTEGER NOT NULL,
                data_dir_exists INTEGER NOT NULL,
                last_detected_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS source_health (
                tool TEXT NOT NULL,
                source_glob TEXT NOT NULL,
                exists_flag INTEGER NOT NULL,
                readable_files INTEGER NOT NULL,
                parsed_records INTEGER NOT NULL,
                failed_files INTEGER NOT NULL,
                latest_event_ts TEXT,
                checked_at TEXT NOT NULL,
                PRIMARY KEY (tool, source_glob, checked_at)
            );
            CREATE TABLE IF NOT EXISTS pricing_rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                model_match TEXT NOT NULL,
                match_kind TEXT NOT NULL,
                provider TEXT,
                currency TEXT NOT NULL DEFAULT 'USD',
                price_input REAL,
                price_output REAL,
                price_cache_read REAL,
                price_cache_write REAL,
                price_reasoning REAL,
                price_tool REAL,
                source TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_aliases (
                canonical_path TEXT PRIMARY KEY,
                alias TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS resources (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                scope TEXT NOT NULL,
                origin_tool TEXT NOT NULL,
                origin_path TEXT NOT NULL,
                origin_locator TEXT,
                enabled_in_json TEXT NOT NULL,
                confidence REAL NOT NULL,
                redacted INTEGER NOT NULL,
                payload_json TEXT NOT NULL,
                provided_by_plugin TEXT,
                last_seen TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_resources_kind ON resources(kind);
            CREATE INDEX IF NOT EXISTS idx_resources_scope ON resources(scope);
            CREATE TABLE IF NOT EXISTS resource_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                scope TEXT NOT NULL,
                description TEXT NOT NULL,
                source TEXT NOT NULL,
                content_fingerprint TEXT NOT NULL,
                required_env_keys_json TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_resource_templates_kind ON resource_templates(kind);
            CREATE TABLE IF NOT EXISTS resource_parse_failures (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                tool TEXT NOT NULL,
                kind TEXT,
                reason TEXT NOT NULL,
                seen_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS resource_changes (
                change_id TEXT PRIMARY KEY,
                sync_id TEXT,
                tool TEXT NOT NULL,
                resource_id TEXT,
                kind TEXT NOT NULL,
                op TEXT NOT NULL,
                target_path TEXT NOT NULL,
                backup_path TEXT,
                status TEXT NOT NULL,
                reverts_change_id TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS resource_backups (
                backup_path TEXT PRIMARY KEY,
                tool TEXT NOT NULL,
                original_path TEXT NOT NULL,
                change_id TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS file_fingerprints (
                tool TEXT NOT NULL,
                path TEXT NOT NULL,
                fingerprint TEXT NOT NULL,
                observed_at TEXT NOT NULL,
                PRIMARY KEY (tool, path)
            );
            CREATE TABLE IF NOT EXISTS sync_operations (
                sync_id TEXT PRIMARY KEY,
                source_resource_id TEXT,
                targets_json TEXT NOT NULL,
                allow_cross_scope INTEGER NOT NULL,
                env_strategy TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sync_presets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                resources_json TEXT NOT NULL,
                targets_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS app_settings (
                setting_key TEXT PRIMARY KEY,
                setting_value TEXT NOT NULL
            );",
        )?;
        self.ensure_usage_cost_source_column()?;
        self.ensure_resource_changes_sync_id_column()?;
        Ok(())
    }

    fn ensure_usage_cost_source_column(&self) -> Result<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(usage_records)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for column in columns {
            if column? == "cost_source" {
                return Ok(());
            }
        }
        self.conn.execute(
            "ALTER TABLE usage_records ADD COLUMN cost_source TEXT NOT NULL DEFAULT 'none'",
            [],
        )?;
        Ok(())
    }

    fn ensure_resource_changes_sync_id_column(&self) -> Result<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(resource_changes)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for column in columns {
            if column? == "sync_id" {
                return Ok(());
            }
        }
        self.conn
            .execute("ALTER TABLE resource_changes ADD COLUMN sync_id TEXT", [])?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT setting_value FROM app_settings WHERE setting_key = ?1")?;
        match stmt.query_row(params![key], |row| row.get(0)) {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO app_settings (setting_key, setting_value) VALUES (?1, ?2)
             ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value",
            params![key, value],
        )?;
        Ok(())
    }
}

struct UsageCostRow {
    id: String,
    model: Option<String>,
    usage: TokenUsage,
}

struct ProjectAggregateRow {
    tool: String,
    project_path: Option<String>,
    records: u64,
    usage: TokenUsage,
    cost_usd: f64,
}

struct ProjectAccumulator {
    canonical_path: String,
    records: u64,
    usage: TokenUsage,
    cost_usd: f64,
    tools: BTreeSet<String>,
    original_paths: BTreeSet<String>,
}

impl ProjectAccumulator {
    fn new(canonical_path: String) -> Self {
        Self {
            canonical_path,
            records: 0,
            usage: TokenUsage::default(),
            cost_usd: 0.0,
            tools: BTreeSet::new(),
            original_paths: BTreeSet::new(),
        }
    }
}

fn usage_time_window_clause(from: Option<&str>, to: Option<&str>) -> (String, Vec<String>) {
    let mut clauses = Vec::new();
    let mut params = Vec::new();
    if let Some(from) = from {
        clauses.push("ts >= ?");
        params.push(from.to_string());
    }
    if let Some(to) = to {
        clauses.push("ts <= ?");
        params.push(to.to_string());
    }
    if clauses.is_empty() {
        (String::new(), params)
    } else {
        (format!(" WHERE {}", clauses.join(" AND ")), params)
    }
}

fn normalize_project_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "(unknown)".to_string();
    }
    let expanded = expand_home(trimmed);
    let without_trailing = trim_trailing_separators(&expanded);
    std::fs::canonicalize(&without_trailing)
        .unwrap_or_else(|_| PathBuf::from(without_trailing))
        .display()
        .to_string()
}

fn expand_home(path: &str) -> String {
    if path == "~" {
        return dirs_next::home_dir()
            .map(|home| home.display().to_string())
            .unwrap_or_else(|| path.to_string());
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return dirs_next::home_dir()
            .map(|home| home.join(rest).display().to_string())
            .unwrap_or_else(|| path.to_string());
    }
    path.to_string()
}

fn trim_trailing_separators(path: &str) -> String {
    let mut value = path.to_string();
    while value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    value
}

fn display_project_path(canonical_path: &str) -> String {
    if canonical_path == "(unknown)" {
        return "(unknown)".to_string();
    }
    Path::new(canonical_path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| canonical_path.to_string())
}

fn best_pricing_rule<'a>(model: &str, rules: &'a [PricingRule]) -> Option<&'a PricingRule> {
    rules
        .iter()
        .find(|rule| rule.match_kind == "exact" && rule.model_match == model)
        .or_else(|| {
            rules
                .iter()
                .filter(|rule| rule.match_kind == "prefix" && model.starts_with(&rule.model_match))
                .max_by_key(|rule| rule.model_match.len())
        })
}

fn calculate_cost_usd(usage: &TokenUsage, rule: &PricingRule) -> f64 {
    let per_million =
        |tokens: u64, price: Option<f64>| price.unwrap_or(0.0) * tokens as f64 / 1_000_000.0;
    per_million(usage.input, rule.price_input)
        + per_million(usage.output, rule.price_output)
        + per_million(usage.cache_read, rule.price_cache_read)
        + per_million(usage.cache_write, rule.price_cache_write)
        + per_million(usage.reasoning, rule.price_reasoning)
        + per_million(usage.tool, rule.price_tool)
}

fn row_to_pricing_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<PricingRule> {
    Ok(PricingRule {
        id: row.get(0)?,
        model_match: row.get(1)?,
        match_kind: row.get(2)?,
        provider: row.get(3)?,
        currency: row.get(4)?,
        price_input: row.get(5)?,
        price_output: row.get(6)?,
        price_cache_read: row.get(7)?,
        price_cache_write: row.get(8)?,
        price_reasoning: row.get(9)?,
        price_tool: row.get(10)?,
        source: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn row_to_project_alias(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectAlias> {
    Ok(ProjectAlias {
        canonical_path: row.get(0)?,
        alias: row.get(1)?,
        updated_at: row.get(2)?,
    })
}

fn row_to_detected_tool(row: &rusqlite::Row<'_>) -> rusqlite::Result<DetectedTool> {
    Ok(DetectedTool {
        id: row.get(0)?,
        display_name: row.get(1)?,
        installed: row.get::<_, i64>(2)? != 0,
        version: row.get(3)?,
        config_dir: row.get(4)?,
        data_dir: row.get(5)?,
        config_dir_exists: row.get::<_, i64>(6)? != 0,
        data_dir_exists: row.get::<_, i64>(7)? != 0,
        last_detected_at: row.get(8)?,
    })
}

fn row_to_source_health(row: &rusqlite::Row<'_>) -> rusqlite::Result<SourceHealth> {
    Ok(SourceHealth {
        tool: row.get(0)?,
        source_glob: row.get(1)?,
        exists: row.get::<_, i64>(2)? != 0,
        readable_files: row.get::<_, i64>(3)? as u64,
        parsed_records: row.get::<_, i64>(4)? as u64,
        failed_files: row.get::<_, i64>(5)? as u64,
        latest_event_ts: row.get(6)?,
        checked_at: row.get(7)?,
    })
}

fn row_to_resource(row: &rusqlite::Row<'_>) -> rusqlite::Result<CanonicalResource> {
    let enabled_in_json: String = row.get(7)?;
    let enabled_in = serde_json::from_str(&enabled_in_json).unwrap_or_default();
    Ok(CanonicalResource {
        id: row.get(0)?,
        kind: row.get(1)?,
        name: row.get(2)?,
        scope: row.get(3)?,
        origin_tool: row.get(4)?,
        origin_path: row.get(5)?,
        origin_locator: row.get(6)?,
        enabled_in,
        confidence: row.get(8)?,
        redacted: row.get::<_, i64>(9)? != 0,
        payload_json: row.get(10)?,
        provided_by_plugin: row.get(11)?,
        last_seen: row.get(12)?,
    })
}

fn row_to_resource_template(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceTemplate> {
    let required_env_keys_json: String = row.get(7)?;
    let required_env_keys = serde_json::from_str(&required_env_keys_json).unwrap_or_default();
    Ok(ResourceTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        scope: row.get(3)?,
        description: row.get(4)?,
        source: row.get(5)?,
        content_fingerprint: row.get(6)?,
        required_env_keys,
        payload_json: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn row_to_resource_parse_failure(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResourceParseFailure> {
    Ok(ResourceParseFailure {
        path: row.get(0)?,
        tool: row.get(1)?,
        kind: row.get(2)?,
        reason: row.get(3)?,
        seen_at: row.get(4)?,
    })
}

fn row_to_resource_change(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceChangeLog> {
    Ok(ResourceChangeLog {
        change_id: row.get(0)?,
        sync_id: row.get(1)?,
        tool: row.get(2)?,
        resource_id: row.get(3)?,
        kind: row.get(4)?,
        op: row.get(5)?,
        target_path: row.get(6)?,
        backup_path: row.get(7)?,
        status: row.get(8)?,
        reverts_change_id: row.get(9)?,
        created_at: row.get(10)?,
    })
}

fn row_to_sync_operation(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncOperation> {
    Ok(SyncOperation {
        sync_id: row.get(0)?,
        source_resource_id: row.get(1)?,
        targets_json: row.get(2)?,
        allow_cross_scope: row.get(3)?,
        env_strategy: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn row_to_sync_preset(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncPreset> {
    Ok(SyncPreset {
        id: row.get(0)?,
        name: row.get(1)?,
        resources_json: row.get(2)?,
        targets_json: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn row_to_resource_backup(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResourceBackup> {
    Ok(ResourceBackup {
        backup_path: row.get(0)?,
        tool: row.get(1)?,
        original_path: row.get(2)?,
        change_id: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn row_to_session_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionMeta> {
    let source_paths_csv: Option<String> = row.get(8)?;
    let mut source_paths = source_paths_csv
        .unwrap_or_default()
        .split(',')
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    source_paths.sort();
    source_paths.dedup();
    Ok(SessionMeta {
        tool: row.get(0)?,
        session_id: row.get(1)?,
        project_path: row.get(2)?,
        first_ts: row.get(3)?,
        last_ts: row.get(4)?,
        records: row.get::<_, i64>(5)? as u64,
        total_tokens: row.get::<_, Option<i64>>(6)?.unwrap_or(0) as u64,
        cost_usd: row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
        source_paths,
    })
}

fn ensure_resource_payload_is_safe(resource: &CanonicalResource) -> Result<()> {
    let value: Value = serde_json::from_str(&resource.payload_json)?;
    if json_value_contains_plain_secret(None, &value) {
        bail!(
            "refusing to persist resource payload with possible plain secret: {}",
            resource.id
        );
    }
    Ok(())
}

fn ensure_resource_template_is_safe(template: &ResourceTemplate) -> Result<()> {
    if template.id.trim().is_empty()
        || template.name.trim().is_empty()
        || template.kind.trim().is_empty()
        || template.scope.trim().is_empty()
        || template.source.trim().is_empty()
        || template.content_fingerprint.trim().is_empty()
    {
        bail!("resource template id, name, kind, scope, source, and fingerprint are required");
    }
    let value: Value = serde_json::from_str(&template.payload_json)?;
    if !value.is_object() {
        bail!(
            "resource template payload_json must be an object: {}",
            template.id
        );
    }
    if json_value_contains_plain_secret(None, &value) {
        bail!(
            "refusing to persist resource template with possible plain secret: {}",
            template.id
        );
    }
    Ok(())
}

fn ensure_sync_preset_is_safe(preset: &SyncPreset) -> Result<()> {
    if preset.id.trim().is_empty() || preset.name.trim().is_empty() {
        bail!("sync preset id and name are required");
    }
    let resources: Value = serde_json::from_str(&preset.resources_json)?;
    let targets: Value = serde_json::from_str(&preset.targets_json)?;
    if !resources.is_array() || !targets.is_array() {
        bail!("sync preset resources_json and targets_json must be arrays");
    }
    if json_value_contains_plain_secret(None, &resources)
        || json_value_contains_plain_secret(None, &targets)
        || json_value_contains_preset_secret_field(&resources)
        || json_value_contains_preset_secret_field(&targets)
    {
        bail!(
            "refusing to persist sync preset with possible secret metadata: {}",
            preset.id
        );
    }
    Ok(())
}

fn json_value_contains_plain_secret(key: Option<&str>, value: &Value) -> bool {
    match value {
        Value::String(text) => {
            if key.is_some_and(is_safe_fingerprint_field) {
                return false;
            }
            looks_plain_secret(text)
        }
        Value::Array(values) => values
            .iter()
            .any(|item| json_value_contains_plain_secret(key, item)),
        Value::Object(map) => map
            .iter()
            .any(|(key, value)| json_value_contains_plain_secret(Some(key), value)),
        _ => false,
    }
}

fn json_value_contains_preset_secret_field(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.iter().any(json_value_contains_preset_secret_field),
        Value::Object(map) => map.iter().any(|(key, value)| {
            is_sync_preset_secret_field(key) || json_value_contains_preset_secret_field(value)
        }),
        _ => false,
    }
}

fn is_sync_preset_secret_field(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower == "env"
        || lower == "env_values"
        || lower == "env_keys"
        || lower == "env_fingerprints"
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("authorization")
        || lower.contains("secret")
        || lower.contains("token")
}

fn is_safe_fingerprint_field(key: &str) -> bool {
    matches!(
        key,
        "sha256_8" | "content_hash" | "body_hashes" | "paragraph_hashes" | "prefix" | "env_keys"
    )
}

fn looks_plain_secret(value: &str) -> bool {
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
    let unique = compact.chars().collect::<BTreeSet<_>>().len();
    has_alpha && has_digit && unique >= 12
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<UsageSummary> {
    Ok(UsageSummary {
        name: row.get(0)?,
        records: row.get::<_, i64>(1)? as u64,
        usage: TokenUsage {
            input: row.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
            output: row.get::<_, Option<i64>>(3)?.unwrap_or(0) as u64,
            cache_read: row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
            cache_write: row.get::<_, Option<i64>>(5)?.unwrap_or(0) as u64,
            reasoning: row.get::<_, Option<i64>>(6)?.unwrap_or(0) as u64,
            tool: row.get::<_, Option<i64>>(7)?.unwrap_or(0) as u64,
        },
        cost_usd: row.get::<_, Option<f64>>(8)?.unwrap_or(0.0),
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::model::{
        CanonicalResource, PricingRule, ProjectAlias, ResourceParseFailure, ResourceTemplate,
        SourcePrecision, SyncPreset, TokenUsage, ToolKind, UsageRecord,
    };

    use super::*;

    #[test]
    fn upserts_records_and_summarizes_by_tool() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let record = UsageRecord {
            id: "r1".to_string(),
            tool: ToolKind::Claude,
            source_path: "/tmp/session.jsonl".to_string(),
            session_id: Some("s1".to_string()),
            timestamp: None,
            project_path: Some("/repo".to_string()),
            model: Some("claude-opus".to_string()),
            usage: TokenUsage {
                input: 1,
                output: 2,
                cache_read: 3,
                cache_write: 4,
                reasoning: 5,
                tool: 6,
            },
            cost_usd: Some(0.5),
            precision: SourcePrecision::Exact,
        };

        assert_eq!(
            store.upsert_records(std::slice::from_ref(&record)).unwrap(),
            1
        );
        assert_eq!(store.upsert_records(&[record]).unwrap(), 1);

        let summaries = store.summary_by_tool(None).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, "claude");
        assert_eq!(summaries[0].records, 1);
        assert_eq!(summaries[0].usage.total(), 21);
        assert_eq!(summaries[0].cost_usd, 0.5);
    }

    #[test]
    fn summarizes_by_project_path() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut left = sample_record("left", 10);
        left.project_path = Some("/repo-a".to_string());
        let mut right = sample_record("right", 20);
        right.project_path = Some("/repo-b".to_string());
        store.upsert_records(&[left, right]).unwrap();

        let summaries = store
            .summary_by_project_filtered(Some("/repo-b"), None)
            .unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, "/repo-b");
        assert_eq!(summaries[0].usage.input, 20);
    }

    #[test]
    fn filters_summary_by_utc_day_prefix() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut today = sample_record("today", 10);
        today.timestamp = Some("2026-05-25T01:00:00Z".parse().unwrap());
        let mut yesterday = sample_record("yesterday", 20);
        yesterday.timestamp = Some("2026-05-24T23:00:00Z".parse().unwrap());
        store.upsert_records(&[today, yesterday]).unwrap();

        let summaries = store
            .summary_by_tool_filtered(None, Some("2026-05-25"))
            .unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].usage.input, 10);
        assert_eq!(summaries[0].usage.total(), 10);
    }

    #[test]
    fn aggregates_daily_tool_totals_for_trend_chart() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut codex = sample_record("codex-1", 30);
        codex.tool = ToolKind::Codex;
        codex.timestamp = Some("2026-05-25T09:00:00Z".parse().unwrap());
        let mut claude = sample_record("claude-1", 20);
        claude.tool = ToolKind::Claude;
        claude.timestamp = Some("2026-05-25T10:00:00Z".parse().unwrap());
        let mut old = sample_record("old", 99);
        old.tool = ToolKind::Gemini;
        old.timestamp = Some("2026-05-23T10:00:00Z".parse().unwrap());
        store.upsert_records(&[codex, claude, old]).unwrap();

        let summaries = store
            .daily_tool_totals(&["2026-05-25".to_string()])
            .unwrap();

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].day, "2026-05-25");
        assert_eq!(summaries[0].tool, "codex");
        assert_eq!(summaries[0].total_tokens, 30);
        assert_eq!(summaries[1].tool, "claude");
        assert_eq!(summaries[1].total_tokens, 20);
    }

    #[test]
    fn persists_tool_registry_and_latest_source_health() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let tool = DetectedTool {
            id: "codex".to_string(),
            display_name: "Codex".to_string(),
            installed: true,
            version: Some("codex 1.0.0".to_string()),
            config_dir: Some("/tmp/.codex".to_string()),
            data_dir: Some("/tmp/.codex/sessions".to_string()),
            config_dir_exists: true,
            data_dir_exists: true,
            last_detected_at: "2026-06-05T00:00:00Z".to_string(),
        };
        let health = SourceHealth {
            tool: "codex".to_string(),
            source_glob: "~/.codex/sessions/**/*.jsonl".to_string(),
            exists: true,
            readable_files: 2,
            parsed_records: 4,
            failed_files: 1,
            latest_event_ts: Some("2026-06-05T01:00:00Z".to_string()),
            checked_at: "2026-06-05T02:00:00Z".to_string(),
        };

        store.upsert_tools(std::slice::from_ref(&tool)).unwrap();
        store
            .insert_source_health(std::slice::from_ref(&health))
            .unwrap();

        assert_eq!(store.list_tools().unwrap(), vec![tool]);
        assert_eq!(store.latest_source_health().unwrap(), vec![health]);
    }

    #[test]
    fn pricing_rules_can_be_upserted_listed_and_deleted() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut rule = sample_pricing_rule("claude-", "prefix", 3.0, 15.0);

        let saved = store.upsert_pricing_rule(&rule).unwrap();
        rule.id = saved.id;
        assert_eq!(store.list_pricing_rules().unwrap(), vec![rule.clone()]);

        let mut edited = rule.clone();
        edited.price_output = Some(20.0);
        let edited = store.upsert_pricing_rule(&edited).unwrap();
        assert_eq!(edited.price_output, Some(20.0));
        assert_eq!(store.list_pricing_rules().unwrap(), vec![edited.clone()]);

        store.delete_pricing_rule(edited.id.unwrap()).unwrap();
        assert!(store.list_pricing_rules().unwrap().is_empty());
    }

    #[test]
    fn sync_presets_can_be_saved_listed_deleted_without_secret_values() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let preset = SyncPreset {
            id: "preset:github-json-targets".to_string(),
            name: "GitHub MCP to JSON tools".to_string(),
            resources_json: r#"["mcp:user:codex:github"]"#.to_string(),
            targets_json: r#"[{"tool":"gemini","scope":"user","project_path":null,"target_path":"/Users/example/.gemini/settings.json","format":"json"}]"#.to_string(),
            updated_at: "2026-06-06T08:00:00Z".to_string(),
        };

        store.save_sync_preset(&preset).unwrap();
        assert_eq!(store.list_sync_presets().unwrap(), vec![preset.clone()]);

        let mut edited = preset.clone();
        edited.name = "GitHub MCP to Gemini".to_string();
        edited.updated_at = "2026-06-06T09:00:00Z".to_string();
        store.save_sync_preset(&edited).unwrap();
        assert_eq!(store.list_sync_presets().unwrap(), vec![edited.clone()]);

        let mut unsafe_preset = edited.clone();
        unsafe_preset.id = "preset:unsafe".to_string();
        unsafe_preset.targets_json = r#"[{"tool":"gemini","target_path":"/tmp/settings.json","env_values":{"GITHUB_TOKEN":"ghp_secret123"}}]"#.to_string();
        let error = store.save_sync_preset(&unsafe_preset).unwrap_err();
        assert!(error.to_string().contains("secret"));

        store.delete_sync_preset(&edited.id).unwrap();
        assert!(store.list_sync_presets().unwrap().is_empty());
    }

    #[test]
    fn recomputes_costs_with_exact_prefix_and_none_sources() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut exact = sample_record("exact", 1_000_000);
        exact.model = Some("claude-opus".to_string());
        exact.usage.output = 1_000_000;
        let mut prefix = sample_record("prefix", 2_000_000);
        prefix.model = Some("claude-haiku".to_string());
        let mut none = sample_record("none", 3_000_000);
        none.model = Some("unknown-model".to_string());
        store.upsert_records(&[exact, prefix, none]).unwrap();
        store
            .upsert_pricing_rule(&sample_pricing_rule("claude-", "prefix", 3.0, 15.0))
            .unwrap();
        store
            .upsert_pricing_rule(&sample_pricing_rule("claude-opus", "exact", 5.0, 25.0))
            .unwrap();

        let result = store.recompute_costs().unwrap();

        assert_eq!(result.updated, 3);
        assert_eq!(result.exact_matches, 1);
        assert_eq!(result.prefix_matches, 1);
        assert_eq!(result.no_matches, 1);
        assert_eq!(
            store.record_cost("exact").unwrap(),
            (Some(30.0), "exact".to_string())
        );
        assert_eq!(
            store.record_cost("prefix").unwrap(),
            (Some(6.0), "prefix".to_string())
        );
        assert_eq!(
            store.record_cost("none").unwrap(),
            (None, "none".to_string())
        );
    }

    #[test]
    fn project_summaries_normalize_alias_and_merge_tools_without_rewriting_usage_paths() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut left = sample_record("left", 10);
        left.tool = ToolKind::Claude;
        left.project_path = Some("/Users/test/work/repo".to_string());
        let mut right = sample_record("right", 20);
        right.tool = ToolKind::Codex;
        right.project_path = Some("/Users/test/work/repo/".to_string());
        let mut unknown = sample_record("unknown", 30);
        unknown.tool = ToolKind::Gemini;
        unknown.project_path = None;
        store.upsert_records(&[left, right, unknown]).unwrap();
        let alias = ProjectAlias {
            canonical_path: "/Users/test/work/repo".to_string(),
            alias: "Work Repo".to_string(),
            updated_at: "2026-06-05T00:00:00Z".to_string(),
        };
        store.set_project_alias(&alias).unwrap();

        let summaries = store.project_summaries().unwrap();
        let repo = summaries
            .iter()
            .find(|summary| summary.canonical_path == "/Users/test/work/repo")
            .unwrap();

        assert_eq!(repo.display_name, "Work Repo");
        assert_eq!(repo.alias.as_deref(), Some("Work Repo"));
        assert_eq!(repo.records, 2);
        assert_eq!(repo.usage.input, 30);
        assert_eq!(repo.tools, vec!["claude".to_string(), "codex".to_string()]);
        assert_eq!(repo.original_paths.len(), 2);
        assert_eq!(
            store.raw_project_paths().unwrap(),
            vec![
                None,
                Some("/Users/test/work/repo".to_string()),
                Some("/Users/test/work/repo/".to_string()),
            ]
        );
    }

    #[test]
    fn persists_resources_and_parse_failures_without_secret_payloads() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let resource = sample_resource("mcp:github");
        let failure = ResourceParseFailure {
            path: "/Users/test/.claude.json".to_string(),
            tool: "claude".to_string(),
            kind: Some("mcp".to_string()),
            reason: "expected value at line 1 column 1".to_string(),
            seen_at: "2026-06-05T00:00:00Z".to_string(),
        };

        assert_eq!(
            store
                .upsert_resources(std::slice::from_ref(&resource))
                .unwrap(),
            1
        );
        store
            .insert_resource_parse_failures(std::slice::from_ref(&failure))
            .unwrap();

        assert_eq!(
            store.list_resources(None, None, None, None).unwrap(),
            vec![resource.clone()]
        );
        assert_eq!(store.get_resource(&resource.id).unwrap(), Some(resource));
        assert_eq!(store.list_resource_parse_failures().unwrap(), vec![failure]);
    }

    #[test]
    fn rejects_resource_payloads_with_plain_secret_values() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut resource = sample_resource("mcp:leaky");
        resource.payload_json =
            r#"{"args":["--token","xoxb-1234567890-ABCDEFGHIJKLMN-opensesame"]}"#.to_string();

        let err = store.upsert_resources(&[resource]).unwrap_err();

        assert!(err.to_string().contains("refusing to persist"));
        assert!(
            store
                .list_resources(None, None, None, None)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn persists_resource_templates_without_secret_payloads() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let template = sample_template("builtin:docs-mcp");

        assert_eq!(
            store
                .upsert_resource_templates(std::slice::from_ref(&template))
                .unwrap(),
            1
        );

        let listed = store.list_resource_templates().unwrap();
        assert_eq!(listed, vec![template.clone()]);
        assert_eq!(
            store.get_resource_template(&template.id).unwrap(),
            Some(template)
        );
    }

    #[test]
    fn rejects_resource_templates_with_plain_secret_values() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut template = sample_template("builtin:leaky-mcp");
        template.payload_json =
            r#"{"command":"npx","args":["-y","server"],"env":{"TOKEN":"ghp_secret1234567890"}}"#
                .to_string();

        let err = store.upsert_resource_templates(&[template]).unwrap_err();

        assert!(err.to_string().contains("refusing to persist"));
        assert!(store.list_resource_templates().unwrap().is_empty());
    }

    #[test]
    fn session_browser_lists_metadata_without_content_fields() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut first = sample_record("s1-a", 10);
        first.tool = ToolKind::Codex;
        first.session_id = Some("session-1".to_string());
        first.project_path = Some("/repo".to_string());
        first.source_path = "/tmp/codex/session.jsonl".to_string();
        first.timestamp = Some("2026-06-06T01:00:00Z".parse().unwrap());
        first.cost_usd = Some(0.25);
        let mut second = sample_record("s1-b", 20);
        second.tool = ToolKind::Codex;
        second.session_id = Some("session-1".to_string());
        second.project_path = Some("/repo".to_string());
        second.source_path = "/tmp/codex/session.jsonl".to_string();
        second.timestamp = Some("2026-06-06T02:00:00Z".parse().unwrap());
        second.cost_usd = Some(0.75);
        let mut other = sample_record("other", 30);
        other.tool = ToolKind::Claude;
        other.session_id = Some("session-2".to_string());
        other.project_path = Some("/other".to_string());
        store.upsert_records(&[first, second, other]).unwrap();

        let sessions = store
            .list_sessions(Some("codex"), Some("/repo"), None, None, Some("session-1"))
            .unwrap();

        assert_eq!(
            sessions,
            vec![SessionMeta {
                session_id: "session-1".to_string(),
                tool: "codex".to_string(),
                project_path: Some("/repo".to_string()),
                first_ts: Some("2026-06-06T01:00:00+00:00".to_string()),
                last_ts: Some("2026-06-06T02:00:00+00:00".to_string()),
                records: 2,
                total_tokens: 30,
                cost_usd: 1.0,
                source_paths: vec!["/tmp/codex/session.jsonl".to_string()],
            }]
        );
        let serialized = serde_json::to_string(&sessions).unwrap();
        assert!(!serialized.contains("prompt"));
        assert!(!serialized.contains("response"));
    }

    #[test]
    fn session_browser_filters_by_time_window() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let mut early = sample_record("early", 10);
        early.tool = ToolKind::Codex;
        early.session_id = Some("early-session".to_string());
        early.timestamp = Some("2026-06-05T23:59:00Z".parse().unwrap());
        let mut inside = sample_record("inside", 20);
        inside.tool = ToolKind::Codex;
        inside.session_id = Some("inside-session".to_string());
        inside.timestamp = Some("2026-06-06T12:00:00Z".parse().unwrap());
        let mut late = sample_record("late", 30);
        late.tool = ToolKind::Codex;
        late.session_id = Some("late-session".to_string());
        late.timestamp = Some("2026-06-07T00:01:00Z".parse().unwrap());
        store.upsert_records(&[early, inside, late]).unwrap();

        let sessions = store
            .list_sessions(
                Some("codex"),
                None,
                Some("2026-06-06T00:00:00+00:00"),
                Some("2026-06-06T23:59:59+00:00"),
                None,
            )
            .unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "inside-session");
    }

    fn sample_record(id: &str, input: u64) -> UsageRecord {
        UsageRecord {
            id: id.to_string(),
            tool: ToolKind::Claude,
            source_path: "/tmp/session.jsonl".to_string(),
            session_id: Some("s1".to_string()),
            timestamp: None,
            project_path: Some("/repo".to_string()),
            model: Some("claude-opus".to_string()),
            usage: TokenUsage {
                input,
                ..TokenUsage::default()
            },
            cost_usd: None,
            precision: SourcePrecision::Exact,
        }
    }

    fn sample_pricing_rule(
        model_match: &str,
        match_kind: &str,
        price_input: f64,
        price_output: f64,
    ) -> PricingRule {
        PricingRule {
            id: None,
            model_match: model_match.to_string(),
            match_kind: match_kind.to_string(),
            provider: None,
            currency: "USD".to_string(),
            price_input: Some(price_input),
            price_output: Some(price_output),
            price_cache_read: None,
            price_cache_write: None,
            price_reasoning: None,
            price_tool: None,
            source: "user".to_string(),
            updated_at: "2026-06-05T00:00:00Z".to_string(),
        }
    }

    fn sample_resource(id: &str) -> CanonicalResource {
        CanonicalResource {
            id: id.to_string(),
            kind: "mcp".to_string(),
            name: "github".to_string(),
            scope: "user".to_string(),
            origin_tool: "claude,codex".to_string(),
            origin_path: "/Users/test/.claude.json".to_string(),
            origin_locator: Some("mcpServers.github".to_string()),
            enabled_in: vec!["claude".to_string(), "codex".to_string()],
            confidence: 1.0,
            redacted: true,
            payload_json: r#"{"env_keys":["GITHUB_TOKEN"],"env_fingerprints":{"GITHUB_TOKEN":{"sha256_8":"abc12345"}}}"#.to_string(),
            provided_by_plugin: None,
            last_seen: "2026-06-05T00:00:00Z".to_string(),
        }
    }

    fn sample_template(id: &str) -> ResourceTemplate {
        ResourceTemplate {
            id: id.to_string(),
            name: "Docs MCP".to_string(),
            kind: "mcp".to_string(),
            scope: "user".to_string(),
            description: "HTTP MCP template for documentation lookup.".to_string(),
            source: "https://example.test/templates/docs-mcp".to_string(),
            content_fingerprint: "abc12345def67890".to_string(),
            required_env_keys: vec!["DOCS_TOKEN".to_string()],
            payload_json: r#"{"transport":"http","command":null,"args":[],"url":"https://example.test/mcp","env_keys":["DOCS_TOKEN"],"env_fingerprints":{}}"#.to_string(),
            updated_at: "2026-06-06T00:00:00Z".to_string(),
        }
    }
}
