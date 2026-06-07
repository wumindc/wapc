//! Tauri command definitions — bridge between frontend and Rust core.
//! @author Claude Sonnet 4.6 (Thinking)

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use chrono::{Duration, Local};
use serde::Serialize;
use tauri::Emitter;
use wapc::{
    adapters, cross_sync, deep_link, export, guide_center,
    headless::{
        HeadlessDashboardConfig, HeadlessDashboardServer,
        start_headless_dashboard as start_headless_dashboard_core,
    },
    model::{
        AdapterCapability, ApplyChangeRequest, ApplyChangeResult, ApplySyncRequest,
        ApplySyncResult, AutoScanConfig, BackupRequest, BackupResult, CanonicalResource,
        CostRecomputeResult, DeepLinkImportPreview, DetectedTool, ExportReportRequest,
        ExportReportResult, InventoryScanResult, PlanDeepLinkImportRequest, PlanSyncRequest,
        PlanSyncResult, PlanTemplateSyncRequest, PricingRule, PrivacyAuditReport, ProjectAlias,
        ProjectSummary, ResourceBackup, ResourceChangeLog, ResourceChangeRequest, ResourceGuide,
        ResourceParseFailure, ResourceTemplate, SessionMeta, SourceHealth, SyncOperation,
        SyncPreset, WritePlan,
    },
    platform_paths::{
        PlatformPathContext, ToolPathVerificationRecord, WapcPaths, verify_tool_path_candidates,
    },
    privacy, resources, scanner,
    store::{DailyToolSummary, UsageStore, UsageSummary},
    sync_engine, template_library, tool_registry,
};

// ── Snapshot returned to the UI ──────────────────────────────────────────────

#[derive(Clone, Debug, Default, Serialize)]
pub struct DesktopSnapshot {
    pub today: Vec<UsageSummary>,
    pub yesterday: Vec<UsageSummary>,
    pub tools: Vec<UsageSummary>,
    pub projects: Vec<UsageSummary>,
    pub daily: Vec<DailyToolSummary>,
    pub trend_days: Vec<String>,
    pub daily_summaries: Vec<UsageSummary>,
    pub scan_records: usize,
    pub db_path: String,
    pub db_exists: bool,
    pub home_path: String,
    pub version: String,
    pub detected_tools: Vec<DetectedTool>,
    pub source_health: Vec<SourceHealth>,
    pub project_summaries: Vec<ProjectSummary>,
    pub privacy_audit: PrivacyAuditReport,
    pub resources: Vec<CanonicalResource>,
    pub resource_parse_failures: Vec<ResourceParseFailure>,
    pub adapter_capabilities: Vec<AdapterCapability>,
    pub tool_path_verifications: Vec<ToolPathVerificationRecord>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct HeadlessDashboardStatus {
    pub running: bool,
    pub bind_host: Option<String>,
    pub port: Option<u16>,
    pub url: Option<String>,
    pub read_only: bool,
}

static HEADLESS_DASHBOARD: OnceLock<Mutex<Option<HeadlessDashboardServer>>> = OnceLock::new();

// ── Helper ───────────────────────────────────────────────────────────────────

fn resolve_paths() -> (PathBuf, PathBuf) {
    let paths = WapcPaths::from_platform_home().expect("cannot resolve WAPC application paths");
    (paths.home_dir, paths.db_path)
}

fn db_path() -> PathBuf {
    resolve_paths().1
}

fn headless_dashboard_slot() -> &'static Mutex<Option<HeadlessDashboardServer>> {
    HEADLESS_DASHBOARD.get_or_init(|| Mutex::new(None))
}

fn headless_status_from_server(
    server: Option<&HeadlessDashboardServer>,
) -> HeadlessDashboardStatus {
    match server {
        Some(server) => HeadlessDashboardStatus {
            running: true,
            bind_host: Some("127.0.0.1".to_string()),
            port: Some(server.port()),
            url: Some(server.url()),
            read_only: true,
        },
        None => HeadlessDashboardStatus {
            running: false,
            bind_host: None,
            port: None,
            url: None,
            read_only: true,
        },
    }
}

fn recent_days(n: i64) -> Vec<String> {
    let today = Local::now().date_naive();
    (0..n)
        .rev()
        .map(|offset| {
            (today - Duration::days(offset))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect()
}

fn project_roots_from_summaries(summaries: &[ProjectSummary]) -> Vec<PathBuf> {
    project_roots_from_paths(
        summaries
            .iter()
            .map(|summary| summary.canonical_path.clone())
            .collect(),
    )
}

fn project_roots_from_paths(paths: Vec<String>) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut roots = Vec::new();
    for value in paths {
        if value == "(unknown)" || value.trim().is_empty() {
            continue;
        }
        let path = PathBuf::from(value);
        if !path.is_dir() {
            continue;
        }
        let key = path.display().to_string();
        if seen.insert(key) {
            roots.push(path);
        }
    }
    roots.sort();
    roots
}

fn tool_path_verifications_for_paths(
    home: &Path,
    project_roots: &[PathBuf],
) -> Vec<ToolPathVerificationRecord> {
    let mut records =
        verify_tool_path_candidates(&PlatformPathContext::current_home_compatible(home));
    for project_root in project_roots {
        let context = PlatformPathContext::current_home_compatible_with_project(
            home,
            Some(project_root.clone()),
        );
        records.extend(
            verify_tool_path_candidates(&context)
                .into_iter()
                .filter(|record| record.scope == "project"),
        );
    }
    records
}

fn plan_resource_change_for_paths(
    home: &Path,
    db: &Path,
    request: ResourceChangeRequest,
) -> Result<WritePlan, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    sync_engine::plan_resource_change(home, &store, request).map_err(|e| e.to_string())
}

fn plan_sync_for_paths(
    home: &Path,
    db: &Path,
    request: PlanSyncRequest,
) -> Result<PlanSyncResult, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    cross_sync::plan_sync(home, &store, request).map_err(|e| e.to_string())
}

fn list_resource_templates_for_path(db: &Path) -> Result<Vec<ResourceTemplate>, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    template_library::seed_builtin_resource_templates(&store).map_err(|e| e.to_string())?;
    store.list_resource_templates().map_err(|e| e.to_string())
}

fn plan_template_sync_for_paths(
    home: &Path,
    db: &Path,
    request: PlanTemplateSyncRequest,
) -> Result<PlanSyncResult, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    template_library::seed_builtin_resource_templates(&store).map_err(|e| e.to_string())?;
    template_library::plan_template_sync(home, &store, request).map_err(|e| e.to_string())
}

fn plan_deep_link_import_for_paths(
    home: &Path,
    db: &Path,
    request: PlanDeepLinkImportRequest,
) -> Result<PlanSyncResult, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    deep_link::plan_deep_link_import(home, &store, request).map_err(|e| e.to_string())
}

fn apply_sync_for_paths(
    home: &Path,
    db: &Path,
    request: ApplySyncRequest,
) -> Result<ApplySyncResult, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    cross_sync::apply_sync(home, &store, request).map_err(|e| e.to_string())
}

fn list_sync_operations_for_path(db: &Path) -> Result<Vec<SyncOperation>, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    store.list_sync_operations().map_err(|e| e.to_string())
}

fn save_sync_preset_for_path(db: &Path, preset: SyncPreset) -> Result<SyncPreset, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    store.save_sync_preset(&preset).map_err(|e| e.to_string())
}

fn list_sync_presets_for_path(db: &Path) -> Result<Vec<SyncPreset>, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    store.list_sync_presets().map_err(|e| e.to_string())
}

fn delete_sync_preset_for_path(db: &Path, id: String) -> Result<(), String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    store.delete_sync_preset(&id).map_err(|e| e.to_string())
}

fn export_sync_presets_for_path(db: &Path, path: &Path) -> Result<ExportReportResult, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    export::export_sync_presets(&store, path).map_err(|e| e.to_string())
}

fn apply_resource_change_for_paths(
    home: &Path,
    db: &Path,
    request: ApplyChangeRequest,
) -> Result<ApplyChangeResult, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    sync_engine::apply_resource_change(home, &store, request).map_err(|e| e.to_string())
}

fn list_changes_for_path(
    db: &Path,
    tool: Option<String>,
) -> Result<Vec<ResourceChangeLog>, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    store
        .list_resource_changes(tool.as_deref())
        .map_err(|e| e.to_string())
}

fn list_backups_for_path(db: &Path, tool: Option<String>) -> Result<Vec<ResourceBackup>, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    store
        .list_resource_backups(tool.as_deref())
        .map_err(|e| e.to_string())
}

fn rollback_change_for_paths(
    home: &Path,
    db: &Path,
    change_id: String,
) -> Result<ApplyChangeResult, String> {
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;
    sync_engine::rollback_resource_change(home, &store, &change_id).map_err(|e| e.to_string())
}

fn get_guide_for_input(
    tool: Option<String>,
    kind: Option<String>,
    resource_id: Option<String>,
) -> ResourceGuide {
    guide_center::get_resource_guide(tool.as_deref(), kind.as_deref(), resource_id.as_deref())
}

pub(crate) fn get_snapshot_for_paths(home: &Path, db: &Path) -> Result<DesktopSnapshot, String> {
    // Performance: get_snapshot only reads from DB; full file scanning is done by scan_now.
    // @author Claude Sonnet 4.6 (Thinking)
    let store = UsageStore::open(db).map_err(|e| e.to_string())?;

    // Read tool registry from DB cache; detect_tools is fast (fs::metadata checks only)
    let detected_tools = tool_registry::detect_tools(home);
    store
        .upsert_tools(&detected_tools)
        .map_err(|e| e.to_string())?;

    // Read source health from DB cache instead of re-scanning all source files
    let source_health = store.latest_source_health().unwrap_or_default();

    let project_summaries = store.project_summaries().map_err(|e| e.to_string())?;
    let project_roots = project_roots_from_summaries(&project_summaries);
    let tool_path_verifications = tool_path_verifications_for_paths(home, &project_roots);

    // Read resources from DB cache instead of re-scanning all project files
    let resources = store
        .list_resources(None, None, None, None)
        .map_err(|e| e.to_string())?;
    let resource_parse_failures = store
        .list_resource_parse_failures()
        .map_err(|e| e.to_string())?;

    // Get record count from DB (O(1)) instead of scanning all .jsonl files
    let scan_records = store.count_records().unwrap_or(0);

    let today = Local::now().format("%Y-%m-%d").to_string();
    let yesterday = (Local::now() - Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let trend_days = recent_days(7);

    Ok(DesktopSnapshot {
        today: store
            .summary_by_tool_filtered(None, Some(&today))
            .map_err(|e| e.to_string())?,
        yesterday: store
            .summary_by_tool_filtered(None, Some(&yesterday))
            .map_err(|e| e.to_string())?,
        tools: store.summary_by_tool(None).map_err(|e| e.to_string())?,
        projects: store
            .summary_by_project_filtered(None, None)
            .map_err(|e| e.to_string())?,
        project_summaries,
        daily: store
            .daily_tool_totals(&trend_days)
            .map_err(|e| e.to_string())?,
        trend_days,
        daily_summaries: store.summary_by_day().map_err(|e| e.to_string())?,
        scan_records,
        db_path: db.display().to_string(),
        db_exists: db.exists(),
        home_path: home.display().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        detected_tools,
        source_health,
        privacy_audit: privacy::privacy_audit(home, db),
        resources,
        resource_parse_failures,
        adapter_capabilities: adapters::adapter_capabilities(),
        tool_path_verifications,
    })
}

// ── Commands ─────────────────────────────────────────────────────────────────

/// Load the full dashboard snapshot from the local SQLite database.
#[tauri::command]
pub async fn get_snapshot() -> Result<DesktopSnapshot, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (home, db) = resolve_paths();
        get_snapshot_for_paths(&home, &db)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Scan and index usage files into the local database.
#[tauri::command]
pub async fn scan_now(app: tauri::AppHandle) -> Result<usize, String> {
    let _ = app.emit("scan-started", ());
    let res = tauri::async_runtime::spawn_blocking(|| {
        let (home, db) = resolve_paths();
        let records = scanner::scan_home(&home).map_err(|e| e.to_string())?;
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.upsert_records(&records).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?;

    let _ = app.emit("scan-finished", ());
    res
}

/// Detect local AI coding tools and persist the registry snapshot.
/// @author codex
#[tauri::command]
pub async fn detect_tools() -> Result<Vec<DetectedTool>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (home, db) = resolve_paths();
        let tools = tool_registry::detect_tools(&home);
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.upsert_tools(&tools).map_err(|e| e.to_string())?;
        Ok(tools)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return the latest persisted Tool Registry rows.
/// @author codex
#[tauri::command]
pub async fn list_tools() -> Result<Vec<DetectedTool>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.list_tools().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Run a read-only Data Source Doctor check and persist the snapshot.
/// @author codex
#[tauri::command]
pub async fn source_health() -> Result<Vec<SourceHealth>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (home, db) = resolve_paths();
        let health = scanner::source_health(&home).map_err(|e| e.to_string())?;
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store
            .insert_source_health(&health)
            .map_err(|e| e.to_string())?;
        Ok(health)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return persisted local pricing rules.
/// @author codex
#[tauri::command]
pub async fn list_pricing_rules() -> Result<Vec<PricingRule>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.list_pricing_rules().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Create or update one local pricing rule.
/// @author codex
#[tauri::command]
pub async fn upsert_pricing_rule(rule: PricingRule) -> Result<PricingRule, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.upsert_pricing_rule(&rule).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Delete one local pricing rule.
/// @author codex
#[tauri::command]
pub async fn delete_pricing_rule(id: i64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.delete_pricing_rule(id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Recompute historical usage costs from persisted local pricing rules.
/// @author codex
#[tauri::command]
pub async fn recompute_costs() -> Result<CostRecomputeResult, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.recompute_costs().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return persisted local project aliases.
/// @author codex
#[tauri::command]
pub async fn list_project_aliases() -> Result<Vec<ProjectAlias>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.list_project_aliases().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Create or update one local project alias.
/// @author codex
#[tauri::command]
pub async fn set_project_alias(alias: ProjectAlias) -> Result<ProjectAlias, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.set_project_alias(&alias).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Export one metadata report to a user-provided local path.
/// @author codex
#[tauri::command]
pub async fn export_report(request: ExportReportRequest) -> Result<ExportReportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        export::export_report(&store, &request).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn export_backup(request: BackupRequest) -> Result<BackupResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store
            .backup_database(Path::new(&request.path))
            .map_err(|e| e.to_string())?;
        Ok(BackupResult {
            success: true,
            path: request.path,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_backup(request: BackupRequest) -> Result<BackupResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let target_path = Path::new(&request.path);
        if !target_path.exists() {
            return Err(format!("Backup file not found at {}", request.path));
        }
        let db_bak = db.with_extension("db.bak");
        if db.exists() {
            let _ = std::fs::copy(&db, &db_bak);
        }
        let wal = db.with_extension("db-wal");
        let shm = db.with_extension("db-shm");
        if wal.exists() {
            let _ = std::fs::remove_file(&wal);
        }
        if shm.exists() {
            let _ = std::fs::remove_file(&shm);
        }

        std::fs::copy(target_path, &db).map_err(|e| e.to_string())?;
        Ok(BackupResult {
            success: true,
            path: request.path,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return the current local privacy audit report.
/// @author codex
#[tauri::command]
pub async fn privacy_audit() -> Result<PrivacyAuditReport, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (home, db) = resolve_paths();
        Ok(privacy::privacy_audit(&home, &db))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Preview a wapc://import deep link without persisting or writing resources.
/// @author codex
#[tauri::command]
pub async fn preview_deep_link_import(url: String) -> Result<DeepLinkImportPreview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        deep_link::preview_deep_link_import(&url).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Plan a wapc://import deep-link sync preview without persisting imported resources.
/// @author codex
#[tauri::command]
pub async fn plan_deep_link_import(
    request: PlanDeepLinkImportRequest,
) -> Result<PlanSyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (home, db) = resolve_paths();
        plan_deep_link_import_for_paths(&home, &db, request)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return the process-local headless read-only dashboard status.
/// @author codex
#[tauri::command]
pub async fn headless_dashboard_status() -> Result<HeadlessDashboardStatus, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let guard = headless_dashboard_slot()
            .lock()
            .map_err(|_| "headless dashboard state lock poisoned".to_string())?;
        Ok(headless_status_from_server(guard.as_ref()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Explicitly start the local read-only headless dashboard on 127.0.0.1.
/// @author codex
#[tauri::command]
pub async fn start_headless_dashboard(
    port: Option<u16>,
) -> Result<HeadlessDashboardStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let mut guard = headless_dashboard_slot()
            .lock()
            .map_err(|_| "headless dashboard state lock poisoned".to_string())?;
        if guard.is_none() {
            let server = start_headless_dashboard_core(HeadlessDashboardConfig {
                bind_host: "127.0.0.1".to_string(),
                port: port.unwrap_or(0),
                db_path: db,
            })
            .map_err(|e| e.to_string())?;
            *guard = Some(server);
        }
        Ok(headless_status_from_server(guard.as_ref()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Stop the process-local headless read-only dashboard.
/// @author codex
#[tauri::command]
pub async fn stop_headless_dashboard() -> Result<HeadlessDashboardStatus, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut guard = headless_dashboard_slot()
            .lock()
            .map_err(|_| "headless dashboard state lock poisoned".to_string())?;
        let _ = guard.take();
        Ok(headless_status_from_server(None))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Run a read-only canonical resource inventory scan and persist the snapshot.
/// @author codex
#[tauri::command]
pub async fn inventory_scan(
    kinds: Option<Vec<String>>,
    project_paths: Option<Vec<String>>,
) -> Result<InventoryScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (home, db) = resolve_paths();
        let kind_refs = kinds
            .as_ref()
            .map(|values| values.iter().map(String::as_str).collect::<Vec<_>>());
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        let project_roots = match project_paths {
            Some(paths) => project_roots_from_paths(paths),
            None => {
                let summaries = store.project_summaries().map_err(|e| e.to_string())?;
                project_roots_from_summaries(&summaries)
            }
        };
        let inventory = resources::scan_inventory_with_project_roots(
            &home,
            &project_roots,
            kind_refs.as_deref(),
        );
        let upserted = store
            .upsert_resources(&inventory.resources)
            .map_err(|e| e.to_string())? as u64;
        store
            .insert_resource_parse_failures(&inventory.failures)
            .map_err(|e| e.to_string())?;
        Ok(InventoryScanResult {
            scanned: inventory.resources.len() as u64,
            upserted,
            failures: inventory.failures.len() as u64,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return persisted canonical resources with optional read-only filters.
/// @author codex
#[tauri::command]
pub async fn list_resources(
    kind: Option<String>,
    tool: Option<String>,
    scope: Option<String>,
    query: Option<String>,
) -> Result<Vec<CanonicalResource>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store
            .list_resources(
                kind.as_deref(),
                tool.as_deref(),
                scope.as_deref(),
                query.as_deref(),
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return one persisted canonical resource by id.
/// @author codex
#[tauri::command]
pub async fn get_resource(id: String) -> Result<Option<CanonicalResource>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store.get_resource(&id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return parse failures captured by the resource inventory scanner.
/// @author codex
#[tauri::command]
pub async fn list_parse_failures() -> Result<Vec<ResourceParseFailure>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store
            .list_resource_parse_failures()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return built-in safe usage guidance for a resource detail panel.
/// @author codex
#[tauri::command]
pub async fn get_guide(
    tool: Option<String>,
    kind: Option<String>,
    resource_id: Option<String>,
) -> Result<ResourceGuide, String> {
    Ok(get_guide_for_input(tool, kind, resource_id))
}

/// Return read-only adapter capabilities for supported tools.
/// @author codex
#[tauri::command]
pub async fn adapter_capabilities() -> Result<Vec<AdapterCapability>, String> {
    Ok(adapters::adapter_capabilities())
}

/// Return session metadata only; prompt/response bodies are never returned.
/// @author codex
#[tauri::command]
pub async fn list_sessions(
    tool: Option<String>,
    project: Option<String>,
    from: Option<String>,
    to: Option<String>,
    query: Option<String>,
) -> Result<Vec<SessionMeta>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
        store
            .list_sessions(
                tool.as_deref(),
                project.as_deref(),
                from.as_deref(),
                to.as_deref(),
                query.as_deref(),
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Produce a safe write plan for a single-tool resource change without writing files.
/// @author codex
#[tauri::command]
pub async fn plan_resource_change(request: ResourceChangeRequest) -> Result<WritePlan, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (home, db) = resolve_paths();
        plan_resource_change_for_paths(&home, &db, request)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Produce Phase 4 cross-tool sync preview plans without writing files.
/// @author codex
#[tauri::command]
pub async fn plan_sync(request: PlanSyncRequest) -> Result<PlanSyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (home, db) = resolve_paths();
        plan_sync_for_paths(&home, &db, request)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// List Phase 5 resource templates, seeding built-ins without writing tool configs.
/// @author codex
#[tauri::command]
pub async fn list_resource_templates() -> Result<Vec<ResourceTemplate>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        list_resource_templates_for_path(&db)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Produce a Sync Engine install preview for a template without persisting it as a resource.
/// @author codex
#[tauri::command]
pub async fn plan_template_sync(
    request: PlanTemplateSyncRequest,
) -> Result<PlanSyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (home, db) = resolve_paths();
        plan_template_sync_for_paths(&home, &db, request)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Apply Phase 4 cross-tool sync plans target-by-target through the Sync Engine.
/// @author codex
#[tauri::command]
pub async fn apply_sync(request: ApplySyncRequest) -> Result<ApplySyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (home, db) = resolve_paths();
        apply_sync_for_paths(&home, &db, request)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return persisted Phase 4 sync operation metadata.
/// @author codex
#[tauri::command]
pub async fn list_sync_operations() -> Result<Vec<SyncOperation>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        list_sync_operations_for_path(&db)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Save or update a local Phase 4 sync preset without storing env values.
/// @author codex
#[tauri::command]
pub async fn save_sync_preset(preset: SyncPreset) -> Result<SyncPreset, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        save_sync_preset_for_path(&db, preset)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return local Phase 4 sync presets.
/// @author codex
#[tauri::command]
pub async fn list_sync_presets() -> Result<Vec<SyncPreset>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        list_sync_presets_for_path(&db)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Delete a local Phase 4 sync preset.
/// @author codex
#[tauri::command]
pub async fn delete_sync_preset(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        delete_sync_preset_for_path(&db, id)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Export local Phase 4 sync presets as secret-free JSON.
/// @author codex
#[tauri::command]
pub async fn export_sync_presets(path: String) -> Result<ExportReportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        export_sync_presets_for_path(&db, std::path::Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Apply a previously generated write plan through the Sync Engine.
/// @author codex
#[tauri::command]
pub async fn apply_resource_change(
    request: ApplyChangeRequest,
) -> Result<ApplyChangeResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (home, db) = resolve_paths();
        apply_resource_change_for_paths(&home, &db, request)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// List persisted Sync Engine change metadata.
/// @author codex
#[tauri::command]
pub async fn list_changes(tool: Option<String>) -> Result<Vec<ResourceChangeLog>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        list_changes_for_path(&db, tool)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// List persisted Sync Engine backup metadata.
/// @author codex
#[tauri::command]
pub async fn list_backups(tool: Option<String>) -> Result<Vec<ResourceBackup>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, db) = resolve_paths();
        list_backups_for_path(&db, tool)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Restore a committed Sync Engine change from its recorded backup.
/// @author codex
#[tauri::command]
pub async fn rollback_change(change_id: String) -> Result<ApplyChangeResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (home, db) = resolve_paths();
        rollback_change_for_paths(&home, &db, change_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get trend data for a given number of days.
#[tauri::command]
pub fn get_trend(days: i64) -> Result<Vec<DailyToolSummary>, String> {
    let (_, db) = resolve_paths();
    let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
    let trend_days = recent_days(days);
    store
        .daily_tool_totals(&trend_days)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_auto_scan_config() -> Result<AutoScanConfig, String> {
    let db = db_path();
    let store = UsageStore::open(&db).map_err(|e| e.to_string())?;

    // Default config if not set
    let default_config = AutoScanConfig {
        enabled: false,
        interval_minutes: 60,
    };

    #[allow(clippy::collapsible_if)]
    if let Some(json) = store
        .get_setting("auto_scan_config")
        .map_err(|e| e.to_string())?
    {
        if let Ok(config) = serde_json::from_str::<AutoScanConfig>(&json) {
            return Ok(config);
        }
    }

    Ok(default_config)
}

#[tauri::command]
pub fn set_auto_scan_config(config: AutoScanConfig) -> Result<(), String> {
    let db = db_path();
    let store = UsageStore::open(&db).map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    store
        .set_setting("auto_scan_config", &json)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn check_update(
    app: tauri::AppHandle,
) -> Result<Option<crate::updater::UpdateAvailablePayload>, String> {
    let conf = app.config().plugins.0.get("updater").cloned();
    let pubkey = conf
        .as_ref()
        .and_then(|v| v.get("pubkey"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    if pubkey.starts_with("PLACEHOLDER") || pubkey.is_empty() {
        return Ok(None);
    }

    let updater = match tauri_plugin_updater::UpdaterExt::updater(&app) {
        Ok(u) => u,
        Err(e) => return Err(e.to_string()),
    };

    let update = updater.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| crate::updater::UpdateAvailablePayload {
        version: u.version.clone(),
        notes: u.body.clone(),
        pub_date: u.date.map(|d| d.to_string()),
    }))
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    crate::updater::download_and_install(app).await
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use wapc::model::{
        ApplyChangeRequest, ApplySyncRequest, CanonicalResource, PlanSyncRequest,
        PlanTemplateSyncRequest, ProjectSummary, ResourceChangeRequest, SyncPreset, SyncTarget,
        TokenUsage,
    };

    use super::*;

    #[test]
    fn project_roots_from_summaries_keeps_existing_directories_only() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-project-roots-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let existing = dir.join("repo");
        std::fs::create_dir_all(&existing).unwrap();
        let missing = dir.join("missing");
        let summaries = vec![
            project_summary(existing.display().to_string()),
            project_summary(missing.display().to_string()),
            project_summary("(unknown)".to_string()),
            project_summary(existing.display().to_string()),
        ];

        let roots = project_roots_from_summaries(&summaries);

        assert_eq!(roots, vec![existing]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tool_path_verifications_alias_home_and_project_paths_without_reading_contents() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-command-path-verify-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = dir.join("workspace/my project");
        fs::create_dir_all(dir.join(".codex")).unwrap();
        fs::create_dir_all(&project).unwrap();
        fs::write(dir.join(".codex/config.toml"), "SHOULD_NOT_BE_READ_SECRET").unwrap();
        fs::write(project.join("AGENTS.md"), "SHOULD_NOT_BE_READ_SECRET").unwrap();

        let records = tool_path_verifications_for_paths(&dir, std::slice::from_ref(&project));
        let serialized = serde_json::to_string(&records).unwrap();

        assert!(records.iter().any(|record| {
            record.tool == "codex"
                && record.scope == "user"
                && record.kind == "mcp_config"
                && record.path == "~/.codex/config.toml"
                && record.exists
                && record.is_file
                && record.read_only
                && !record.write_supported
        }));
        assert!(records.iter().any(|record| {
            record.tool == "codex"
                && record.scope == "project"
                && record.kind == "project_instruction_file"
                && record.path == "<project>/AGENTS.md"
                && record.exists
                && record.is_file
                && record.read_only
                && !record.write_supported
        }));
        assert!(!serialized.contains(dir.to_string_lossy().as_ref()));
        assert!(!serialized.contains("SHOULD_NOT_BE_READ_SECRET"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn headless_dashboard_commands_start_disabled_and_stop_explicitly() {
        let _ = tauri::async_runtime::block_on(stop_headless_dashboard()).unwrap();
        let initial = tauri::async_runtime::block_on(headless_dashboard_status()).unwrap();

        assert!(!initial.running);
        assert!(initial.url.is_none());

        let started = tauri::async_runtime::block_on(start_headless_dashboard(None)).unwrap();

        assert!(started.running);
        assert_eq!(started.bind_host.as_deref(), Some("127.0.0.1"));
        assert!(started.port.unwrap() > 0);
        assert!(
            started
                .url
                .as_deref()
                .unwrap()
                .starts_with("http://127.0.0.1:")
        );

        let stopped = tauri::async_runtime::block_on(stop_headless_dashboard()).unwrap();
        assert!(!stopped.running);
        assert!(stopped.url.is_none());
    }

    #[test]
    fn preview_deep_link_import_command_returns_safe_preview() {
        let link = format!(
            "wapc://import?source={}&resource={}",
            percent_encode("https://example.test/templates/docs-mcp"),
            percent_encode(
                r#"{"kind":"mcp","name":"docs","scope":"user","payload":{"transport":"http","url":"https://example.test/mcp","env_keys":["DOCS_TOKEN"],"env_fingerprints":{}}}"#
            )
        );

        let preview = tauri::async_runtime::block_on(preview_deep_link_import(link)).unwrap();

        assert_eq!(preview.schema, "wapc.deep_link_import_preview.v1");
        assert_eq!(preview.resource.origin_tool, "deep-link");
        assert_eq!(preview.resource.name, "docs");
        assert!(preview.resource.redacted);
        assert!(preview.risks.is_empty());
    }

    #[test]
    fn plan_deep_link_import_helper_generates_sync_preview_without_persisting_resource() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-command-plan-deep-link-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join(".wapc/wapc.db");
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

        let result = plan_deep_link_import_for_paths(
            &dir,
            &db,
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
        let store = UsageStore::open(&db).unwrap();

        assert_eq!(result.targets[0].status, "planned");
        assert!(result.source_resource_id.starts_with("deep-link:mcp:docs:"));
        assert_eq!(result.targets[0].required_env_keys, vec!["DOCS_TOKEN"]);
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
    fn plan_and_apply_resource_change_helpers_use_sync_engine_without_real_home() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-command-sync-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let config = dir.join(".claude.json");
        fs::write(
            &config,
            r#"{"mcpServers":{"github":{"command":"npx"},"docs":{"url":"https://example.test/mcp"}}}"#,
        )
        .unwrap();
        let db = dir.join(".wapc/wapc.db");

        let plan = plan_resource_change_for_paths(
            &dir,
            &db,
            ResourceChangeRequest {
                tool: "claude".to_string(),
                kind: "mcp".to_string(),
                op: "disable".to_string(),
                resource_id: Some("mcp:github".to_string()),
                target_path: config.display().to_string(),
                resource_name: "github".to_string(),
            },
        )
        .unwrap();

        assert!(fs::read_to_string(&config).unwrap().contains("\"github\""));

        let result = apply_resource_change_for_paths(
            &dir,
            &db,
            ApplyChangeRequest {
                plan,
                confirm_drift: false,
                sync_id: None,
                force_verify_failure: false,
            },
        )
        .unwrap();
        let changes = list_changes_for_path(&db, None).unwrap();
        let backups = list_backups_for_path(&db, None).unwrap();

        assert_eq!(result.status, "committed");
        assert!(!fs::read_to_string(&config).unwrap().contains("\"github\""));
        assert_eq!(changes.len(), 1);
        assert_eq!(backups.len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rollback_change_helper_restores_previous_resource_state() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-command-rollback-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let config = dir.join(".claude.json");
        fs::write(
            &config,
            r#"{"mcpServers":{"github":{"command":"npx"},"docs":{"url":"https://example.test/mcp"}}}"#,
        )
        .unwrap();
        let original = fs::read_to_string(&config).unwrap();
        let db = dir.join(".wapc/wapc.db");
        let plan = plan_resource_change_for_paths(
            &dir,
            &db,
            ResourceChangeRequest {
                tool: "claude".to_string(),
                kind: "mcp".to_string(),
                op: "disable".to_string(),
                resource_id: Some("mcp:github".to_string()),
                target_path: config.display().to_string(),
                resource_name: "github".to_string(),
            },
        )
        .unwrap();
        let result = apply_resource_change_for_paths(
            &dir,
            &db,
            ApplyChangeRequest {
                plan,
                confirm_drift: false,
                sync_id: None,
                force_verify_failure: false,
            },
        )
        .unwrap();

        let rollback = rollback_change_for_paths(&dir, &db, result.change_id.clone()).unwrap();

        assert_eq!(rollback.status, "committed");
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
        let changes = list_changes_for_path(&db, None).unwrap();
        assert!(changes.iter().any(|change| {
            change.reverts_change_id.as_deref() == Some(result.change_id.as_str())
                && change.op == "rollback"
        }));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn plan_sync_helper_generates_cross_tool_preview_without_writing() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-command-plan-sync-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[CanonicalResource {
                id: "mcp:docs".to_string(),
                kind: "mcp".to_string(),
                name: "docs".to_string(),
                scope: "user".to_string(),
                origin_tool: "claude".to_string(),
                origin_path: "/tmp/source.json".to_string(),
                origin_locator: Some("mcpServers.docs".to_string()),
                enabled_in: vec!["claude".to_string()],
                confidence: 1.0,
                redacted: false,
                payload_json: r#"{"transport":"http","command":null,"args":[],"url":"https://example.test/mcp","env_keys":[],"env_fingerprints":{}}"#.to_string(),
                provided_by_plugin: None,
                last_seen: "2026-06-06T00:00:00Z".to_string(),
            }])
            .unwrap();
        let target_path = dir.join(".gemini/settings.json");
        fs::create_dir_all(target_path.parent().unwrap()).unwrap();
        fs::write(&target_path, r#"{"mcpServers":{}}"#).unwrap();
        let before = fs::read_to_string(&target_path).unwrap();

        let result = plan_sync_for_paths(
            &dir,
            &db,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![SyncTarget {
                    tool: "gemini".to_string(),
                    scope: "user".to_string(),
                    project_path: None,
                    target_path: target_path.display().to_string(),
                    format: "json".to_string(),
                }],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        assert_eq!(result.targets[0].status, "planned");
        assert!(
            result.targets[0]
                .plan
                .as_ref()
                .unwrap()
                .preview_after
                .contains("\"docs\"")
        );
        assert_eq!(fs::read_to_string(&target_path).unwrap(), before);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn template_commands_seed_list_and_plan_without_writing_resources() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-command-template-sync-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join(".wapc/wapc.db");
        let target_path = dir.join(".claude.json");
        fs::write(&target_path, r#"{"mcpServers":{}}"#).unwrap();

        let templates = list_resource_templates_for_path(&db).unwrap();
        let context7 = templates
            .iter()
            .find(|template| template.id == "builtin:context7-mcp")
            .unwrap();
        let before = fs::read_to_string(&target_path).unwrap();

        let result = plan_template_sync_for_paths(
            &dir,
            &db,
            PlanTemplateSyncRequest {
                template_id: context7.id.clone(),
                targets: vec![SyncTarget {
                    tool: "claude".to_string(),
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
        let store = UsageStore::open(&db).unwrap();

        assert_eq!(result.targets[0].status, "planned");
        assert!(
            result
                .source_resource_id
                .starts_with("template:builtin:context7-mcp:")
        );
        assert!(
            result.targets[0]
                .plan
                .as_ref()
                .unwrap()
                .preview_after
                .contains("@upstash/context7-mcp")
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
    fn get_guide_helper_links_selected_resource_to_safe_usage_guidance() {
        let guide = get_guide_for_input(
            Some("claude".to_string()),
            Some("mcp".to_string()),
            Some("mcp:user:claude:github".to_string()),
        );
        let text = serde_json::to_string(&guide).unwrap();

        assert_eq!(guide.id, "guide:claude:mcp");
        assert_eq!(guide.kind, "mcp");
        assert!(guide.summary.contains("MCP"));
        assert!(
            guide
                .unsupported_actions
                .iter()
                .any(|item| item.contains("enterprise"))
        );
        assert!(!text.contains("secret-token"));
    }

    #[test]
    fn apply_sync_command_helper_uses_cross_sync_without_real_home() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-command-apply-sync-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join(".wapc/wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_resources(&[CanonicalResource {
                id: "mcp:docs".to_string(),
                kind: "mcp".to_string(),
                name: "docs".to_string(),
                scope: "user".to_string(),
                origin_tool: "claude".to_string(),
                origin_path: "/tmp/source.json".to_string(),
                origin_locator: Some("mcpServers.docs".to_string()),
                enabled_in: vec!["claude".to_string()],
                confidence: 1.0,
                redacted: false,
                payload_json: r#"{"transport":"http","command":null,"args":[],"url":"https://example.test/mcp","env_keys":[],"env_fingerprints":{}}"#.to_string(),
                provided_by_plugin: None,
                last_seen: "2026-06-06T00:00:00Z".to_string(),
            }])
            .unwrap();
        let target_path = dir.join(".claude.json");
        fs::write(&target_path, r#"{"mcpServers":{}}"#).unwrap();
        let planned = plan_sync_for_paths(
            &dir,
            &db,
            PlanSyncRequest {
                resource_id: "mcp:docs".to_string(),
                targets: vec![SyncTarget {
                    tool: "claude".to_string(),
                    scope: "user".to_string(),
                    project_path: None,
                    target_path: target_path.display().to_string(),
                    format: "json".to_string(),
                }],
                allow_cross_scope: false,
                env_strategy: "reuse".to_string(),
            },
        )
        .unwrap();

        let result = apply_sync_for_paths(
            &dir,
            &db,
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
        let operations = list_sync_operations_for_path(&db).unwrap();

        assert_eq!(result.changes[0].status, "committed");
        assert_eq!(operations[0].sync_id, result.sync_id);
        assert!(
            fs::read_to_string(&target_path)
                .unwrap()
                .contains("\"docs\"")
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sync_preset_command_helpers_persist_without_real_home() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-command-sync-preset-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join(".wapc/wapc.db");
        let preset = SyncPreset {
            id: "preset:github-json-targets".to_string(),
            name: "GitHub MCP to JSON tools".to_string(),
            resources_json: r#"["mcp:user:codex:github"]"#.to_string(),
            targets_json: r#"[{"tool":"gemini","scope":"user","project_path":null,"target_path":"/Users/example/.gemini/settings.json","format":"json"}]"#.to_string(),
            updated_at: "2026-06-06T08:00:00Z".to_string(),
        };

        let saved = save_sync_preset_for_path(&db, preset.clone()).unwrap();
        assert_eq!(saved, preset);
        assert_eq!(
            list_sync_presets_for_path(&db).unwrap(),
            vec![preset.clone()]
        );

        delete_sync_preset_for_path(&db, preset.id.clone()).unwrap();
        assert!(list_sync_presets_for_path(&db).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn export_sync_preset_helper_writes_json_without_real_home() {
        let dir = std::env::temp_dir().join(format!(
            "wapc-command-export-sync-preset-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join(".wapc/wapc.db");
        let out = dir.join("exports/sync-presets.json");
        let preset = SyncPreset {
            id: "preset:github".to_string(),
            name: "GitHub MCP targets".to_string(),
            resources_json: r#"["mcp:user:codex:github"]"#.to_string(),
            targets_json: r#"[{"tool":"gemini","scope":"user","project_path":null,"target_path":"/Users/example/.gemini/settings.json","format":"json"}]"#.to_string(),
            updated_at: "2026-06-06T08:00:00Z".to_string(),
        };
        save_sync_preset_for_path(&db, preset).unwrap();

        let result = export_sync_presets_for_path(&db, &out).unwrap();

        let content = fs::read_to_string(&out).unwrap();
        assert_eq!(result.path, out.display().to_string());
        assert!(content.contains("wapc.sync_presets.v1"));
        assert!(content.contains("GitHub MCP targets"));
        assert!(!content.contains("env_values"));
        fs::remove_dir_all(&dir).unwrap();
    }

    fn project_summary(canonical_path: String) -> ProjectSummary {
        ProjectSummary {
            canonical_path,
            display_name: "repo".to_string(),
            alias: None,
            original_paths: Vec::new(),
            tools: Vec::new(),
            records: 0,
            usage: TokenUsage::default(),
            cost_usd: 0.0,
        }
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
