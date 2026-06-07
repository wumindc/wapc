/**
 * TypeScript type definitions matching Rust backend structs.
 * @author codex
 */

export interface TokenUsage {
  input: number
  output: number
  cache_read: number
  cache_write: number
  reasoning: number
  tool: number
}

export interface UsageSummary {
  name: string
  records: number
  usage: TokenUsage
  cost_usd: number
}

export interface DailyToolSummary {
  day: string
  tool: string
  total_tokens: number
}

export interface DetectedTool {
  id: string
  display_name: string
  installed: boolean
  version: string | null
  config_dir: string | null
  data_dir: string | null
  config_dir_exists: boolean
  data_dir_exists: boolean
  last_detected_at: string
}

export interface SourceHealth {
  tool: string
  source_glob: string
  exists: boolean
  readable_files: number
  parsed_records: number
  failed_files: number
  latest_event_ts: string | null
  checked_at: string
}

export interface PricingRule {
  id: number | null
  model_match: string
  match_kind: 'exact' | 'prefix' | string
  provider: string | null
  currency: string
  price_input: number | null
  price_output: number | null
  price_cache_read: number | null
  price_cache_write: number | null
  price_reasoning: number | null
  price_tool: number | null
  source: 'builtin' | 'user' | string
  updated_at: string
}

export interface CostRecomputeResult {
  updated: number
  exact_matches: number
  prefix_matches: number
  no_matches: number
}

export interface ProjectAlias {
  canonical_path: string
  alias: string
  updated_at: string
}

export interface ProjectSummary {
  canonical_path: string
  display_name: string
  alias: string | null
  original_paths: string[]
  tools: string[]
  records: number
  usage: TokenUsage
  cost_usd: number
}

export interface ExportReportRequest {
  view: 'tools' | 'projects' | 'daily' | 'redacted' | string
  format: 'csv' | 'json' | 'markdown' | string
  path: string
  from?: string | null
  to?: string | null
  include_fixture?: boolean
  include_project_aliases?: boolean
}

export interface ExportReportResult {
  path: string
  bytes_written: number
}

export interface BackupRequest {
  path: string
}

export interface BackupResult {
  success: boolean
  path: string
}

export interface PrivacyAuditReport {
  generated_at: string
  local_only: boolean
  db_path: string
  read_sources: PrivacyAuditSource[]
  stored_tables: PrivacyAuditTable[]
  forbidden_fields: string[]
  export_boundary: string
}

export interface HeadlessDashboardStatus {
  running: boolean
  bind_host: string | null
  port: number | null
  url: string | null
  read_only: boolean
}

export interface PrivacyAuditSource {
  name: string
  path: string
  purpose: string
  reads_body: boolean
  writes_source: boolean
}

export interface PrivacyAuditTable {
  name: string
  fields: string[]
}

export interface CanonicalResource {
  id: string
  kind: string
  name: string
  scope: string
  origin_tool: string
  origin_path: string
  origin_locator: string | null
  enabled_in: string[]
  confidence: number
  redacted: boolean
  payload_json: string
  provided_by_plugin: string | null
  last_seen: string
}

export interface ResourceGuideSection {
  title: string
  body: string
}

export interface ResourceGuide {
  id: string
  tool: string | null
  kind: string
  title: string
  summary: string
  sections: ResourceGuideSection[]
  risks: string[]
  unsupported_actions: string[]
  updated_at: string
}

export interface DeepLinkImportPreview {
  schema: string
  source: string
  content_fingerprint: string
  resource: CanonicalResource
  risks: string[]
}

export interface ResourceTemplate {
  id: string
  name: string
  kind: string
  scope: string
  description: string
  source: string
  content_fingerprint: string
  required_env_keys: string[]
  payload_json: string
  updated_at: string
}

export interface ResourceParseFailure {
  path: string
  tool: string
  kind: string | null
  reason: string
  seen_at: string
}

export interface InventoryScanResult {
  scanned: number
  upserted: number
  failures: number
}

export interface AdapterCapability {
  tool: string
  display_name: string
  resource_kinds: string[]
  scopes: string[]
  transports: string[]
  read_only: boolean
  notes: string[]
}

export interface ToolPathVerificationRecord {
  tool: string
  platform: string
  scope: string
  kind: string
  path: string
  candidate_verified: boolean
  exists: boolean
  is_file: boolean
  is_dir: boolean
  read_only: boolean
  write_supported: boolean
}

export interface SessionMeta {
  session_id: string
  tool: string
  project_path: string | null
  first_ts: string | null
  last_ts: string | null
  records: number
  total_tokens: number
  cost_usd: number
  source_paths: string[]
}

export interface ResourceChangeRequest {
  tool: string
  kind: string
  op: string
  resource_id: string | null
  target_path: string
  resource_name: string
}

export interface WritePlanRisk {
  code: string
  message: string
  severity: string
}

export interface WritePlan {
  plan_id: string
  tool: string
  kind: string
  op: string
  resource_id: string | null
  resource_name: string
  target_path: string
  target_scope?: string | null
  target_project_path?: string | null
  before_fingerprint: string
  after_fingerprint: string
  diff: string
  preview_before: string
  preview_after: string
  requires_backup: boolean
  risks: WritePlanRisk[]
  created_at: string
}

export interface ApplyChangeRequest {
  plan: WritePlan
  confirm_drift: boolean
  sync_id?: string | null
}

export interface ApplyChangeResult {
  change_id: string
  backup_path: string | null
  status: string
}

export interface ResourceChangeLog {
  change_id: string
  sync_id: string | null
  tool: string
  resource_id: string | null
  kind: string
  op: string
  target_path: string
  backup_path: string | null
  status: string
  reverts_change_id: string | null
  created_at: string
}

export interface ResourceBackup {
  backup_path: string
  tool: string
  original_path: string
  change_id: string | null
  created_at: string
}

export interface SyncTarget {
  tool: string
  scope: string
  project_path: string | null
  target_path: string
  format: string
}

export interface PlanSyncRequest {
  resource_id: string
  targets: SyncTarget[]
  allow_cross_scope: boolean
  env_strategy: 'reuse' | 'manual' | 'skip' | string
}

export interface PlanDeepLinkImportRequest {
  url: string
  targets: SyncTarget[]
  allow_cross_scope: boolean
  env_strategy: 'reuse' | 'manual' | 'skip' | string
}

export interface SyncTargetPlan {
  target: SyncTarget
  status: 'planned' | 'unsupported' | 'requires_input' | string
  reason: string | null
  required_env_keys: string[]
  plan: WritePlan | null
}

export interface PlanSyncResult {
  source_resource_id: string
  created_at: string
  targets: SyncTargetPlan[]
}

export interface ApplySyncRequest {
  plans: WritePlan[]
  confirm_drift: boolean
  allow_cross_scope: boolean
  env_strategy?: string | null
  env_values?: Record<string, string>
  deep_link_url?: string | null
}

export interface ApplySyncTargetResult {
  plan_id: string
  target_path: string
  status: string
  change_id: string | null
  backup_path: string | null
  reason: string | null
}

export interface ApplySyncResult {
  sync_id: string
  changes: ApplySyncTargetResult[]
}

export interface SyncOperation {
  sync_id: string
  source_resource_id: string | null
  targets_json: string
  allow_cross_scope: boolean
  env_strategy: string
  created_at: string
}

export interface SyncPreset {
  id: string
  name: string
  resources_json: string
  targets_json: string
  updated_at: string
}

export interface DesktopSnapshot {
  today: UsageSummary[]
  yesterday: UsageSummary[]
  tools: UsageSummary[]
  projects: UsageSummary[]
  daily: DailyToolSummary[]
  trend_days: string[]
  daily_summaries: UsageSummary[]
  scan_records: number
  db_path: string
  db_exists: boolean
  home_path: string
  version: string
  detected_tools: DetectedTool[]
  source_health: SourceHealth[]
  project_summaries: ProjectSummary[]
  privacy_audit: PrivacyAuditReport
  resources: CanonicalResource[]
  resource_parse_failures: ResourceParseFailure[]
  adapter_capabilities: AdapterCapability[]
  tool_path_verifications: ToolPathVerificationRecord[]
}

// ── Derived helpers ───────────────────────────────────────────────────────────

export function totalTokensToday(snapshot: DesktopSnapshot): number {
  return snapshot.today.reduce((sum, s) => sum + tokenTotal(s.usage), 0)
}

export function totalRecordsToday(snapshot: DesktopSnapshot): number {
  return snapshot.today.reduce((sum, s) => sum + s.records, 0)
}

export function estimatedCostToday(snapshot: DesktopSnapshot): number {
  return snapshot.today.reduce((sum, s) => sum + s.cost_usd, 0)
}

export function tokenTotal(usage: TokenUsage): number {
  return usage.input + usage.output + usage.cache_read + usage.cache_write + usage.reasoning + usage.tool
}

export type ToolName = 'claude' | 'codex' | 'gemini' | 'opencode' | string

export interface ToolMeta {
  displayName: string
  initial: string
  color: string     // hex
  bgColor: string   // tailwind css var or hex
}

export const TOOL_META: Record<string, ToolMeta> = {
  claude: {
    displayName: 'Claude Code',
    initial: 'C',
    color: '#CC785C',
    bgColor: 'rgba(204,120,92,0.12)',
  },
  codex: {
    displayName: 'Codex',
    initial: 'C',
    color: '#1F6FEB',
    bgColor: 'rgba(31,111,235,0.12)',
  },
  gemini: {
    displayName: 'Gemini CLI',
    initial: 'G',
    color: '#7E57EB',
    bgColor: 'rgba(126,87,235,0.12)',
  },
  opencode: {
    displayName: 'OpenCode',
    initial: 'O',
    color: '#F1761F',
    bgColor: 'rgba(241,118,31,0.12)',
  },
  cursor: {
    displayName: 'Cursor',
    initial: 'C',
    color: '#3B82F6', // blue-500
    bgColor: 'rgba(59,130,246,0.12)',
  },
  trae: {
    displayName: 'Trae',
    initial: 'T',
    color: '#8B5CF6', // violet-500
    bgColor: 'rgba(139,92,246,0.12)',
  },
  qoder: {
    displayName: 'Qoder',
    initial: 'Q',
    color: '#10B981', // emerald-500
    bgColor: 'rgba(16,185,129,0.12)',
  },
  kiro: {
    displayName: 'Kiro',
    initial: 'K',
    color: '#F59E0B', // amber-500
    bgColor: 'rgba(245,158,11,0.12)',
  },
  'antigravity ide': {
    displayName: 'Antigravity IDE',
    initial: 'A',
    color: '#EC4899', // pink-500
    bgColor: 'rgba(236,72,153,0.12)',
  },
}

export function getToolMeta(name: string): ToolMeta {
  return (
    TOOL_META[name.toLowerCase()] ?? {
      displayName: name,
      initial: name.charAt(0).toUpperCase(),
      color: '#4B5563',
      bgColor: 'rgba(75,85,99,0.12)',
    }
  )
}

export const TOOL_COLORS = ['#1F6FEB', '#12A0A6', '#7E57EB', '#F1761F', '#4B5563']

export function toolColorByIndex(index: number): string {
  return TOOL_COLORS[index % TOOL_COLORS.length]
}
