//! Local metadata report export.
//! @author codex

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use crate::{
    model::{ExportReportRequest, ExportReportResult, SyncPreset, TokenUsage},
    store::{ProjectModelSummary, UsageStore, UsageSummary},
};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn export_report(
    store: &UsageStore,
    request: &ExportReportRequest,
) -> Result<ExportReportResult> {
    let content = render_report(store, request)?;
    let path = Path::new(&request.path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content.as_bytes())?;
    Ok(ExportReportResult {
        path: path.display().to_string(),
        bytes_written: content.len() as u64,
    })
}

pub fn export_sync_presets(store: &UsageStore, path: &Path) -> Result<ExportReportResult> {
    let presets = store.list_sync_presets()?;
    let export = SyncPresetExport {
        schema: "wapc.sync_presets.v1",
        presets: presets
            .iter()
            .map(exported_sync_preset)
            .collect::<Result<Vec<_>>>()?,
    };
    let content = serde_json::to_string_pretty(&export)? + "\n";
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content.as_bytes())?;
    Ok(ExportReportResult {
        path: path.display().to_string(),
        bytes_written: content.len() as u64,
    })
}

fn render_report(store: &UsageStore, request: &ExportReportRequest) -> Result<String> {
    match request.view.as_str() {
        "tools" => render_usage_summaries(&store.summary_by_tool(None)?, &request.format, "tool"),
        "projects" => render_project_summaries(store, &request.format),
        "daily" => render_usage_summaries(&store.summary_by_day()?, &request.format, "day"),
        "redacted" => render_redacted_report(store, &request.format, request),
        other => bail!("unsupported export view: {other}"),
    }
}

fn render_redacted_report(
    store: &UsageStore,
    format: &str,
    request: &ExportReportRequest,
) -> Result<String> {
    let window = report_time_window(request)?;
    let from = window.from.as_ref().map(DateTime::<Utc>::to_rfc3339);
    let to = window.to.as_ref().map(DateTime::<Utc>::to_rfc3339);
    let projects = store.project_summaries_in_window(from.as_deref(), to.as_deref())?;
    let model_rows = store.project_model_summaries_in_window(from.as_deref(), to.as_deref())?;
    let tool_breakdown = redacted_tool_breakdown(&model_rows);
    let report_model_breakdown = redacted_report_model_breakdown(&model_rows);
    let models_by_project = model_rows.into_iter().fold(
        BTreeMap::<String, Vec<ProjectModelSummary>>::new(),
        |mut groups, row| {
            groups
                .entry(row.canonical_path.clone())
                .or_default()
                .push(row);
            groups
        },
    );
    let mut fixture_records = Vec::new();
    let redacted_projects = projects
        .into_iter()
        .enumerate()
        .map(|(project_index, project)| {
            let model_rows = models_by_project
                .get(&project.canonical_path)
                .cloned()
                .unwrap_or_default();
            if request.include_fixture {
                fixture_records.extend(model_rows.iter().enumerate().map(|(row_index, row)| {
                    synthetic_fixture_record(project_index, row_index, row)
                }));
            }
            let models = model_rows
                .iter()
                .map(|row| row.model.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            RedactedProject {
                project_hash: stable_hash_16(&project.canonical_path),
                project_alias: request
                    .include_project_aliases
                    .then(|| project.alias.clone())
                    .flatten(),
                records: project.records,
                total_tokens: project.usage.total(),
                usage: project.usage,
                cost_usd: project.cost_usd,
                tools: project.tools,
                models,
                model_breakdown: model_rows
                    .into_iter()
                    .map(redacted_model_breakdown)
                    .collect(),
            }
        })
        .collect();
    let report = RedactedReport {
        schema: "wapc.redacted_report.v1",
        generated_at: Utc::now().to_rfc3339(),
        window: RedactedReportWindow { from, to },
        tool_breakdown,
        model_breakdown: report_model_breakdown,
        projects: redacted_projects,
        fixture: request
            .include_fixture
            .then(|| redacted_report_fixture(fixture_records)),
    };
    match format {
        "json" => Ok(serde_json::to_string_pretty(&report)? + "\n"),
        "markdown" => render_redacted_markdown(&report),
        other => bail!("unsupported export format: {other}"),
    }
}

fn report_time_window(request: &ExportReportRequest) -> Result<ReportTimeWindow> {
    let from = parse_report_time_bound("from", request.from.as_deref())?;
    let to = parse_report_time_bound("to", request.to.as_deref())?;
    if let (Some(from), Some(to)) = (&from, &to)
        && to < from
    {
        bail!("report time window is invalid: to must be greater than or equal to from");
    }
    Ok(ReportTimeWindow { from, to })
}

fn parse_report_time_bound(label: &str, value: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .with_context(|| format!("report time window {label} must be RFC3339"))
                .map(|dt| dt.with_timezone(&Utc))
        })
        .transpose()
}

fn render_redacted_markdown(report: &RedactedReport) -> Result<String> {
    let include_alias_column = report
        .projects
        .iter()
        .any(|project| project.project_alias.is_some());
    let mut lines = vec![
        "# WAPC Redacted Team Report".to_string(),
        String::new(),
        format!("- Schema: `{}`", report.schema),
        format!("- Generated At: `{}`", report.generated_at),
        format!(
            "- Window: `{}` to `{}`",
            report.window.from.as_deref().unwrap_or("unbounded"),
            report.window.to.as_deref().unwrap_or("unbounded")
        ),
        String::new(),
    ];
    if !report.tool_breakdown.is_empty() {
        lines.extend([
            "## Tool Summary".to_string(),
            String::new(),
            "| Tool | Records | Tokens | Cost USD |".to_string(),
            "| --- | ---: | ---: | ---: |".to_string(),
        ]);
        for row in &report.tool_breakdown {
            lines.push(format!(
                "| {} | {} | {} | {:.6} |",
                md_cell(&row.tool),
                row.records,
                row.total_tokens,
                row.cost_usd
            ));
        }
        lines.push(String::new());
    }
    if !report.model_breakdown.is_empty() {
        lines.extend([
            "## Model Summary".to_string(),
            String::new(),
            "| Tool | Model | Records | Tokens | Cost USD |".to_string(),
            "| --- | --- | ---: | ---: | ---: |".to_string(),
        ]);
        for row in &report.model_breakdown {
            lines.push(format!(
                "| {} | {} | {} | {} | {:.6} |",
                md_cell(&row.tool),
                md_cell(&row.model),
                row.records,
                row.total_tokens,
                row.cost_usd
            ));
        }
        lines.push(String::new());
    }
    lines.extend(["## Project Summary".to_string(), String::new()]);
    if include_alias_column {
        lines.extend([
            "| Project Hash | Project Alias | Records | Tokens | Cost USD | Tools | Models |"
                .to_string(),
            "| --- | --- | ---: | ---: | ---: | --- | --- |".to_string(),
        ]);
    } else {
        lines.extend([
            "| Project Hash | Records | Tokens | Cost USD | Tools | Models |".to_string(),
            "| --- | ---: | ---: | ---: | --- | --- |".to_string(),
        ]);
    }
    for project in &report.projects {
        if include_alias_column {
            lines.push(format!(
                "| {} | {} | {} | {} | {:.6} | {} | {} |",
                project.project_hash,
                md_cell(project.project_alias.as_deref().unwrap_or("")),
                project.records,
                project.total_tokens,
                project.cost_usd,
                md_cell(&project.tools.join(", ")),
                md_cell(&project.models.join(", "))
            ));
        } else {
            lines.push(format!(
                "| {} | {} | {} | {:.6} | {} | {} |",
                project.project_hash,
                project.records,
                project.total_tokens,
                project.cost_usd,
                md_cell(&project.tools.join(", ")),
                md_cell(&project.models.join(", "))
            ));
        }
    }
    if let Some(fixture) = &report.fixture {
        lines.extend([
            String::new(),
            "## Synthetic Fixture".to_string(),
            String::new(),
            format!("- Schema: `{}`", fixture.schema),
            format!("- Seed: `{}`", fixture.seed),
            "- Synthetic: `true`".to_string(),
            String::new(),
            "| Fixture Project | Tool | Model | Records | Tokens | Cost USD |".to_string(),
            "| --- | --- | --- | ---: | ---: | ---: |".to_string(),
        ]);
        for record in &fixture.records {
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {:.6} |",
                record.project_key,
                md_cell(&record.tool),
                md_cell(&record.model),
                record.records,
                record.usage.total(),
                record.cost_usd
            ));
        }
    }
    Ok(lines.join("\n") + "\n")
}

fn redacted_tool_breakdown(rows: &[ProjectModelSummary]) -> Vec<RedactedToolBreakdown> {
    let mut aggregates = BTreeMap::<String, SummaryAccumulator>::new();
    for row in rows {
        aggregates
            .entry(row.tool.clone())
            .or_default()
            .add(row.records, &row.usage, row.cost_usd);
    }
    let mut output = aggregates
        .into_iter()
        .map(|(tool, summary)| RedactedToolBreakdown {
            tool,
            records: summary.records,
            total_tokens: summary.usage.total(),
            usage: summary.usage,
            cost_usd: summary.cost_usd,
        })
        .collect::<Vec<_>>();
    output.sort_by(|left, right| {
        right
            .total_tokens
            .cmp(&left.total_tokens)
            .then_with(|| left.tool.cmp(&right.tool))
    });
    output
}

fn redacted_report_model_breakdown(rows: &[ProjectModelSummary]) -> Vec<RedactedModelBreakdown> {
    let mut aggregates = BTreeMap::<(String, String), SummaryAccumulator>::new();
    for row in rows {
        aggregates
            .entry((row.tool.clone(), row.model.clone()))
            .or_default()
            .add(row.records, &row.usage, row.cost_usd);
    }
    let mut output = aggregates
        .into_iter()
        .map(|((tool, model), summary)| RedactedModelBreakdown {
            tool,
            model,
            records: summary.records,
            total_tokens: summary.usage.total(),
            cost_usd: summary.cost_usd,
        })
        .collect::<Vec<_>>();
    output.sort_by(|left, right| {
        right
            .total_tokens
            .cmp(&left.total_tokens)
            .then_with(|| left.tool.cmp(&right.tool))
            .then_with(|| left.model.cmp(&right.model))
    });
    output
}

fn redacted_model_breakdown(row: ProjectModelSummary) -> RedactedModelBreakdown {
    RedactedModelBreakdown {
        tool: row.tool,
        model: row.model,
        records: row.records,
        total_tokens: row.usage.total(),
        cost_usd: row.cost_usd,
    }
}

fn synthetic_fixture_record(
    project_index: usize,
    row_index: usize,
    row: &ProjectModelSummary,
) -> SyntheticFixtureRecord {
    SyntheticFixtureRecord {
        id: format!(
            "fixture-record-{:03}-{:03}",
            project_index + 1,
            row_index + 1
        ),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        project_key: format!("fixture-project-{:03}", project_index + 1),
        tool: row.tool.clone(),
        model: row.model.clone(),
        records: row.records,
        usage: row.usage.clone(),
        cost_usd: row.cost_usd,
    }
}

fn redacted_report_fixture(records: Vec<SyntheticFixtureRecord>) -> RedactedReportFixture {
    RedactedReportFixture {
        schema: "wapc.redacted_report_fixture.v1",
        synthetic: true,
        seed: "wapc-redacted-report-fixture-v1".to_string(),
        records,
    }
}

fn stable_hash_16(value: &str) -> String {
    let digest = Sha256::digest(format!("wapc-redacted-report-v1:{value}").as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn render_project_summaries(store: &UsageStore, format: &str) -> Result<String> {
    let summaries = store.project_summaries()?;
    match format {
        "json" => Ok(serde_json::to_string_pretty(&summaries)?),
        "csv" => {
            let mut lines = vec![
                "project,canonical_path,records,total_tokens,cost_usd,tools,original_paths"
                    .to_string(),
            ];
            for row in summaries {
                lines.push(csv_row(&[
                    row.display_name,
                    row.canonical_path,
                    row.records.to_string(),
                    row.usage.total().to_string(),
                    row.cost_usd.to_string(),
                    row.tools.join(";"),
                    row.original_paths.join(";"),
                ]));
            }
            Ok(lines.join("\n") + "\n")
        }
        "markdown" => {
            let mut lines = vec![
                "| Project | Canonical Path | Records | Tokens | Cost USD | Tools |".to_string(),
                "| --- | --- | ---: | ---: | ---: | --- |".to_string(),
            ];
            for row in summaries {
                lines.push(format!(
                    "| {} | {} | {} | {} | {:.6} | {} |",
                    md_cell(&row.display_name),
                    md_cell(&row.canonical_path),
                    row.records,
                    row.usage.total(),
                    row.cost_usd,
                    md_cell(&row.tools.join(", "))
                ));
            }
            Ok(lines.join("\n") + "\n")
        }
        other => bail!("unsupported export format: {other}"),
    }
}

fn render_usage_summaries(rows: &[UsageSummary], format: &str, name_label: &str) -> Result<String> {
    match format {
        "json" => Ok(serde_json::to_string_pretty(rows)?),
        "csv" => {
            let mut lines = vec![format!("{name_label},records,total_tokens,cost_usd")];
            for row in rows {
                lines.push(csv_row(&[
                    row.name.clone(),
                    row.records.to_string(),
                    row.usage.total().to_string(),
                    row.cost_usd.to_string(),
                ]));
            }
            Ok(lines.join("\n") + "\n")
        }
        "markdown" => {
            let mut lines = vec![
                format!(
                    "| {} | Records | Tokens | Cost USD |",
                    title_label(name_label)
                ),
                "| --- | ---: | ---: | ---: |".to_string(),
            ];
            for row in rows {
                lines.push(format!(
                    "| {} | {} | {} | {:.6} |",
                    md_cell(&row.name),
                    row.records,
                    row.usage.total(),
                    row.cost_usd
                ));
            }
            Ok(lines.join("\n") + "\n")
        }
        other => bail!("unsupported export format: {other}"),
    }
}

#[derive(Serialize)]
struct SyncPresetExport {
    schema: &'static str,
    presets: Vec<ExportedSyncPreset>,
}

#[derive(Serialize)]
struct RedactedReport {
    schema: &'static str,
    generated_at: String,
    window: RedactedReportWindow,
    tool_breakdown: Vec<RedactedToolBreakdown>,
    model_breakdown: Vec<RedactedModelBreakdown>,
    projects: Vec<RedactedProject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixture: Option<RedactedReportFixture>,
}

struct ReportTimeWindow {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct RedactedReportWindow {
    from: Option<String>,
    to: Option<String>,
}

#[derive(Serialize)]
struct RedactedToolBreakdown {
    tool: String,
    records: u64,
    total_tokens: u64,
    usage: TokenUsage,
    cost_usd: f64,
}

#[derive(Serialize)]
struct RedactedProject {
    project_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_alias: Option<String>,
    records: u64,
    total_tokens: u64,
    usage: TokenUsage,
    cost_usd: f64,
    tools: Vec<String>,
    models: Vec<String>,
    model_breakdown: Vec<RedactedModelBreakdown>,
}

#[derive(Serialize)]
struct RedactedModelBreakdown {
    tool: String,
    model: String,
    records: u64,
    total_tokens: u64,
    cost_usd: f64,
}

#[derive(Serialize)]
struct RedactedReportFixture {
    schema: &'static str,
    synthetic: bool,
    seed: String,
    records: Vec<SyntheticFixtureRecord>,
}

#[derive(Serialize)]
struct SyntheticFixtureRecord {
    id: String,
    timestamp: String,
    project_key: String,
    tool: String,
    model: String,
    records: u64,
    usage: TokenUsage,
    cost_usd: f64,
}

#[derive(Default)]
struct SummaryAccumulator {
    records: u64,
    usage: TokenUsage,
    cost_usd: f64,
}

impl SummaryAccumulator {
    fn add(&mut self, records: u64, usage: &TokenUsage, cost_usd: f64) {
        self.records += records;
        self.usage = self.usage.clone() + usage.clone();
        self.cost_usd += cost_usd;
    }
}

#[derive(Serialize)]
struct ExportedSyncPreset {
    id: String,
    name: String,
    resources: Vec<String>,
    targets: Vec<Value>,
    updated_at: String,
}

fn exported_sync_preset(preset: &SyncPreset) -> Result<ExportedSyncPreset> {
    let resources = serde_json::from_str::<Vec<String>>(&preset.resources_json)?;
    let targets = serde_json::from_str::<Vec<Value>>(&preset.targets_json)?;
    Ok(ExportedSyncPreset {
        id: preset.id.clone(),
        name: preset.name.clone(),
        resources,
        targets,
        updated_at: preset.updated_at.clone(),
    })
}

fn csv_row(cells: &[String]) -> String {
    cells
        .iter()
        .map(|cell| csv_cell(cell))
        .collect::<Vec<_>>()
        .join(",")
}

fn csv_cell(value: &str) -> String {
    let cell = if spreadsheet_formula_risk(value) {
        format!("'{value}")
    } else {
        value.to_string()
    };
    if cell.contains(',')
        || cell.contains('"')
        || cell.contains('\n')
        || cell.contains('\r')
        || cell.contains('\t')
    {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell
    }
}

fn md_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn spreadsheet_formula_risk(value: &str) -> bool {
    let trimmed = value.trim_start_matches([' ', '\t']);
    matches!(
        trimmed.chars().next(),
        Some('=') | Some('+') | Some('-') | Some('@')
    )
}

fn title_label(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::{
        model::{
            ExportReportRequest, ProjectAlias, SourcePrecision, SyncPreset, TokenUsage, ToolKind,
            UsageRecord,
        },
        store::UsageStore,
    };

    use super::*;

    #[test]
    fn exports_project_report_as_markdown_without_source_text() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let out = dir.path().join("reports/projects.md");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_records(&[sample_record("r1", "/repo", "secret prompt")])
            .unwrap();

        let result = export_report(
            &store,
            &ExportReportRequest {
                view: "projects".to_string(),
                format: "markdown".to_string(),
                path: out.display().to_string(),
                from: None,
                to: None,
                include_fixture: false,
                include_project_aliases: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(&out).unwrap();
        assert_eq!(result.path, out.display().to_string());
        assert!(result.bytes_written > 0);
        assert!(
            content.contains("| Project | Canonical Path | Records | Tokens | Cost USD | Tools |")
        );
        assert!(content.contains("/repo"));
        assert!(!content.contains("secret prompt"));
    }

    #[test]
    fn exports_tools_report_as_csv_with_escaped_cells() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let out = dir.path().join("tools.csv");
        let store = UsageStore::open(&db).unwrap();
        let mut record = sample_record("r1", "/repo", "none");
        record.tool = ToolKind::Claude;
        store.upsert_records(&[record]).unwrap();

        export_report(
            &store,
            &ExportReportRequest {
                view: "tools".to_string(),
                format: "csv".to_string(),
                path: out.display().to_string(),
                from: None,
                to: None,
                include_fixture: false,
                include_project_aliases: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(&out).unwrap();
        assert!(content.starts_with("tool,records,total_tokens,cost_usd"));
        assert!(content.contains("claude,1,10,0"));
    }

    #[test]
    fn csv_cells_neutralize_spreadsheet_formula_prefixes() {
        assert_eq!(csv_cell("=cmd|' /C calc'!A0"), "'=cmd|' /C calc'!A0");
        assert_eq!(csv_cell("+SUM(1,2)"), "\"'+SUM(1,2)\"");
        assert_eq!(csv_cell("-10"), "'-10");
        assert_eq!(csv_cell("@danger"), "'@danger");
        assert_eq!(csv_cell(" =hidden"), "' =hidden");
        assert_eq!(csv_cell("\t=hidden"), "\"'\t=hidden\"");
        assert_eq!(csv_cell("plain text"), "plain text");
    }

    #[test]
    fn exports_daily_report_as_json() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let out = dir.path().join("daily.json");
        let store = UsageStore::open(&db).unwrap();
        let mut record = sample_record("r1", "/repo", "none");
        record.timestamp = Some("2026-06-05T01:00:00Z".parse().unwrap());
        store.upsert_records(&[record]).unwrap();

        export_report(
            &store,
            &ExportReportRequest {
                view: "daily".to_string(),
                format: "json".to_string(),
                path: out.display().to_string(),
                from: None,
                to: None,
                include_fixture: false,
                include_project_aliases: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(&out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json[0]["name"], "2026-06-05");
        assert_eq!(json[0]["records"], 1);
    }

    #[test]
    fn exports_sync_presets_as_json_without_secret_values() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let out = dir.path().join("sync-presets.json");
        let store = UsageStore::open(&db).unwrap();
        store
            .save_sync_preset(&SyncPreset {
                id: "preset:github".to_string(),
                name: "GitHub MCP targets".to_string(),
                resources_json: r#"["mcp:user:codex:github"]"#.to_string(),
                targets_json: r#"[{"tool":"gemini","scope":"user","project_path":null,"target_path":"/Users/example/.gemini/settings.json","format":"json"}]"#.to_string(),
                updated_at: "2026-06-06T08:00:00Z".to_string(),
            })
            .unwrap();

        let result = export_sync_presets(&store, &out).unwrap();

        let content = fs::read_to_string(&out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(result.path, out.display().to_string());
        assert!(result.bytes_written > 0);
        assert_eq!(json["schema"], "wapc.sync_presets.v1");
        assert_eq!(json["presets"][0]["name"], "GitHub MCP targets");
        assert_eq!(json["presets"][0]["targets"][0]["tool"], "gemini");
        assert!(!content.contains("qa-secret"));
        assert!(!content.contains("env_values"));
    }

    #[test]
    fn exports_redacted_team_report_without_paths_names_or_bodies() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let out = dir.path().join("team-redacted.json");
        let store = UsageStore::open(&db).unwrap();
        let mut record = sample_record(
            "r1",
            "/Users/alice/work/secret-client",
            "prompt with sk-test-secret",
        );
        record.source_path =
            "/Users/alice/.claude/projects/secret-client/session.jsonl".to_string();
        record.session_id = Some("secret-client-session".to_string());
        record.model = Some("claude-opus".to_string());
        store.upsert_records(&[record]).unwrap();

        let result = export_report(
            &store,
            &ExportReportRequest {
                view: "redacted".to_string(),
                format: "json".to_string(),
                path: out.display().to_string(),
                from: None,
                to: None,
                include_fixture: false,
                include_project_aliases: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(&out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(result.path, out.display().to_string());
        assert_eq!(json["schema"], "wapc.redacted_report.v1");
        assert_eq!(
            json["projects"][0]["project_hash"].as_str().unwrap().len(),
            16
        );
        assert_eq!(json["projects"][0]["records"], 1);
        assert_eq!(json["projects"][0]["models"][0], "claude-opus");
        assert!(!content.contains("/Users/alice"));
        assert!(!content.contains("secret-client"));
        assert!(!content.contains("secret-client-session"));
        assert!(!content.contains("sk-test-secret"));
        assert!(!content.contains("session.jsonl"));
    }

    #[test]
    fn redacted_team_report_only_includes_project_alias_when_explicitly_requested() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let strict_out = dir.path().join("team-redacted-strict.json");
        let alias_out = dir.path().join("team-redacted-alias.json");
        let store = UsageStore::open(&db).unwrap();
        let project_path = "/Users/alice/work/secret-client";
        let mut record = sample_record("r1", project_path, "prompt with sk-test-secret");
        record.source_path =
            "/Users/alice/.claude/projects/secret-client/session.jsonl".to_string();
        record.session_id = Some("secret-client-session".to_string());
        record.model = Some("claude-opus".to_string());
        store.upsert_records(&[record]).unwrap();
        store
            .set_project_alias(&ProjectAlias {
                canonical_path: project_path.to_string(),
                alias: "Team Alpha".to_string(),
                updated_at: "2026-06-07T00:00:00Z".to_string(),
            })
            .unwrap();

        export_report(
            &store,
            &ExportReportRequest {
                view: "redacted".to_string(),
                format: "json".to_string(),
                path: strict_out.display().to_string(),
                from: None,
                to: None,
                include_fixture: false,
                include_project_aliases: false,
            },
        )
        .unwrap();
        export_report(
            &store,
            &ExportReportRequest {
                view: "redacted".to_string(),
                format: "json".to_string(),
                path: alias_out.display().to_string(),
                from: None,
                to: None,
                include_fixture: false,
                include_project_aliases: true,
            },
        )
        .unwrap();

        let strict_content = fs::read_to_string(&strict_out).unwrap();
        let alias_content = fs::read_to_string(&alias_out).unwrap();
        let strict_json: serde_json::Value = serde_json::from_str(&strict_content).unwrap();
        let alias_json: serde_json::Value = serde_json::from_str(&alias_content).unwrap();
        assert!(strict_json["projects"][0].get("project_alias").is_none());
        assert_eq!(alias_json["projects"][0]["project_alias"], "Team Alpha");
        for content in [&strict_content, &alias_content] {
            assert!(!content.contains("/Users/alice"));
            assert!(!content.contains("secret-client"));
            assert!(!content.contains("secret-client-session"));
            assert!(!content.contains("sk-test-secret"));
            assert!(!content.contains("session.jsonl"));
        }
    }

    #[test]
    fn redacted_team_report_includes_top_level_tool_and_model_summaries() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let out = dir.path().join("team-redacted-summary.json");
        let store = UsageStore::open(&db).unwrap();
        let mut claude = sample_record(
            "claude-r1",
            "/Users/alice/work/secret-client",
            "prompt with sk-test-secret",
        );
        claude.source_path =
            "/Users/alice/.claude/projects/secret-client/session.jsonl".to_string();
        claude.session_id = Some("secret-client-session".to_string());
        claude.model = Some("claude-opus".to_string());
        claude.timestamp = Some("2026-06-05T12:00:00Z".parse().unwrap());
        claude.usage.input = 10;
        claude.usage.output = 20;
        claude.cost_usd = Some(0.30);
        let mut codex = sample_record(
            "codex-r1",
            "/Users/alice/work/quiet-client",
            "response body with ghp_secret",
        );
        codex.tool = ToolKind::Codex;
        codex.source_path = "/Users/alice/.codex/sessions/quiet-client/session.jsonl".to_string();
        codex.session_id = Some("quiet-client-session".to_string());
        codex.model = Some("gpt-5-codex".to_string());
        codex.timestamp = Some("2026-06-05T13:00:00Z".parse().unwrap());
        codex.usage.input = 7;
        codex.usage.output = 11;
        codex.cost_usd = Some(0.12);
        store.upsert_records(&[claude, codex]).unwrap();

        export_report(
            &store,
            &ExportReportRequest {
                view: "redacted".to_string(),
                format: "json".to_string(),
                path: out.display().to_string(),
                from: Some("2026-06-05T00:00:00Z".to_string()),
                to: Some("2026-06-06T00:00:00Z".to_string()),
                include_fixture: false,
                include_project_aliases: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(&out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        let tools = json["tool_breakdown"].as_array().unwrap();
        let models = json["model_breakdown"].as_array().unwrap();
        let claude_tool = tools
            .iter()
            .find(|row| row["tool"] == "claude")
            .expect("claude tool summary");
        let codex_model = models
            .iter()
            .find(|row| row["tool"] == "codex" && row["model"] == "gpt-5-codex")
            .expect("codex model summary");
        assert_eq!(claude_tool["records"], 1);
        assert_eq!(claude_tool["total_tokens"], 30);
        assert_eq!(codex_model["records"], 1);
        assert_eq!(codex_model["total_tokens"], 18);
        assert_eq!(json["projects"].as_array().unwrap().len(), 2);
        for forbidden in [
            "/Users/alice",
            "secret-client",
            "quiet-client",
            "secret-client-session",
            "quiet-client-session",
            "sk-test-secret",
            "ghp_secret",
            "session.jsonl",
        ] {
            assert!(!content.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[test]
    fn exports_redacted_team_report_with_time_window_only_includes_matching_records() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let out = dir.path().join("team-redacted-window.json");
        let store = UsageStore::open(&db).unwrap();
        let mut inside =
            sample_record("inside", "/Users/alice/work/inside-client", "inside prompt");
        inside.timestamp = Some("2026-06-05T12:00:00Z".parse().unwrap());
        inside.model = Some("claude-opus".to_string());
        let mut outside = sample_record(
            "outside",
            "/Users/alice/work/outside-client",
            "outside prompt",
        );
        outside.timestamp = Some("2026-06-01T12:00:00Z".parse().unwrap());
        outside.model = Some("claude-haiku".to_string());
        store.upsert_records(&[inside, outside]).unwrap();

        export_report(
            &store,
            &ExportReportRequest {
                view: "redacted".to_string(),
                format: "json".to_string(),
                path: out.display().to_string(),
                from: Some("2026-06-05T00:00:00Z".to_string()),
                to: Some("2026-06-06T00:00:00Z".to_string()),
                include_fixture: false,
                include_project_aliases: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(&out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["window"]["from"], "2026-06-05T00:00:00+00:00");
        assert_eq!(json["window"]["to"], "2026-06-06T00:00:00+00:00");
        assert_eq!(json["projects"].as_array().unwrap().len(), 1);
        assert_eq!(json["projects"][0]["records"], 1);
        assert_eq!(json["projects"][0]["models"][0], "claude-opus");
        assert!(!content.contains("claude-haiku"));
        assert!(!content.contains("inside-client"));
        assert!(!content.contains("outside-client"));
    }

    #[test]
    fn exports_redacted_team_report_with_synthetic_fixture_when_requested() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let out = dir.path().join("team-redacted-fixture.json");
        let store = UsageStore::open(&db).unwrap();
        let mut record = sample_record(
            "r1",
            "/Users/alice/work/secret-client",
            "prompt with sk-test-secret",
        );
        record.source_path =
            "/Users/alice/.claude/projects/secret-client/session.jsonl".to_string();
        record.session_id = Some("secret-client-session".to_string());
        record.model = Some("claude-opus".to_string());
        record.timestamp = Some("2026-06-05T12:00:00Z".parse().unwrap());
        store.upsert_records(&[record]).unwrap();

        export_report(
            &store,
            &ExportReportRequest {
                view: "redacted".to_string(),
                format: "json".to_string(),
                path: out.display().to_string(),
                from: None,
                to: None,
                include_fixture: true,
                include_project_aliases: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(&out).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["fixture"]["schema"], "wapc.redacted_report_fixture.v1");
        assert_eq!(json["fixture"]["synthetic"], true);
        assert_eq!(
            json["fixture"]["records"][0]["project_key"],
            "fixture-project-001"
        );
        assert_eq!(json["fixture"]["records"][0]["tool"], "claude");
        assert_eq!(json["fixture"]["records"][0]["model"], "claude-opus");
        assert_eq!(json["fixture"]["records"][0]["usage"]["input"], 10);
        assert!(!content.contains("/Users/alice"));
        assert!(!content.contains("secret-client"));
        assert!(!content.contains("secret-client-session"));
        assert!(!content.contains("sk-test-secret"));
        assert!(!content.contains("session.jsonl"));
    }

    #[test]
    fn rejects_redacted_team_report_time_window_when_to_is_before_from() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();

        let error = export_report(
            &store,
            &ExportReportRequest {
                view: "redacted".to_string(),
                format: "json".to_string(),
                path: dir
                    .path()
                    .join("team-redacted-window.json")
                    .display()
                    .to_string(),
                from: Some("2026-06-06T00:00:00Z".to_string()),
                to: Some("2026-06-05T00:00:00Z".to_string()),
                include_fixture: false,
                include_project_aliases: false,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("report time window"));
    }

    #[test]
    fn rejects_unknown_export_view() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        let error = export_report(
            &store,
            &ExportReportRequest {
                view: "messages".to_string(),
                format: "json".to_string(),
                path: dir.path().join("bad.json").display().to_string(),
                from: None,
                to: None,
                include_fixture: false,
                include_project_aliases: false,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("unsupported export view"));
    }

    fn sample_record(id: &str, project_path: &str, secret: &str) -> UsageRecord {
        UsageRecord {
            id: id.to_string(),
            tool: ToolKind::Claude,
            source_path: format!("/tmp/{secret}.jsonl"),
            session_id: Some("s1".to_string()),
            timestamp: None,
            project_path: Some(project_path.to_string()),
            model: Some("claude-opus".to_string()),
            usage: TokenUsage {
                input: 10,
                ..TokenUsage::default()
            },
            cost_usd: None,
            precision: SourcePrecision::Exact,
        }
    }
}
