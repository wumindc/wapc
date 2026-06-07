# WAPC Docs Roadmap Iteration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the PRD and design documents under `docs/` into iterative, verified product increments, starting with the Phase 1 read-only observatory foundation.

**Architecture:** WAPC remains a Rust Core library plus Tauri commands plus React desktop UI. Phase 1 work adds read-only Tool Registry and Data Source Doctor primitives in Rust Core, persists snapshots in `~/.wapc/wapc.db`, exposes them through Tauri commands, and renders real desktop states.

**Tech Stack:** Rust 2024, rusqlite, serde, walkdir, Tauri 2, React, TypeScript, Vite.

---

## Planning Notes

- `docs/prd/README.md` says WAPC has no CLI and all "commands" are Tauri commands.
- `docs/cc-switch-reference-roadmap.md` still lists CLI acceptance items. For this iteration, CLI-shaped acceptance is translated into Rust functions and Tauri commands. CLI resurrection is out of scope unless the PRD is revised.
- Worktree is already dirty with many user changes. Do not revert unrelated edits.
- All code comments created by Codex must include `@author codex`.
- Production standard: every shipped capability must use real local data and real persistence paths. No demo-only fake data, fake workflows, or hardcoded business outcomes are allowed. Unsupported or unfinished capabilities must remain explicit later-work items.

## Roadmap Coverage

- Phase 1: observatory hardening, tool detection, source health, pricing, project attribution, export.
- Phase 2: read-only resource inventory with canonical resources and adapters.
- Phase 3: safe write pipeline and single-tool resource management.
- Phase 4: cross-tool sync and injection.
- Phase 5: templates, deep links, redacted reports, headless read-only mode, signing/notarization, cross-platform feasibility.

This plan implements a first shippable Phase 1 slice:

- Tool Registry core model and detector.
- Data Source Doctor core model and scanner health.
- SQLite persistence for `tools` and `source_health`.
- Tauri commands `detect_tools`, `list_tools`, and `source_health`.
- Desktop Tools and Auto Scan pages show real registry and source-health data.

## Files

- Create: `src/tool_registry.rs` for tool definitions, read-only detection, and tests.
- Modify: `src/scanner.rs` to expose source definitions and health checks.
- Modify: `src/model.rs` to add serializable `DetectedTool` and `SourceHealth` models.
- Modify: `src/store.rs` to migrate and persist `tools` / `source_health`.
- Modify: `src/lib.rs` to export the new module.
- Modify: `src-tauri/src/commands.rs` to expose Phase 1 Tauri commands and include registry data in `DesktopSnapshot`.
- Modify: `src-tauri/src/lib.rs` to register new commands.
- Modify: `ui/src/types/index.ts` to mirror the new backend structs.
- Modify: `ui/src/pages/ToolsPage.tsx` to show detected tool installation/config/data status.
- Modify: `ui/src/pages/AutoScanPage.tsx` to show real source health counts instead of static rows.

## Task 1: Tool Registry Core

**Files:**
- Create: `src/tool_registry.rs`
- Modify: `src/model.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Define serializable registry model**

Add `DetectedTool` fields: `id`, `display_name`, `installed`, `version`, `config_dir`, `data_dir`, `config_dir_exists`, `data_dir_exists`, `last_detected_at`.

- [x] **Step 2: Implement read-only detection**

Detect Claude Code, Codex, Gemini CLI, and OpenCode by config/data directories and executable availability. Version detection is best effort and returns `unknown` on failure.

- [x] **Step 3: Add tests**

Run: `cargo test tool_registry`
Expected: PASS.

## Task 2: Data Source Doctor Core

**Files:**
- Modify: `src/model.rs`
- Modify: `src/scanner.rs`

- [x] **Step 1: Define `SourceHealth`**

Fields: `tool`, `source_glob`, `exists`, `readable_files`, `parsed_records`, `failed_files`, `latest_event_ts`, `checked_at`.

- [x] **Step 2: Implement source health scan**

Reuse known collector roots. Count matching files, parse successes, parse failures, and latest event timestamp without reading or storing prompt/response content.

- [x] **Step 3: Add tests**

Run: `cargo test scanner`
Expected: PASS.

## Task 3: SQLite Persistence

**Files:**
- Modify: `src/store.rs`

- [x] **Step 1: Add schema migrations**

Create `tools` and `source_health` tables from Phase 1 PRD.

- [x] **Step 2: Add upsert/list methods**

Add `upsert_tools`, `list_tools`, `insert_source_health`, and `latest_source_health`.

- [x] **Step 3: Add tests**

Run: `cargo test store`
Expected: PASS.

## Task 4: Tauri Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Add commands**

Expose `detect_tools`, `list_tools`, and `source_health`.

- [x] **Step 2: Enrich snapshot**

Add `detected_tools` and `source_health` to `DesktopSnapshot`.

- [x] **Step 3: Verify build**

Run: `cargo test --workspace`
Expected: PASS.

## Task 5: Desktop UI Wiring

**Files:**
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/ToolsPage.tsx`
- Modify: `ui/src/pages/AutoScanPage.tsx`

- [x] **Step 1: Add frontend types**

Mirror `DetectedTool` and `SourceHealth`.

- [x] **Step 2: Render tool registry**

Tools page shows installed status, version, config/data directory existence, and last detected time.

- [x] **Step 3: Render source health**

Auto Scan page shows exists/readable/parsed/failed/latest-event values from backend data.

- [x] **Step 4: Verify UI build**

Run: `cd ui && yarn build`
Expected: PASS.

## Task 6: Pricing Rules Core

**Files:**
- Modify: `src/model.rs`
- Modify: `src/store.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Create: `ui/src/pages/PricingPage.tsx`
- Modify: `ui/src/components/layout/Sidebar.tsx`
- Modify: `ui/src/components/layout/Header.tsx`
- Modify: `ui/src/App.tsx`

Production boundary for this slice:

- Implement real local user pricing rules, persisted in SQLite.
- Implement deterministic historical cost recomputation from persisted `usage_records`.
- Do not hardcode unverified vendor prices. Built-in official default prices remain a later task that must be sourced from current official provider pricing.
- No matching rule must set `cost_usd = NULL` and `cost_source = 'none'`.

- [x] **Step 1: Define pricing models**

Add `PricingRule` and `CostRecomputeResult` structs. Rule fields include `id`, `model_match`, `match_kind`, optional `provider`, per-token-bucket prices, `currency`, `source`, and `updated_at`.

- [x] **Step 2: Add store migration and CRUD**

Create `pricing_rules`; add `usage_records.cost_source` migration; implement `list_pricing_rules`, `upsert_pricing_rule`, and `delete_pricing_rule`.

- [x] **Step 3: Add recompute implementation**

Match exact model before prefix; update each usage record's `cost_usd` and `cost_source`; leave no-match records with `NULL` cost.

- [x] **Step 4: Add Tauri commands**

Expose `list_pricing_rules`, `upsert_pricing_rule`, `delete_pricing_rule`, and `recompute_costs`.

- [x] **Step 5: Add desktop pricing page**

Add a real desktop page that lists local rules from SQLite, saves/deletes through Tauri commands, and triggers historical cost recomputation. Do not use mock pricing rows.

- [x] **Step 6: Verify**

Run: `cargo test pricing`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

## Task 7: Project Attribution Core

**Files:**
- Modify: `src/model.rs`
- Modify: `src/store.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/ProjectsPage.tsx`

Production boundary for this slice:

- Normalize paths only for display and aggregation. Preserve `usage_records.project_path` exactly as collected.
- Persist local aliases in SQLite, keyed by canonical path.
- Aggregate records from multiple tools that point at the same canonical path into one project row.
- Use real database rows and Tauri commands; no hardcoded project examples.

- [x] **Step 1: Define project attribution models**

Add `ProjectAlias` and `ProjectSummary` structs. `ProjectSummary` includes canonical path, display name, alias, original path samples, record count, token usage, cost, and tool ids.

- [x] **Step 2: Add store migration and attribution queries**

Create `project_aliases`; implement path normalization, `list_project_aliases`, `set_project_alias`, and `project_summaries`.

- [x] **Step 3: Add tests**

Cover trailing slash normalization, `~` expansion, alias precedence, and cross-tool aggregation.

- [x] **Step 4: Add Tauri commands and snapshot data**

Expose `list_project_aliases`, `set_project_alias`, and add `project_summaries` to `DesktopSnapshot`.

- [x] **Step 5: Update projects page**

Render canonical project rows, alias names, original path hover text, tool list, and inline alias editing through Tauri.

- [x] **Step 6: Verify**

Run: `cargo test project`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

## Task 8: Export Reports

**Files:**
- Modify: `src/model.rs`
- Create: `src/export.rs`
- Modify: `src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Create: `ui/src/pages/ExportPage.tsx`
- Modify: `ui/src/components/layout/Sidebar.tsx`
- Modify: `ui/src/components/layout/Header.tsx`
- Modify: `ui/src/App.tsx`

Production boundary for this slice:

- Export only persisted metadata and aggregate summaries from SQLite-backed data.
- Support `tools`, `projects`, and `daily` views.
- Support CSV, JSON, and Markdown.
- The caller supplies an explicit output file path. The backend creates parent directories if needed and writes the file atomically enough for this read-only report flow.
- Do not export prompt, response, source code, or tool output text.

- [x] **Step 1: Define export request/result models**

Add `ExportReportRequest` and `ExportReportResult`.

- [x] **Step 2: Implement renderers**

Create `src/export.rs` with CSV escaping, JSON serialization, and Markdown table rendering for all supported views.

- [x] **Step 3: Add tests**

Cover CSV escaping, Markdown rendering, JSON output, path creation, and unknown view/format errors.

- [x] **Step 4: Add Tauri command**

Expose `export_report`.

- [x] **Step 5: Add desktop export page**

Provide a real export page with view, format, and output path inputs. Invoke `export_report` and show the written file path.

- [x] **Step 6: Verify**

Run: `cargo test export`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

## Task 9: Privacy Audit

**Files:**
- Modify: `src/model.rs`
- Create: `src/privacy.rs`
- Modify: `src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/OtherPages.tsx`

Production boundary for this slice:

- Generate a real audit report from the implemented Phase 1 capabilities, not static marketing text.
- Cover read-only source paths, version command behavior, SQLite tables/fields, export boundaries, pricing/project alias persistence, and explicit non-stored fields.
- State that prompt/response/source/tool-output bodies and key material are not persisted.

- [x] **Step 1: Define privacy audit models**

Add `PrivacyAuditReport`, `PrivacyAuditSource`, and `PrivacyAuditTable` structs.

- [x] **Step 2: Implement report generator**

Create `src/privacy.rs` with deterministic report generation from home/db paths and the known scanner/tool registry/export capabilities.

- [x] **Step 3: Add tests**

Cover scanner paths, stored tables, and forbidden fields.

- [x] **Step 4: Add Tauri command**

Expose `privacy_audit` and include it in the desktop snapshot.

- [x] **Step 5: Update About page**

Render the real audit report instead of only static privacy copy.

- [x] **Step 6: Verify**

Run: `cargo test privacy`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

- [x] Run `cargo test --workspace`.
- [x] Run `cd ui && yarn build`.
- [x] Run `git diff --check`.
- [x] Review `git diff` for accidental unrelated edits.

## Later Work Packages

- Phase 1 completed in this plan: tool registry, source health, pricing rules, project attribution, export, privacy-audit.
- Phase 2 remaining after Task 22: final AC audit and real Tauri desktop runtime verification; do not move to write/sync phases until the read-only Resource Center evidence is current.
- Desktop verification finding after Task 23: `cargo tauri build` fails because `beforeBuildCommand` points at `../ui`, which resolves outside the WAPC repo in this Tauri CLI invocation.
- Phase 3: Sync Engine, drift detection, backup/rollback UI, guide center.
- Phase 4: cross-tool sync, env strategy, scope rules, sync history and presets.
- Phase 5: template library, deep links, redacted reports, headless read-only mode, macOS signing/notarization, cross-platform feasibility.

## Task 10: Resource Inventory Foundation + MCP Detector

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/model.rs`
- Create: `src/resources.rs`
- Modify: `src/lib.rs`
- Modify: `src/store.rs`
- Modify: `src/privacy.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Create: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/src/components/layout/Sidebar.tsx`
- Modify: `ui/src/components/layout/Header.tsx`
- Modify: `ui/src/App.tsx`

Production boundary for this slice:

- Implement canonical resource envelope and MCP payload only. Other resource kinds remain explicit later work.
- Read Claude Code, Codex, Gemini CLI, and Cursor MCP files only. No writes.
- JSON MCP files use `serde_json`; Codex TOML uses a TOML parser dependency.
- Env values and suspicious args are never stored raw; store env key names plus fingerprints only.
- Parse failures are recorded as path/tool/kind/reason metadata only.

- [x] **Step 1: Define resource models**

Add `CanonicalResource`, `ResourceParseFailure`, `InventoryScanResult`, and MCP payload structs.

- [x] **Step 2: Implement redaction and MCP detectors**

Create `src/resources.rs` with JSON/TOML MCP parsing, env fingerprinting, stable resource ids, and parse failure collection.

- [x] **Step 3: Add store migration and queries**

Create `resources` and `resource_parse_failures`; implement resource upsert/list/get and parse failure insert/list.

- [x] **Step 4: Add Tauri commands and snapshot data**

Expose `inventory_scan`, `list_resources`, `get_resource`, and `list_parse_failures`.

- [x] **Step 5: Add read-only resource center page**

Render resource rows, filters/search, redaction/confidence indicators, and parse failure summary. No write/sync controls.

- [x] **Step 6: Verify**

Run: `cargo test resource`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

## Task 11: Skills + Instructions Read-Only Detectors

**Files:**
- Modify: `src/resources.rs`
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Add real read-only detectors for Skills and Instructions.
- Skills scope is user-level Claude Code skills under `~/.claude/skills/<name>/` for this slice.
- Instructions scope is user-level instruction files for Claude Code, Codex, Gemini CLI, and Cursor rules under known user config paths.
- Do not store instruction body text or Skill file content. Store file inventories, byte counts, content fingerprints, heading labels, and paragraph fingerprints only.
- Parse/read failures are recorded as metadata and must not interrupt other resources.

- [x] **Step 1: Add red tests for Skill and Instruction privacy boundaries**

Run: `cargo test resource`
Expected: FAIL before implementation.

- [x] **Step 2: Implement Skill detector**

Scan `~/.claude/skills/*/SKILL.md`; record skill name, manifest path, file inventory, aggregate content hash, and byte count without storing file contents.

- [x] **Step 3: Implement Instruction detector**

Scan user-level `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `.cursor/rules/*.mdc`, and `.cursorrules`; record dialect, headings, paragraph hashes, byte count, and content hash without storing body text.

- [x] **Step 4: Update privacy audit**

List new read sources and forbidden content fields.

- [x] **Step 5: Verify**

Run: `cargo test resource`
Expected: PASS.

Run: `cargo test privacy`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

## Task 12: Plugins + Subagents Read-Only Detectors

**Files:**
- Modify: `src/resources.rs`
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Add real read-only detectors for Claude Code user plugins and user subagents.
- Plugins scope is `~/.claude/plugins/<name>/`; store plugin name, optional manifest metadata, component counts, file inventory, and aggregate hashes only.
- Subagents scope is `~/.claude/agents/*.md`; parse Markdown frontmatter for name/model/allowed tools and store body structure fingerprints only.
- Do not store plugin file content or subagent body text.
- Parse/read failures are recorded as metadata and must not interrupt other resources.

- [x] **Step 1: Add red tests for Plugin and Subagent privacy boundaries**

Run: `cargo test resource`
Expected: FAIL before implementation.

- [x] **Step 2: Implement Plugin detector**

Scan `~/.claude/plugins/*`; record manifest metadata if present, component counts, file inventory, and content hash without file contents.

- [x] **Step 3: Implement Subagent detector**

Scan `~/.claude/agents/*.md`; parse frontmatter metadata and body fingerprints without storing body text.

- [x] **Step 4: Update privacy audit**

List plugin/subagent read sources and forbidden content fields.

- [x] **Step 5: Verify**

Run: `cargo test resource`
Expected: PASS.

Run: `cargo test privacy`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

## Task 13: Adapter Capabilities + Session Browser

**Files:**
- Modify: `src/model.rs`
- Create: `src/adapters.rs`
- Modify: `src/lib.rs`
- Modify: `src/store.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Create: `ui/src/pages/SessionsPage.tsx`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/src/components/layout/Sidebar.tsx`
- Modify: `ui/src/components/layout/Header.tsx`
- Modify: `ui/src/App.tsx`

Production boundary for this slice:

- `adapter_capabilities` must describe the current read-only detector capabilities, not future write/sync aspirations.
- `list_sessions` must aggregate from persisted `usage_records` metadata only.
- Session Browser must never return or render prompt, response, message body, source code, or tool output text.
- The UI must label the page as metadata-only and expose filters/search over safe metadata fields.

- [x] **Step 1: Add models and red tests**

Add `AdapterCapability` and `SessionMeta`; cover serialization and no-body-field boundaries.

- [x] **Step 2: Implement Store session query**

Aggregate `usage_records` by tool/session/project, returning first/last timestamp, record count, token total, cost, and source paths only.

- [x] **Step 3: Implement adapter capability declarations**

Add read-only capabilities for Claude Code, Codex, Gemini CLI, and Cursor based on implemented detectors.

- [x] **Step 4: Add Tauri commands and snapshot data**

Expose `adapter_capabilities` and `list_sessions`; include capabilities in `DesktopSnapshot`.

- [x] **Step 5: Add desktop UI**

Add Session Browser page and render adapter capabilities on Resource Center.

- [x] **Step 6: Verify**

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 14: Phase 2 Filter Contract Hardening

**Files:**
- Modify: `src/resources.rs`
- Modify: `src/store.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/src/pages/SessionsPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `inventory_scan` must support the PRD `{kinds?: string[]}` contract and only run selected detector families when provided.
- Resource Center filters must include kind, tool, scope, query, and redaction state without adding any write/sync controls.
- Session Browser must expose time filters while still returning and rendering metadata only.
- Existing persisted resources remain read-only; this task does not implement writes, sync, or content-body storage.

- [x] **Step 1: Add red tests**

Add a failing resource scanner test for kind-limited scans and a session browser test for time-window filtering.

- [x] **Step 2: Implement kind-limited inventory scanning**

Add `scan_inventory_with_kinds`; keep `scan_inventory` as the all-kind convenience path.

- [x] **Step 3: Wire Tauri command contract**

Change `inventory_scan` to accept optional `kinds` and pass it to the read-only scanner.

- [x] **Step 4: Harden desktop filters**

Add Resource Center scope/redaction filters and Session Browser date filters.

- [x] **Step 5: Verify**

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 15: Phase 2 Privacy Audit Field Boundaries

**Files:**
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Audit output must explain what Phase 2 resource payload metadata may be persisted.
- Audit output must explicitly forbid session prompt/response/message bodies.
- This task only changes audit reporting and tests; it does not change scanner persistence behavior.

- [x] **Step 1: Add red test**

Add a privacy audit test for resource payload field names and session-body forbidden fields.

- [x] **Step 2: Implement audit field boundaries**

Expand `resources` table field descriptions and forbidden fields.

- [x] **Step 3: Verify**

Run: `cargo test privacy`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 16: Project-Level Resource Discovery

**Files:**
- Modify: `src/resources.rs`
- Modify: `src/adapters.rs`
- Modify: `src/privacy.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Project scanning is read-only and limited to known project roots, not a home-directory crawl.
- Project roots come from existing project attribution or an explicit caller-provided list.
- Store only metadata, fingerprints, and redacted payloads for project-level resources.
- Supported project resources in this slice: `<project>/.mcp.json`, `<project>/.cursor/mcp.json`, `<project>/AGENTS.md`, `<project>/CLAUDE.md`, `<project>/GEMINI.md`, `<project>/.cursorrules`, `<project>/.cursor/rules/*.mdc`, `<project>/.claude/skills/*/SKILL.md`, and `<project>/.claude/agents/*.md`.

- [x] **Step 1: Add red tests**

Add resource scanner, Tauri project-root filtering, adapter capability, and privacy-audit tests for project-level support.

- [x] **Step 2: Implement scanner project roots**

Add `scan_inventory_with_project_roots`; keep existing user-level APIs as convenience wrappers.

- [x] **Step 3: Wire desktop scan inputs**

Use existing project summaries for snapshot scans and `inventory_scan`; accept optional explicit `project_paths`.

- [x] **Step 4: Update audit/capability surfaces**

Declare project scope in adapter capabilities and privacy-audit read sources.

- [x] **Step 5: Verify**

Run: `cargo test resources::tests::scans_project_level_resources_without_body_or_secret_values`
Expected: PASS.

Run: `cargo test -p wapc-app project_roots_from_summaries_keeps_existing_directories_only`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 17: Resource Payload Secret Guardrails

**Files:**
- Modify: `src/resources.rs`
- Modify: `src/store.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Extend scanner-side redaction for high-risk MCP args before `payload_json` is built.
- Add a store-side final gate before persisting `resources.payload_json`.
- The final gate must skip known fingerprint/hash fields but reject obvious plain secret strings.
- This task does not add write/sync behavior and does not persist any secret values.

- [x] **Step 1: Add red tests**

Add a resource scanner test for high-risk MCP args and a store test that rejects a leaky resource payload.

- [x] **Step 2: Extend scanner redaction**

Recognize common token prefixes and high-entropy argument strings.

- [x] **Step 3: Add store persistence guard**

Parse `payload_json` recursively before upsert and refuse known plain secret patterns.

- [x] **Step 4: Verify**

Run: `cargo test redacts_high_risk_mcp_args_before_payload_json`
Expected: PASS.

Run: `cargo test rejects_resource_payloads_with_plain_secret_values`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 18: Resource Inventory Fixture Audit

**Files:**
- Modify: `src/resources.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Add a deterministic audit helper for fixture/manual expectation checks.
- The fixture must cover all five Phase 2 resource kinds and both user/project scopes.
- The fixture must include a bad config file and prove parse failures do not block valid resources.
- Test fixture data is only for verification; no sample/demo resources are shipped through product runtime.

- [x] **Step 1: Add red test**

Create a complete inventory fixture and expected kind/scope/failure counts.

- [x] **Step 2: Implement audit helper**

Add `InventoryAuditExpectation`, `InventoryAuditReport`, and `audit_inventory_fixture`.

- [x] **Step 3: Verify**

Run: `cargo test audits_inventory_fixture_counts_against_manual_expectations`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 19: Resource Center Four-State Polish

**Files:**
- Add: `ui/src/pages/resourceCenterState.ts`
- Add: `ui/tests/resourceCenterState.test.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/package.json`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Resource Center must expose explicit loading, empty, error, and normal states from real inventory data.
- Parse failures must be summarized without hiding successfully parsed resources.
- Empty copy must describe all supported resource kinds instead of MCP-only discovery.
- No write, enable/disable, sync, injection, or sample resource workflows are introduced.

- [x] **Step 1: Add red test**

Add a dependency-free Node test for Resource Center state classification and summary counts.

- [x] **Step 2: Implement state helper and UI polish**

Extract pure state/summary helpers and wire ResourcesPage to a clearer state banner, parse failure summary, loading body, empty body, error body, and normal table.

- [x] **Step 3: Verify**

Run: `cd ui && yarn test`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 20: Adapter and Session Audit Wording Polish

**Files:**
- Modify: `src/privacy.rs`
- Modify: `src/adapters.rs`
- Modify: `ui/src/pages/SessionsPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Adapter capabilities must describe implemented Phase 2 read-only inventory only.
- Session Browser and privacy audit must explicitly state metadata-only behavior and forbid prompt/response/message body persistence.
- No write, sync, injection, enable/disable, or body-preview workflows are introduced.

- [x] **Step 1: Add red tests**

Add audit and adapter tests that require read-only and metadata-only wording.

- [x] **Step 2: Update audit/capability/UI wording**

Strengthen privacy-audit table fields, adapter notes, and Session Browser boundary copy.

- [x] **Step 3: Verify**

Run: `cargo test privacy_audit_names_adapter_and_session_metadata_boundaries`
Expected: PASS.

Run: `cargo test adapter_capability_notes_exclude_write_sync_promises`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn test`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 21: Plugin-Provided Resource Fixtures

**Files:**
- Modify: `src/resources.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Claude plugin directories may contain plugin-provided MCP and subagent resources.
- Plugin-provided resources must be normalized as canonical resources with `provided_by_plugin`.
- Plugin-provided MCP secrets and subagent bodies must obey the same redaction/fingerprint-only storage rules.
- This task remains read-only and does not add command/hook execution or sync/write behavior.

- [x] **Step 1: Add red test**

Add a fixture for `~/.claude/plugins/<plugin>/mcp/*.json` and `agents/*.md`, expecting plugin-provided canonical resources.

- [x] **Step 2: Implement plugin component detectors**

Extend plugin scanning to emit plugin-provided MCP/subagent resources with safe payloads and metadata-only persistence.

- [x] **Step 3: Verify**

Run: `cargo test scans_plugin_provided_resources_with_provider_relationship`
Expected: PASS.

Run: `cargo test audits_inventory_fixture_counts_against_manual_expectations`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `cd ui && yarn test`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 22: Resource Center Kind Grouping UI

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Resource Center must expose the FR-51 kind grouping as a first-class read-only navigation surface.
- Grouping must use the five canonical Phase 2 kinds in stable order and real resource counts.
- Selecting a group must only filter the local list; it must not add write/sync/enable controls or fake resources.

- [x] **Step 1: Add red test**

Add a frontend helper test for stable canonical kind groups and counts.

- [x] **Step 2: Implement helper and UI**

Render a responsive kind-group rail next to the resource list and keep it synchronized with the existing kind filter.

- [x] **Step 3: Verify**

Run: `cd ui && yarn test`
Expected: PASS.

Run: `cd ui && yarn build`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS.

Run: `git diff --check`
Expected: PASS.

## Task 23: Phase 2 Acceptance Audit and Desktop Verification

**Files:**
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Audit Phase 2 against the PRD acceptance criteria using current code, tests, commands, and build/runtime evidence.
- Treat tests as evidence only where they directly cover the requirement.
- Do not declare Phase 2 complete unless desktop runtime evidence is current and AC coverage is explicit.

- [x] **Step 1: AC evidence matrix**

| AC | Requirement | Current evidence | Status |
| --- | --- | --- | --- |
| AC-1 | Resource Center lists all 5 resource kinds and counts match a manual fixture. | `resources::tests::audits_inventory_fixture_counts_against_manual_expectations`; `ui/tests/resourceCenterState.test.ts` stable kind groups; `tests::get_snapshot_returns_desktop_bootstrap_data` verifies the desktop command bridge can load real local Resource Center bootstrap data. | Code/test evidence present; desktop command bridge verified. |
| AC-2 | Same MCP across tools merges into one resource with correct `enabled_in`. | `resources::tests::scans_json_and_toml_mcp_configs_with_redacted_env_values`. | Code/test evidence present. |
| AC-3 | MCP details show env key names + fingerprints and no raw values. | `resources::tests::scans_json_and_toml_mcp_configs_with_redacted_env_values`; `resources::tests::summarizes_mcp_command_paths_without_persisting_raw_command`; `store::tests::persists_resources_and_parse_failures_without_secret_payloads`; `store::tests::rejects_resource_payloads_with_plain_secret_values`; `tests::get_snapshot_returns_desktop_bootstrap_data`. | Code/test evidence present; live desktop bootstrap no longer trips the secret guard. |
| AC-4 | Instruction resources persist structure fingerprints only. | `resources::tests::scans_instruction_files_as_structure_fingerprints_only`; `privacy::tests::privacy_audit_names_phase_two_resource_and_session_field_boundaries`. | Code/test evidence present. |
| AC-5 | Low-confidence resources are visibly marked. | `ResourcesPage.tsx` renders low-confidence warning for selected resource with `confidence < 0.8`; scanner sets low confidence for incomplete MCP/no-heading instructions. | Static code evidence present; screenshot-level desktop visual proof still limited by macOS capture restrictions. |
| AC-6 | Session Browser returns/renders metadata only, no bodies. | `store::tests::session_browser_lists_metadata_without_content_fields`; `model::tests::session_meta_serializes_without_body_fields`; `SessionsPage.tsx` metadata-only copy. | Code/test evidence present. |
| AC-7 | Bad fixtures enter `resource_parse_failures` and do not block valid resources. | `resources::tests::records_parse_failure_without_stopping_inventory_scan`; `resources::tests::audits_inventory_fixture_counts_against_manual_expectations`. | Code/test evidence present. |
| AC-8 | `privacy-audit` covers Phase 2 reads and stored fields. | `privacy::tests::privacy_audit_names_phase_two_resource_and_session_field_boundaries`; `privacy::tests::privacy_audit_names_project_level_resource_scan_boundaries`; `privacy::tests::privacy_audit_names_adapter_and_session_metadata_boundaries`. | Code/test evidence present. |

- [x] **Step 2: Desktop build/runtime evidence**

Run a real Tauri desktop build or dev launch from `src-tauri`, record whether it starts with the React bundle and Tauri command bridge.

Evidence recorded on 2026-06-06:

- `cargo tauri build` from `src-tauri` succeeds and bundles `target/release/bundle/macos/WAPC.app`.
- `tests::get_snapshot_returns_desktop_bootstrap_data` passes against the real local home/database path, covering the Tauri command bridge path that previously caused a blank desktop window.
- CGWindow lists an onscreen `WAPC` window with bounds `1180x780`; macOS rejected subsequent window-level screenshots with `could not create image from window`, so visual screenshot proof remains environment-limited.
- Root cause fixed during runtime verification: Resource Center bootstrap failed when the store secret guard rejected safe metadata that looked high-entropy. MCP command paths are now summarized as metadata, plugin file paths are stored as hashes/extensions/depth, and env key/prefix metadata is treated as safe metadata rather than raw secret values.

- [x] **Step 3: Verify**

Run: `cargo test --workspace`
Result: PASS, 53 core tests + 5 Tauri helper tests + doc-tests.

Run: `cd ui && yarn test`
Result: PASS, 5 tests.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `git diff --check`
Result: PASS.

## Task 73: macOS Tool Path Metadata Verification

**Files:**
- Modify: `src/platform_paths.rs`
- Add: `docs/design/macos-path-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/README.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Verify only filesystem metadata for macOS candidate paths.
- Do not read config contents, session bodies, prompt/response text, source files, or secrets.
- Redact user home as `~` and project root as `<project>` in verification output.
- Do not claim content parsing, runtime MCP connection, OAuth/header behavior, Windows/Linux verification, or any new write support.

- [x] **Step 1: Add metadata-only path verification test**

Added `verifies_macos_candidates_from_filesystem_without_reading_contents` in `src/platform_paths.rs`. The test creates temp user/project candidates with file contents set to `SHOULD_NOT_BE_READ_SECRET`, then asserts the verification report contains only metadata and never includes that string.

Initial focused run exposed a privacy gap: project paths were not redacted to `<project>` in verification output.

Run: `cargo test -p wapc platform_paths::tests -- --nocapture`
Result: FAIL before project-root redaction, because project-scope paths did not satisfy the redacted-path assertion.

- [x] **Step 2: Implement verifier**

Added `verify_tool_path_candidates` and `ToolPathVerificationRecord`. The verifier:

- derives candidates from `tool_path_candidates`
- calls only `fs::metadata`
- emits tool/platform/scope/kind/path/candidate_verified/exists/is_file/is_dir/read_only/write_supported
- folds user home to `~`
- folds explicit project root to `<project>`

Run: `cargo test -p wapc platform_paths::tests -- --nocapture`
Result: PASS, 3 tests.

- [x] **Step 3: Record local macOS evidence**

Added `docs/design/macos-path-verification.md` with a current-machine metadata table for Claude Code, Codex, Gemini CLI, OpenCode, and Cursor user/project candidates. The evidence uses redacted `~` and `<project>` paths and explicitly states that no file contents were read.

Updated `docs/design/tool-adapter-matrix.md` to mark `配置文件真实路径(user / project)在 macOS 上核验` complete, with a narrow caveat that this only covers current macOS metadata evidence and the new no-content-read unit test.

Updated `docs/README.md` to link the macOS path verification evidence.

- [x] **Step 4: Verify full guardrails**

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 120 core tests + 24 Tauri helper tests + doc-tests.

Run: `git diff --check`
Result: PASS.

## Task 74: VS Code MCP Top-Level Field Official Verification

**Files:**
- Modify: `docs/design/mcp-field-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Use only official VS Code documentation for current MCP field shape.
- Resolve the `servers` vs `mcpServers` documentation question only.
- Do not claim VS Code user profile path, runtime read behavior, OAuth/header flow, or write support is complete.

- [x] **Step 1: Check official source**

Source: `https://code.visualstudio.com/docs/agents/reference/mcp-configuration`

Finding:

- VS Code stores MCP configuration in `mcp.json`.
- Workspace file is `.vscode/mcp.json`; user profile is also supported by VS Code, but WAPC has not done local user-profile path discovery yet.
- Top-level sections are `servers`, optional `inputs`, and optional `sandbox`.
- stdio servers use `type: "stdio"` with `command` / `args` / `env`.
- HTTP/SSE servers use `type: "http"` or `"sse"` with `url`, optional `headers`, and optional `oauth`.

- [x] **Step 2: Update docs**

Updated `docs/design/mcp-field-verification.md` to reference the VS Code MCP configuration reference directly instead of using the OpenAI Docs MCP example as the VS Code source.

Updated `docs/design/tool-adapter-matrix.md` to state VS Code top-level `servers` is officially verified and to replace the old pending item with the real remaining gap: VS Code user profile `mcp.json` true path and runtime read behavior.

- [x] **Step 3: Verify doc guardrails**

Run: `rg -n "servers.*mcpServers|mcp-field-verification|VS Code Copilot|user profile|code.visualstudio.com/docs/agents/reference/mcp-configuration" docs/design/tool-adapter-matrix.md docs/design/mcp-field-verification.md docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
Result: PASS. Current matrix and MCP field verification doc point at VS Code official reference; historical Task 72 remains as execution history and Task 74 records the correction.

Run: `git diff --check`
Result: PASS.

## Task 75: OpenCode Instructions and Skills Candidate Verification

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/resources.rs`
- Modify: `src/privacy.rs`
- Add: `docs/design/opencode-resource-verification.md`
- Modify: `docs/design/macos-path-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/README.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Use official OpenCode docs for rules and skills mechanism.
- Add read-only PathResolver candidates for OpenCode native instruction and skill paths.
- Treat OpenCode `AGENTS.md` as an `agents-md` instruction dialect for read-only resource metadata.
- Do not scan OpenCode skill bodies yet.
- Do not enable OpenCode skill install/sync/write, symlink/copy, permission management, or rollback.

- [x] **Step 1: Add PathResolver test**

Extended `resolves_cross_platform_tool_path_candidates_without_touching_filesystem` to require macOS OpenCode candidates:

- user `~/.config/opencode/AGENTS.md`
- user `~/.config/opencode/skills`
- project `<project>/AGENTS.md`
- project `<project>/.opencode/skills`

Run: `cargo test -p wapc platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: FAIL before implementation because the OpenCode instruction candidate was missing.

- [x] **Step 2: Implement candidates and read-only instruction dialect**

Updated `tool_path_candidates` so OpenCode emits:

- user `InstructionFile` at `opencode_config/AGENTS.md`
- user `SkillDir` at `opencode_config/skills`
- project `ProjectInstructionFile` at `AGENTS.md`
- project `ProjectSkillDir` at `.opencode/skills`

Updated `instruction_dialect` so OpenCode `AGENTS.md` is treated as `agents-md`. Updated privacy source naming for OpenCode user instructions.

Run: `cargo test -p wapc platform_paths::tests -- --nocapture`
Result: PASS, 3 tests.

Run: `cargo test -p wapc resources::tests -- --nocapture`
Result: PASS, 23 tests.

Run: `cargo test -p wapc privacy::tests::privacy_audit_uses_path_resolver_for_user_resource_path_sources -- --nocapture`
Result: PASS.

- [x] **Step 3: Update docs and evidence**

Added `docs/design/opencode-resource-verification.md`, based on official OpenCode docs:

- `https://dev.opencode.ai/docs/rules`
- `https://dev.opencode.ai/docs/skills`

Updated `docs/design/tool-adapter-matrix.md` to replace the old "OpenCode instruction/skills mechanism pending" item with the true remaining gap: Resource Center skill scanning, permission strategy, template install, and write rollback.

Updated `docs/design/macos-path-verification.md` with current local metadata for the new OpenCode candidates:

- `~/.config/opencode/AGENTS.md`: missing
- `~/.config/opencode/skills`: dir
- `<project>/AGENTS.md`: missing
- `<project>/.opencode/skills`: missing

- [x] **Step 4: Verify full guardrails**

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 120 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "OpenCode|opencode-resource-verification|~/.config/opencode/AGENTS.md|.opencode/skills|skills 机制是否存在" docs/design/tool-adapter-matrix.md docs/design/opencode-resource-verification.md docs/design/macos-path-verification.md docs/README.md`
Result: PASS. The old "skills mechanism existence" pending item is gone from the current matrix; the remaining OpenCode gap is skill scanning/install/write behavior.

Run: `git diff --check`
Result: PASS.

## Task 76: VS Code Workspace MCP Read-Only Inventory

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/resources.rs`
- Add: `tests/fixtures/resource_inventory/redacted-home/work/redacted-repo/.vscode/mcp.json`
- Modify: `docs/design/mcp-field-verification.md`
- Modify: `docs/design/macos-path-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement only workspace/project scope VS Code MCP read-only inventory.
- Use official VS Code MCP reference for `.vscode/mcp.json` and top-level `servers`.
- Do not infer or hardcode VS Code user profile `mcp.json` path.
- Do not enable VS Code MCP write/sync target support.
- Do not claim OAuth/header runtime connection behavior is verified.

- [x] **Step 1: Add RED tests**

Added a PathResolver assertion requiring project `.vscode/mcp.json` as a read-only `ProjectMcpConfig` candidate.

Added `scans_project_vscode_mcp_servers_without_secret_values`, which creates a project `.vscode/mcp.json` with top-level `servers`, HTTP transport, and an `Authorization` header. The test requires:

- `origin_tool = vscode`
- `origin_locator = servers.context7`
- project scope
- redacted payload
- no raw `Authorization` value persisted

Run: `cargo test -p wapc platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: FAIL before implementation because the `vscode` project candidate was missing.

Run: `cargo test -p wapc resources::tests::scans_project_vscode_mcp_servers_without_secret_values -- --nocapture`
Result: FAIL before implementation because scanner had no VS Code project source.

- [x] **Step 2: Implement workspace MCP scanner**

Updated `tool_path_candidates` to include project `.vscode/mcp.json` for tool `vscode`.

Updated `mcp_source_config` so `vscode` uses JSON root key `servers`.

Run: `cargo test -p wapc platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resources::tests::scans_project_vscode_mcp_servers_without_secret_values -- --nocapture`
Result: PASS.

- [x] **Step 3: Add checked-in redacted fixture and docs**

Added checked-in fixture `tests/fixtures/resource_inventory/redacted-home/work/redacted-repo/.vscode/mcp.json` with safe `${input:...}` variable usage and no real secret values. Updated checked-in fixture audit expectations to count the new project MCP resource.

Updated docs:

- `docs/design/mcp-field-verification.md`: VS Code workspace `.vscode/mcp.json` is now read-only scanned; user profile path and runtime behavior remain unsupported.
- `docs/design/tool-adapter-matrix.md`: VS Code workspace MCP read-only inventory is supported; user profile and writes remain pending.
- `docs/design/macos-path-verification.md`: current project `.vscode/mcp.json` metadata is missing but candidate-covered.

- [x] **Step 4: Verify full guardrails**

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: FAIL once on `cloned_ref_to_slice_refs`, then PASS after replacing `&[project.clone()]` with `std::slice::from_ref(&project)`.

Run: `cargo test --workspace`
Result: PASS, 121 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "vscode|\\.vscode/mcp\\.json|servers\\.context7|VS Code workspace" src/platform_paths.rs src/resources.rs docs/design/mcp-field-verification.md docs/design/tool-adapter-matrix.md docs/design/macos-path-verification.md tests/fixtures/resource_inventory/redacted-home/work/redacted-repo/.vscode/mcp.json`
Result: PASS.

Run: `git diff --check`
Result: PASS.

## Task 77: VS Code Copilot Instructions Read-Only Inventory

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/resources.rs`
- Add: `tests/fixtures/resource_inventory/redacted-home/work/redacted-repo/.github/copilot-instructions.md`
- Add: `docs/design/vscode-copilot-resource-verification.md`
- Modify: `docs/README.md`
- Modify: `docs/design/macos-path-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement only workspace/project scope `.github/copilot-instructions.md` read-only inventory.
- Use official VS Code custom instructions documentation for the path and behavior.
- Store only instruction structure fingerprints, not body text.
- Do not implement `.instructions.md`, user profile instructions, organization instructions, AGENTS/CLAUDE fallback runtime priority, or any instruction write/sync target.

- [x] **Step 1: Add RED tests**

Official source: `https://code.visualstudio.com/docs/copilot/customization/custom-instructions`

Finding: VS Code automatically detects `.github/copilot-instructions.md` in the workspace root and applies it as workspace-wide custom instructions.

Added a PathResolver assertion requiring project `.github/copilot-instructions.md` as a read-only `ProjectInstructionFile` candidate for tool `vscode`.

Added `scans_project_vscode_copilot_instructions_without_body_text`, which creates a project `.github/copilot-instructions.md` and requires:

- `origin_tool = vscode`
- `kind = instruction`
- project scope
- `enabled_in = ["vscode"]`
- title/structure fingerprint persisted
- no raw body text persisted

Run: `cargo test -p wapc platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: FAIL before implementation because the `vscode` project instruction candidate was missing.

Run: `cargo test -p wapc resources::tests::scans_project_vscode_copilot_instructions_without_body_text -- --nocapture`
Result: FAIL before implementation because scanner had no VS Code Copilot instruction source.

- [x] **Step 2: Implement read-only instruction scanner**

Updated `tool_path_candidates` to include project `.github/copilot-instructions.md` for tool `vscode`.

Updated `instruction_dialect` so `vscode` maps to `copilot`.

Run: `cargo test -p wapc platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resources::tests::scans_project_vscode_copilot_instructions_without_body_text -- --nocapture`
Result: PASS.

- [x] **Step 3: Add checked-in redacted fixture and docs**

Added checked-in fixture `tests/fixtures/resource_inventory/redacted-home/work/redacted-repo/.github/copilot-instructions.md` with safe non-secret text. Updated checked-in fixture audit expectations to count the new project instruction resource.

Added `docs/design/vscode-copilot-resource-verification.md`, covering official VS Code workspace MCP and Copilot instructions references plus current unsupported boundaries.

Updated docs:

- `docs/README.md`: link new VS Code Copilot resource verification doc.
- `docs/design/tool-adapter-matrix.md`: mark Copilot instructions as project/workspace read-only scanned, with `.instructions.md`, user profile, organization instructions, fallback priority, and writes still pending.
- `docs/design/macos-path-verification.md`: current project `.github/copilot-instructions.md` metadata is missing but candidate-covered.

- [x] **Step 4: Verify full guardrails**

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 122 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "copilot-instructions|vscode-copilot-resource-verification|origin_tool == \"vscode\"|ProjectInstructionFile|copilot" src/platform_paths.rs src/resources.rs docs/README.md docs/design/tool-adapter-matrix.md docs/design/vscode-copilot-resource-verification.md docs/design/macos-path-verification.md tests/fixtures/resource_inventory/redacted-home/work/redacted-repo/.github/copilot-instructions.md`
Result: PASS.

Run: `git diff --check`
Result: PASS.

## Task 78: VS Code Adapter Capability Declaration

**Files:**
- Modify: `src/adapters.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Declare only the VS Code Copilot read-only capabilities that are already implemented: project `.vscode/mcp.json` and project `.github/copilot-instructions.md`.
- Do not declare user profile support, runtime OAuth/header validation, sync target support, or write support.
- Capability notes must describe metadata/redacted/fingerprint persistence boundaries.

- [x] **Step 1: Add RED capability test**

Updated `declares_current_phase_two_read_only_capabilities` to require a fifth adapter capability:

- `tool = vscode`
- `resource_kinds = ["mcp", "instruction"]`
- `scopes = ["project"]`
- notes mention `.vscode/mcp.json` and `.github/copilot-instructions.md`

Run: `cargo test -p wapc adapters::tests::declares_current_phase_two_read_only_capabilities -- --nocapture`
Result: FAIL before implementation because only 4 capabilities were declared.

- [x] **Step 2: Implement capability**

Added `VS Code Copilot` capability in `src/adapters.rs` with:

- project-scope MCP and instruction support
- stdio/http/sse transport metadata for MCP
- read-only notes
- explicit unsupported boundary for user profile paths, OAuth/header runtime behavior, and writes

Run: `cargo test -p wapc adapters::tests -- --nocapture`
Result: FAIL once because the new notes did not explicitly mention metadata/redacted/fingerprint persistence boundary; PASS after updating the note to say it stores redacted MCP metadata and instruction fingerprints.

- [x] **Step 3: Verify full guardrails**

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 122 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "vscode|VS Code Copilot|adapter_capabilities|\\.vscode/mcp\\.json|copilot-instructions|read-only" src/adapters.rs docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
Result: PASS.

Run: `git diff --check`
Result: PASS.

## Task 79: OpenCode Skill Read-Only Inventory

**Files:**
- Modify: `src/resources.rs`
- Modify: `src/adapters.rs`
- Modify: `docs/design/opencode-resource-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Add: `tests/fixtures/resource_inventory/redacted-home/.config/opencode/skills/release-check/SKILL.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement only read-only OpenCode native skill scanning for `~/.config/opencode/skills/<name>/SKILL.md` and `<project>/.opencode/skills/<name>/SKILL.md`.
- Persist only metadata, file hashes, frontmatter keys, and description fingerprints; do not store skill body text or description text.
- Do not implement OpenCode MCP parsing, skill install/sync/write, symlink/copy permission strategy, or rollback.
- Keep `.agents/skills` and Claude-compatible fallback documented but unsupported by this scanner slice.

- [x] **Step 1: Add RED OpenCode skill inventory tests**

Added `scans_opencode_skills_from_user_and_project_roots_without_body_text`, covering user and project OpenCode native skill roots.

Run: `cargo test -p wapc resources::tests::scans_opencode_skills_from_user_and_project_roots_without_body_text -- --nocapture`
Result: FAIL before implementation because the scanner only read Claude skill roots.

Updated the checked-in redacted fixture audit to require an OpenCode skill resource.

Run: `cargo test -p wapc resources::tests::audits_checked_in_redacted_inventory_fixture -- --nocapture`
Result: FAIL before fixture addition with expected skill/user count mismatches.

- [x] **Step 2: Implement read-only OpenCode skill scanning**

Updated `read_skill_resources` so PathResolver-provided OpenCode user/project skill roots are scanned with the existing safe skill file inventory path.

Updated skill payloads to include:

- `frontmatter_keys`
- `frontmatter_metadata.schema`
- OpenCode `declared_name`, `license`, `compatibility`, `metadata_present`
- `description_fingerprint` instead of description text

Added checked-in redacted OpenCode skill fixture under `.config/opencode/skills/release-check/SKILL.md`.

Run: `cargo test -p wapc resources::tests::scans_opencode_skills_from_user_and_project_roots_without_body_text -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resources::tests::audits_checked_in_redacted_inventory_fixture -- --nocapture`
Result: PASS.

- [x] **Step 3: Declare adapter capability**

Updated `declares_current_phase_two_read_only_capabilities` to require an `opencode` capability:

- `resource_kinds = ["instruction", "skill"]`
- `scopes = ["user", "project"]`
- notes mention `~/.config/opencode/AGENTS.md` and `.opencode/skills`

Run: `cargo test -p wapc adapters::tests::declares_current_phase_two_read_only_capabilities -- --nocapture`
Result: FAIL before implementation because only 5 capabilities were declared.

Added `OpenCode` adapter capability with explicit read-only metadata/fingerprint notes and unsupported boundary for OpenCode MCP parsing, skill install/sync/write, permission strategy, and rollback.

Run: `cargo test -p wapc adapters::tests -- --nocapture`
Result: PASS.

- [x] **Step 4: Verify full guardrails**

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 123 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "OpenCode|opencode-skill-frontmatter-v1|release-check|\\.opencode/skills|unsupported" src/resources.rs src/adapters.rs docs/design/opencode-resource-verification.md docs/design/tool-adapter-matrix.md docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md tests/fixtures/resource_inventory/redacted-home/.config/opencode/skills/release-check/SKILL.md`
Result: PASS.

Run: `git diff --check`
Result: PASS after plan-status update.

## Task 80: OpenCode MCP Read-Only Inventory

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/resources.rs`
- Modify: `src/adapters.rs`
- Add: `tests/fixtures/resource_inventory/redacted-home/.config/opencode/opencode.json`
- Modify: `docs/design/opencode-resource-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/design/mcp-field-verification.md`
- Modify: `docs/design/macos-path-verification.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement only read-only OpenCode MCP metadata scanning for user/project `opencode.json` top-level `mcp`.
- Normalize OpenCode `type: "local"` to canonical `stdio` and `type: "remote"` to canonical `http`.
- Persist only redacted MCP metadata; `environment` and headers keep keys/fingerprints only.
- Do not implement OpenCode MCP runtime connection, auth/OAuth state inspection, `mcp-auth.json` reading, CLI auth/debug/list integration, or writes/sync.

- [x] **Step 1: Add RED PathResolver and scanner tests**

Extended `resolves_cross_platform_tool_path_candidates_without_touching_filesystem` to require:

- user `~/.config/opencode/opencode.json` as `McpConfig`
- project `<project>/opencode.json` as `ProjectMcpConfig`

Run: `cargo test -p wapc platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: FAIL before implementation because the OpenCode MCP candidates were missing.

Added `scans_opencode_mcp_from_user_and_project_config_without_secret_values`, covering:

- user local MCP with `command` array and `environment`
- project remote MCP with `url`, `headers`, and `oauth: false`
- redaction of command args, environment values, and header values

Run: `cargo test -p wapc resources::tests::scans_opencode_mcp_from_user_and_project_config_without_secret_values -- --nocapture`
Result: FAIL before implementation because no OpenCode MCP source was scanned.

- [x] **Step 2: Implement read-only OpenCode MCP scan**

Updated `tool_path_candidates` so OpenCode emits:

- user `McpConfig` at `opencode_config/opencode.json`
- project `ProjectMcpConfig` at `<project>/opencode.json`

Updated Resource Inventory so:

- `opencode` maps to JSON root key `mcp`
- OpenCode `command` arrays split into command metadata + redacted args
- `environment` is handled like env and only stores keys/fingerprints
- `type: "local"` maps to canonical `stdio`
- `type: "remote"` maps to canonical `http`

Run: `cargo test -p wapc platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resources::tests::scans_opencode_mcp_from_user_and_project_config_without_secret_values -- --nocapture`
Result: PASS.

- [x] **Step 3: Add checked-in fixture and capability/docs updates**

Updated checked-in redacted fixture audit to require an OpenCode MCP resource.

Run: `cargo test -p wapc resources::tests::audits_checked_in_redacted_inventory_fixture -- --nocapture`
Result: FAIL before fixture addition with expected MCP/user count mismatches; PASS after adding `.config/opencode/opencode.json`.

Updated adapter capability so OpenCode declares `mcp`, `instruction`, and `skill` read-only resources, with explicit unsupported boundary for runtime auth/OAuth state and writes.

Updated docs:

- `docs/design/opencode-resource-verification.md`: OpenCode MCP official fields and current scanner boundary.
- `docs/design/tool-adapter-matrix.md`: OpenCode `mcp` mapping and remaining unsupported items.
- `docs/design/mcp-field-verification.md`: official OpenCode MCP field/transport row.
- `docs/design/macos-path-verification.md`: current metadata for `~/.config/opencode/opencode.json` and `<project>/opencode.json`.

- [x] **Step 4: Verify full guardrails**

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 124 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "OpenCode|opencode.json|opencode.*mcp|mcp-auth|type: \"local\"|type: \"remote\"|unsupported" src/platform_paths.rs src/resources.rs src/adapters.rs docs/design/opencode-resource-verification.md docs/design/tool-adapter-matrix.md docs/design/mcp-field-verification.md docs/design/macos-path-verification.md docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md tests/fixtures/resource_inventory/redacted-home/.config/opencode/opencode.json`
Result: PASS.

Run: `git diff --check`
Result: PASS after plan-status update.

## Task 81: OpenCode Disabled MCP Enabled-In Semantics

**Files:**
- Modify: `src/resources.rs`
- Modify: `tests/fixtures/resource_inventory/redacted-home/.config/opencode/opencode.json`
- Modify: `docs/design/opencode-resource-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/design/mcp-field-verification.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Honor OpenCode `enabled:false` as a real disabled configuration state.
- Still inventory disabled MCP configs so users can see and audit them.
- Do not count disabled OpenCode MCP configs in `enabled_in`.
- Persist only non-secret `enabled` metadata; do not read OpenCode runtime auth/OAuth status or credentials.

- [x] **Step 1: Add RED disabled-state test**

Added `scans_opencode_disabled_mcp_without_marking_it_enabled`, which writes a user `opencode.json` MCP entry with `enabled:false` and expects:

- resource is still inventoried
- `enabled_in` is empty
- payload contains `"enabled":false`

Run: `cargo test -p wapc resources::tests::scans_opencode_disabled_mcp_without_marking_it_enabled -- --nocapture`
Result: FAIL before implementation because disabled OpenCode MCP was still marked `enabled_in=["opencode"]`.

- [x] **Step 2: Implement enabled metadata**

Updated `McpPayload` to include optional `enabled`.

Updated canonical MCP payload JSON to persist `enabled`.

Updated `mcp_resource_with_provider` so `enabled:false` yields an empty `enabled_in` list while still returning the resource.

Run: `cargo test -p wapc resources::tests::scans_opencode_disabled_mcp_without_marking_it_enabled -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resources::tests::scans_opencode_mcp_from_user_and_project_config_without_secret_values -- --nocapture`
Result: PASS.

- [x] **Step 3: Extend fixture and docs**

Added `disabled-docs` to the checked-in OpenCode fixture with `enabled:false`.

Updated checked-in fixture audit expectations and assertions:

- MCP count increased by one
- disabled OpenCode MCP is present
- disabled OpenCode MCP has empty `enabled_in`
- payload contains `"enabled":false`

Run: `cargo test -p wapc resources::tests::audits_checked_in_redacted_inventory_fixture -- --nocapture`
Result: PASS.

Updated docs:

- `docs/design/opencode-resource-verification.md`
- `docs/design/tool-adapter-matrix.md`
- `docs/design/mcp-field-verification.md`

- [x] **Step 4: Verify full guardrails**

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 125 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "enabled:false|enabled_in|disabled-docs|OpenCode|mcp-auth|unsupported" src/resources.rs docs/design/opencode-resource-verification.md docs/design/tool-adapter-matrix.md docs/design/mcp-field-verification.md docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md tests/fixtures/resource_inventory/redacted-home/.config/opencode/opencode.json`
Result: PASS.

Run: `git diff --check`
Result: PASS after plan-status update.

## Task 82: Resource Center MCP Disable Dialect Gating

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/design/opencode-resource-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Resource Center management actions must expose only write paths already backed by Sync Engine tests.
- `disable_mcp` is currently limited to user-scope Claude/Cursor JSON MCP entries under `mcpServers`.
- OpenCode `opencode.json` top-level `mcp`, VS Code `.vscode/mcp.json` top-level `servers`, Codex TOML sync preview, runtime auth/OAuth, plugin-owned resources, and enterprise resources remain visibly read-only for this management action.
- Read-only inventory support must not imply write support.

- [x] **Step 1: Add RED UI capability test**

Added `keeps read-only scanned mcp dialects out of disable actions` coverage for:

- OpenCode user `opencode.json` with `origin_locator=mcp.docs`
- VS Code workspace `.vscode/mcp.json` with `origin_locator=servers.docs`

Expected both to return disabled Resource Management capability with reason `当前仅支持 Claude/Cursor JSON MCP 禁用`.

Run: `yarn --cwd ui test`
Result: FAIL before implementation because OpenCode JSON MCP still produced an enabled `disable_mcp` action.

- [x] **Step 2: Gate Resource Center management by supported JSON MCP dialect**

Updated `getResourceManagementCapability` so user JSON MCP resources are writable only when:

- `origin_tool` is `claude` or `cursor`
- `origin_locator` starts with `mcpServers.`

Also corrected the positive UI contract fixture to use Claude `mcpServers` instead of a mismatched Codex tool name pointing at `.claude.json`.

Run: `yarn --cwd ui test`
Result: PASS, 24 UI state tests.

- [x] **Step 3: Verify full guardrails**

Run: `yarn --cwd ui test`
Result: PASS, 24 UI state tests.

Run: `yarn --cwd ui lint`
Result: FAIL on existing repository-wide lint debt outside this slice, including `ThemeProvider.tsx`, `TrendChart.tsx`, `hooks.ts`, `AnalyticsPage.tsx`, `ExportPage.tsx`, `PricingPage.tsx`, `ResourcesPage.tsx`, `SessionsPage.tsx`, and `TokensPage.tsx`.

Run: `yarn --cwd ui eslint src/pages/resourceCenterState.ts tests/resourceCenterState.test.ts`
Result: PASS for files changed in this slice.

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 125 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "Claude/Cursor JSON MCP 禁用|opencode|vscode|disable_mcp|mcpServers|servers|unsupported" ui/src/pages/resourceCenterState.ts ui/tests/resourceCenterState.test.ts docs/design/tool-adapter-matrix.md docs/design/opencode-resource-verification.md docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
Result: PASS.

Run: `git diff --check`
Result: PASS after plan-status update.

## Task 83: UI Lint Gate Production Hardening

**Files:**
- Add: `ui/src/components/themeContext.ts`
- Add: `ui/src/components/useTheme.ts`
- Modify: `ui/src/components/ThemeProvider.tsx`
- Modify: `ui/src/components/layout/Sidebar.tsx`
- Modify: `ui/src/components/charts/TrendChart.tsx`
- Modify: `ui/src/hooks/hooks.ts`
- Modify: `ui/src/pages/AnalyticsPage.tsx`
- Modify: `ui/src/pages/ExportPage.tsx`
- Modify: `ui/src/pages/PricingPage.tsx`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/src/pages/SessionsPage.tsx`
- Modify: `ui/src/pages/TokensPage.tsx`
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Treat `yarn --cwd ui lint` as a real release gate, not a best-effort warning.
- Fix lint root causes without disabling rules or replacing real flows with placeholders.
- Preserve existing Resource Center, report export, pricing, session, chart, and token table behavior.
- Keep unsupported/write boundaries unchanged; this task only hardens the UI code gate.

- [x] **Step 1: Reproduce RED lint gate**

Run: `yarn --cwd ui lint`
Result: FAIL before implementation with 25 errors and 2 warnings:

- `react-refresh/only-export-components` in `ThemeProvider.tsx`
- `@typescript-eslint/no-explicit-any` in chart tooltip/legend callbacks
- `react-hooks/set-state-in-effect` in snapshot, pricing, export, resource, and session flows
- `react-hooks/static-components` in `TokensPage.tsx`
- `react-hooks/exhaustive-deps` warning in `ResourcesPage.tsx`

- [x] **Step 2: Fix lint root causes**

Implemented:

- split theme context and `useTheme` hook out of the component-only `ThemeProvider.tsx`
- changed initial async load effects to async microtasks or event-time state updates
- typed Recharts callback values with `unknown` plus explicit conversion helpers
- moved `TokensPage` sort indicator out of render-time component creation
- made Resource Center template/sync reset effects dependency-complete and async-scheduled
- treated missing `origin_locator` as read-only for Resource Center MCP disable gating

Run: `yarn --cwd ui lint`
Result: PASS, no warnings.

- [x] **Step 3: Verify build and full guardrails**

Run: `yarn --cwd ui test`
Result: PASS, 24 UI state tests.

Run: `yarn --cwd ui build`
Result: PASS, with existing Vite chunk-size warning.

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 125 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "useTheme|ThemeProviderContext|no-explicit-any|queueMicrotask|SortIndicator|origin_locator" ui/src docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
Result: PASS.

Run: `git diff --check`
Result: PASS after plan-status update.

## Task 84: CI and Release UI Lint Gate Enforcement

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `docs/release/macos-signing-notarization.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Promote the now-green UI lint command from local hygiene to required CI/release evidence.
- Do not allow PR, push, or release gates to pass if `yarn --cwd ui lint` fails.
- Keep release signing/notarization boundaries unchanged; this task only adds a stricter pre-build quality gate.

- [x] **Step 1: Add RED workflow contracts**

Extended existing workflow contract tests to require `yarn --cwd ui lint` in:

- cross-platform core smoke CI job
- macOS desktop CI job
- release workflow gates

Run: `cargo test -p wapc-app workflow -- --nocapture`
Result: FAIL before implementation. The three workflow tests failed because `.github/workflows/ci.yml` and `.github/workflows/release.yml` did not contain `yarn --cwd ui lint`.

- [x] **Step 2: Add lint gates to CI and release**

Updated `.github/workflows/ci.yml`:

- cross-platform smoke now runs `yarn --cwd ui lint` before UI tests/build
- macOS desktop gate now runs `yarn --cwd ui lint` before UI tests/build

Updated `.github/workflows/release.yml`:

- release gates now run `yarn --cwd ui lint` between `cargo test --workspace` and UI test/build

Updated release docs to list the lint command as part of release gate evidence.

Run: `cargo test -p wapc-app workflow -- --nocapture`
Result: PASS, 3 workflow contract tests.

- [x] **Step 3: Verify full guardrails**

Run: `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); YAML.load_file(".github/workflows/release.yml"); puts "workflow yaml ok"'`
Result: PASS, with Ruby warning about world-writable `/opt/homebrew/bin` in PATH.

Run: `yarn --cwd ui lint`
Result: PASS.

Run: `yarn --cwd ui test`
Result: PASS, 24 UI state tests.

Run: `yarn --cwd ui build`
Result: PASS, with existing Vite chunk-size warning.

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 125 core tests + 24 Tauri helper tests + doc-tests.

Run: `rg -n "yarn --cwd ui lint|Run release gates|Lint UI|workflow yaml ok" .github/workflows/ci.yml .github/workflows/release.yml src-tauri/src/lib.rs docs/release/macos-signing-notarization.md docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
Result: PASS.

Run: `git diff --check`
Result: PASS after plan-status update.

## Task 85: Cross-Platform Smoke Documentation Scope Alignment

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/design/cross-platform-feasibility.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Documentation must match the actual GitHub Actions scope.
- Do not overclaim full non-macOS Tauri GUI bundle or full workspace support.
- Cross-platform smoke evidence currently covers non-Tauri-GUI Rust core plus UI lint/test/build on `ubuntu-latest` and `windows-latest`.
- Windows/Linux real tool-path fixtures, runtime behavior, Tauri GUI bundle, and write flows remain unsupported or pending verification.

- [x] **Step 1: Add RED documentation contract**

Added `cross_platform_docs_match_core_smoke_ci_scope` to require both `docs/design/tool-adapter-matrix.md` and `docs/design/cross-platform-feasibility.md` to document:

- `cross-platform core smoke CI`
- `cargo clippy --workspace --exclude wapc-app --all-targets -- -D warnings`
- `cargo test --workspace --exclude wapc-app`
- `yarn --cwd ui lint`
- `yarn --cwd ui test`
- `yarn --cwd ui build`
- `不构建 Tauri GUI bundle`

The test also rejects the stale Go/No-Go wording that implied full `cargo test --workspace` on non-macOS.

Run: `cargo test -p wapc-app cross_platform_docs_match_core_smoke_ci_scope -- --nocapture`
Result: FAIL before documentation update because the tool adapter matrix did not document `cross-platform core smoke CI`.

- [x] **Step 2: Align docs with current CI reality**

Updated `docs/design/tool-adapter-matrix.md`:

- marked the cross-platform core smoke CI checklist item as complete
- described the actual commands run on `ubuntu-latest` / `windows-latest`
- kept Tauri GUI bundle, real platform fixtures, and write support explicitly unfinished

Updated `docs/design/cross-platform-feasibility.md`:

- replaced broad full-workspace wording with the exact Rust core + UI lint/test/build smoke scope
- clarified Windows/Linux do not build the Tauri GUI bundle

Run: `cargo test -p wapc-app cross_platform_docs_match_core_smoke_ci_scope -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify full guardrails**

Run: `cargo test -p wapc-app workflow -- --nocapture`
Result: PASS, 3 workflow contract tests.

Run: `cargo test -p wapc-app cross_platform_docs_match_core_smoke_ci_scope -- --nocapture`
Result: PASS.

Run: `yarn --cwd ui lint`
Result: PASS.

Run: `yarn --cwd ui test`
Result: PASS, 24 UI state tests.

Run: `yarn --cwd ui build`
Result: PASS, with existing Vite chunk-size warning.

Run: `cargo fmt --check`
Result: FAIL before `cargo fmt`, then PASS after formatting the new Rust contract test.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 125 core tests + 25 Tauri helper tests + doc-tests.

Run: `rg -n 'cross-platform core smoke CI|cargo test --workspace --exclude wapc-app|yarn --cwd ui lint|不构建 Tauri GUI bundle|cargo test --workspace\` 在 \`ubuntu-latest' docs/design/tool-adapter-matrix.md docs/design/cross-platform-feasibility.md src-tauri/src/lib.rs docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
Result: PASS; the stale full-workspace phrase appears only inside the negative Rust assertion, not as a docs claim.

Run: `git diff --check`
Result: PASS after plan-status update.

## Task 86: README Source Build Verification Gate

**Files:**
- Modify: `README.md`
- Modify: `src-tauri/src/lib.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- README source-build instructions must not imply that a raw package build is enough for local verification.
- Local contributors should see the same quality gates used by CI/release before building the desktop app.
- This task does not claim Apple notarization or Gatekeeper acceptance is complete; those still require real Apple Developer credentials and clean-machine validation.

- [x] **Step 1: Add RED README contract**

Extended `readme_documents_release_gate_without_pretending_notarization_is_done` to require the README source-build section to include:

- `本地验收`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `yarn --cwd ui lint`
- `yarn --cwd ui test`
- `yarn --cwd ui build`
- `cargo tauri build --manifest-path src-tauri/Cargo.toml`

Run: `cargo test -p wapc-app readme_documents_release_gate_without_pretending_notarization_is_done -- --nocapture`
Result: FAIL before README update because the source-build section did not contain `本地验收`.

- [x] **Step 2: Update README source-build instructions**

Updated `README.md` so source builds now show:

- install UI dependencies
- run local verification gates matching CI/release
- then build the desktop app

Run: `cargo test -p wapc-app readme_documents_release_gate_without_pretending_notarization_is_done -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify full guardrails**

Run:

- `cargo test -p wapc-app readme_documents_release_gate_without_pretending_notarization_is_done -- --nocapture`
- `yarn --cwd ui lint`
- `yarn --cwd ui test`
- `yarn --cwd ui build`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `rg -n "本地验收|cargo fmt --check|cargo clippy --workspace --all-targets -- -D warnings|cargo test --workspace|yarn --cwd ui lint|yarn --cwd ui test|yarn --cwd ui build|cargo tauri build --manifest-path src-tauri/Cargo.toml|Gatekeeper" README.md src-tauri/src/lib.rs docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
- `git diff --check`

Result: PASS.

Notes:

- `yarn --cwd ui build` still reports the existing Vite chunk-size warning after minification; build exits successfully.
- README now requires local verification gates before `cargo tauri build --manifest-path src-tauri/Cargo.toml`.
- README still avoids claiming Apple notarization or clean-machine Gatekeeper validation is complete.

## Task 87: Phase 4 Apply Sync Forged Plan Boundary

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync` must execute only plans tied to a real persisted canonical resource or a registered resource template fingerprint.
- Forged plans without `resource_id`, unknown source resources, kind/name mismatches, enterprise/managed sources, and plugin-provided sources must fail per target before any file write.
- Preserve batch independence: a rejected forged target records a failed target result without stopping other targets.
- Preserve template preview compatibility by accepting `template:<id>:<fingerprint>` only when the template exists and the fingerprint matches.

- [x] **Step 1: Add RED forged-plan test**

Added `apply_sync_rejects_forged_plan_without_source_resource_id`, which generates a real plan through `plan_sync`, removes its `resource_id`, then asserts `apply_sync` rejects the target and leaves the target config unchanged.

Run: `cargo test -p wapc apply_sync_rejects_forged_plan_without_source_resource_id -- --nocapture`
Result: FAIL before implementation because the forged plan was committed.

- [x] **Step 2: Validate apply source before materialization/write**

Added per-target validation before env placeholder materialization and Sync Engine execution:

- ordinary resources must exist in SQLite and match plan kind/name
- enterprise/managed and plugin-provided resources remain read-only
- template plans must reference an existing template id with matching content fingerprint
- only `mcp` `sync` plans are accepted by `apply_sync`

Run: `cargo test -p wapc apply_sync_rejects_forged_plan_without_source_resource_id -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

Notes:

- `cargo test -p wapc cross_sync -- --nocapture` passed with 16 cross-sync tests, including forged-plan rejection and existing JSON/TOML/env sync paths.
- `cargo test --workspace` passed with 126 core tests and 25 Tauri helper tests.

## Task 88: Phase 4 Apply Sync Single-Source History Integrity

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- A single `apply_sync` operation must represent one source resource only, because `sync_operations.source_resource_id` is singular.
- Mixing plans generated from different source resources must fail before any target write, backup, resource change, or sync operation is recorded.
- Preserve supported behavior for one source syncing to multiple targets.
- Preserve Task 87 behavior: plans without `resource_id` still fail per target through forged-plan validation instead of being converted into a fake source.

- [x] **Step 1: Add RED mixed-source audit test**

Added `apply_sync_rejects_mixed_source_resources_before_writing_or_history`, which generates two real plans from two real resources, combines them into one apply request, and asserts:

- `apply_sync` rejects the request
- both target config files remain unchanged
- no `sync_operations` row is recorded
- no `resource_changes` row is recorded

Run: `cargo test -p wapc apply_sync_rejects_mixed_source_resources_before_writing_or_history -- --nocapture`
Result: FAIL before implementation because both targets were committed under one ambiguous sync operation.

- [x] **Step 2: Reject mixed source ids before sync history/write**

Added request-level validation at the start of `apply_sync` that allows at most one non-empty `resource_id` across all plans.

Run: `cargo test -p wapc apply_sync_rejects_mixed_source_resources_before_writing_or_history -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

Notes:

- `cargo test -p wapc cross_sync -- --nocapture` passed with 17 cross-sync tests.
- `cargo test --workspace` passed with 127 core tests and 25 Tauri helper tests.

## Task 89: Phase 5 Template Apply Source Name Integrity

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Template-derived apply plans must still match the registered template canonical resource name, not only the template id and fingerprint.
- A forged template plan with a mismatched `resource_name` must fail before backup/write/verify.
- Preserve existing per-target failure history semantics: rejected targets may record a failed `resource_changes` row, but no backup path or backup row should be created.
- Preserve normal template install preview compatibility.

- [x] **Step 1: Add RED template name mismatch test**

Added `apply_sync_rejects_template_plan_name_mismatch_before_backup_or_write`, which:

- seeds the real built-in Context7 MCP template
- generates a real template install preview through `plan_template_sync`
- mutates the resulting plan `resource_name`
- applies the forged plan
- asserts the target file is unchanged, the failed change has no backup path, and no `resource_backups` row exists

Run: `cargo test -p wapc apply_sync_rejects_template_plan_name_mismatch_before_backup_or_write -- --nocapture`
Result: FAIL before implementation because the failure occurred later in Sync Engine verification instead of template source-name validation.

- [x] **Step 2: Validate template canonical name before write**

Updated template plan validation so `apply_sync` reconstructs the canonical resource from the stored template and requires its `name` to match the plan `resource_name`.

Run: `cargo test -p wapc apply_sync_rejects_template_plan_name_mismatch_before_backup_or_write -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo test -p wapc template_library -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

Notes:

- `cargo test -p wapc cross_sync -- --nocapture` passed with 18 cross-sync tests.
- `cargo test -p wapc template_library -- --nocapture` passed with 2 template-library tests.
- `cargo test --workspace` passed with 128 core tests and 25 Tauri helper tests.

## Task 90: Phase 4 Apply Sync Preview Mutation Boundary

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- A valid source `resource_id` must not be enough to authorize arbitrary `preview_after` content.
- `apply_sync` plans may only add or replace the selected MCP resource entry named by `resource_name`.
- A forged plan that injects unrelated MCP entries must fail before backup/write/verify.
- Preserve legitimate JSON MCP, Codex TOML MCP, env placeholder, template, and multi-target same-source sync behavior.

- [x] **Step 1: Add RED unrelated MCP injection test**

Added `apply_sync_rejects_plan_that_injects_unrelated_mcp_entry`, which:

- generates a real JSON MCP sync plan through `plan_sync`
- mutates `preview_after` to add an unrelated `evil` MCP entry
- recomputes `after_fingerprint` to simulate a forged but internally consistent plan
- asserts `apply_sync` rejects the target and leaves the target file unchanged

Run: `cargo test -p wapc apply_sync_rejects_plan_that_injects_unrelated_mcp_entry -- --nocapture`
Result: FAIL before implementation because the forged plan committed.

- [x] **Step 2: Validate preview scope before write**

Added apply-time preview mutation validation for JSON `mcpServers` and TOML `mcp_servers` targets:

- parse `preview_before`
- clone it
- replace only the selected `resource_name` MCP entry with the entry from `preview_after`
- compare canonical expected content to `preview_after`
- reject any extra add/remove/modify outside the selected MCP resource

Run: `cargo test -p wapc apply_sync_rejects_plan_that_injects_unrelated_mcp_entry -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

Notes:

- `cargo test -p wapc cross_sync -- --nocapture` passed with 19 cross-sync tests.
- `cargo test --workspace` passed with 129 core tests and 25 Tauri helper tests.

## Task 91: Phase 4 Apply Sync Selected MCP Payload Integrity

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- A valid source id and selected resource name must not be enough to authorize a mutated selected MCP entry.
- `apply_sync` must verify the selected MCP entry still matches the source payload URL or command/args.
- The validation must cover JSON `mcpServers` and TOML `mcp_servers`.
- Env placeholders and env strategy materialization remain memory-only and must not weaken URL/command/args integrity.

- [x] **Step 1: Add RED selected endpoint mutation test**

Added `apply_sync_rejects_plan_that_mutates_selected_mcp_endpoint`, which:

- generates a real JSON MCP sync plan through `plan_sync`
- mutates only the selected `docs` MCP entry URL to an unrelated endpoint
- recomputes `after_fingerprint` to simulate a forged but internally consistent plan
- asserts `apply_sync` rejects the target and leaves the target file unchanged

Run: `cargo test -p wapc apply_sync_rejects_plan_that_mutates_selected_mcp_endpoint -- --nocapture`
Result: FAIL before implementation because the forged selected MCP endpoint committed.

- [x] **Step 2: Validate selected entry against source payload**

Added apply-time validation that parses the persisted resource/template payload and compares:

- URL MCP entries: target `url` must match source `url`
- command MCP entries: target `command` and `args` must match source `command` and `args`
- JSON and TOML targets are both covered

Run: `cargo test -p wapc apply_sync_rejects_plan_that_mutates_selected_mcp_endpoint -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

Notes:

- `cargo test -p wapc cross_sync -- --nocapture` passed with 20 cross-sync tests.
- `cargo test --workspace` passed with 130 core tests and 25 Tauri helper tests.

## Task 92: Phase 4 Apply Sync Selected MCP Transport Integrity

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- URL-based MCP sync plans must preserve the source payload transport/type, not only the URL.
- A forged plan that keeps the same URL but mutates JSON/TOML `type` must fail before backup/write/verify.
- Existing JSON MCP, Codex TOML MCP, command MCP, env placeholder, template, and multi-target same-source sync behavior must remain supported.

- [x] **Step 1: Add RED selected transport mutation test**

Added `apply_sync_rejects_plan_that_mutates_selected_mcp_transport`, which:

- generates a real JSON MCP sync plan through `plan_sync`
- mutates only the selected `docs` MCP entry `type` from source `http` to `sse`
- recomputes `after_fingerprint`
- asserts `apply_sync` rejects the target and leaves the target file unchanged

Run: `cargo test -p wapc apply_sync_rejects_plan_that_mutates_selected_mcp_transport -- --nocapture`
Result: FAIL before implementation because the forged transport committed.

- [x] **Step 2: Validate URL transport/type against source payload**

Updated selected entry validation so URL-based MCP entries must match the source payload `transport` (defaulting to `http`) against target `type` for both JSON and TOML targets.

Run: `cargo test -p wapc apply_sync_rejects_plan_that_mutates_selected_mcp_transport -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

Notes:

- `cargo test -p wapc cross_sync -- --nocapture` passed with 21 cross-sync tests.
- `cargo test --workspace` passed with 131 core tests and 25 Tauri helper tests.

## Task 93: Phase 4 Apply Sync Env Preview Secret Boundary

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync` preview plans must not carry raw env secret values.
- Selected MCP `env` keys must exactly match the source payload `env_keys`.
- Before materialization, env values may only be empty skip values or WAPC manual/reuse placeholders for the same key.
- Manual env values remain supported through `ApplySyncRequest.env_values` and are still memory-only until written to the target config after confirmation.

- [x] **Step 1: Add RED raw env preview test**

Added `apply_sync_rejects_plan_with_raw_env_value_in_preview`, which:

- generates a real manual-env JSON MCP sync plan through `plan_sync`
- mutates `<WAPC_MANUAL_ENV:GITHUB_TOKEN>` to a raw token-like value
- recomputes `after_fingerprint`
- asserts `apply_sync` rejects the target, leaves the target file unchanged, and creates no backup

Run: `cargo test -p wapc apply_sync_rejects_plan_with_raw_env_value_in_preview -- --nocapture`
Result: FAIL before implementation because the raw env preview value committed.

- [x] **Step 2: Validate env preview keys and placeholders**

Added JSON/TOML selected entry env validation:

- source `env_keys` become the exact allowed env key set
- no extra or missing env keys are accepted
- values must be `""`, `<WAPC_MANUAL_ENV:key>`, or `<WAPC_REUSE_ENV:key>` before materialization

Run: `cargo test -p wapc apply_sync_rejects_plan_with_raw_env_value_in_preview -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

Notes:

- `cargo test -p wapc cross_sync -- --nocapture` passed with 22 cross-sync tests.
- `cargo test --workspace` passed with 132 core tests and 25 Tauri helper tests.

## Task 94: Phase 4 Apply Sync Selected MCP Field Whitelist

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- A forged selected MCP entry must not be able to add unsupported fields such as `headers`, `Authorization`, OAuth/runtime flags, or other target-specific behavior.
- URL MCP entries may contain only `url`, `type`, and optionally `env` when the source declares env keys.
- Command MCP entries may contain only `command`, `args`, and optionally `env` when the source declares env keys.
- Keep current supported JSON/TOML URL, command, env, template, and multi-target same-source sync behavior.

- [x] **Step 1: Add RED selected extra-field test**

Added `apply_sync_rejects_plan_with_unrelated_selected_mcp_field`, which:

- generates a real JSON URL MCP sync plan through `plan_sync`
- mutates the selected `docs` entry to add `headers.Authorization`
- recomputes `after_fingerprint`
- asserts `apply_sync` rejects the target, leaves the target file unchanged, and creates no backup

Run: `cargo test -p wapc apply_sync_rejects_plan_with_unrelated_selected_mcp_field -- --nocapture`
Result: FAIL before implementation because the forged headers field committed.

- [x] **Step 2: Validate selected entry field whitelist**

Added JSON/TOML selected entry field validation:

- URL source payloads allow only `url`, `type`, and optional `env`
- command source payloads allow only `command`, `args`, and optional `env`
- `env` is allowed only when source `env_keys` is non-empty

Run: `cargo test -p wapc apply_sync_rejects_plan_with_unrelated_selected_mcp_field -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

Notes:

- `cargo test -p wapc cross_sync -- --nocapture` passed with 23 cross-sync tests.
- `cargo test --workspace` passed with 133 core tests and 25 Tauri helper tests.

## Task 95: Phase 4 Apply Sync WritePlan Fingerprint Self-Consistency

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync` must reject internally inconsistent `WritePlan` fingerprints before backup/write/verify.
- `preview_before` must hash to `before_fingerprint`; `preview_after` must hash to `after_fingerprint`.
- A forged fingerprint mismatch should still produce a failed target result for audit, but no backup path or backup row.
- Preserve legitimate env materialization: placeholder previews validate before materialization, then memory-only env values recompute the final `after_fingerprint` before Sync Engine write.

- [x] **Step 1: Add RED forged fingerprint test**

Added `apply_sync_rejects_plan_with_inconsistent_after_fingerprint_before_backup`, which:

- generates a real JSON MCP sync plan through `plan_sync`
- mutates only `after_fingerprint`
- asserts `apply_sync` rejects the target, leaves the target file unchanged, and records a failed change with no backup path

Run: `cargo test -p wapc apply_sync_rejects_plan_with_inconsistent_after_fingerprint_before_backup -- --nocapture`
Result: FAIL before implementation because the mismatch failed later during Sync Engine verification rather than apply-time fingerprint validation.

- [x] **Step 2: Validate WritePlan fingerprint self-consistency**

Added apply-time fingerprint validation:

- `sha256(preview_before) == before_fingerprint`
- `sha256(preview_after) == after_fingerprint`

Run: `cargo test -p wapc apply_sync_rejects_plan_with_inconsistent_after_fingerprint_before_backup -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 24 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 134 core tests and 25 Tauri helper tests.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 95 files.

## Task 96: Phase 4 Apply Sync Target Fingerprint Binding

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync` must only write target files that were observed during `plan_sync`.
- A plan retargeted to another file with identical content must be rejected before backup/write/verify.
- The rejection should still produce a failed target result for audit, with no backup path or backup row.
- Legitimate env materialization and template plans must continue to work because their target fingerprints are recorded by the same planning flow.

- [x] **Step 1: Add RED retargeted plan test**

Added `apply_sync_rejects_plan_retargeted_to_unplanned_file_before_backup`, which:

- generates a real JSON MCP sync plan through `plan_sync`
- mutates only `target_path` to another unplanned JSON file with identical preimage content
- asserts `apply_sync` rejects the target, leaves both files unchanged, and records a failed change with no backup path

Run: `cargo test -p wapc apply_sync_rejects_plan_retargeted_to_unplanned_file_before_backup -- --nocapture`
Result: FAIL before implementation because the retargeted plan was incorrectly committed.

- [x] **Step 2: Bind apply-time target to planned file fingerprint**

Added apply-time target fingerprint validation:

- the target `(tool, target_path)` must have a fingerprint recorded during planning
- the recorded fingerprint must match the plan `before_fingerprint`

Run: `cargo test -p wapc apply_sync_rejects_plan_retargeted_to_unplanned_file_before_backup -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 25 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 135 core tests and 25 Tauri helper tests.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 96 files.

## Task 97: Phase 4 Apply Sync Plan ID Self-Consistency

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync` must reject a plan whose `plan_id` no longer matches the planned source, target path, before/after fingerprints, and creation time.
- The plan id check must not mask more specific payload/env/field safety failures.
- Rejection must happen before backup/write/verify and still record a failed target result for audit.
- Legitimate template, env materialization, cross-scope authorized, JSON, and TOML plans must continue to apply through the existing planning flow.

- [x] **Step 1: Add RED forged plan_id test**

Added `apply_sync_rejects_plan_with_forged_plan_id_before_backup`, which:

- generates a real JSON MCP sync plan through `plan_sync`
- mutates only `plan_id`
- asserts `apply_sync` rejects the target, leaves the target file unchanged, and records a failed change with no backup path

Run: `cargo test -p wapc apply_sync_rejects_plan_with_forged_plan_id_before_backup -- --nocapture`
Result: FAIL before implementation because the forged `plan_id` was incorrectly committed.

- [x] **Step 2: Validate plan_id self-consistency after semantic safety checks**

Added apply-time `plan_id` validation:

- recompute the expected id from `resource_id`, `tool`, `target_path`, `before_fingerprint`, `after_fingerprint`, and `created_at`
- reject mismatches before backup/write
- run this after source payload / env / selected MCP safety checks so existing specific failure reasons stay intact

Run: `cargo test -p wapc apply_sync_rejects_plan_with_forged_plan_id_before_backup -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 26 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 136 core tests and 25 Tauri helper tests.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 97 files.

## Task 98: Phase 4 Apply Sync Diff Self-Consistency

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync` must reject a plan whose `diff` no longer matches `preview_before` and `preview_after`.
- A forged diff must not let the UI/user-confirmed preview describe a different change from the bytes that will be written.
- Rejection must happen before backup/write/verify and still record a failed target result for audit.
- Legitimate env materialization must continue to recompute the final diff after in-memory secret substitution.

- [x] **Step 1: Add RED forged diff test**

Added `apply_sync_rejects_plan_with_forged_diff_before_backup`, which:

- generates a real JSON MCP sync plan through `plan_sync`
- mutates only `diff`
- asserts `apply_sync` rejects the target, leaves the target file unchanged, and records a failed change with no backup path

Run: `cargo test -p wapc apply_sync_rejects_plan_with_forged_diff_before_backup -- --nocapture`
Result: FAIL before implementation because the forged `diff` plan was incorrectly committed.

- [x] **Step 2: Validate diff self-consistency before writing**

Added apply-time `diff` validation:

- recompute `line_diff(preview_before, preview_after)`
- reject mismatches before backup/write
- keep env materialization behavior intact because `materialize_env_placeholders` recomputes the final diff after in-memory substitution

Run: `cargo test -p wapc apply_sync_rejects_plan_with_forged_diff_before_backup -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 27 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 137 core tests and 25 Tauri helper tests.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 98 files.

## Task 99: Phase 4 Apply Sync Confirmation Metadata Self-Consistency

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync` must reject a plan whose `requires_backup` no longer matches the Phase 4 cross-tool write policy.
- `apply_sync` must reject a plan whose `risks` metadata no longer matches the high-risk cross-tool config write warning generated by the backend.
- A forged plan must not let the confirmation UI describe a high-risk external config write as low-risk, no-risk, or no-backup.
- Rejection must happen before backup/write/verify and still record a failed target result for audit.

- [x] **Step 1: Add RED forged confirmation metadata tests**

Added:

- `apply_sync_rejects_plan_with_forged_backup_requirement_before_backup`
- `apply_sync_rejects_plan_with_forged_risk_metadata_before_backup`

These tests generate a real JSON MCP sync plan through `plan_sync`, mutate only the confirmation metadata, and assert `apply_sync` rejects the target with no file write or backup row.

Run: `cargo test -p wapc apply_sync_rejects_plan_with_forged_ -- --nocapture`
Result: FAIL before implementation because forged `requires_backup` and forged `risks` plans were incorrectly committed.

- [x] **Step 2: Validate confirmation metadata before writing**

Added apply-time confirmation metadata validation:

- `requires_backup` must remain `true`
- `risks` must match the backend-generated `cross_tool_config_write` high-risk warning
- JSON and TOML planning now reuse the same `cross_tool_write_risks()` helper to avoid drift

Run: `cargo test -p wapc apply_sync_rejects_plan_with_forged_ -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 29 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 139 core tests and 25 Tauri helper tests.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 99 files.

## Task 100: Phase 4 Apply Sync Manual Env Values Key Boundary

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync.env_values` must only contain keys required by `<WAPC_MANUAL_ENV:key>` placeholders in the submitted plans.
- Unexpected `env_values` keys must be rejected before sync history, failed change history, backup, or target writes.
- Legitimate manual env materialization must continue to use memory-only values and must not persist raw secrets into sync operation metadata or change logs.
- Reuse and skip strategies must remain unaffected because they do not require manual env values.

- [x] **Step 1: Add RED unexpected env_values test**

Added `apply_sync_rejects_unexpected_manual_env_values_before_history_or_write`, which:

- generates a real manual-env JSON MCP sync plan through `plan_sync`
- submits the required `GITHUB_TOKEN` plus an unexpected `UNUSED_TOKEN`
- asserts `apply_sync` rejects the request before file write, sync operation history, change history, or backup rows

Run: `cargo test -p wapc apply_sync_rejects_unexpected_manual_env_values_before_history_or_write -- --nocapture`
Result: FAIL before implementation because the extra env value was silently ignored and the target was incorrectly committed.

- [x] **Step 2: Validate env_values against manual placeholders before history**

Added request-level `env_values` validation:

- collect allowed keys from `<WAPC_MANUAL_ENV:key>` placeholders in all submitted plan previews
- reject any `env_values` key outside that allowed set before creating `sync_operations`
- leave missing required manual values to the existing materialization error path

Run: `cargo test -p wapc apply_sync_rejects_unexpected_manual_env_values_before_history_or_write -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 30 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 140 core tests and 25 Tauri helper tests after rerunning a transient `database is locked` failure in `get_snapshot_returns_desktop_bootstrap_data`.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 100 files.

## Task 101: Phase 4 Apply Sync Env Strategy Placeholder Binding

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync.env_strategy` must match the placeholder strategy used by submitted plans.
- `<WAPC_MANUAL_ENV:key>` placeholders require `env_strategy=manual`.
- `<WAPC_REUSE_ENV:key>` placeholders require `env_strategy=reuse`.
- Strategy mismatches must be rejected before sync history, failed change history, backup, or target writes, so the audit label cannot disagree with the actual secret materialization path.

- [x] **Step 1: Add RED forged strategy/placeholder mismatch test**

Added `apply_sync_rejects_env_strategy_placeholder_mismatch_before_history_or_write`, which:

- generates a real `reuse` JSON MCP sync plan with an existing target env value
- mutates `<WAPC_REUSE_ENV:GITHUB_TOKEN>` to `<WAPC_MANUAL_ENV:GITHUB_TOKEN>`
- recomputes fingerprint, diff, and plan id so the forged plan is internally self-consistent
- submits `env_strategy=reuse` plus a manual env value
- asserts `apply_sync` rejects before file write, sync operation history, change history, or backup rows

Run: `cargo test -p wapc apply_sync_rejects_env_strategy_placeholder_mismatch_before_history_or_write -- --nocapture`
Result: FAIL before implementation because the forged plan wrote `manual-secret` while the operation was labeled `reuse`.

- [x] **Step 2: Validate env_strategy against placeholders before history**

Added request-level env strategy validation:

- collect manual and reuse placeholder keys from submitted plan previews
- reject manual placeholders unless the request strategy is `manual`
- reject reuse placeholders unless the request strategy is `reuse`
- leave skip/no-env plans unaffected

Run: `cargo test -p wapc apply_sync_rejects_env_strategy_placeholder_mismatch_before_history_or_write -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 31 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 141 core tests and 25 Tauri helper tests.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 101 files.

## Task 102: Phase 4 Apply Sync Non-Empty Plan Request

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync` must reject requests with no submitted plans.
- Empty plan requests must not create `sync_operations`, failed change history, backups, or a fake successful sync id.
- Legitimate one-target and multi-target apply requests must remain unchanged.

- [x] **Step 1: Add RED empty plans test**

Added `apply_sync_rejects_empty_plan_request_before_history`, which:

- submits an `ApplySyncRequest` with `plans: []`
- asserts `apply_sync` rejects the request
- asserts no sync operation, change, or backup rows are created

Run: `cargo test -p wapc apply_sync_rejects_empty_plan_request_before_history -- --nocapture`
Result: FAIL before implementation because an empty request returned `Ok(ApplySyncResult { changes: [] })` and generated a fake sync id.

- [x] **Step 2: Reject empty apply requests before history**

Added `enforce_non_empty_apply_request` at the start of `apply_sync`, before source, env, history, backup, or write handling.

Run: `cargo test -p wapc apply_sync_rejects_empty_plan_request_before_history -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 32 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 142 core tests and 25 Tauri helper tests.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 102 files.

## Task 103: Phase 4 Apply Sync Env Strategy Value Whitelist

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `apply_sync` must reject unsupported `env_strategy` values before history, backup, or file write handling.
- Apply-time sync audit metadata must only persist known strategy labels: `none`, `reuse`, `manual`, or `skip`.
- Existing `reuse`, `manual`, `skip`, empty, and absent strategy flows must remain compatible with previously generated plans.

- [x] **Step 1: Add RED unknown env strategy test**

Added `apply_sync_rejects_unknown_env_strategy_before_history_or_write`, which:

- creates a real JSON MCP sync plan
- submits it to `apply_sync` with `env_strategy: "side-channel"`
- asserts the request is rejected
- asserts the target file is unchanged
- asserts no sync operation, change, or backup rows are created

Run: `cargo test -p wapc apply_sync_rejects_unknown_env_strategy_before_history_or_write -- --nocapture`
Result: FAIL before implementation because the invalid strategy returned `Ok`, committed the file write, and persisted sync history.

- [x] **Step 2: Validate apply env strategy values before history**

Added a shared apply-time strategy label helper and `validate_apply_env_strategy_value`.

Allowed values:

- `none`
- `reuse`
- `manual`
- `skip`

Run: `cargo test -p wapc apply_sync_rejects_unknown_env_strategy_before_history_or_write -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 33 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 143 core tests and 25 Tauri helper tests.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 103 files.

## Task 104: Phase 4 Apply Sync Skip Env Audit Label

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- A skip-env plan must not write secret values or require manual env input.
- If apply uses a skip-env plan without an explicit apply-time `env_strategy`, `sync_operations.env_strategy` must still record `skip`, not `none`.
- Ordinary no-env sync plans must remain `none` when the apply request has no env strategy.
- Existing `reuse` and `manual` env flows must remain unchanged.

- [x] **Step 1: Add RED skip audit assertion**

Extended `env_strategy_apply_sync_skip_env_writes_empty_placeholder` to assert the persisted sync operation uses `env_strategy = "skip"` when the plan writes an empty env placeholder.

Run: `cargo test -p wapc env_strategy_apply_sync_skip_env_writes_empty_placeholder -- --nocapture`
Result: FAIL before implementation because the target file wrote the empty env value correctly, but `sync_operations.env_strategy` persisted `none`.

- [x] **Step 2: Infer skip strategy from plan preview when apply strategy is absent**

Added apply-time strategy normalization that:

- keeps explicit `reuse`, `manual`, `skip`, and `none` labels valid
- keeps ordinary no-env plans as `none`
- detects JSON/TOML MCP previews where the selected resource env table contains empty string values and records the operation as `skip`

Run: `cargo test -p wapc env_strategy_apply_sync_skip_env_writes_empty_placeholder -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 33 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 143 core tests and 25 Tauri helper tests.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 104 files.

## Task 105: Phase 4 Sync Operation Target Scope Metadata

**Files:**
- Modify: `src/model.rs`
- Modify: `src/sync_engine.rs`
- Modify: `src/cross_sync.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 4 `sync_operations.targets_json` must preserve target metadata required by the PRD: `tool`, `scope`, and `project_path` where available.
- Project target sync history must remain explainable after the write, without re-inferring project scope from target file paths.
- Phase 3 single-tool `WritePlan` compatibility must remain intact, because those plans do not originate from a `SyncTarget`.
- Existing serialized plans must remain accepted by using optional/default target metadata fields.

- [x] **Step 1: Add RED target metadata audit assertion**

Extended `plan_sync_accepts_project_target_with_explicit_project_path_and_apply_writes_it` to parse `sync_operations.targets_json` after apply and assert:

- `scope == "project"`
- `project_path == <explicit project path>`

Run: `cargo test -p wapc plan_sync_accepts_project_target_with_explicit_project_path_and_apply_writes_it -- --nocapture`
Result: FAIL after fixing the test type because `targets_json.scope` was `null`.

- [x] **Step 2: Persist target scope metadata through WritePlan and apply history**

Added optional/default fields to `WritePlan`:

- `target_scope`
- `target_project_path`

Set them from `SyncTarget` during Phase 4 JSON/TOML cross-tool planning, kept them `None` for Phase 3 single-tool plans, and included them in `sync_operations.targets_json`.

Mirrored the optional fields in `ui/src/types/index.ts`.

Run: `cargo test -p wapc plan_sync_accepts_project_target_with_explicit_project_path_and_apply_writes_it -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn build`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 33 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 143 core tests and 25 Tauri helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 105 files.

## Task 106: Phase 4 Sync Plan Target Metadata Integrity

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Target scope metadata persisted in `sync_operations.targets_json` must be bound to the generated plan, not trusted as mutable client-side audit data.
- A plan with forged `target_scope` or `target_project_path` must fail before backup/write while preserving normal cross-tool project sync.
- Existing serialized plans without target metadata must remain accepted through the legacy plan id path.
- Plan id helper code must remain lint-clean without suppressing `clippy::too_many_arguments`.

- [x] **Step 1: Add RED forged target metadata test**

Added `apply_sync_rejects_plan_with_forged_target_scope_metadata_before_history_write`, which:

- generates a real project-target cross-tool sync plan
- mutates `target_scope` from `project` to `user`
- clears `target_project_path`
- asserts apply returns a failed target, leaves the target file unchanged, and creates no backup

Run: `cargo test -p wapc apply_sync_rejects_plan_with_forged_target_scope_metadata_before_history_write -- --nocapture`
Result: FAIL before implementation because the forged metadata plan was committed.

- [x] **Step 2: Bind target metadata into new cross-tool plan ids**

Updated cross-tool `sync_plan_id` generation and self-consistency validation to include:

- `target_scope`
- `target_project_path`

Kept a legacy plan id path for plans where both metadata fields are absent.

Refactored the plan id inputs into `SyncPlanIdParts` after `clippy` reported the helper had too many arguments.

Run: `cargo test -p wapc apply_sync_rejects_plan_with_forged_target_scope_metadata_before_history_write -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn build`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 34 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` initially failed on `clippy::too_many_arguments`, then passed after refactoring to `SyncPlanIdParts`.
- `cargo test --workspace` passed with 144 core tests and 25 Tauri helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 106 files.

## Task 107: Phase 4 Apply Sync Target Metadata Semantic Validation

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- A self-consistent recomputed `plan_id` must not be enough to forge target audit metadata.
- If `target_scope = project`, `target_project_path` must be present and the target file must be under that project path.
- `target_project_path` must not appear for non-project target metadata.
- Enterprise/managed target metadata remains read-only at apply time.
- Legacy plans with no target metadata remain accepted.

- [x] **Step 1: Add RED self-consistent false project_path test**

Added `apply_sync_rejects_plan_with_self_consistent_but_false_project_path_metadata`, which:

- generates a real project-target cross-tool sync plan
- changes `target_project_path` to a different directory
- recomputes `plan_id` using the modified metadata
- asserts apply returns a failed target, leaves the target file unchanged, and creates no backup

Run: `cargo test -p wapc apply_sync_rejects_plan_with_self_consistent_but_false_project_path_metadata -- --nocapture`
Result: FAIL before implementation because the forged metadata plan was committed.

- [x] **Step 2: Validate target metadata semantics at apply time**

Added `validate_plan_target_metadata`, called before source payload validation and write execution:

- no target metadata: allowed for legacy plans unless `target_project_path` is present
- `project`: requires `target_project_path` and target path under it
- `enterprise` / `managed`: rejected
- non-project scopes: reject unexpected `target_project_path`

Run: `cargo test -p wapc apply_sync_rejects_plan_with_self_consistent_but_false_project_path_metadata -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn build`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 35 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 145 core tests and 25 Tauri helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 107 files.

## Task 108: Phase 4 Apply Sync Cross-Scope Authorization Recheck

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Cross-scope authorization must be enforced at apply time, not only at plan time.
- A user -> project plan generated with explicit cross-scope authorization must not be committed if the apply request later sets `allow_cross_scope = false`.
- `sync_operations.allow_cross_scope` must not under-report a committed cross-scope write.
- Legacy plans without target scope metadata remain accepted.

- [x] **Step 1: Add RED apply authorization test**

Added `apply_sync_rejects_cross_scope_plan_when_apply_authorization_is_false`, which:

- generates a real user-scope source to project-scope target plan with `allow_cross_scope = true`
- applies it with `allow_cross_scope = false`
- asserts the target fails, the file is unchanged, and no backup is created

Run: `cargo test -p wapc apply_sync_rejects_cross_scope_plan_when_apply_authorization_is_false -- --nocapture`
Result: FAIL before implementation because the plan was committed and history could under-report the cross-scope authorization.

- [x] **Step 2: Recheck source/target scope at apply time**

Passed `ApplySyncRequest.allow_cross_scope` into `validate_apply_plan_source` and added `validate_plan_scope_authorization`.

The recheck compares persisted source scope with `WritePlan.target_scope` when target metadata is present. Cross-scope plans require explicit apply-time authorization.

Run: `cargo test -p wapc apply_sync_rejects_cross_scope_plan_when_apply_authorization_is_false -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn build`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 36 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 146 core tests and 25 Tauri helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 108 files.

## Task 109: Phase 4 Enterprise Source Read-Only Sync

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- PRD FR-32 says enterprise/managed resources may act as read-only sources and must not be writable targets.
- `apply_sync` must not reject an enterprise/managed source solely because of its scope when the actual write target is allowed.
- Enterprise/managed targets remain unsupported.
- Plugin-provided sources remain read-only and are still rejected for write sync.

- [x] **Step 1: Add RED enterprise source apply test**

Added `apply_sync_allows_enterprise_source_as_read_only_sync_source`, which:

- creates a real enterprise-scope MCP source in SQLite
- plans a sync into a user-scope JSON MCP target with explicit cross-scope authorization
- applies the plan with explicit cross-scope authorization
- asserts the target is committed and the user target file contains the synced resource

Run: `cargo test -p wapc apply_sync_allows_enterprise_source_as_read_only_sync_source -- --nocapture`
Result: FAIL before implementation because apply rejected the enterprise source even though the target was writable.

- [x] **Step 2: Allow enterprise/managed as read-only sources**

Removed the ordinary-source apply rejection for `source.scope == enterprise || managed`.

Kept:

- target-side `enterprise` / `managed` rejection
- plugin-provided source rejection
- cross-scope authorization checks
- source payload/name/kind validation

Run: `cargo test -p wapc apply_sync_allows_enterprise_source_as_read_only_sync_source -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify guardrails**

Run:

- `cargo test -p wapc cross_sync -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn build`

Result: PASS.

- `cargo test -p wapc cross_sync -- --nocapture` passed with 37 cross-sync tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 147 core tests and 25 Tauri helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` also passed for the active untracked Task 109 files.

## Task 110: Phase 4 Rollback Record Backend Guard

**Files:**
- Modify: `src/sync_engine.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 3/4 rollback is an audit action over original committed writes.
- UI already hides rollback actions for rollback records, but the backend command must enforce the same boundary.
- A rollback-generated `resource_changes` row must not be rollbackable again, otherwise a user or caller can flip state through the backend and create confusing revert chains.
- The guard must preserve the original `sync_id` chain and leave the target file unchanged when rejecting a rollback-of-rollback request.

- [x] **Step 1: Add RED rollback record guard test**

Added `rollback_resource_change_rejects_rollback_records`, which:

- applies a real MCP write with a `sync_id`
- rolls that change back through the real Sync Engine rollback path
- attempts to rollback the generated rollback change
- asserts the backend rejects it as not rollbackable, the file remains restored, and only the original change plus one rollback change exist

Run: `cargo test -p wapc rollback_resource_change_rejects_rollback_records -- --nocapture`
Result: FAIL before implementation because the rollback record was accepted and committed a second revert change.

- [x] **Step 2: Enforce rollback record boundary in core rollback**

Updated `rollback_resource_change` to reject records where:

- `op == "rollback"`
- or `reverts_change_id` is present

Run: `cargo test -p wapc rollback_resource_change_rejects_rollback_records -- --nocapture`
Result: PASS.

- [x] **Step 3: Verify rollback and workspace guardrails**

Run:

- `cargo test -p wapc rollback_resource_change -- --nocapture`
- `cargo test -p wapc sync_engine -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS.

- `cargo test -p wapc rollback_resource_change -- --nocapture` passed with 2 rollback tests.
- `cargo test -p wapc sync_engine -- --nocapture` passed with 8 Sync Engine tests.
- `cargo fmt --check` passed after mechanical formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 148 core tests and 25 Tauri helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` produced no whitespace warnings for the active untracked files checked.

## Task 111: Phase 5 Deep Link Duplicate Query Guard

**Files:**
- Modify: `src/deep_link.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `wapc://import` is an external input surface and must parse a single unambiguous source/resource pair.
- Duplicate `source` or `resource` query parameters must not be silently accepted by taking the first value.
- Rejecting duplicate parameters keeps the preview, fingerprint, and later Sync Engine handoff tied to exactly one visible canonical resource definition.

- [x] **Step 1: Add RED duplicate parameter test**

Added `rejects_duplicate_deep_link_query_parameters`, which builds a real percent-encoded `wapc://import` link with two `resource` parameters and asserts the preview parser rejects it with a duplicate-parameter error.

Run: `cargo test -p wapc rejects_duplicate_deep_link_query_parameters -- --nocapture`
Result: FAIL before implementation because the parser accepted the first `resource` value and generated a preview.

- [x] **Step 2: Reject ambiguous query parameters**

Updated `query_param` to scan all query pairs and reject a second occurrence of the requested parameter instead of returning the first match.

Run:

- `cargo test -p wapc rejects_duplicate_deep_link_query_parameters -- --nocapture`
- `cargo test -p wapc deep_link -- --nocapture`

Result: PASS.

- [x] **Step 3: Verify deep link and workspace guardrails**

Run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn lint`
- `cd ui && yarn test`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS.

- `cargo fmt --check` passed after mechanical formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 149 core tests and 25 Tauri helper tests.
- `cd ui && yarn lint` passed.
- `cd ui && yarn test` passed with 24 frontend helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` produced no whitespace warnings for the active untracked files checked.

## Task 112: Phase 5 Deep Link Malformed Percent-Encoding Guard

**Files:**
- Modify: `src/deep_link.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- `wapc://import` query decoding must be strict because it is an external input surface.
- A malformed percent-encoded query value must be rejected before any preview, fingerprint, or canonical resource is produced.
- The parser must not preserve a dangling `%` as literal source or payload text.

- [x] **Step 1: Add RED malformed percent-encoding test**

Added `rejects_malformed_deep_link_percent_encoding`, which builds a real `wapc://import` link whose final `source` query value ends with a dangling `%`.

Run: `cargo test -p wapc rejects_malformed_deep_link_percent_encoding -- --nocapture`
Result: FAIL before implementation because the parser accepted `https://example.test/templates/%` as a source and generated a safe-preview object.

- [x] **Step 2: Enforce strict percent decoding**

Updated `percent_decode` so every `%` must be followed by exactly two valid hexadecimal digits; malformed sequences now return a `malformed percent encoding` error.

Run:

- `cargo test -p wapc rejects_malformed_deep_link_percent_encoding -- --nocapture`
- `cargo test -p wapc deep_link -- --nocapture`

Result: PASS.

- [x] **Step 3: Verify deep link and workspace guardrails**

Run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn lint`
- `cd ui && yarn test`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS.

- `cargo fmt --check` passed after mechanical formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 150 core tests and 25 Tauri helper tests.
- `cd ui && yarn lint` passed.
- `cd ui && yarn test` passed with 24 frontend helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` produced no whitespace warnings for the active untracked files checked.

## Task 113: Phase 5 Deep Link Source Control Character Guard

**Files:**
- Modify: `src/deep_link.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Deep link `source` is displayed in preview and persisted into the preview resource `origin_path`.
- A source containing decoded control characters must be rejected before preview/fingerprint generation.
- HTTPS prefix checks must not allow newline-injected sources to appear safe.

- [x] **Step 1: Add RED source control-character test**

Added `rejects_deep_link_source_with_control_characters`, which builds a real `wapc://import` link where the source contains percent-encoded newline `%0A`.

Run: `cargo test -p wapc rejects_deep_link_source_with_control_characters -- --nocapture`
Result: FAIL before implementation because the parser accepted the newline-containing source, preserved it in `origin_path`, and generated a no-risk preview.

- [x] **Step 2: Validate source before preview generation**

Added `validate_source` and call it before parsing the resource payload. It rejects empty sources and any decoded control characters.

Run:

- `cargo test -p wapc rejects_deep_link_source_with_control_characters -- --nocapture`
- `cargo test -p wapc deep_link -- --nocapture`

Result: PASS.

- [x] **Step 3: Verify deep link and workspace guardrails**

Run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn lint`
- `cd ui && yarn test`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS.

- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 151 core tests and 25 Tauri helper tests.
- `cd ui && yarn lint` passed.
- `cd ui && yarn test` passed with 24 frontend helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` produced no whitespace warnings for the active untracked files checked.

## Task 114: Phase 5 Deep Link Source Raw Secret Guard

**Files:**
- Modify: `src/deep_link.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-B2 says deep links must carry resource structure, not secrets.
- The `source` parameter is visible in preview and copied into `origin_path`, so it must not carry token-like query values.
- Privacy audit wording already says deep links reject token-like secrets; this must cover source URLs, not only the resource payload.

- [x] **Step 1: Add RED source raw token test**

Added `rejects_deep_link_source_with_raw_token`, which builds a real percent-encoded `wapc://import` link where the source URL contains `access_token=ghp_secret1234567890`.

Run: `cargo test -p wapc rejects_deep_link_source_with_raw_token -- --nocapture`
Result: FAIL before implementation because the parser accepted the token-bearing source, copied it into `origin_path`, and generated a no-risk preview.

- [x] **Step 2: Validate source for raw token-like values**

Updated `validate_source` to reject raw secret-like source strings before preview generation. Added `contains_raw_secret` for substring-style source checks while keeping the existing payload string checks intact.

Run:

- `cargo test -p wapc rejects_deep_link_source_with_raw_token -- --nocapture`
- `cargo test -p wapc deep_link -- --nocapture`

Result: PASS.

- [x] **Step 3: Verify deep link and workspace guardrails**

Run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn lint`
- `cd ui && yarn test`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS.

- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 152 core tests and 25 Tauri helper tests.
- `cd ui && yarn lint` passed.
- `cd ui && yarn test` passed with 24 frontend helper tests.
- `cd ui && yarn build` passed, with the existing Vite chunk-size warning.
- `git diff --check` passed for tracked changes; `git diff --no-index --check /dev/null ...` produced no whitespace warnings for the active untracked files checked.

## Task 115: Phase 5 Deep Link Payload URL Raw Secret Guard

**Files:**
- Modify: `src/deep_link.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-B2 says deep links must not carry raw secrets.
- Payload string fields may themselves be URLs; token-like values embedded in those URLs must not be serialized into `payload_json`.
- The parser should reject raw token-like substrings consistently in both source and payload strings.

- [x] **Step 1: Add RED payload URL raw token test**

Added `rejects_deep_link_payload_url_with_raw_token`, which builds a real percent-encoded `wapc://import` link whose MCP payload URL contains `access_token=ghp_secret1234567890`.

Run: `cargo test -p wapc rejects_deep_link_payload_url_with_raw_token -- --nocapture`
Result: FAIL before implementation because the parser accepted the payload URL and serialized the token into preview `payload_json`.

- [x] **Step 2: Reuse substring-style raw secret detection for payload strings**

Updated payload string validation to call `contains_raw_secret` instead of only checking whether a string starts with a token-like prefix.

Run:

- `cargo test -p wapc rejects_deep_link_payload_url_with_raw_token -- --nocapture`
- `cargo test -p wapc deep_link -- --nocapture`

Result: PASS.

- [x] **Step 3: Verify deep link and workspace guardrails**

Run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn lint`
- `cd ui && yarn test`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS.

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (153 core tests, 25 Tauri helper tests, doc-tests passed)
- `cd ui && yarn lint`
- `cd ui && yarn test` (24 helper tests)
- `cd ui && yarn build` (passes with existing Vite chunk-size warning)
- `git diff --check`
- `git diff --no-index --check /dev/null src/deep_link.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md` (no whitespace warnings; exit code 1 is expected for no-index comparison)

## Task 116: Phase 5 Deep Link URL-Encoded Bearer Secret Guard

**Files:**
- Modify: `src/deep_link.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-B2 says deep links must not carry raw secrets.
- Payload string fields may be URLs; URL-encoded token markers such as `Bearer%20...` still represent raw secrets and must not be serialized into `payload_json`.
- The parser should reject URL-encoded Bearer markers consistently for both `source` and payload strings.

- [x] **Step 1: Add RED URL-encoded Bearer payload test**

Added `rejects_deep_link_payload_url_with_url_encoded_bearer_token`, which builds a real percent-encoded `wapc://import` link whose MCP payload URL contains `authorization=Bearer%20live-token`.

Run: `cargo test -p wapc rejects_deep_link_payload_url_with_url_encoded_bearer_token -- --nocapture`
Result: FAIL before implementation because the parser accepted the payload URL and serialized `Bearer%20live-token` into preview `payload_json`.

- [x] **Step 2: Detect URL-encoded Bearer markers as raw secrets**

Extended `contains_raw_secret` so URL-encoded and form-encoded Bearer markers are treated as raw secrets.

Run:

- `cargo test -p wapc rejects_deep_link_payload_url_with_url_encoded_bearer_token -- --nocapture`
- `cargo test -p wapc deep_link -- --nocapture`

Result: PASS.

- [x] **Step 3: Verify deep link and workspace guardrails**

Run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn lint`
- `cd ui && yarn test`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS.

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (154 core tests, 25 Tauri helper tests, doc-tests passed)
- `cd ui && yarn lint`
- `cd ui && yarn test` (24 helper tests)
- `cd ui && yarn build` (passes with existing Vite chunk-size warning)
- `git diff --check`
- `git diff --no-index --check /dev/null src/deep_link.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md` (no whitespace warnings; exit code 1 is expected for no-index comparison)

## Task 117: Phase 5 Deep Link URL-Encoded Bearer Whitespace Guard

**Files:**
- Modify: `src/deep_link.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-B2 says deep links must not carry raw secrets.
- Bearer tokens may be separated from the scheme by URL-encoded whitespace such as `%09`, `%0A`, or `%0D`, not only `%20` or `+`.
- The parser should reject those encoded whitespace variants in both source URLs and payload strings before serializing or previewing the resource.

- [x] **Step 1: Add RED source URL-encoded tab Bearer test**

Added `rejects_deep_link_source_url_with_url_encoded_bearer_tab_token`, which builds a real percent-encoded `wapc://import` link whose `source` URL contains `authorization=Bearer%09live-token`.

Run: `cargo test -p wapc rejects_deep_link_source_url_with_url_encoded_bearer_tab_token -- --nocapture`
Result: FAIL before implementation because the parser accepted the source URL and exposed `Bearer%09live-token` in the preview source/origin path.

- [x] **Step 2: Detect URL-encoded Bearer whitespace variants**

Replaced the ad hoc encoded Bearer checks with `contains_encoded_bearer_separator`, covering `%20`, `+`, `%09`, `%0A`, and `%0D` separators.

Run:

- `cargo test -p wapc rejects_deep_link_source_url_with_url_encoded_bearer_tab_token -- --nocapture`
- `cargo test -p wapc deep_link -- --nocapture`

Result: PASS.

- [x] **Step 3: Verify deep link and workspace guardrails**

Run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn lint`
- `cd ui && yarn test`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS after applying `cargo fmt`.

- `cargo fmt --check` initially failed on helper array formatting; `cargo fmt` applied the required formatting.
- `cargo fmt --check` passed after formatting.
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (155 core tests, 25 Tauri helper tests, doc-tests passed)
- `cd ui && yarn lint`
- `cd ui && yarn test` (24 helper tests)
- `cd ui && yarn build` (passes with existing Vite chunk-size warning)
- `git diff --check`
- `git diff --no-index --check /dev/null src/deep_link.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md` (no whitespace warnings; exit code 1 is expected for no-index comparison)

## Task 118: Phase 5 Deep Link Percent-Encoded Token Prefix Guard

**Files:**
- Modify: `src/deep_link.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-B2 says deep links must not carry raw secrets.
- Token prefixes may be URL-encoded inside nested URL strings, such as `ghp%5F...` instead of `ghp_...`.
- Secret detection should scan a percent-decoded view of source and payload strings without turning malformed deep-link parameters into accepted input.

- [x] **Step 1: Add RED payload URL percent-encoded GitHub token prefix test**

Added `rejects_deep_link_payload_url_with_percent_encoded_token_prefix`, which builds a real percent-encoded `wapc://import` link whose MCP payload URL contains `access_token=ghp%5Fsecret1234567890`.

Run: `cargo test -p wapc rejects_deep_link_payload_url_with_percent_encoded_token_prefix -- --nocapture`
Result: FAIL before implementation because the parser accepted the payload URL and serialized `ghp%5Fsecret1234567890` into preview `payload_json`.

- [x] **Step 2: Scan percent-decoded string views for raw secret markers**

Split raw secret matching into `contains_raw_secret_marker` and updated `contains_raw_secret` to also scan a percent-decoded view of each source/payload string when decoding succeeds.

Run:

- `cargo test -p wapc rejects_deep_link_payload_url_with_percent_encoded_token_prefix -- --nocapture`
- `cargo test -p wapc deep_link -- --nocapture`

Result: PASS.

- [x] **Step 3: Verify deep link and workspace guardrails**

Run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn lint`
- `cd ui && yarn test`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS.

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (156 core tests, 25 Tauri helper tests, doc-tests passed)
- `cd ui && yarn lint`
- `cd ui && yarn test` (24 helper tests)
- `cd ui && yarn build` (passes with existing Vite chunk-size warning)
- `git diff --check`
- `git diff --no-index --check /dev/null src/deep_link.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md` (no whitespace warnings; exit code 1 is expected for no-index comparison)

## Task 119: Phase 5 Deep Link Double-Encoded Token Prefix Guard

**Files:**
- Modify: `src/deep_link.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-B2 says deep links must not carry raw secrets.
- Token prefixes may be double URL-encoded inside nested URL strings, such as `ghp%255F...`, where one decode only reveals `ghp%5F...`.
- Secret detection should use a bounded repeated percent-decoding pass so common encoded secret markers are rejected without creating an unbounded parser loop.

- [x] **Step 1: Add RED payload URL double-encoded GitHub token prefix test**

Added `rejects_deep_link_payload_url_with_double_encoded_token_prefix`, which builds a real percent-encoded `wapc://import` link whose MCP payload URL contains `access_token=ghp%255Fsecret1234567890`.

Run: `cargo test -p wapc rejects_deep_link_payload_url_with_double_encoded_token_prefix -- --nocapture`
Result: FAIL before implementation because the parser accepted the payload URL and serialized `ghp%255Fsecret1234567890` into preview `payload_json`.

- [x] **Step 2: Add bounded repeated percent-decoded secret marker scan**

Updated `contains_raw_secret` to scan the original string plus up to three percent-decoded generations, returning early when decoding fails or stabilizes.

Run:

- `cargo test -p wapc rejects_deep_link_payload_url_with_double_encoded_token_prefix -- --nocapture`
- `cargo test -p wapc deep_link -- --nocapture`

Result: PASS.

- [x] **Step 3: Verify deep link and workspace guardrails**

Run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn lint`
- `cd ui && yarn test`
- `cd ui && yarn build`
- `git diff --check`

Result: PASS.

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (157 core tests, 25 Tauri helper tests, doc-tests passed)
- `cd ui && yarn lint`
- `cd ui && yarn test` (24 helper tests)
- `cd ui && yarn build` (passes with existing Vite chunk-size warning)
- `git diff --check`
- `git diff --no-index --check /dev/null src/deep_link.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md` (no whitespace warnings; exit code 1 is expected for no-index comparison)

## Task 120: Phase 5 Deep Link Complete Preview UI Contract

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-B3 says deep-link import must show source and complete content preview before any sync flow.
- The Resource Center panel must display the canonical payload JSON returned by the backend preview command, not just resource name/scope/fingerprint.
- The panel remains preview-only; it must not imply that paste/preview writes resources or installs templates.

- [x] **Step 1: Add RED deep-link complete preview helper test**

Added `builds complete deep link preview summary from backend preview payload`, which constructs a real `DeepLinkImportPreview` shape and asserts the UI state helper exposes resource label, scope, source, content fingerprint, risks, boundary text, and the exact backend `payload_json`.

Run: `cd ui && yarn test`
Result: FAIL before implementation because `buildDeepLinkPreviewSummary` was not exported.

- [x] **Step 2: Add deep-link preview summary helper and UI payload rendering**

Added `buildDeepLinkPreviewSummary` in `resourceCenterState.ts` and wired the Resource Center deep-link preview panel to display:

- preview-only boundary text
- resource label, scope, source, and content fingerprint
- formatted canonical `payload_json`
- backend risk messages

- [x] **Step 3: Verify frontend and workspace guardrails**

Run:

- `cd ui && yarn test`
- `cd ui && yarn lint`
- `cd ui && yarn build`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

Result: PASS.

- `cd ui && yarn test` (25 helper tests)
- `cd ui && yarn lint`
- `cd ui && yarn build` (passes with existing Vite chunk-size warning)
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (157 core tests, 25 Tauri helper tests, doc-tests passed)
- `git diff --check`
- `git diff --check -- ui/src/pages/resourceCenterState.ts ui/src/pages/ResourcesPage.tsx ui/tests/resourceCenterState.test.ts`
- `git diff --no-index --check /dev/null src/deep_link.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md` (no whitespace warnings; exit code 1 is expected for no-index comparison)

## Task 121: Phase 5 Deep Link Sync Preview Command

**Files:**
- Modify: `src/model.rs`
- Modify: `src/deep_link.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-B3/AC-B requires deep-link import to move from preview into target selection and safe write preview.
- The backend must generate sync previews through the existing `cross_sync` engine, not through frontend-only JSON assembly.
- Deep-link planning must not persist imported resources before the user explicitly applies a safe write plan.
- Raw secret rejection must reuse the same deep-link parser path used by preview.

- [x] **Step 1: Add RED command-helper test for deep-link sync planning**

Add a Tauri helper test that builds a safe `wapc://import` MCP link, plans it into a real local target file, asserts the target preview contains the imported MCP server, and asserts `UsageStore::list_resources` remains empty.

Run: `cargo test -p wapc-app plan_deep_link_import -- --nocapture`
Result: FAIL before implementation because `PlanDeepLinkImportRequest` and `deep_link::plan_deep_link_import` did not exist.

- [x] **Step 2: Add model and core deep-link planning function**

Introduce a `PlanDeepLinkImportRequest` and a core function that parses the deep-link URL, converts the preview resource into a transient source, and calls `cross_sync::plan_sync_from_resource`.

Added `PlanDeepLinkImportRequest`, `deep_link::plan_deep_link_import`, and a core module test proving a deep-link import can generate a real sync preview without persisting the imported resource or writing the target file.

- [x] **Step 3: Expose Tauri command and frontend type contract**

Register `plan_deep_link_import` with Tauri and add the TypeScript request shape for future UI wiring.

Registered `plan_deep_link_import` in the Tauri command handler and added `PlanDeepLinkImportRequest` to the UI type contract.

- [x] **Step 4: Verify workspace guardrails**

Run focused and full verification after implementation.

Focused runs:

- `cargo test -p wapc-app plan_deep_link_import -- --nocapture` (passes, 1 command-helper test)
- `cargo test -p wapc deep_link -- --nocapture` (passes, 16 deep-link/privacy boundary tests)
- `cargo fmt --check`

Full verification:

- `cd ui && yarn test` (25 helper tests)
- `cd ui && yarn lint`
- `cd ui && yarn build` (passes with existing Vite chunk-size warning)
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (158 core tests, 26 Tauri helper tests, doc-tests passed)
- `git diff --check`
- `git diff --check -- src/model.rs src-tauri/src/commands.rs src-tauri/src/lib.rs ui/src/types/index.ts ui/src/pages/resourceCenterState.ts ui/src/pages/ResourcesPage.tsx ui/tests/resourceCenterState.test.ts`
- `git diff --no-index --check /dev/null src/deep_link.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null src-tauri/src/commands.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null src-tauri/src/lib.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null ui/src/types/index.ts` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md` (no whitespace warnings; exit code 1 is expected for no-index comparison)

## Task 122: Phase 5 Deep Link Target Selection UI

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Deep-link preview must offer real Sync Engine target selection after backend preview succeeds.
- The frontend request must call `plan_deep_link_import`; it must not synthesize `PlanSyncResult`.
- Because deep-link sources are intentionally not persisted yet, this slice exposes safe write preview only and must keep final apply unavailable for deep-link plans.
- Target selection must reuse the existing `SyncTarget` shape and cross-scope authorization rules.

- [x] **Step 1: Add RED helper test for deep-link plan request assembly**

Add a frontend helper test that uses the backend preview resource shape, builds real sync target options, selects targets, and expects a trimmed `PlanDeepLinkImportRequest` with real `SyncTarget` payloads.

Run: `cd ui && yarn test`
Result: FAIL before implementation because `buildDeepLinkImportPlanRequest` was not exported.

- [x] **Step 2: Wire Resource Center deep-link target controls**

After a successful deep-link preview, show selectable targets, env strategy, optional project path, and cross-scope authorization. Planning must call `plan_deep_link_import`.

Added `buildDeepLinkImportPlanRequest`, deep-link target options, env strategy, optional project path, cross-scope authorization, and a `plan_deep_link_import` invocation path.

- [x] **Step 3: Keep deep-link apply unavailable until verifiable source apply exists**

Reuse the sync preview dialog for deep-link plans, but disable final apply with explicit UI copy instead of allowing a failing or fake write flow.

Added `applyDisabledReason` to `SyncPreviewDialog`; deep-link plans can inspect real Sync Engine previews but cannot trigger a fake/failing final apply.

- [x] **Step 4: Verify frontend and workspace guardrails**

Run focused frontend tests plus the standard workspace guardrails after implementation.

Verification:

- `cd ui && yarn test` (26 helper tests)
- `cd ui && yarn lint`
- `cd ui && yarn build` (passes with existing Vite chunk-size warning)
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (158 core tests, 26 Tauri helper tests, doc-tests passed)

Browser check:

- Started `cd ui && yarn dev --host 127.0.0.1` at `http://127.0.0.1:5173/`.
- In-app browser opened the page with no console errors, but standalone Vite lacks Tauri `invoke` runtime and shows `本机快照暂不可用` / `Cannot read properties of undefined (reading 'invoke')`.
- Result: visual verification is blocked in plain browser mode; no visual pass is claimed for this slice.

## Task 123: Phase 5 Deep Link Verified Apply

**Files:**
- Modify: `src/model.rs`
- Modify: `src/cross_sync.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Deep-link safe write must not persist imported resources before apply.
- Apply must re-parse the original `wapc://import` URL and compare it with the planned `deep-link:` resource id, name, kind, scope, payload, target metadata, fingerprints, and diff.
- Raw secret rejection must still happen through the deep-link parser at apply time.
- Frontend may enable final apply only when it sends the same original deep-link URL alongside the Sync Engine plans.

- [x] **Step 1: Add RED deep-link apply tests**

Add tests proving a deep-link plan can commit when `deep_link_url` matches, and rejects a substituted `deep_link_url` before target write.

Run: `cargo test -p wapc deep_link_plan -- --nocapture`
Result: FAIL before implementation because `ApplySyncRequest` did not have `deep_link_url`.

- [x] **Step 2: Add backend apply source override validation**

Extend `ApplySyncRequest` with optional `deep_link_url`, re-parse it during `validate_apply_plan_source`, and reject mismatched or missing deep-link source proofs.

Added optional `deep_link_url` to `ApplySyncRequest`; deep-link plans now re-parse the original URL at apply time, compare source id/kind/name/payload/scope, and keep imported resources out of the resource table.

- [x] **Step 3: Enable frontend deep-link apply with source proof**

Track the URL that generated the current deep-link sync preview, pass it into `apply_sync`, and remove the temporary unavailable banner for verified deep-link plans.

Frontend now stores the URL that generated the current deep-link plan and passes it as `deep_link_url` during apply. Re-preview and dialog close clear that source proof.

- [x] **Step 4: Verify focused and full guardrails**

Run focused backend/frontend tests, then the standard workspace gates.

Focused runs:

- `cargo test -p wapc deep_link -- --nocapture` (18 deep-link/privacy/apply boundary tests)
- `cd ui && yarn test` (26 helper tests)
- `cd ui && yarn build` (passes with existing Vite chunk-size warning)

Full verification:

- `cd ui && yarn lint`
- `cargo fmt --check`
- `cargo test -p wapc-app apply_sync -- --nocapture` (1 Tauri apply helper test)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (160 core tests, 26 Tauri helper tests, doc-tests passed)
- `git diff --check`
- `git diff --check -- src/model.rs src/cross_sync.rs ui/src/types/index.ts ui/src/pages/ResourcesPage.tsx docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
- `git diff --no-index --check /dev/null src/cross_sync.rs` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null ui/src/types/index.ts` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null ui/src/pages/ResourcesPage.tsx` (no whitespace warnings; exit code 1 is expected for no-index comparison)
- `git diff --no-index --check /dev/null docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md` (no whitespace warnings; exit code 1 is expected for no-index comparison)

## Task 124: Phase 5 Redacted Report Explicit Project Alias

**Files:**
- Modify: `src/model.rs`
- Modify: `src/export.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/ExportPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-C2/AC-C allows project paths to become aliases or hashes, but project names must not leak unless the user explicitly chooses alias disclosure.
- Default redacted report behavior remains strict: only `project_hash`, no canonical path, original path, session id, source path, prompt/body, or secret values.
- When alias disclosure is enabled, only user-defined `project_aliases.alias` may appear; raw paths and derived folder names must remain absent.
- Synthetic fixture output remains synthetic and must not inherit real aliases.

- [x] **Step 1: Add RED export tests for explicit aliases**

Add tests proving default redacted JSON omits aliases and explicit alias mode includes only the configured alias while still excluding raw paths/project names/secrets.

- [x] **Step 2: Extend backend report request and renderer**

Add `include_project_aliases` to `ExportReportRequest` and render optional `project_alias` only for redacted reports when explicitly requested.

- [x] **Step 3: Add UI option and TypeScript contract**

Expose a redacted-report checkbox for project aliases, pass the flag to `export_report`, and update export boundary copy.

- [x] **Step 4: Verify focused and full guardrails**

Run focused export/UI tests, then the standard workspace gates.

Focused verification:

- `cargo test -p wapc redacted_team_report -- --nocapture`
- `cargo test -p wapc export_report_request -- --nocapture`
- `cd ui && yarn lint`
- `cd ui && yarn build` (passes; Vite reports existing chunk-size warning)

Full guardrails:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn test`
- `git diff --check`
- `git diff --check --no-index -- /dev/null <new-or-untracked-touched-file>` for touched untracked files

## Task 125: Phase 5 Redacted Report Top-Level Tool and Model Summaries

**Files:**
- Modify: `src/export.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-C1 requires redacted reports to summarize by tool, project, and model.
- Keep project rows redacted exactly as before: no raw canonical path, original path, source path, session id, prompt/body, or secret values.
- Top-level tool/model summaries must be derived from real stored `usage_records` metadata and respect the requested report time window.
- Do not add synthetic product outcomes; fixture output remains explicitly synthetic and separate.

- [x] **Step 1: Add RED export test for top-level summaries**

Add a test proving redacted JSON contains team-wide `tool_breakdown` and `model_breakdown` while continuing to exclude raw project paths, folder names, session ids, source paths, body text, and secrets.

- [x] **Step 2: Aggregate summaries from existing project/model rows**

Reuse the redacted report metadata aggregation and compute tool-level and model-level totals without reading or serializing raw content fields.

- [x] **Step 3: Render JSON and Markdown summaries**

Add top-level JSON fields and Markdown sections for tool/model summaries while keeping existing project rows and fixture behavior compatible.

- [x] **Step 4: Verify focused and full guardrails**

Run focused export tests and standard workspace gates.

Focused verification:

- `cargo test -p wapc redacted_team_report_includes_top_level_tool_and_model_summaries -- --nocapture` (failed before implementation because `tool_breakdown` was absent; passes after implementation)
- `cargo test -p wapc redacted_team_report -- --nocapture`

Full guardrails:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn test`
- `cd ui && yarn lint`
- `cd ui && yarn build` (passes; Vite reports existing chunk-size warning)
- `git diff --check`
- `git diff --check --no-index -- /dev/null <new-or-untracked-touched-file>` for touched untracked files

## Task 126: Phase 5 Deep Link Desktop Scheme Registration and App Entry

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `ui/package.json`
- Modify: `ui/yarn.lock`
- Modify: `ui/src/App.tsx`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Create: `ui/src/hooks/deep-link.ts`
- Create: `ui/tests/deep-link.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-B1 requires the desktop app to register the `wapc://` scheme, not only parse pasted strings.
- macOS deep-link registration must be static in Tauri config; runtime-only registration is not enough.
- Opening a `wapc://import?...` URL must route the user into Resource Center with the URL prefilled for safe preview; it must not persist imported resources or write targets.
- The existing parser, target selection, Sync Engine preview, and verified apply boundaries remain authoritative.

- [x] **Step 1: Add RED registration and UI dispatch tests**

Add tests proving the Tauri app declares the `wapc` desktop scheme, initializes the deep-link plugin, includes the UI plugin dependency, and dispatches only `wapc://import?...` URLs into a handler.

- [x] **Step 2: Register Tauri deep-link plugin and static scheme**

Add the Rust plugin dependency, initialize it in the app builder, and configure `plugins.deep-link.desktop.schemes = ["wapc"]`.

- [x] **Step 3: Wire frontend deep-link entry into Resource Center**

Add a small tested runtime adapter for `getCurrent` / `onOpenUrl`, route valid import links to the Resources page, and prefill the deep-link input without auto-applying or writing.

- [x] **Step 4: Verify focused and full guardrails**

Run focused Tauri/UI tests and standard workspace gates.

Focused verification:

- `cargo test -p wapc-app tauri_registers_wapc_deep_link_scheme_statically -- --nocapture` (failed before implementation because `plugins.deep-link.desktop.schemes` was absent; passes after implementation)
- `cd ui && node --test --experimental-strip-types tests/deep-link.test.ts` (failed before implementation because `ui/src/hooks/deep-link.ts` was absent; passes after implementation)
- `cargo check -p wapc-app`
- `cd ui && yarn test`
- `cd ui && yarn lint`
- `cd ui && yarn build` (passes; Vite reports existing chunk-size warning)

Full guardrails:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo tauri build` from `src-tauri` (passes; produces `target/release/bundle/macos/WAPC.app`)
- `plutil -p target/release/bundle/macos/WAPC.app/Contents/Info.plist | rg -n "CFBundleURLTypes|CFBundleURLSchemes|wapc"` confirms `CFBundleURLSchemes` contains `wapc`
- `git diff --check`
- `git diff --check --no-index -- /dev/null <new-or-untracked-touched-file>` for touched untracked files

## Task 127: Phase 5 Template and Deep-Link Manual Env Apply Guard

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 FR-A3 requires template installs with env placeholders to require user input without persisting raw values.
- Phase 5 FR-B2/AC-B applies the same manual-env boundary to imported deep-link resources.
- The UI must block apply when manual env values are missing or whitespace-only, and show which keys are missing before calling `apply_sync`.
- Raw env values still remain memory-only and are sent only in the explicit apply request.

- [x] **Step 1: Add RED UI state helper tests**

Add tests proving manual env apply validation returns a missing-key reason for empty or whitespace-only values, and returns null when all required keys are provided.

- [x] **Step 2: Implement reusable apply validation helper**

Add a small helper in Resource Center state logic that derives required keys from `PlanSyncResult` and validates the current env strategy/value map.

- [x] **Step 3: Wire validation into SyncPreviewDialog**

Use the helper to disable the Apply button and show a clear reason in the dialog for template/deep-link/manual sync plans.

- [x] **Step 4: Verify focused and full guardrails**

Run focused UI tests and standard workspace gates.

Focused verification:

- `cd ui && node --test --experimental-strip-types tests/resourceCenterState.test.ts` (failed before implementation because `getManualEnvApplyDisabledReason` was absent; passes after implementation)
- `cd ui && yarn lint`
- `cd ui && yarn build` (passes; Vite reports existing chunk-size warning)

Full guardrails:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cd ui && yarn test`
- `cd ui && yarn lint`
- `cd ui && yarn build` (passes; Vite reports existing chunk-size warning)
- `git diff --check`
- `git diff --check --no-index -- /dev/null <new-or-untracked-touched-file>` for touched untracked files

## Task 128: Phase 5 Non-macOS Candidate Verification Boundary

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 WP5.F keeps Windows/Linux path support as read-only candidate metadata until real platform fixture evidence exists.
- `candidate_verified` must not become true for Linux/Windows Codex or Gemini MCP paths based only on path-shape samples.
- `privacy-audit` must describe Linux/Windows candidate paths as unverified and write unsupported, without prompt/response/source body/key material.
- This task does not check off Windows/Linux real-machine path verification, platform fixtures, Tauri GUI bundle support, or any non-macOS write support.

- [x] **Step 1: Add RED candidate verification tests**

Extended `resolves_cross_platform_tool_path_candidates_without_touching_filesystem` so Linux Codex MCP and Linux Gemini MCP candidates must remain `verified=false`, `read_only=true`, and `write_supported=false`.

Focused run:

- `cargo test -p wapc platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`

Result: FAIL before implementation because Linux Codex MCP was incorrectly marked verified.

- [x] **Step 2: Add RED privacy-audit assertion**

Extended `privacy_audit_names_cross_platform_candidate_path_boundaries` so Linux Gemini MCP candidate purpose must include `unverified`.

Focused run:

- `cargo test -p wapc privacy::tests::privacy_audit_names_cross_platform_candidate_path_boundaries -- --nocapture`

Result: FAIL before implementation because privacy-audit described Linux Gemini MCP as verified by local fixture.

- [x] **Step 3: Tighten PathResolver verification flags**

Removed Linux-specific verification shortcuts for Codex config/data/session/MCP candidates and Gemini config/MCP candidates. Non-macOS candidates still resolve concrete candidate paths, but remain read-only, unverified, and write unsupported.

- [x] **Step 4: Verify focused and workspace guardrails**

Verification:

- `cargo test -p wapc platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture` (PASS)
- `cargo test -p wapc privacy::tests::privacy_audit_names_cross_platform_candidate_path_boundaries -- --nocapture` (PASS)
- `cargo fmt --check` (failed before `cargo fmt` due assertion wrapping; PASS after formatting)
- `cargo clippy --workspace --all-targets -- -D warnings` (PASS)
- `cargo test --workspace` (PASS: 162 core tests, 27 Tauri helper tests, 0 main tests, 0 doctests)
- `git diff --check` (PASS)

## Task 129: Phase 5 Path Verification Desktop Visibility

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `ui/src/types/index.ts`
- Create: `ui/src/pages/toolPathState.ts`
- Modify: `ui/src/pages/ToolsPage.tsx`
- Create: `ui/tests/toolPathState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 5 WP5.F requires UI to show unverified platform candidates as `待核验` / `unsupported`.
- The desktop snapshot may expose only metadata from PathResolver verification: tool, platform, scope, kind, aliased path, existence, file/dir flags, read-only, and write-supported status.
- The verifier must not read config/session/instruction file contents and must not expose real home/project absolute paths in the serialized records.
- This task does not add Windows/Linux real-machine fixtures, Tauri GUI bundles, or any new write target support.

- [x] **Step 1: Add RED backend visibility test**

Added `tool_path_verifications_alias_home_and_project_paths_without_reading_contents`. It writes secret-looking fixture content under a temp home/project, calls the new verification helper, and asserts:

- user Codex MCP path is aliased as `~/.codex/config.toml`
- project Codex instruction path is aliased as `<project>/AGENTS.md`
- records preserve exists/is_file/read_only/write_supported metadata
- serialized output does not contain the temp absolute home path or file content

Focused run:

- `cargo test -p wapc-app commands::tests::tool_path_verifications_alias_home_and_project_paths_without_reading_contents -- --nocapture`

Result: FAIL before implementation because `tool_path_verifications_for_paths` did not exist; PASS after implementation.

- [x] **Step 2: Add RED frontend summary tests**

Added `ui/tests/toolPathState.test.ts` for `buildToolPathVerificationSummary`.

Focused run:

- `node --test --experimental-strip-types ui/tests/toolPathState.test.ts`

Result: FAIL before implementation because `ui/src/pages/toolPathState.ts` did not exist; PASS after implementation.

- [x] **Step 3: Expose path verification records in desktop snapshot**

Derived `Serialize` for `ToolPathVerificationRecord`, added `tool_path_verifications` to `DesktopSnapshot`, and populated it from current home plus known existing project roots. Project paths are emitted only from project-scoped candidate records to avoid duplicating user candidates per project root.

- [x] **Step 4: Render path verification on Tools page**

Tools page now shows a `本机路径核验` table with tool/platform/scope/kind/status/write/path columns. Status uses `已核验` or `待核验`; write state uses `可写` or `只读 / unsupported`.

- [x] **Step 5: Verify focused, UI, and workspace guardrails**

Verification:

- `cargo test -p wapc-app commands::tests::tool_path_verifications_alias_home_and_project_paths_without_reading_contents -- --nocapture` (PASS)
- `node --test --experimental-strip-types ui/tests/toolPathState.test.ts` (PASS)
- `cargo fmt --check` (failed once before `cargo fmt` due helper wrapping; PASS after formatting)
- `cargo clippy --workspace --all-targets -- -D warnings` (failed once on cloned ref in test; PASS after switching to `std::slice::from_ref`)
- `cargo test -p wapc-app tests::tauri_commands_resolve_app_paths_through_core_path_resolver -- --nocapture` (failed once because the static import contract expected `platform_paths::WapcPaths`; PASS after updating the contract to accept grouped PathResolver imports while still requiring `WapcPaths` and `PlatformPathContext`)
- `cargo test --workspace` (PASS: 162 core tests, 28 Tauri helper tests, 0 main tests, 0 doctests)
- `cd ui && yarn test` (PASS: 31 tests)
- `cd ui && yarn lint` (PASS)
- `cd ui && yarn build` (PASS; Vite reports existing chunk-size warning)
- `git diff --check` (PASS)
- no-index whitespace check for new untracked files touched in this task (PASS)

## Task 130: Phase 1 Export User-Selected Directory Defaults

**Files:**
- Create: `ui/src/pages/exportState.ts`
- Create: `ui/tests/exportState.test.ts`
- Modify: `ui/src/pages/ExportPage.tsx`
- Modify: `ui/package.json`
- Modify: `ui/yarn.lock`
- Modify: `src-tauri/Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src-tauri/src/lib.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 1 FR-52 requires report export files to default into a user-selected directory with filenames containing the view name and date.
- The UI must use the real Tauri dialog plugin for directory selection; no fake picker, demo path, or hardcoded user directory.
- The existing explicit path input remains editable for advanced users, but directory selection generates a concrete output file path before calling the real `export_report` command.
- This task does not change the metadata-only export boundary or add cloud upload.

- [x] **Step 1: Add RED export path helper tests**

Added `ui/tests/exportState.test.ts` for:

- default filenames: `wapc-<view>-YYYY-MM-DD.<ext>`
- Markdown extension `.md`
- macOS/Linux and Windows directory separator handling
- empty directory returns no suggested path
- redacted CSV normalizes to JSON

Focused run:

- `node --test --experimental-strip-types ui/tests/exportState.test.ts`

Result: FAIL before implementation because `ui/src/pages/exportState.ts` did not exist; PASS after implementation.

- [x] **Step 2: Implement export path helpers**

Created `ui/src/pages/exportState.ts` with `buildDefaultExportFilename`, `buildExportPath`, `suggestExportPath`, and `normalizeExportFormat`.

- [x] **Step 3: Wire real directory selection into Export page**

`ExportPage` now imports `open` from `@tauri-apps/plugin-dialog`, adds a `选择目录` button, and fills `输出文件路径` with the generated path. Changing view/format after a directory is selected updates the suggested path. Export still calls the real `export_report` Tauri command.

- [x] **Step 4: Add dialog plugin dependencies and registration**

Added `@tauri-apps/plugin-dialog` and `tauri-plugin-dialog`, registered `.plugin(tauri_plugin_dialog::init())`, and added `tauri_registers_dialog_plugin_for_user_selected_exports` to prevent regressing to a fake picker.

- [x] **Step 5: Verify focused, UI, and workspace guardrails**

Verification:

- `node --test --experimental-strip-types ui/tests/exportState.test.ts` (PASS)
- `cargo test -p wapc-app tests::tauri_registers_dialog_plugin_for_user_selected_exports -- --nocapture` (PASS)
- `cd ui && yarn test` (PASS: 35 tests)
- `cd ui && yarn lint` (PASS)
- `cd ui && yarn build` (PASS; Vite reports existing chunk-size warning)
- `cargo fmt --check` (PASS)
- `cargo clippy --workspace --all-targets -- -D warnings` (PASS)
- `cargo test --workspace` (PASS: 162 core tests, 29 Tauri helper tests, 0 main tests, 0 doctests)
- `git diff --check` (PASS)
- no-index whitespace check for new untracked files touched in this task (PASS)

## Task 131: Phase 3 Backup Source Change Visibility

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 3 FR-41 says the backup list UI must show the source change.
- Use only real persisted `resource_backups` metadata from `list_backups`; do not synthesize backup rows or infer fake changes.
- Backup contents may contain secrets, so UI must continue to show only metadata: backup path, original path, tool, time, and source `change_id`.

- [x] **Step 1: Add RED backup summary test**

Added `summarizes resource backups with source change and original path`, asserting that a `ResourceBackup` summary includes:

- backup path
- original path
- source `change_id`
- a visible `来源变更 <change_id>` label

Focused run:

- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts`

Result: FAIL before implementation because `buildResourceBackupSummary` was missing; PASS after implementation.

- [x] **Step 2: Implement backup summary helper**

Added `buildResourceBackupSummary` to keep backup source-change labels deterministic and testable.

- [x] **Step 3: Render source change and original path in Resource Center**

Updated the backup panel to show backup path, tool/time, original path, and source change label. Empty `change_id` remains explicit as `无来源变更记录`.

- [x] **Step 4: Verify UI and workspace guardrails**

Verification:

- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts` (PASS)
- `cd ui && yarn test` (PASS: 36 tests)
- `cd ui && yarn lint` (PASS)
- `cd ui && yarn build` (PASS; Vite reports existing chunk-size warning)
- `cargo fmt --check` (PASS)
- `cargo clippy --workspace --all-targets -- -D warnings` (PASS)
- `cargo test --workspace` (PASS: 162 core tests, 29 Tauri helper tests, 0 main tests, 0 doctests)
- `git diff --check` (PASS)
- no-index whitespace check for touched untracked files (PASS)

## Task 132: Phase 3 Drift Rescan Option

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 3 FR-23 requires a way to use the current tool state after drift instead of only allowing overwrite.
- The rescan action must call real Resource Inventory commands (`inventory_scan`, `list_resources`, `list_parse_failures`) and clear the stale write preview.
- Do not fake a refreshed state locally, do not mutate resources in the browser, and do not write target files during rescan.

- [x] **Step 1: Add RED drift action tests**

Added `builds drift resolution actions with real rescan and explicit overwrite`, asserting that drift state exposes:

- `以工具现状为准重新识别`
- `确认覆盖当前状态`
- no rescan action when drift is not present

Focused run:

- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts`

Result: FAIL before implementation because `buildDriftResolutionActions` was missing; PASS after implementation.

- [x] **Step 2: Implement drift action helper**

Added `buildDriftResolutionActions` so drift UI labels and disabled states are deterministic and testable.

- [x] **Step 3: Wire rescan into WritePreviewDialog**

When drift is detected, the write preview now shows both actions:

- `以工具现状为准重新识别`: calls the existing real inventory refresh flow, clears stale `writePlan`, clears drift confirmation, and asks the user to generate a new preview.
- `确认覆盖当前状态`: keeps the existing explicit `confirm_drift=true` apply path.

- [x] **Step 4: Verify UI and workspace guardrails**

Verification:

- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts` (PASS)
- `cd ui && yarn test` (PASS: 37 tests)
- `cd ui && yarn lint` (PASS)
- `cd ui && yarn build` (PASS; Vite reports existing chunk-size warning)
- `cargo fmt --check` (PASS)
- `cargo clippy --workspace --all-targets -- -D warnings` (PASS)
- `cargo test --workspace` (PASS: 162 core tests, 29 Tauri helper tests, 0 main tests, 0 doctests)
- `git diff --check` (PASS)
- no-index whitespace check for touched untracked files (PASS)

## Task 133: Phase 3 Explicit Unsupported MCP Management Actions

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 3 FR-31 names MCP enable/disable/edit/delete management, but the current production-safe write contract only supports disabling user-scope Claude/Cursor JSON MCP entries under `mcpServers`.
- The UI must not pretend enable/edit/delete are implemented; it must show them as explicit unsupported actions with no backend request.
- Unsupported resources must remain visibly read-only across all management actions.
- The existing `disable_mcp` path must continue to use Sync Engine preview instead of direct browser mutation or fake local state.

- [x] **Step 1: Add RED management action list tests**

Added `builds explicit resource management action list with unsupported operations visible`, asserting that a supported JSON MCP resource exposes:

- `禁用 MCP` enabled with a real `disable` request
- `启用 MCP` disabled with no request
- `编辑 MCP` disabled with no request
- `删除 MCP` disabled with no request
- unsupported action reasons include `暂未开放`

The same test asserts enterprise resources keep every action disabled with the existing read-only reason.

Focused run:

- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts`

Result: FAIL before implementation because `buildResourceManagementActions` was missing; PASS after implementation.

- [x] **Step 2: Implement explicit action helper**

Added `buildResourceManagementActions` as the Resource Center management action source of truth:

- `disable_mcp` delegates to the existing `getResourceManagementCapability`.
- `enable_mcp`, `edit_mcp`, and `delete_mcp` are explicit unsupported options.
- If the selected resource is already read-only, every action reuses the concrete read-only reason.

- [x] **Step 3: Render the action list in Resource Center**

Replaced the single management button with a fixed action list in the resource detail panel.

Only actions with a real `request` call `plan_resource_change`; unsupported actions render disabled with their reason and never call the backend.

- [x] **Step 4: Verify UI and workspace guardrails**

Verification:

- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts` (PASS)
- `cd ui && yarn test` (PASS: 38 tests)
- `cd ui && yarn lint` (PASS)
- `cd ui && yarn build` (PASS; Vite reports existing chunk-size warning)
- Browser check at `http://127.0.0.1:5173/` (LIMITED: plain Vite browser lacks Tauri `invoke`, so the app correctly shows `本机快照暂不可用`; real Resource Center visual state requires Tauri runtime)
- `cargo fmt --check` (PASS)
- `cargo clippy --workspace --all-targets -- -D warnings` (PASS)
- `cargo test --workspace` (PASS: 162 core tests, 29 Tauri helper tests, 0 main tests, 0 doctests)
- `git diff --check` (PASS)
- no-index whitespace check for touched untracked files (PASS)

## Task 134: Phase 3 Sync Engine Idempotent Re-Apply

**Files:**
- Modify: `src/sync_engine.rs`
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 3 FR-17 requires the write pipeline to be idempotent: repeating the same plan must not create new side effects.
- If a target file already matches a plan's `after_fingerprint`, `apply_resource_change` must return an explicit no-op result.
- No-op apply must not write the target file, create a backup, insert a change log row, or report a committed write in the UI.
- Drift protection must still block files that differ from both the original `before_fingerprint` and the expected `after_fingerprint`.

- [x] **Step 1: Add RED Sync Engine idempotence test**

Added `apply_disable_mcp_json_entry_is_idempotent_for_the_same_plan`, asserting:

- first apply commits normally
- second apply of the same `WritePlan` returns `status = "noop"`
- no second backup is recorded
- no second change log row is recorded
- target file remains at the expected `preview_after` content

Focused run:

- `cargo test -p wapc apply_disable_mcp_json_entry_is_idempotent_for_the_same_plan -- --nocapture`

Result: FAIL before implementation because the second apply was treated as drift.

- [x] **Step 2: Implement no-op detection before drift failure**

Updated `apply_resource_change` so current target fingerprint equal to `plan.after_fingerprint` returns:

- `status = "noop"`
- `backup_path = null`
- no backup, write, verify, change log, or fingerprint update side effect

Focused verification:

- `cargo test -p wapc apply_disable_mcp_json_entry_is_idempotent_for_the_same_plan -- --nocapture` (PASS)
- `cargo test -p wapc sync_engine -- --nocapture` (PASS: 9 tests)

- [x] **Step 3: Keep UI wording honest for no-op results**

Added `formatApplyChangeNotice` and wired Resource Center apply feedback through it:

- committed results still show `已提交变更 <change_id>`
- no-op results show `写入计划已应用，当前文件已是目标状态，未产生新变更`

Focused run:

- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts`

Result: FAIL before implementation because `formatApplyChangeNotice` was missing; PASS after implementation.

- [x] **Step 4: Verify workspace guardrails**

Verification:

- `cargo test -p wapc apply_disable_mcp_json_entry_is_idempotent_for_the_same_plan -- --nocapture` (PASS)
- `cargo test -p wapc sync_engine -- --nocapture` (PASS: 9 tests)
- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts` (PASS: 30 tests)
- `cd ui && yarn test` (PASS: 39 tests)
- `cd ui && yarn lint` (PASS)
- `cd ui && yarn build` (PASS; Vite reports existing chunk-size warning)
- `cargo fmt --check` (PASS)
- `cargo clippy --workspace --all-targets -- -D warnings` (PASS)
- `cargo test --workspace` (PASS: 163 core tests, 29 Tauri helper tests, 0 main tests, 0 doctests)
- `git diff --check` (PASS)
- no-index whitespace check for touched untracked files (PASS)

## Task 135: Phase 4 Sync Apply No-op Visibility

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Phase 4 sync applies may receive target-level `noop` results from the Sync Engine when a target is already at the planned state.
- The UI must not collapse no-op targets into either committed writes or failures.
- No-op targets must remain non-rollbackable and must explain that no new change was produced.
- Do not add fake backend changes or fake target rows; this slice only makes the real returned status visible and honest.

- [x] **Step 1: Add RED sync no-op summary tests**

Extended `summarizes sync apply result targets with rollback eligibility` with a `noop` target and added `formatApplySyncNotice`.

The test asserts:

- `noopCount = 1`
- no-op target is not rollbackable
- no-op target gets reason `目标已是同步后的状态，未产生新变更`
- sync apply notice includes `已是最新 1 个目标`

Focused run:

- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts`

Result: FAIL before implementation because `formatApplySyncNotice` was missing; PASS after implementation.

- [x] **Step 2: Implement no-op sync summary and notice helpers**

Updated `buildSyncApplyResultSummary` to count no-op target statuses separately and provide a default no-op reason when the backend reason is empty.

Added `formatApplySyncNotice` so Resource Center toast copy lists committed, no-op, and failed target counts.

- [x] **Step 3: Render no-op count in the sync result panel**

Updated the sync result panel counter from:

- `成功 N · 失败 M`

to:

- `成功 N · 已是最新 K · 失败 M`

- [x] **Step 4: Verify UI guardrails**

Verification:

- `node --test --experimental-strip-types ui/tests/resourceCenterState.test.ts` (PASS: 30 tests)
- `cd ui && yarn test` (PASS: 39 tests)
- `cd ui && yarn lint` (PASS)
- `cd ui && yarn build` (PASS; Vite reports existing chunk-size warning)
- `git diff --check` (PASS)
- no-index whitespace check for touched untracked files (PASS)

## Task 136: Phase 4 Apply Sync No-op Backend Contract

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Reapplying the same sync plan to a target already at the planned state must be a real no-op, not a drift failure.
- `apply_sync` target results must not expose a fake `change_id` when no `resource_changes` row was created.
- No-op sync targets must not create new resource change rows or backups.
- Existing forged-plan, target fingerprint, scope, env, and deep-link safety checks must remain enforced.

- [x] **Step 1: Add RED backend no-op sync apply test**

Added `apply_sync_reports_reapplied_targets_as_noop_without_fake_change_ids`, asserting:

- first `apply_sync` commits normally
- second apply of the same plan returns one target with `status = "noop"`
- no-op target has `change_id = null`
- no-op target has `backup_path = null`
- no-op target has reason `target already matches planned state; no change was written`
- persisted `resource_changes` and `resource_backups` counts do not increase on reapply

Focused run:

- `cargo test -p wapc apply_sync_reports_reapplied_targets_as_noop_without_fake_change_ids -- --nocapture`

Result: FAIL before implementation because the second apply was first rejected as a target fingerprint failure; after allowing already-applied fingerprints it failed because the no-op target still exposed a fake change id.

- [x] **Step 2: Allow already-applied target fingerprints**

Updated apply-time target fingerprint validation to accept either:

- `plan.before_fingerprint`: target is still at planned pre-write state
- `plan.after_fingerprint`: target already matches planned post-write state

All other plan self-consistency and target metadata checks remain unchanged.

- [x] **Step 3: Map no-op sync target results honestly**

Updated `apply_sync` target result mapping so Sync Engine no-op results return:

- `status = "noop"`
- `change_id = null`
- `backup_path = null`
- a clear no-op reason

- [x] **Step 4: Verify backend and workspace guardrails**

Verification:

- `cargo test -p wapc apply_sync_reports_reapplied_targets_as_noop_without_fake_change_ids -- --nocapture` (PASS)
- `cargo test -p wapc cross_sync -- --nocapture` (PASS: 40 tests)
- `cd ui && yarn test` (PASS: 39 tests)
- `cd ui && yarn build` (PASS; Vite reports existing chunk-size warning)
- `cargo fmt --check` (PASS after running `cargo fmt`)
- `cargo clippy --workspace --all-targets -- -D warnings` (PASS)
- `cargo test --workspace` (initial run hit a Tauri helper `database is locked` race on the default local DB; targeted test passed)
- `cargo test --workspace -- --test-threads=1` (PASS: 164 core tests, 29 Tauri helper tests, 0 main tests, 0 doctests)
- `git diff --check` (PASS)
- no-index whitespace check for touched untracked files (PASS)

## Task 137: Tauri Snapshot Test Isolation from Local DB

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Tauri helper tests must not read from or write to the user's default local WAPC database.
- The tested snapshot path must stay on the same production logic as the runtime command.
- No fake snapshot records or test-only runtime branches are allowed.
- The test must prove its `home_path` and `db_path` are isolated temporary paths.

- [x] **Step 1: Extract path-parameterized snapshot loader**

Extracted `get_snapshot_for_paths(home, db)` from the Tauri `get_snapshot` command.
The command still resolves the platform home at runtime and calls the same helper with the default app paths.

- [x] **Step 2: Move bootstrap snapshot test to an isolated temp home**

Updated `get_snapshot_returns_desktop_bootstrap_data` to create a temporary home and `.wapc/wapc.db` path, then assert the returned snapshot points to those isolated paths.

- [x] **Step 3: Verify Tauri helper and workspace guardrails**

Verification:

- `cargo test -p wapc-app get_snapshot_returns_desktop_bootstrap_data -- --nocapture` (PASS)
- `cargo test -p wapc-app -- --test-threads=1` (PASS: 29 Tauri helper tests, 0 main tests, 0 doctests)
- `cargo fmt --check` (PASS after running `cargo fmt`)
- `cargo clippy --workspace --all-targets -- -D warnings` (PASS)
- `cargo test --workspace -- --test-threads=1` (PASS: 164 core tests, 29 Tauri helper tests, 0 main tests, 0 doctests)
- `git diff --check` (PASS)
- no-index whitespace check for touched files (PASS)

## Task 138: Codex TOML Sync Support Documentation Boundary

**Files:**
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- The adapter matrix must not list already implemented and tested Codex TOML cross-tool sync as unsupported.
- The unsupported boundary must still remain explicit for OpenCode, VS Code, enterprise, non-macOS, instruction/frontmatter, skill/plugin, and subagent writes.
- This is a documentation contract correction only; no runtime behavior changes.

- [x] **Step 1: Verify implementation support**

Confirmed backend support through `cross_sync::plan_sync` for `("codex", "toml")` and existing tests such as `plan_sync_generates_codex_toml_plan_and_apply_writes_mcp_server`.

Confirmed frontend target generation includes `~/.codex/config.toml` with `format = "toml"` in `buildSyncTargetOptions`.

- [x] **Step 2: Correct adapter matrix wording**

Updated the write-path evidence note so it states cross-tool sync supports Claude/Cursor/Gemini JSON MCP and Codex TOML MCP preview/write, while unsupported write categories remain explicitly listed.

- [x] **Step 3: Verify documentation guardrails**

Verification:

- `rg -n "Codex TOML sync preview|cross-tool sync|跨工具 JSON/TOML MCP sync|unsupported" docs/design/tool-adapter-matrix.md` (PASS: no stale `Codex TOML sync preview` unsupported wording remains)
- `git diff --check` (PASS)
- no-index whitespace check for touched docs (PASS)

## Task 139: README and Changelog Roadmap State Alignment

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- User-facing open-source entry docs must not describe already implemented Resource Center and cross-tool sync capabilities as only long-term roadmap items.
- Release wording must stay honest: macOS signing/notarization gates exist, but official download readiness still depends on real Apple Developer credentials and clean-machine Gatekeeper verification.
- Non-macOS write support must remain explicitly unsupported until real platform fixture and rollback e2e evidence exists.

- [x] **Step 1: Align README roadmap with current implementation**

Updated the README roadmap from generic near/mid/long-term buckets to:

- implemented local capabilities
- macOS release gate closure
- Windows/Linux follow-up evaluation with unsupported write boundaries

- [x] **Step 2: Expand Unreleased changelog**

Added release-note coverage for Resource Center inventory, Sync Engine, cross-tool MCP sync, template/deep-link flows, privacy/export surfaces, Headless read-only dashboard, macOS release gates, and the latest documentation/test fixes.

- [x] **Step 3: Verify public docs wording**

Verification:

- `rg -n "已落地|发布收口|后续评估|Cross-tool MCP Sync|Resource Center|unsupported|下载即用" README.md CHANGELOG.md` (PASS)
- `git diff --check` (PASS)
- no-index whitespace check for touched docs (PASS after removing an existing trailing space in `CHANGELOG.md`)

## Task 140: README Non-invasive Write Boundary Alignment

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- README must not promise WAPC never modifies tool files while the product supports confirmed Sync Engine writes.
- The non-invasive positioning must stay true: no command replacement, no CLI wrapper, no proxy/certificate interception, default read-only detection.
- Any external tool file write must be described as explicitly confirmed and routed through preview, backup, atomic write, verify, and rollback records.

- [x] **Step 1: Replace absolute no-write wording**

Changed the intro from "不动你的工具文件" to an explicit safe-write boundary: only confirmed management actions after diff preview may modify target tool files through the Sync Engine safety chain.

- [x] **Step 2: Clarify the non-invasive principle**

Changed the principle bullet from "不修改任何 AI 工具文件" to "默认只读识别; external writes require preview confirmation, backup, atomic write, verify, and rollback records."

- [x] **Step 3: Verify README wording**

Verification:

- `rg -n "不动你的工具文件|不修改任何 AI 工具文件|预览 diff|默认只读识别|原子写|回滚" README.md` (PASS: old absolute no-write wording is absent; confirmed safe-write wording is present)
- `git diff --check` (PASS)
- no-index whitespace check for touched docs (PASS)

## Task 141: CSV Export Formula Injection Guard

**Files:**
- Modify: `src/export.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- CSV exports are user-opened artifacts and must not emit cells that spreadsheet tools can interpret as formulas.
- JSON and Markdown report semantics must remain unchanged.
- Existing CSV escaping for commas, quotes, CR/LF, and ordinary text must remain stable.

- [x] **Step 1: Add RED formula-prefix CSV test**

Added `csv_cells_neutralize_spreadsheet_formula_prefixes`, covering cells beginning with:

- `=`
- `+`
- `-`
- `@`
- leading whitespace followed by a formula prefix
- tab followed by a formula prefix

Focused run:

- `cargo test -p wapc csv_cells_neutralize_spreadsheet_formula_prefixes -- --nocapture`

Result: FAIL before implementation because `csv_cell` emitted `=cmd|' /C calc'!A0` unchanged.

- [x] **Step 2: Neutralize formula-risk CSV cells**

Updated `csv_cell` to prefix formula-risk values with a literal apostrophe before CSV quoting.

Updated quoting rules to quote tab-containing cells as well as comma, quote, CR, and LF cells.

- [ ] **Step 3: Verify export and workspace guardrails**

Verification so far:

- `cargo test -p wapc csv_cells_neutralize_spreadsheet_formula_prefixes -- --nocapture` (PASS)
- `cargo test -p wapc export::tests -- --nocapture` (PASS: 12 tests)

Planned verification:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace -- --test-threads=1`
- `git diff --check`
- no-index whitespace check for touched files

## Task 28: Phase 4 Cross-Tool Sync Plan Foundation

**Files:**
- Create: `src/cross_sync.rs`
- Modify: `src/model.rs`
- Modify: `src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement only WP4.1: `plan_sync` for real persisted canonical resources and real target file paths.
- Generate preview plans for the concrete safe subset: non-env MCP resources into JSON MCP targets (`mcpServers`).
- Do not write files in this task; `apply_sync`, sync history, presets, and UI controls remain later tasks.
- Do not fake TOML/Codex or unsupported targets. Return explicit unsupported target results with reasons.
- Enforce Phase 4 safety boundaries at plan time: missing source resource, unsupported kind, unsupported env strategy, env values requiring manual input, enterprise target, and cross-scope target without explicit opt-in must all be visible as target errors.

- [x] **Step 1: Add red cross-sync plan tests**

Add tests for:

- `plan_sync_json_mcp_generates_one_plan_per_supported_target_without_writing`
- `plan_sync_reports_unsupported_targets_without_fake_plans`
- `plan_sync_rejects_cross_scope_by_default`
- `plan_sync_requires_manual_env_when_target_lacks_existing_value`

Run: `cargo test -p wapc cross_sync`
Result: FAIL before implementation because `plan_sync`, `PlanSyncRequest`, and `SyncTarget` did not exist; PASS after implementation with 4 tests.

- [x] **Step 2: Implement Phase 4 plan models**

Add serializable models:

- `SyncTarget`
- `PlanSyncRequest`
- `SyncTargetPlan`
- `PlanSyncResult`

- [x] **Step 3: Implement cross-sync plan engine**

Add `src/cross_sync.rs` with:

- lookup of source `CanonicalResource` from SQLite
- JSON MCP target rendering under `mcpServers`
- capability/format/scope/env guardrails
- per-target result status: `planned`, `unsupported`, or `requires_input`
- plan-only behavior that records fingerprints but never writes target files

- [x] **Step 4: Expose Tauri command and frontend types**

Expose `plan_sync` as a Tauri command and mirror its request/result types in TypeScript. No UI buttons yet.

- [x] **Step 5: Verify**

Run: `cargo test -p wapc cross_sync`
Result: PASS, 4 Phase 4 plan tests.

Run: `cargo test -p wapc-app plan_sync_helper`
Result: FAIL before command helper implementation, then PASS with `plan_sync_helper_generates_cross_tool_preview_without_writing`.

Run: `cargo test --workspace`
Result: PASS, 64 core tests, 10 Tauri tests, 0 main tests, 0 doctests.

Run: `cd ui && yarn test`
Result: PASS, 7 tests, with Node `module.register()` deprecation warning.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `cargo fmt --check`
Result: PASS.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 29: Phase 4 Apply Sync Execution and History Foundation

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `src/sync_engine.rs`
- Modify: `src/model.rs`
- Modify: `src/store.rs`
- Modify: `src/privacy.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement `apply_sync` for already generated `WritePlan`s from Task 28.
- Each target must execute through the existing Sync Engine backup / atomic write / verify / rollback-on-failure path.
- Targets must be independent: one target drift/failure must not stop another target from committing.
- Persist one `sync_operations` row and tag each successful or failed `resource_changes` row with `sync_id`.
- Keep env values out of scope for this slice; `apply_sync` must not accept or persist secret env values yet.
- Do not add UI controls yet.

- [x] **Step 1: Add red apply-sync tests**

Add tests for:

- `apply_sync_commits_successful_targets_and_isolates_drift_failures`
- `apply_sync_records_sync_operation_and_change_sync_ids`
- `apply_sync_command_helper_uses_cross_sync_without_real_home`

Run: `cargo test -p wapc apply_sync`
Result: FAIL before implementation because `apply_sync`, `ApplySyncRequest`, `list_sync_operations`, and `ResourceChangeLog.sync_id` did not exist; PASS after implementation.

Run: `cargo test -p wapc-app apply_sync_command`
Result: FAIL before command helper implementation; PASS after exposing the command bridge helper.

- [x] **Step 2: Add Phase 4 apply models and persistence**

Add serializable models:

- `ApplySyncRequest`
- `ApplySyncTargetResult`
- `ApplySyncResult`
- `SyncOperation`

Add SQLite persistence:

- `sync_operations`
- nullable `resource_changes.sync_id`
- list/insert methods for sync operations

- [x] **Step 3: Implement batch apply via Sync Engine**

Update Sync Engine verification so `op=sync` verifies the final file fingerprint and MCP entry presence, while `op=disable` keeps removal verification. Add `cross_sync::apply_sync` that applies each target independently and records per-target results.

- [x] **Step 4: Expose Tauri command and frontend types**

Expose `apply_sync` and `list_sync_operations` as Tauri commands and mirror the types in TypeScript. No UI controls yet.

- [x] **Step 5: Verify**

Run: `cargo test -p wapc apply_sync`
Result: PASS, 2 apply-sync tests.

Run: `cargo test -p wapc-app apply_sync_command`
Result: PASS, 1 Tauri command helper test.

Run: `cargo test -p wapc phase_four_sync_metadata`
Result: FAIL before privacy audit updates; PASS after adding `sync_operations`, `sync_id`, and env boundary wording.

Run: `cargo test --workspace`
Result: PASS, 67 core tests, 11 Tauri tests, 0 main tests, 0 doctests.

Run: `cd ui && yarn test`
Result: PASS, 7 tests, with Node `module.register()` deprecation warning.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `cargo fmt --check`
Result: PASS.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 30: Phase 4 Env Strategy Memory-Only Sync

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `src/model.rs`
- Modify: `src/privacy.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 4 WP4.2 for JSON MCP targets only.
- `plan_sync` may return env placeholders and required env keys, but must not return raw secret env values in preview data.
- `apply_sync` may accept manual `env_values` and use them only in memory while materializing the final write plan.
- `reuse` must read existing target env values at apply time without exposing them in preview output.
- `skip` must write explicit empty env placeholders for requested keys.
- No env values may be stored in `sync_operations`, `resource_changes`, backup metadata, or privacy audit fields.
- No UI controls yet.

- [x] **Step 1: Add red env strategy tests**

Add tests for:

- `plan_sync_reuses_existing_env_without_exposing_secret_in_preview`
- `apply_sync_manual_env_uses_memory_value_without_persisting_secret`
- `apply_sync_skip_env_writes_empty_placeholder`

Run: `cargo test -p wapc env_strategy`
Result: FAIL before implementation because `ApplySyncRequest.env_values` did not exist; PASS after implementation with 3 env strategy tests.

- [x] **Step 2: Extend request/result models**

Add memory-only `ApplySyncRequest.env_values` and plan env metadata fields required by the UI.

- [x] **Step 3: Implement env materialization**

Implement plan placeholders and `apply_sync` materialization for `reuse`, `manual`, and `skip`, recomputing fingerprints and diff after materialization but before entering Sync Engine.

- [x] **Step 4: Update command bridge, frontend types, and privacy tests**

Expose the new request fields through Tauri and TypeScript, and verify the privacy audit still states env values never persist.

- [x] **Step 5: Verify**

Run: `cargo test -p wapc env_strategy`
Result: PASS, 3 env strategy tests.

Run: `cargo test -p wapc apply_sync`
Result: PASS, 4 apply-sync tests.

Run: `cargo test --workspace`
Result: PASS, 70 core tests, 11 Tauri tests, 0 main tests, 0 doctests.

Run: `cd ui && yarn test`
Result: PASS, 7 tests, with Node `module.register()` deprecation warning.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `cargo fmt --check`
Result: PASS.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 31: Phase 4 Resource Center Batch Sync Preview UI

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Add a visible Resource Center "同步到…" workflow backed by real Tauri commands `plan_sync` and `apply_sync`.
- Generate target candidates from real local paths under `snapshot.home_path` for currently supported JSON MCP adapters only: Claude, Gemini, and Cursor.
- Show unsupported/requires-input target results returned by backend; do not pretend TOML/Codex or missing target files are supported.
- Manual env values are entered only in the apply modal and passed to `apply_sync`; they must not be stored in React state longer than the modal lifecycle after apply/cancel.
- Do not implement sync presets, project-scope target selection, or Codex TOML write UI in this task.

- [x] **Step 1: Add red frontend sync helper tests**

Add tests for:

- `buildSyncTargetOptions` returns real JSON MCP targets from home path and excludes source tool.
- `canPlanResourceSync` enables only user-scope MCP resources for Phase 4 UI.
- `syncPlanHasApplicableTargets` returns true only when backend returned at least one planned target.

Run: `cd ui && yarn test`
Result: FAIL before helper implementation because `buildSyncTargetOptions` was not exported; PASS after implementation with 10 tests.

- [x] **Step 2: Implement helper functions**

Add pure state helpers for target options, sync eligibility, applicable plan status, and default manual env values.

- [x] **Step 3: Add Resource Center sync panel and batch preview dialog**

Add controls for target selection, env strategy, plan preview, target statuses, per-target diff panes, manual env values, and apply result messages. Use only real `invoke('plan_sync')` / `invoke('apply_sync')` calls.

- [x] **Step 4: Rendered UI QA**

Run the local frontend through a deterministic Tauri invoke harness or Browser path and verify Resource Center renders the sync panel, preview dialog, unsupported target messaging, and apply controls without console errors.

Rendered QA evidence:

- Served production `ui/dist` from `/tmp/wapc-ui-qa` with a temporary `window.__TAURI_INTERNALS__.invoke` harness.
- Browser page identity: `http://localhost:4177/`, title `WAPC — AI Coding Token Observer`.
- Not blank: DOM contains WAPC shell and overview content.
- Resource Center navigation renders `资源盘点`, `跨工具同步`, target paths `.gemini/settings.json` and `.cursor/mcp.json`.
- Manual env strategy + `同步到...` opens `同步预览`.
- Preview shows one `planned` target, one `unsupported` target, `GITHUB_TOKEN`, `确认同步`, and no raw secret text.
- Filling manual env and confirming sync closes the dialog, shows `同步完成 sync:qa，成功 1 个目标`, and does not render the entered secret after close.
- Console health: no app errors; existing Recharts overview warning remains outside this Resource Center flow.
- Screenshot capture through Browser failed with `Page.captureScreenshot` timeout, so visual evidence is DOM/interaction based for this run.

- [x] **Step 5: Verify**

Run: `cd ui && yarn test`
Result: PASS, 10 tests, with Node `module.register()` deprecation warning.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cargo test --workspace`
Result: PASS, 70 core tests, 11 Tauri tests, 0 main tests, 0 doctests.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `cargo fmt --check`
Result: PASS.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 34: Phase 4 Sync Preset JSON Export

**Files:**
- Modify: `src/export.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement FR-42 export for real local `sync_presets` stored in SQLite.
- Export only schema metadata, resource ids, target metadata, names, ids, and timestamps.
- Do not export env values or key material.
- Do not implement preset import, fake presets, or default path guessing in this task.

- [x] **Step 1: Add red core export test**

Added `exports_sync_presets_as_json_without_secret_values`.

Run: `cargo test -p wapc exports_sync_presets_as_json_without_secret_values`
Result: FAIL before implementation because `export_sync_presets` did not exist; PASS after implementation.

- [x] **Step 2: Implement JSON export**

Added `export::export_sync_presets(store, path)` that:

- Reads real presets from SQLite.
- Parses `resources_json` into `resources`.
- Parses `targets_json` into `targets`.
- Writes stable pretty JSON with schema `wapc.sync_presets.v1`.
- Returns `ExportReportResult { path, bytes_written }`.

- [x] **Step 3: Add red Tauri helper test**

Added `export_sync_preset_helper_writes_json_without_real_home`.

Run: `cargo test -p wapc-app export_sync_preset_helper_writes_json_without_real_home`
Result: FAIL before helper implementation because `export_sync_presets_for_path` did not exist; PASS after helper and command implementation.

- [x] **Step 4: Expose Tauri command**

Added and registered `export_sync_presets(path) -> ExportReportResult`.

- [x] **Step 5: Add Resource Center export UI**

Resource Center sync preset panel now:

- Accepts an explicit JSON export path from the user.
- Enables export only when at least one preset exists and a path is present.
- Calls real `invoke('export_sync_presets', { path })`.
- Displays backend-returned path and byte count.

- [x] **Step 6: Rendered UI QA**

Rendered QA evidence:

- Served production `ui/dist` from `/tmp/wapc-ui-qa` with a temporary `window.__TAURI_INTERNALS__.invoke` harness.
- Browser page identity: `http://localhost:4177/index.html`, title `WAPC — AI Coding Token Observer`.
- Not blank: DOM contains WAPC shell and Resource Center navigation.
- Resource Center renders `Existing Gemini preset`, `导出 JSON 路径`, and a scoped preset `导出` button.
- Filling `/tmp/wapc-sync-presets-export.json` and clicking the scoped export button shows `已导出同步预设 /tmp/wapc-sync-presets-export.json，512 bytes`.
- Existing preset remains visible after export.
- QA secret `qa-export-secret-value` is not present in DOM.
- Console health: no app errors; existing Recharts overview warning remains outside this Resource Center flow.
- Browser screenshot capture failed with `Page.captureScreenshot` timeout, so visual evidence is DOM/interaction based for this run.

- [x] **Step 7: Verify**

Run: `cargo test -p wapc exports_sync_presets_as_json_without_secret_values`
Result: PASS, 1 sync preset export test.

Run: `cargo test -p wapc-app export_sync_preset_helper_writes_json_without_real_home`
Result: PASS, 1 Tauri sync preset export helper test.

Run: `cargo test -p wapc sync_presets`
Result: PASS, 2 sync preset tests.

Run: `cd ui && yarn test`
Result: PASS, 16 tests, with Node `module.register()` deprecation warning.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cargo test --workspace`
Result: PASS, 72 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `cargo fmt --check`
Result: PASS.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 35: Phase 4 Explicit Cross-Scope Authorization Propagation

**Files:**
- Modify: `src/model.rs`
- Modify: `src/cross_sync.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Preserve Phase 4 FR-31 semantics: cross-scope sync remains opt-in and visible.
- Persist the actual apply-time `allow_cross_scope` flag in `sync_operations`.
- Keep current Resource Center write controls limited to real user-scope JSON MCP targets; do not add fake project target rows.
- Add a UI state/helper foundation so future project targets require explicit cross-scope authorization before plan/apply.
- Do not store env values or key material.

- [x] **Step 1: Add cross-scope apply persistence coverage**

Added `apply_sync_persists_explicit_cross_scope_authorization`, covering a project-scope source planned to a user-scope JSON MCP target with explicit `allow_cross_scope = true`.

Run: `cargo test -p wapc apply_sync_persists_explicit_cross_scope_authorization`
Result: PASS, 1 cross-scope apply persistence test.

- [x] **Step 2: Propagate allow_cross_scope through apply_sync**

Added `ApplySyncRequest.allow_cross_scope` and changed `cross_sync::apply_sync` to persist the request value into `sync_operations.allow_cross_scope`. Existing Rust and Tauri helper callers pass `false` unless explicitly testing cross-scope authorization.

Run: `cargo test -p wapc cross_sync`
Result: PASS, 10 cross-sync tests.

Run: `cargo test -p wapc-app apply_sync_command`
Result: PASS, 1 Tauri apply-sync helper test.

- [x] **Step 3: Add Resource Center cross-scope policy helper and UI state**

Added `selectedSyncTargetsRequireCrossScope(resource, targets, selectedTools)` and test coverage for same-scope and mixed-scope selections. Resource Center now:

- Tracks `syncAllowCrossScope` separately from selected targets.
- Sends `allow_cross_scope=false` for same-scope plan/apply requests.
- Requires the explicit "允许跨 Scope" checkbox before cross-scope plan/apply can run when future project targets are available.
- Keeps current generated target options user-scope only, avoiding fake project-scope write targets.

Run: `cd ui && yarn test`
Result: PASS, 17 tests, with Node `module.register()` deprecation warning.

- [x] **Step 4: Rendered Resource Center QA**

Rendered QA evidence:

- Served production `ui/dist` from `/tmp/wapc-ui-qa` with a temporary `window.__TAURI_INTERNALS__.invoke` harness.
- Browser page identity: `http://127.0.0.1:4177/index.html`, title `WAPC — AI Coding Token Observer`.
- Resource Center renders `资源盘点`, `跨工具同步`, `允许跨 Scope`, `当前所选目标同 Scope`, `Existing Gemini preset`, and existing history label `跨 Scope`.
- Clicking `同步到...` opens `同步预览`; clicking `确认同步` shows `同步完成 sync:applyqa`.
- Same-scope plan/apply requests both carry `allow_cross_scope=false`.
- QA secret strings are absent from DOM.
- Console health: no app errors; existing Recharts width/height warning remains outside this Resource Center flow.
- Screenshot saved to `/tmp/wapc-cross-scope-resource-center.png`.

- [x] **Step 5: Verify**

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cargo test --workspace`
Result: PASS, 73 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 36: Phase 4 Codex TOML Cross-Tool Sync Target

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `src/sync_engine.rs`
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement the Phase 4 AC-1 format split for user-scope MCP sync: Codex target uses `~/.codex/config.toml`, Cursor/Gemini/Claude targets continue using JSON MCP files.
- Generate real TOML `WritePlan` previews for URL-based MCP resources under `[mcp_servers.<name>]`.
- Apply Codex TOML plans through the existing Sync Engine backup / atomic write / fingerprint verify path.
- Keep unsupported command MCP reconstruction explicit; do not fake command/args from redacted payloads.
- Do not add project-scope target UI in this task.

- [x] **Step 1: Add red backend Codex TOML test**

Added `plan_sync_generates_codex_toml_plan_and_apply_writes_mcp_server`.

Run: `cargo test -p wapc plan_sync_generates_codex_toml_plan_and_apply_writes_mcp_server`
Result: FAIL before implementation because Codex TOML targets returned `unsupported`; PASS after implementation.

- [x] **Step 2: Implement TOML MCP planning and apply verification**

Added Codex TOML support in `cross_sync::plan_sync`:

- `tool=codex`, `format=toml` maps to `~/.codex/config.toml` style TOML.
- URL MCP resources become `[mcp_servers.<name>]` TOML entries with `url` and `type`.
- Env placeholders use the same memory-only materialization path for TOML values.
- Unsupported format/tool combinations remain explicit target errors.

Updated Sync Engine verification so TOML sync plans verify `mcp_servers.<resource_name>` instead of parsing every sync target as JSON.

Run: `cargo test -p wapc cross_sync`
Result: PASS, 11 cross-sync tests.

Run: `cargo test -p wapc sync_engine`
Result: PASS, 6 Sync Engine tests.

- [x] **Step 3: Add Resource Center Codex target**

Resource Center target construction now includes real Codex user-scope TOML target:

- `tool=codex`
- `target_path=<home>/.codex/config.toml`
- `format=toml`

Existing source-tool filtering still removes the source tool from target options.

Run: `cd ui && yarn test`
Result: PASS, 17 tests, with Node `module.register()` deprecation warning.

- [x] **Step 4: Rendered Resource Center QA**

Rendered QA evidence:

- Served production `ui/dist` from `/tmp/wapc-ui-qa` with a temporary `window.__TAURI_INTERNALS__.invoke` harness.
- Browser page identity: `http://127.0.0.1:4177/index.html`, title `WAPC — AI Coding Token Observer`.
- Flow: Resource Center opens for a Claude MCP source; Codex target is visible at `/Users/example/.codex/config.toml`.
- Explicitly selected Codex and Cursor while leaving Gemini unchecked.
- `plan_sync` request targets were exactly:
  - `codex:toml:/Users/example/.codex/config.toml`
  - `cursor:json:/Users/example/.cursor/mcp.json`
- Preview rendered TOML/Cursor evidence including `[mcp_servers.docs]` and Cursor JSON target path.
- `apply_sync` request plans were exactly `codex` and `cursor`, with `allow_cross_scope=false`.
- Confirmation showed `同步完成 sync:codexcursorqa`.
- QA secret strings were absent from DOM.
- Console health: no app errors; existing Recharts width/height warning remains outside this Resource Center flow.
- Screenshot saved to `/tmp/wapc-codex-cursor-resource-center.png`.

- [x] **Step 5: Verify**

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cargo test --workspace`
Result: PASS, 74 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 37: Phase 4 Project Source Cross-Scope Resource Center Flow

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 4 AC-6 at the Resource Center UI layer for project-scope MCP sources.
- Project MCP resources may enter sync planning, but generated user-scope targets require explicit "允许跨 Scope" authorization.
- Enterprise/managed resources remain read-only in the sync UI.
- Keep targets real user-scope config files only; do not create fake project target rows.
- Continue relying on backend `plan_sync` enforcement for final cross-scope rejection/allow semantics.

- [x] **Step 1: Add red frontend policy tests**

Added tests requiring:

- `canPlanResourceSync` enables user and project MCP resources.
- Enterprise and managed resources remain disabled.
- Project MCP source resources build real user target options and require cross-scope authorization.

Run: `cd ui && yarn test`
Result: FAIL before implementation because project scope resources were disabled and target options were empty; PASS after implementation.

- [x] **Step 2: Open project MCP source sync capability**

Updated Resource Center state helpers:

- `canPlanResourceSync` enables `scope=user` and `scope=project` MCP resources.
- `scope=enterprise` and `scope=managed` return read-only capability copy.
- Project source reason explicitly says syncing to user targets requires cross-scope authorization.
- Existing `selectedSyncTargetsRequireCrossScope` marks project source to user target selection as requiring authorization.

Run: `cd ui && yarn test`
Result: PASS, 18 tests, with Node `module.register()` deprecation warning.

- [x] **Step 3: Verify backend cross-scope guard still holds**

Run: `cargo test -p wapc cross_scope`
Result: PASS, covering default cross-scope rejection and explicit allow persistence.

Run: `cargo test -p wapc cross_sync`
Result: PASS, 11 cross-sync tests.

- [x] **Step 4: Rendered Resource Center QA**

Rendered QA evidence:

- Served production `ui/dist` from `/tmp/wapc-ui-qa` with a temporary `window.__TAURI_INTERNALS__.invoke` harness.
- Browser page identity: `http://127.0.0.1:4177/index.html`, title `WAPC — AI Coding Token Observer`.
- Flow: Resource Center opens for project MCP source `/Users/example/project/.cursor/mcp.json`.
- Project source renders sync panel copy `project 资源同步到 user 目标前需显式允许跨 Scope`.
- Before checking "允许跨 Scope", the `同步到...` button is disabled.
- The cross-scope checkbox is enabled and shows `本次目标跨 user/project，需要显式授权`.
- After checking it, the `同步到...` button becomes enabled.
- `plan_sync` request carries `allow_cross_scope=true`.
- `apply_sync` request carries `allow_cross_scope=true`.
- Request targets were real user configs:
  - `codex:user:toml:/Users/example/.codex/config.toml`
  - `claude:user:json:/Users/example/.claude.json`
- Confirmation showed `同步完成 sync:projectscopeqa`.
- QA secret strings were absent from DOM.
- Console health: no app errors; existing Recharts width/height warning remains outside this Resource Center flow.
- Screenshot saved to `/tmp/wapc-project-cross-scope-resource-center.png`.

- [x] **Step 5: Verify**

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cargo test --workspace`
Result: PASS, 74 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 38: Phase 4 Per-Target Sync Result Rollback UI

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 4 AC-7 at the Resource Center sync result layer.
- After `apply_sync`, show real per-target results from backend `ApplySyncResult`.
- Successful target rows with `change_id` and `backup_path` can call the real `rollback_change` command directly.
- Failed target rows show backend reason and do not expose rollback as available.
- Do not mutate resources locally; after rollback, refresh history and inventory through backend commands.

- [x] **Step 1: Add red sync apply result summary test**

Added `buildSyncApplyResultSummary` contract coverage:

- Summarizes `sync_id`, committed count, failed count.
- Preserves per-target plan id, target path, change id, backup path, reason.
- Marks only committed targets with a change id and backup path as rollbackable.

Run: `cd ui && yarn test`
Result: FAIL before implementation because `buildSyncApplyResultSummary` did not exist; PASS after implementation.

- [x] **Step 2: Add Resource Center sync result panel**

Resource Center now:

- Stores the last real `ApplySyncResult` after `apply_sync`.
- Renders a `同步结果` panel with sync id, success/failure counts, per-target status, path, change id, and backend reason.
- Shows per-target rollback buttons only when `buildSyncApplyResultSummary` marks the target rollbackable.
- Calls real `rollback_change` with the selected target `change_id`.
- Clears the transient sync result and refreshes history/inventory after rollback.

Run: `cd ui && yarn test`
Result: PASS, 19 tests, with Node `module.register()` deprecation warning.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

- [x] **Step 3: Rendered Resource Center QA**

Rendered QA evidence:

- Served production `ui/dist` from `/tmp/wapc-ui-qa` with a temporary `window.__TAURI_INTERNALS__.invoke` harness.
- Browser page identity: `http://127.0.0.1:4177/index.html`, title `WAPC — AI Coding Token Observer`.
- Flow: Resource Center opens for a Claude MCP source, selects Codex and Cursor targets, plans sync, confirms sync, then uses the `同步结果` panel rollback control.
- `apply_sync` request plans were exactly `codex` and `cursor`.
- The `同步结果` panel showed `成功 2`, `chg:codex`, and `chg:cursor`.
- Clicking the first target rollback button called `rollback_change` with only `chg:codex`.
- Confirmation showed `已回滚变更 chg:codex，新记录 rollback:chg:codex`.
- Cursor was not rolled back; recorded rollback calls were exactly `["chg:codex"]`.
- QA secret strings were absent from DOM.
- Console health: no app errors; existing Recharts width/height warning remains outside this Resource Center flow.
- Screenshot saved to `/tmp/wapc-sync-result-rollback.png`.

- [x] **Step 4: Verify**

Run: `cargo test --workspace`
Result: PASS, 74 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 39: Phase 4 Privacy Audit Sync Preset and Target Write Boundaries

**Files:**
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 4 WP4.6 / AC-8 privacy-audit coverage for current sync features.
- Explicitly disclose cross-tool sync target writes to Claude, Codex, Gemini, and Cursor MCP config files.
- Explicitly disclose `sync_presets` metadata storage and JSON export boundaries.
- Keep env values, raw target secrets, key material, prompt/response/source/tool-output bodies, and backup contents out of the database/export boundary.
- Do not claim cloud/provider privacy controls; this is local audit metadata only.

- [x] **Step 1: Add red privacy audit coverage**

Added `privacy_audit_names_phase_four_sync_preset_export_and_target_write_boundaries`, requiring:

- Sync target sources for Codex and Cursor config files with `writes_source=true`.
- `sync_presets` table fields for resource ids, target metadata, no env values, and no key material.
- Export boundary copy for sync preset JSON exports excluding env values and key material.
- Forbidden fields for sync preset env values and key material.

Run: `cargo test -p wapc privacy_audit_names_phase_four_sync_preset_export_and_target_write_boundaries`
Result: FAIL before implementation because sync target writes and `sync_presets` were not disclosed; PASS after implementation.

- [x] **Step 2: Update privacy audit report**

Updated `privacy_audit` to include:

- `Claude MCP sync target`, `Codex MCP sync target`, `Gemini MCP sync target`, and `Cursor MCP sync target` entries with `reads_body=true` and `writes_source=true`.
- `sync_presets` stored table with `resources_json resource ids`, `targets_json metadata`, `no env values`, and `no key material`.
- Export boundary wording that cross-tool sync writes MCP target config files only after explicit confirmation.
- Export boundary wording that sync preset JSON exports exclude env values and key material.
- Forbidden fields: `sync preset env value`, `sync preset key material`, and `sync history raw target secret`.

Run: `cargo test -p wapc privacy`
Result: PASS, 8 privacy tests.

- [x] **Step 3: Verify**

Run: `cargo test --workspace`
Result: PASS, 75 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 72: MCP Field and Transport Official Documentation Verification

**Files:**
- Add: `docs/design/mcp-field-verification.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Verify MCP field names and transport values against official documentation, not secondary blog posts.
- Keep runtime connection, OAuth, headers, actual local path side effects, and Windows/Linux behavior out of scope unless separately verified.
- Do not expand WAPC write support from documentation evidence alone.

- [x] **Step 1: Add official-doc verification artifact**

Created `docs/design/mcp-field-verification.md`.

The document records official-source evidence for:

- Claude Code: `mcpServers`, JSON `type` + `url`, `http`, `stdio`, and `streamable-http` alias behavior.
- Codex: `~/.codex/config.toml`, `[mcp_servers.<name>]`, and remote `url` from OpenAI Docs MCP setup.
- Gemini CLI: `settings.json` top-level `mcpServers`, `command`, `url`, `httpUrl`, `env`, `headers`, and `stdio` / `sse` / `http` transport flags.
- Cursor: global/project `mcp.json`, top-level `mcpServers`, `command`, `args`, `env`, `url`, `headers`, and `stdio` / `SSE` / `Streamable HTTP` transport descriptions.
- VS Code Copilot: `.vscode/mcp.json`, top-level `servers`, `type: "http"`, and `url` from the OpenAI Docs MCP page.

- [x] **Step 2: Update adapter matrix**

Updated `docs/design/tool-adapter-matrix.md` to mark `MCP 字段名与 transport 取值核验` complete.

Added a boundary note that this is official-document field/transport verification only. It does not prove current-machine runtime connection, OAuth/header behavior, all client versions, or Windows/Linux compatibility.

Updated the remaining verification list so Codex/Gemini field names are no longer listed as pure unknowns; remaining work is runtime connection/version compatibility plus OAuth/header/env expansion behavior.

- [x] **Step 3: Verify docs and guardrails**

Run: `rg -n "mcp-field-verification|MCP 字段名|运行态|OAuth|headers|transport|待核验各工具|字段命名需核验" docs/design/tool-adapter-matrix.md docs/design/mcp-field-verification.md`
Result: PASS after removing stale `待核验各工具 type 取值` and `Codex 字段命名需核验` wording.

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 119 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 71: Cursor Rule Frontmatter Generation and Parse Round Trip

**Files:**
- Modify: `src/resources.rs`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Add a deterministic in-memory generator for Cursor `.mdc` rule frontmatter.
- Verify generated content can be scanned by the real Resource Inventory path.
- Persist only safe metadata and fingerprints after scanning; raw body text and raw description text must not enter `payload_json`.
- Do not open instruction/frontmatter writes in this slice. Write support must still go through a later Sync Engine preview/backup/write/verify/rollback task.

- [x] **Step 1: Add red generation/parse test**

Added `renders_cursor_rule_frontmatter_that_scanner_can_parse_without_persisting_body`.

The test:

- calls `render_cursor_rule_document` with description, globs, `alwaysApply=true`, and a Markdown body
- writes the generated text to a temporary `.cursor/rules/generated.mdc`
- scans it through real `scan_inventory`
- requires `cursor-rules-frontmatter-v1`, `globs`, `always_apply`, and `description_fingerprint` in `payload_json`
- asserts raw description text and raw body text are absent from persisted payload

Run: `cargo test -p wapc renders_cursor_rule_frontmatter_that_scanner_can_parse_without_persisting_body -- --nocapture`
Result: FAIL before implementation because `render_cursor_rule_document` did not exist; PASS after implementation.

- [x] **Step 2: Add Cursor rule renderer**

Added `render_cursor_rule_document(description, globs, always_apply, body)`.

The renderer:

- validates non-empty description, globs, and body
- rejects empty or multi-line glob values
- emits deterministic Cursor `.mdc` frontmatter fields: `description`, `globs`, and `alwaysApply`
- quotes string values safely for the current inline frontmatter format
- returns text only; it does not write files or persist body text

- [x] **Step 3: Verify focused behavior**

Run: `cargo test -p wapc renders_cursor_rule_frontmatter_that_scanner_can_parse_without_persisting_body -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resources::tests -- --nocapture`
Result: PASS, 23 resource tests.

- [x] **Step 4: Update adapter matrix**

Updated `docs/design/tool-adapter-matrix.md` to mark `指令方言 frontmatter 生成/解析` complete.

Added a boundary note that this only proves in-memory Cursor `.mdc` generation plus scanner parse/persist safety. Instruction/frontmatter writes remain unsupported until a separate Sync Engine-backed implementation exists.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: FAIL before formatting; PASS after `cargo fmt`.

Run: `cargo test --workspace`
Result: PASS, 119 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 70: Supported Write Path Backup and Atomic Verify Evidence

**Files:**
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- This is an evidence-consolidation task, not a new write feature.
- Mark the adapter matrix write-path item complete only for currently opened write paths.
- Preserve explicit unsupported boundaries for enterprise, non-macOS writes, instruction/frontmatter writes, skills, plugins, and subagents.
- Do not claim every tool adapter or every platform has write support.

- [x] **Step 1: Audit current write-path evidence**

Current evidence already present in the repo:

- `sync_engine::tests::apply_disable_mcp_json_entry_backs_up_writes_verifies_and_commits_change`
- `sync_engine::tests::apply_disable_mcp_json_entry_blocks_unconfirmed_drift_without_writing`
- `sync_engine::tests::apply_disable_mcp_json_entry_rolls_back_when_verify_fails`
- `sync_engine::tests::apply_disable_mcp_json_entry_rotates_old_tool_backups`
- `sync_engine::tests::rollback_resource_change_restores_backup_and_records_revert_change`
- `cross_sync::tests::plan_sync_generates_codex_toml_plan_and_apply_writes_mcp_server`
- `cross_sync::tests::apply_sync_commits_successful_targets_and_isolates_drift_failures`
- `src-tauri` command helper tests for plan/apply/list backup and rollback paths

These tests show that the supported JSON MCP disable and JSON/TOML MCP sync apply paths run through Sync Engine backup, atomic write, verify, commit/failure record, and rollback behavior.

- [x] **Step 2: Update adapter matrix with scoped completion**

Updated `docs/design/tool-adapter-matrix.md` to mark `写入路径的备份与原子写验证` complete.

Added a boundary note that this applies only to currently opened write paths:

- single-tool JSON MCP disable
- cross-tool JSON/TOML MCP sync
- Resource Center/Tauri command helpers that call the same Sync Engine

The note explicitly keeps enterprise, non-macOS, instruction/frontmatter, skill, plugin, and subagent writes unsupported.

- [x] **Step 3: Verify guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 118 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 69: Canonical JSON/TOML Sync Preview Formatting

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Stabilize WAPC-generated cross-tool MCP sync preview formatting for supported JSON/TOML targets.
- Verify generated preview content is idempotent: parse and canonical-format again yields byte-identical output.
- Keep this scoped to generated sync preview content; do not claim preservation of original comments, original hand formatting, or unsupported future tool dialects.

- [x] **Step 1: Add red formatting stability test**

Added `sync_preview_formatting_is_idempotent_for_json_and_toml_targets`.

The test plans one JSON target and one TOML target through real `plan_sync`, then asserts:

- JSON `preview_after` equals `canonical_json_preview(preview_after)`
- TOML `preview_after` equals `canonical_toml_preview(preview_after)`
- target files are still written only through existing plan/apply behavior, not by the formatter test itself

Run: `cargo test -p wapc sync_preview_formatting_is_idempotent_for_json_and_toml_targets -- --nocapture`
Result: FAIL before implementation because `canonical_json_preview` and `canonical_toml_preview` did not exist; PASS after implementation.

- [x] **Step 2: Add shared canonical preview formatters**

Updated `src/cross_sync.rs` with:

- `canonical_json_preview`
- `canonical_json_value`
- `canonical_toml_preview`
- `canonical_toml_value`

Updated both planning and env-placeholder materialization paths to use those helpers instead of direct serializer calls.

- [x] **Step 3: Verify focused sync behavior**

Run: `cargo test -p wapc sync_preview_formatting_is_idempotent_for_json_and_toml_targets -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc cross_sync::tests -- --nocapture`
Result: PASS, 15 cross-sync tests.

- [x] **Step 4: Update adapter matrix**

Updated `docs/design/tool-adapter-matrix.md` to mark `格式化稳定(TOML/JSON 序列化结果可重复,利于 diff)` complete.

Added a boundary note that this covers WAPC-generated cross-tool MCP sync previews only, not original comment preservation, hand formatting preservation, or all future tool dialects.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS after marking test-only preview helper functions with `#[cfg(test)]`.

Run: `cargo test --workspace`
Result: PASS, 118 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 68: Checked-In Redacted Resource Inventory Fixture

**Files:**
- Modify: `src/resources.rs`
- Add: `tests/fixtures/resource_inventory/redacted-home/.claude.json`
- Add: `tests/fixtures/resource_inventory/redacted-home/.codex/AGENTS.md`
- Add: `tests/fixtures/resource_inventory/redacted-home/.claude/skills/reviewer/SKILL.md`
- Add: `tests/fixtures/resource_inventory/redacted-home/.claude/plugins/context-tools/plugin.json`
- Add: `tests/fixtures/resource_inventory/redacted-home/.claude/plugins/context-tools/mcp/context.json`
- Add: `tests/fixtures/resource_inventory/redacted-home/.claude/plugins/context-tools/agents/helper.md`
- Add: `tests/fixtures/resource_inventory/redacted-home/.claude/agents/reviewer.md`
- Add: `tests/fixtures/resource_inventory/redacted-home/.cursor/rules/react.mdc`
- Add: `tests/fixtures/resource_inventory/redacted-home/work/redacted-repo/.cursor/mcp.json`
- Add: `tests/fixtures/resource_inventory/redacted-home/work/redacted-repo/AGENTS.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Add a checked-in, reviewable, redacted fixture for Resource Inventory.
- Cover all Phase 2 resource kinds and both user/project scopes.
- Include plugin-provided resources without counting them as independent plugin roots only.
- Use placeholder values only; no real user paths, prompt/response/source bodies, or key material.
- Keep fixture data in tests only; do not route sample data through production runtime defaults.

- [x] **Step 1: Add red checked-in fixture contract**

Added `audits_checked_in_redacted_inventory_fixture`.

The test requires:

- `tests/fixtures/resource_inventory/redacted-home` exists
- fixture files do not contain known secret-token prefixes, real macOS user paths, or `secret-client`
- scanning the fixture plus explicit project root produces expected resource counts
- scanned resource payloads do not contain placeholder token values or body text
- at least one instruction resource includes `frontmatter_metadata`

Run: `cargo test -p wapc audits_checked_in_redacted_inventory_fixture -- --nocapture`
Result: FAIL before fixture files existed; PASS after adding the checked-in fixture.

- [x] **Step 2: Add redacted fixture files**

Added fixture coverage for:

- Claude user MCP in `.claude.json`
- Codex user `AGENTS.md`
- Claude user skill
- Claude plugin metadata
- plugin-provided MCP
- plugin-provided subagent
- Claude user subagent
- Cursor user `.mdc` rule with frontmatter
- Cursor project MCP
- project `AGENTS.md`

All env values use `__WAPC_REDACTED_FIXTURE_TOKEN__`, and the test verifies that this placeholder does not persist into scanned resource payloads.

- [x] **Step 3: Verify Resource Inventory fixture behavior**

Run: `cargo test -p wapc audits_checked_in_redacted_inventory_fixture -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resources::tests -- --nocapture`
Result: PASS, 22 resource tests.

- [x] **Step 4: Update adapter matrix**

Updated `docs/design/tool-adapter-matrix.md` to mark `脱敏 fixture 入库,补单测` complete.

Added a note that the checked-in fixture covers user/project MCP, instruction, skill, plugin, subagent, and plugin-provided resources, while test assertions guard against real secret shapes, real user paths, and persisted body text.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: FAIL before formatting; PASS after `cargo fmt`.

Run: `cargo test --workspace`
Result: PASS, 117 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 67: Cursor Rule Frontmatter Parsing Boundary

**Files:**
- Modify: `src/resources.rs`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Add structured parsing for Cursor `.mdc` instruction frontmatter in the read-only Resource Inventory path.
- Persist only safe metadata and fingerprints: `globs`, `alwaysApply`, frontmatter keys, and a description fingerprint.
- Continue excluding raw instruction body text and raw frontmatter description text from `payload_json`.
- Do not generate or write instruction frontmatter in this slice; keep the adapter checklist item unchecked until generation/write behavior has real Sync Engine coverage.

- [x] **Step 1: Add red Cursor frontmatter test**

Added `parses_cursor_rule_frontmatter_without_storing_description_or_body`.

The test creates a real temporary user-scope Cursor rule file:

- `~/.cursor/rules/react.mdc`
- frontmatter `description`, `globs`, and `alwaysApply`
- a Markdown body that must not be stored

It requires:

- the instruction resource is detected as `origin_tool=cursor`
- `payload_json` contains `cursor-rules-frontmatter-v1`
- `payload_json` contains structured `always_apply`
- `payload_json` keeps glob metadata such as `ui/**/*.tsx`
- `payload_json` contains `description_fingerprint`
- raw description and raw body text are absent

Run: `cargo test -p wapc parses_cursor_rule_frontmatter_without_storing_description_or_body -- --nocapture`
Result: FAIL before implementation because `frontmatter_metadata` was not present; PASS after implementation.

- [x] **Step 2: Add read-only frontmatter metadata extraction**

Updated `instruction_resource` to include `frontmatter_metadata`.

Added helpers:

- `frontmatter_metadata`
- `cursor_rule_frontmatter_metadata`
- `sorted_keys`
- `parse_bool`

For Cursor `.mdc` rules, WAPC now stores:

- schema `cursor-rules-frontmatter-v1`
- sorted frontmatter key names
- `description_fingerprint` with length and `sha256_8`
- parsed `globs`
- parsed `always_apply`

- [x] **Step 3: Verify Resource Inventory behavior**

Run: `cargo test -p wapc parses_cursor_rule_frontmatter_without_storing_description_or_body -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resources::tests -- --nocapture`
Result: PASS, 21 resource tests.

- [x] **Step 4: Update matrix without overclaiming**

Updated `docs/design/tool-adapter-matrix.md` to state that Cursor `.mdc` parsing-side structure has unit coverage, while frontmatter generation/write remains incomplete. The checklist item `指令方言 frontmatter 生成/解析` remains unchecked.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: FAIL before formatting; PASS after `cargo fmt`.

Run: `cargo test --workspace`
Result: PASS, 116 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 66: MCP Header and Top-Level Secret Redaction

**Files:**
- Modify: `src/resources.rs`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Expand local Resource Inventory canonical payload redaction for remote MCP configs before payload JSON can be stored.
- Preserve useful non-secret structure: transport, URL, header key names, and stable secret fingerprints.
- Do not store raw `headers.Authorization`, token-like header values, top-level `apiKey`, or token-like top-level fields.
- Do not mark remote MCP field/transport compatibility as true-platform verified; this slice only covers local redaction behavior.

- [x] **Step 1: Add red Resource Inventory test**

Added `fingerprints_mcp_headers_and_api_key_without_storing_secret_values`.

The test writes a `.claude.json` remote MCP config with:

- `type: "sse"`
- `url`
- `headers.Authorization`
- `headers.X-Api-Key`
- `headers.Accept`
- top-level `apiKey`

It requires:

- the MCP resource is marked `redacted`
- `payload_json` includes `header_keys`, `header_fingerprints`, and `sensitive_field_fingerprints`
- sensitive key names and `sha256_8` fingerprints remain visible
- raw `Authorization`, `X-Api-Key`, top-level `apiKey`, and benign header values are not stored

Run: `cargo test -p wapc fingerprints_mcp_headers_and_api_key_without_storing_secret_values -- --nocapture`
Result: FAIL before implementation because the resource was not marked redacted for headers/top-level `apiKey`; PASS after implementation.

- [x] **Step 2: Expand MCP canonical payload redaction**

Updated `src/resources.rs` so `McpPayload` now carries:

- `header_keys`
- `header_fingerprints`
- `sensitive_field_fingerprints`

Added local redaction helpers for:

- sensitive header names such as `Authorization` and `X-Api-Key`
- token-like header values
- top-level sensitive keys such as `apiKey`, `token`, `accessToken`, and `secret`

Sensitive headers and top-level fields keep only length and `sha256_8` fingerprints. Benign header values are excluded from the stored canonical payload while the header key names remain available for structure matching.

- [x] **Step 3: Verify focused behavior**

Run: `cargo test -p wapc fingerprints_mcp_headers_and_api_key_without_storing_secret_values -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_json_and_toml_mcp_configs_with_redacted_env_values -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc redacts_high_risk_mcp_args_before_payload_json -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_plugin_provided_resources_with_provider_relationship -- --nocapture`
Result: PASS.

- [x] **Step 4: Update adapter matrix narrowly**

Updated `docs/design/tool-adapter-matrix.md` to mark the local sensitive-field redaction rule complete.

Added an explicit note that this means local canonical payload unit coverage for env values, args token-like values, top-level `apiKey`/token fields, `headers.Authorization`, and `X-Api-Key`; it does not mean remote MCP `type`/`url`/`headers` field names or transports are true platform/tool verified.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: FAIL before formatting; PASS after `cargo fmt`.

Run: `cargo test --workspace`
Result: PASS, 115 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 65: Project Instruction Sources via PathResolver

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/resources.rs`
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Route project-scope instruction file/rules directory sources through explicit project-root `PlatformPathContext` candidates.
- Preserve the explicit project-root boundary: no implicit current-directory project inference.
- Preserve existing instruction structure fingerprinting behavior and body exclusion.
- Do not add instruction write/sync support.

- [x] **Step 1: Add red project instruction contract test**

Added `project_instruction_sources_use_path_resolver`.

The test asserts against production source only and checks:

- `src/resources.rs` references `ToolPathKind::ProjectInstructionFile`.
- `src/resources.rs` references `ToolPathKind::ProjectInstructionDir`.
- production code no longer hardcodes:
  - `project.join("CLAUDE.md")`
  - `project.join("AGENTS.md")`
  - `project.join("GEMINI.md")`
  - `project.join(".cursorrules")`
  - `project.join(".cursor/rules")`

Run: `cargo test -p wapc project_instruction_sources_use_path_resolver -- --nocapture`
Result: FAIL before implementation because `ProjectInstructionFile` did not exist.

- [x] **Step 2: Add project instruction candidates**

Updated `src/platform_paths.rs` with:

- `ToolPathKind::ProjectInstructionFile`
- `ToolPathKind::ProjectInstructionDir`

`push_project_candidates` now emits:

- `<project>/CLAUDE.md`
- `<project>/AGENTS.md`
- `<project>/GEMINI.md`
- `<project>/.cursorrules`
- `<project>/.cursor/rules`

All remain read-only and write unsupported; only macOS sample candidates are marked verified.

- [x] **Step 3: Read project instruction sources from PathResolver**

Updated `src/resources.rs` so `project_instruction_sources(project, now)` now:

- builds candidates from the explicit project root
- maps project instruction files to the existing dialects
- reads Cursor `.mdc` rules from `ProjectInstructionDir`
- keeps project paths attached to `project_path`

Updated `src/privacy.rs` only to keep enum matching and display names exhaustive; project placeholder privacy sources remain explicit in this slice.

- [x] **Step 4: Verify focused behavior**

Run: `cargo test -p wapc project_instruction_sources_use_path_resolver -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_project_level_resources_without_body_or_secret_values -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc audits_inventory_fixture_counts_against_manual_expectations -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: PASS.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS after formatting `src/platform_paths.rs` and `src/resources.rs`.

Run: `cargo test --workspace`
Result: PASS, 114 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 64: Project Claude Ecosystem Roots via PathResolver

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/resources.rs`
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Route project-scope Claude Code skills/subagents roots through explicit project-root `PlatformPathContext` candidates.
- Preserve the existing project-root explicit boundary: no implicit current-directory project inference.
- Preserve existing read-only inventory behavior and structure-fingerprint privacy boundary.
- Do not add install/write/sync support for project skills or subagents.

- [x] **Step 1: Add red Resource Inventory contract test**

Added `project_claude_ecosystem_roots_use_path_resolver`.

The test asserts against production source only and checks:

- `src/resources.rs` references `ToolPathKind::ProjectSkillDir`.
- `src/resources.rs` references `ToolPathKind::ProjectSubagentDir`.
- production code no longer hardcodes:
  - `project.join(".claude/skills")`
  - `project.join(".claude/agents")`

Run: `cargo test -p wapc project_claude_ecosystem_roots_use_path_resolver -- --nocapture`
Result: FAIL before implementation because project skill resources hardcoded `project.join(".claude/skills")`.

- [x] **Step 2: Add project ecosystem candidates**

Updated `src/platform_paths.rs` with:

- `ToolPathKind::ProjectSkillDir`
- `ToolPathKind::ProjectSubagentDir`

`push_project_candidates` now emits:

- `<project>/.claude/skills`
- `<project>/.claude/agents`

Both remain read-only and write unsupported; only macOS sample candidates are marked verified.

- [x] **Step 3: Read project roots from PathResolver**

Updated `src/resources.rs` so:

- `read_skill_resources` reads project Claude skills from `ProjectSkillDir` candidates.
- `read_subagent_resources` reads project Claude subagents from `ProjectSubagentDir` candidates.
- `project_tool_roots(project, tool, kind)` builds candidates only from the explicit project root.

Updated `src/privacy.rs` only to keep enum matching and candidate display exhaustive; project placeholder privacy sources remain explicit in this slice.

- [x] **Step 4: Verify focused behavior**

Run: `cargo test -p wapc project_claude_ecosystem_roots_use_path_resolver -- --nocapture`
Result: PASS after restricting the contract check to production source so test fixtures can still create real project paths.

Run: `cargo test -p wapc scans_project_level_resources_without_body_or_secret_values -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc audits_inventory_fixture_counts_against_manual_expectations -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: PASS.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS after formatting `src/platform_paths.rs`.

Run: `cargo test --workspace`
Result: PASS, 113 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 63: Project MCP Sources via PathResolver

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/resources.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Route project-scope Claude/Cursor MCP source paths through `PlatformPathContext` and `tool_path_candidates`.
- Preserve the existing project-root explicit boundary: project paths are still derived only from caller-provided project roots.
- Preserve existing project MCP parser behavior for Claude `.mcp.json` and Cursor `.cursor/mcp.json`.
- Do not add implicit current-directory project inference or broaden write support.

- [x] **Step 1: Add red project MCP contract test**

Added `project_mcp_sources_use_path_resolver_candidates`.

The test asserts:

- `src/resources.rs` references `ToolPathKind::ProjectMcpConfig`.
- project MCP source construction no longer hardcodes:
  - `project.join(".mcp.json")`
  - `project.join(".cursor/mcp.json")`

Run: `cargo test -p wapc project_mcp_sources_use_path_resolver_candidates -- --nocapture`
Result: FAIL before implementation because `project_mcp_sources` hardcoded `project.join(".mcp.json")`.

- [x] **Step 2: Add current-platform project context helper**

Updated `src/platform_paths.rs` with `PlatformPathContext::current_home_compatible_with_project(home_dir, project_root)`, preserving existing platform config/data behavior while allowing a caller-provided project root.

- [x] **Step 3: Generate project MCP sources from ProjectMcpConfig candidates**

Updated `src/resources.rs` so `project_mcp_sources(project_roots)` now:

- builds a current-platform context with each explicit project root
- calls `tool_path_candidates(&context)`
- filters to `scope == "project"` and `ToolPathKind::ProjectMcpConfig`
- maps known tools through the existing MCP parser config

Existing Claude/Cursor JSON parser contracts remain unchanged.

- [x] **Step 4: Verify focused behavior**

Run: `cargo test -p wapc project_mcp_sources_use_path_resolver_candidates -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_project_level_resources_without_body_or_secret_values -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc audits_inventory_fixture_counts_against_manual_expectations -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: PASS.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS after formatting `src/resources.rs`.

Run: `cargo test --workspace`
Result: PASS, 112 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 62: Privacy Audit User Resource Sources via PathResolver

**Files:**
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Route privacy-audit user-scope skills/instructions/plugins/subagents source paths through `PlatformPathContext` and `tool_path_candidates`.
- Preserve existing privacy semantics: these sources remain metadata/fingerprint inventory with `reads_body=false` and `writes_source=false`.
- Keep project placeholder sources and WAPC backup source explicit in this slice.
- Do not add write/install/sync support for skills, plugins, subagents, or instructions.

- [x] **Step 1: Add red privacy-audit path contract test**

Added `privacy_audit_uses_path_resolver_for_user_resource_path_sources`.

The test asserts:

- `src/privacy.rs` uses `candidate_resource_source`.
- privacy-audit user resource source construction no longer hardcodes:
  - `.claude/skills`
  - `.claude/CLAUDE.md`
  - `.codex/AGENTS.md`
  - `.gemini/GEMINI.md`
  - `.cursor/rules`
  - `.cursorrules`
  - `.claude/plugins`
  - `.claude/agents`

Run: `cargo test -p wapc privacy_audit_uses_path_resolver_for_user_resource_path_sources -- --nocapture`
Result: FAIL before implementation because privacy audit hardcoded `home.join(".claude/skills")`.

- [x] **Step 2: Generate user resource audit sources from PathResolver**

Updated `src/privacy.rs` so `current_tool_candidate_sources(home)` now maps these `ToolPathKind` values into `PrivacyAuditSource` entries:

- `SkillDir` -> read-only skill inventory
- `InstructionFile` / `InstructionDir` -> instruction structure fingerprinting
- `PluginDir` -> read-only plugin inventory
- `SubagentDir` -> subagent metadata and structure fingerprinting

Preserved existing labels:

- `Claude Code skills`
- `Claude Code user instructions`
- `Codex user instructions`
- `Gemini user instructions`
- `Cursor user rules`
- `Cursor legacy rules`
- `Claude Code plugins`
- `Claude Code subagents`

- [x] **Step 3: Preserve existing privacy boundaries**

Focused tests confirmed:

- Phase 1 read-source/table/forbidden-field coverage remains present.
- Phase 2 resource/session forbidden-field coverage remains present.
- Project-level resource scan placeholder boundaries remain explicit.

- [x] **Step 4: Verify focused behavior**

Run: `cargo test -p wapc privacy_audit_uses_path_resolver_for_user_resource_path_sources -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc privacy_audit_covers_phase_one_sources_tables_and_forbidden_fields -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc privacy_audit_names_phase_two_resource_and_session_field_boundaries -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc privacy_audit_names_project_level_resource_scan_boundaries -- --nocapture`
Result: PASS.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 111 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 61: User Instruction Sources via PathResolver

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/resources.rs`
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Route user-scope instruction file/rules directory sources through `PlatformPathContext` and `tool_path_candidates`.
- Preserve existing macOS-compatible behavior for Claude, Codex, Gemini, and Cursor user instruction inventory.
- Keep project-scope instruction sources explicitly project-root derived in this slice.
- Keep non-macOS instruction candidates read-only and unverified unless backed by real fixture evidence.
- Do not add instruction write/sync support.

- [x] **Step 1: Add red Resource Inventory contract test**

Added `user_instruction_sources_use_path_resolver`.

The test asserts:

- `src/resources.rs` references `ToolPathKind::InstructionFile`.
- `src/resources.rs` references `ToolPathKind::InstructionDir`.
- user-scope instruction source construction no longer hardcodes:
  - `.claude/CLAUDE.md`
  - `.codex/AGENTS.md`
  - `.gemini/GEMINI.md`
  - `.cursorrules`
  - `.cursor/rules`

Run: `cargo test -p wapc user_instruction_sources_use_path_resolver -- --nocapture`
Result: FAIL before implementation because `read_instruction_resources` hardcoded `home.join(".claude/CLAUDE.md")`.

- [x] **Step 2: Add instruction candidates to PathResolver**

Updated `src/platform_paths.rs` with new `ToolPathKind` variants:

- `InstructionFile`
- `InstructionDir`

Added user-scope instruction candidates:

- Claude Code: `.claude/CLAUDE.md`
- Codex: `.codex/AGENTS.md`
- Gemini CLI: `.gemini/GEMINI.md`
- Cursor legacy: `.cursorrules`
- Cursor rules directory: `.cursor/rules`

Extended `resolves_cross_platform_tool_path_candidates_without_touching_filesystem` to assert macOS Claude/Cursor instruction candidates and Linux Codex instruction candidate. Linux instruction candidate remains unverified.

- [x] **Step 3: Read user instruction sources from PathResolver**

Updated `src/resources.rs` so `read_instruction_resources` now:

- uses `user_instruction_sources(home, now)` for user-scope instruction files and Cursor `.mdc` rules
- maps known tools to the existing dialects:
  - Claude: `claude-md`
  - Codex: `agents-md`
  - Gemini: `gemini-md`
  - Cursor legacy: `cursor-rules-legacy`
  - Cursor rules dir entries: `cursor-rules`
- keeps project instruction sources explicit and project-root derived

Updated `src/privacy.rs` only to keep enum matches exhaustive and display names available for the new path kinds.

- [x] **Step 4: Verify focused behavior**

Run: `cargo test -p wapc user_instruction_sources_use_path_resolver -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_instruction_files_as_structure_fingerprints_only -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_project_level_resources_without_body_or_secret_values -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc audits_inventory_fixture_counts_against_manual_expectations -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: PASS after tightening Linux Codex/Gemini instruction candidates to unverified.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 110 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 60: Claude User Ecosystem Roots via PathResolver

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/resources.rs`
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Route user-scope Claude Code skills/plugins/subagents resource roots through `PlatformPathContext` and `tool_path_candidates`.
- Keep project-scope Claude skills/subagents explicitly project-root derived in this slice.
- Keep the privacy-audit skills/plugins/subagents source wording explicit in this slice; only add display support for the new path kinds.
- Do not add write/install/sync support for skills, plugins, or subagents.

- [x] **Step 1: Add red Resource Inventory contract test**

Added `claude_user_ecosystem_roots_use_path_resolver`.

The test asserts:

- `src/resources.rs` references `ToolPathKind::SkillDir`.
- `src/resources.rs` references `ToolPathKind::PluginDir`.
- `src/resources.rs` references `ToolPathKind::SubagentDir`.
- user-scope Claude ecosystem roots no longer hardcode:
  - `.claude/skills`
  - `.claude/plugins`
  - `.claude/agents`

Run: `cargo test -p wapc claude_user_ecosystem_roots_use_path_resolver -- --nocapture`
Result: FAIL before implementation because `read_skill_resources` hardcoded `home.join(".claude/skills")`.

- [x] **Step 2: Add Claude ecosystem root candidates**

Updated `src/platform_paths.rs` with new `ToolPathKind` variants:

- `SkillDir`
- `PluginDir`
- `SubagentDir`

Added user-scope Claude Code candidates:

- `.claude/skills`
- `.claude/plugins`
- `.claude/agents`

Extended `resolves_cross_platform_tool_path_candidates_without_touching_filesystem` to assert these macOS-compatible candidates are verified, read-only, and write unsupported.

- [x] **Step 3: Read user roots from PathResolver**

Updated `src/resources.rs` so:

- `read_skill_resources` reads user Claude skills from `ToolPathKind::SkillDir`.
- `read_plugin_resources` reads user Claude plugins from `ToolPathKind::PluginDir`.
- `read_subagent_resources` reads user Claude subagents from `ToolPathKind::SubagentDir`.

Project-scope roots remain explicit project root joins.

Updated `src/privacy.rs` only to keep matches exhaustive and display the new kinds; privacy-audit sources for skills/plugins/subagents remain explicit pending a dedicated audit slice.

- [x] **Step 4: Verify focused behavior**

Run: `cargo test -p wapc claude_user_ecosystem_roots_use_path_resolver -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_claude_skills_without_storing_file_contents -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_claude_plugins_without_storing_file_contents -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_claude_subagents_without_storing_body_text -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: PASS.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 109 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 59: Privacy Audit Current Tool Paths via PathResolver

**Files:**
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Route current-platform privacy-audit sources for session data, config directories, and MCP config files through `PlatformPathContext` and `tool_path_candidates`.
- Keep skills, instructions, plugins, subagents, backups, and project placeholder sources on the existing explicit audit boundary in this slice.
- Preserve the existing read/write privacy semantics: inventory reads do not read bodies; sync target entries read/write only after explicit confirmation.
- Do not claim Windows/Linux true path verification or broaden write support.

- [x] **Step 1: Add red privacy-audit path contract test**

Added `privacy_audit_uses_path_resolver_for_current_tool_path_sources`.

The test asserts:

- `src/privacy.rs` uses `current_tool_candidate_sources`.
- current session/config/MCP audit source construction no longer directly hardcodes `home.join(...)` for:
  - `.claude/projects`
  - `.codex/sessions`
  - `.codex/archived_sessions`
  - `.gemini/tmp`
  - `.local/share/opencode/storage`
  - `.claude`
  - `.codex`
  - `.gemini`
  - `.config/opencode`
  - `.claude.json`
  - `.codex/config.toml`
  - `.gemini/settings.json`
  - `.cursor/mcp.json`

Run: `cargo test -p wapc privacy_audit_uses_path_resolver_for_current_tool_path_sources -- --nocapture`
Result: FAIL before implementation because privacy audit hardcoded `home.join(".claude/projects")`.

- [x] **Step 2: Generate current tool audit sources from PathResolver**

Updated `src/privacy.rs` so `read_sources(home)` starts with `current_tool_candidate_sources(home)`, which:

- builds `PlatformPathContext::current_home_compatible(home)`
- calls `tool_path_candidates(&context)`
- maps `SessionData` to usage metadata parsing sources
- maps `ConfigDir` to tool presence detection sources
- maps `McpConfig` to both read-only inventory sources and sync target write-after-confirmation sources

The helper preserves the prior source labels for Claude, Codex archived sessions, Gemini chats, OpenCode storage, MCP configs, and sync target entries.

- [x] **Step 3: Preserve existing privacy boundaries**

Focused tests confirmed:

- Phase 1 source/table/forbidden-field coverage remains present.
- Phase 4 sync preset export and target write boundary still names Codex/Cursor sync targets.
- Cross-platform candidate path boundary remains metadata-only, read-only, and write unsupported.

- [x] **Step 4: Verify focused behavior**

Run: `cargo test -p wapc privacy_audit_uses_path_resolver_for_current_tool_path_sources -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc privacy_audit_covers_phase_one_sources_tables_and_forbidden_fields -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc privacy_audit_names_phase_four_sync_preset_export_and_target_write_boundaries -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc privacy_audit_names_cross_platform_candidate_path_boundaries -- --nocapture`
Result: PASS.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: first run passed 108 core tests but one Tauri test failed with `database is locked`.

Run: `cargo test -p wapc-app get_snapshot_returns_desktop_bootstrap_data -- --nocapture`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 108 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 58: Session Scanner Source Roots via PathResolver

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/scanner.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Route read-only session/source scanner roots through `PlatformPathContext` and `tool_path_candidates`.
- Preserve existing macOS-compatible scan behavior for Claude, Codex, Gemini, and OpenCode usage/session sources.
- Keep non-macOS candidate paths read-only and do not claim Windows/Linux true path verification.
- Do not enable any non-macOS write, sync, template apply, or deep-link apply support.

- [x] **Step 1: Add red scanner path contract test**

Added `session_scanner_uses_path_resolver_for_source_roots`.

The test asserts:

- `src/scanner.rs` references `tool_path_candidates`.
- `src/scanner.rs` references `ToolPathKind::SessionData`.
- `scan_home` and `audit_paths` no longer directly hardcode `home.join(...)` source roots for:
  - `.claude/projects`
  - `.codex/sessions`
  - `.codex/archived_sessions`
  - `.gemini/tmp`
  - `.local/share/opencode/storage`

Run: `cargo test -p wapc session_scanner_uses_path_resolver_for_source_roots -- --nocapture`
Result: FAIL before implementation because `scan_home` hardcoded `&home.join(".claude/projects")`.

- [x] **Step 2: Add SessionData candidates to PathResolver**

Updated `src/platform_paths.rs` so `tool_path_candidates` emits `ToolPathKind::SessionData` candidates for:

- Claude Code: `.claude/projects`
- Codex: `.codex/sessions`
- Codex archive: `.codex/archived_sessions`
- Gemini CLI: `.gemini/tmp`
- OpenCode: `opencode/storage` through the platform data directory strategy

Extended `resolves_cross_platform_tool_path_candidates_without_touching_filesystem` to assert SessionData coverage for Codex archived sessions and Linux OpenCode storage.

Run: `cargo test -p wapc resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: FAIL first because OpenCode Linux SessionData was accidentally marked verified; PASS after aligning it with the existing unverified DataDir boundary.

- [x] **Step 3: Scan session sources from PathResolver candidates**

Updated `src/scanner.rs` so:

- `scan_home` iterates resolved SessionData source roots.
- `audit_paths` returns resolved SessionData source roots.
- `source_health` checks resolved SessionData source roots.
- parser selection remains tied to explicit source definitions, not inferred from file paths.

Existing parser behavior was preserved.

- [x] **Step 4: Verify focused behavior**

Run: `cargo test -p wapc session_scanner_uses_path_resolver_for_source_roots -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_known_tool_directories_under_home -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc source_health_counts_parse_successes_and_failures -- --nocapture`
Result: PASS.

- [x] **Step 5: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS after formatting `src/scanner.rs`.

Run: `cargo test --workspace`
Result: PASS, 107 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS.

## Task 57: Resource Inventory User MCP Sources via PathResolver

**Files:**
- Modify: `src/resources.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Route user-scope MCP source paths through `PlatformPathContext` and `tool_path_candidates`.
- Preserve existing macOS-compatible behavior for Claude, Codex, Gemini, and Cursor user MCP configs.
- Keep project-scope MCP sources explicit and project-root derived in this slice.
- Do not claim Windows/Linux true path verification, platform fixtures, or write support.

- [x] **Step 1: Add red Resource Inventory path contract test**

Added `mcp_sources_use_path_resolver_for_user_scope_paths`.

The test asserts:

- `src/resources.rs` references `tool_path_candidates`.
- `src/resources.rs` references `ToolPathKind::McpConfig`.
- user-scope MCP source construction no longer hardcodes direct `home.join(...)` paths for:
  - `.claude.json`
  - `.codex/config.toml`
  - `.gemini/settings.json`
  - `.cursor/mcp.json`

Run: `cargo test -p wapc mcp_sources_use_path_resolver_for_user_scope_paths -- --nocapture`
Result: FAIL before implementation because `mcp_sources` hardcoded `home.join(".claude.json")` and sibling user MCP config paths.

- [x] **Step 2: Generate user MCP sources from PathResolver**

Updated `src/resources.rs` so `mcp_sources(home)` now:

- builds `PlatformPathContext::current_home_compatible(home)`
- calls `tool_path_candidates(&context)`
- filters to `scope == "user"` and `ToolPathKind::McpConfig`
- maps known MCP source tools to their existing parser contracts:
  - Claude, Gemini, Cursor: JSON with `mcpServers`
  - Codex: TOML with `mcp_servers`

Unknown or not-yet-supported MCP candidate tools are skipped until their parser/root contract is implemented.

- [x] **Step 3: Verify focused behavior**

Run: `cargo test -p wapc mcp_sources_use_path_resolver_for_user_scope_paths -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scans_json_and_toml_mcp_configs_with_redacted_env_values -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc scan_inventory_with_kinds_limits_detector_families -- --nocapture`
Result: PASS.

- [x] **Step 4: Verify full guardrails**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 106 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS.

## Task 56: Phase 5 F3 Privacy Audit for Cross-platform Candidate Paths

**Files:**
- Modify: `src/privacy.rs`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Extend privacy-audit to name Windows/Linux candidate paths introduced by PathResolver.
- Candidate paths must be described as read-only, unverified where applicable, and write unsupported.
- This is not Windows/Linux true path verification and does not add runtime fixture ingestion.
- Do not mark the Go/No-Go fixture or write-support checklist items complete.
- Continue forbidding prompt/response/source body and secret values in platform fixtures.

- [x] **Step 1: Add red privacy-audit test**

Added `privacy_audit_names_cross_platform_candidate_path_boundaries`.

The test asserts:

- Windows Codex MCP candidate path is named.
- Linux Gemini MCP candidate path is named.
- candidate purposes include `read-only candidate`, `unverified`, and `write unsupported`.
- candidates do not read bodies or write sources.
- export boundary mentions Windows/Linux candidate paths.
- forbidden fields include platform fixture prompt/response/source body and secret values.

Result: FAIL before implementation because privacy-audit had no Windows/Linux candidate path sources.

- [x] **Step 2: Generate candidate audit sources from PathResolver**

Updated `src/privacy.rs` to call `tool_path_candidates` for deterministic Windows/Linux sample contexts and add privacy audit read sources for MCP config candidates. Each candidate source:

- has `reads_body=false`
- has `writes_source=false`
- states whether it is unverified
- states write support remains unsupported pending real platform fixture and rollback e2e evidence

Updated forbidden fields with:

- `platform fixture prompt body`
- `platform fixture response body`
- `platform fixture source body`
- `platform fixture secret value`

Updated the export boundary to state that Windows/Linux candidate paths are metadata-only and do not imply support.

- [x] **Step 3: Preserve Go/No-Go honesty**

Updated `docs/design/tool-adapter-matrix.md` to mention privacy-audit candidate coverage while leaving Windows/Linux true path fixture and write support checklist items unchecked.

- [x] **Step 4: Verify**

Run: `cargo test -p wapc privacy_audit_names_cross_platform_candidate_path_boundaries -- --nocapture`
Result: PASS.

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS after formatting `src/privacy.rs`.

Run: `cargo test --workspace`
Result: PASS, 105 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 55: Phase 5 F2 Cross-platform Tool PathResolver Candidates

**Files:**
- Modify: `src/platform_paths.rs`
- Modify: `src/tool_registry.rs`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement platform-aware candidate path samples for macOS, Windows, and Linux without touching the filesystem.
- Preserve current macOS/home-compatible tool detection behavior.
- Route Tool Registry config/data directory detection through the path resolver.
- Do not claim Windows/Linux real support, artifact support, or write support. Non-macOS candidates remain read-only, unverified unless explicitly marked by existing macOS behavior or Linux candidate docs.
- Do not move Windows/Linux write/sync/template/deep-link apply flows out of `unsupported`.

- [x] **Step 1: Add red PathResolver and Tool Registry contract tests**

Added:

- `platform_paths::tests::resolves_cross_platform_tool_path_candidates_without_touching_filesystem`
- `tool_registry::tests::registry_detector_uses_platform_path_resolver_for_tool_directories`

Result:

- RED: missing `PlatformPathContext`, `ToolPathKind`, and `tool_path_candidates`.
- RED: Tool Registry still contained hardcoded config/data directory fields.

- [x] **Step 2: Implement platform-aware candidate paths**

Added:

- `PlatformKind`
- `ToolPathKind`
- `PlatformPathContext`
- `ToolPathCandidate`
- `tool_path_candidates`
- `tool_registry_paths_for_home`

The candidate set covers:

- macOS-compatible current home paths for Claude, Codex, Gemini, OpenCode, and Cursor.
- Windows samples with drive-letter/AppData style roots.
- Linux samples with XDG config/data roots.
- explicit project-root derived Claude/Cursor project MCP paths, including paths with spaces.

- [x] **Step 3: Route Tool Registry through resolver**

Updated `src/tool_registry.rs` so `detect_tools(home)` resolves config/data directories via `tool_registry_paths_for_home(home)` and no longer owns per-tool config/data path constants.

Updated `docs/design/tool-adapter-matrix.md` to mark only the PathResolver sample coverage item complete, with a note that Windows/Linux true path verification and write support remain incomplete.

- [x] **Step 4: Verify**

Run: `cargo test -p wapc resolves_cross_platform_tool_path_candidates_without_touching_filesystem -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc registry_detector_uses_platform_path_resolver_for_tool_directories -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc detects_codex_from_config_and_data_directories -- --nocapture`
Result: PASS.

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS after formatting test code and removing clippy `cmp_owned` warnings.

Run: `cargo test --workspace`
Result: PASS, 104 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 54: Phase 3 Guide Center and Enterprise Write Gating

**Files:**
- Modify: `src/model.rs`
- Create: `src/guide_center.rs`
- Modify: `src/lib.rs`
- Modify: `src/sync_engine.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 3 AC-7 as built-in resource usage guidance linked to the selected resource's tool/kind/resource id.
- Implement Phase 3 AC-9 as backend write-plan gating for `enterprise` and `managed` resources; UI-only disabled state is not sufficient.
- Guide content is version-bundled safety/help metadata only. It must not read, persist, or render prompt bodies, resource bodies, source code, tool output, or secret values.
- Do not broaden write support beyond the existing user-scoped JSON MCP disable flow.

- [x] **Step 1: Add red Guide Center and enterprise gating tests**

Added:

- `model::tests::resource_guide_serializes_safe_usage_sections`
- `sync_engine::tests::plan_resource_change_rejects_enterprise_resources_before_file_write_preview`
- `ui/tests/resourceCenterState.test.ts` guide summary contract

Result:

- RED: Rust failed because `ResourceGuide` / `ResourceGuideSection` were missing.
- RED: UI failed because `buildResourceGuideSummary` was not exported.

- [x] **Step 2: Implement Guide Center core and Tauri command**

Added:

- `ResourceGuide` and `ResourceGuideSection`
- `src/guide_center.rs` with deterministic built-in guidance for MCP, instruction, skill, plugin, subagent, and unknown resources
- `get_guide(tool?, kind?, resource_id?)` Tauri command
- command helper coverage for linking a selected resource to safe usage guidance

- [x] **Step 3: Enforce backend write gating and resource detail UI**

Implemented:

- Sync Engine now checks persisted `resource_id` before generating a write preview.
- `enterprise` / `managed` resources return explicit read-only errors before file preview.
- plugin-provided resources remain backend read-only.
- Resource detail loads guide data through the real Tauri command and renders sections, risks, and unsupported actions.

- [x] **Step 4: Verify**

Run: `cargo test -p wapc guide_center -- --nocapture`
Result: PASS, 2 Guide Center tests.

Run: `cargo test -p wapc plan_resource_change_rejects_enterprise_resources_before_file_write_preview -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc-app get_guide_helper_links_selected_resource_to_safe_usage_guidance -- --nocapture`
Result: PASS.

Run: `cd ui && yarn test --test-name-pattern guide`
Result: PASS.

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 102 core tests, 24 Tauri tests, 0 main tests, 0 doctests.

Run: `cd ui && yarn test`
Result: PASS, 23 tests.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size warning.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 53: Phase 5 F1 Cross-platform Core Smoke CI

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 5 F1 as CI smoke coverage only.
- Do not claim Windows/Linux desktop support.
- Do not build or upload Windows/Linux release artifacts.
- Keep Tauri bundle verification on macOS desktop gates.

- [x] **Step 1: Add red cross-platform smoke CI contract**

Added `ci_workflow_has_non_release_cross_platform_smoke_gates`.

The test requires `.github/workflows/ci.yml` to define:

- `cross-platform-smoke`
- `runs-on: ${{ matrix.os }}`
- `ubuntu-latest`
- `windows-latest`
- `cargo clippy --workspace --exclude wapc-app --all-targets -- -D warnings`
- `cargo test --workspace --exclude wapc-app`
- `yarn --cwd ui test`
- `yarn --cwd ui build`

The test also rejects release-producing steps inside the smoke job:

- `cargo tauri build`
- `actions/upload-artifact`

Run: `cargo test -p wapc-app ci_workflow_has_non_release_cross_platform_smoke_gates -- --nocapture`
Result: FAIL before implementation because CI had no cross-platform smoke job.

- [x] **Step 2: Add Ubuntu/Windows core smoke job**

Updated `.github/workflows/ci.yml` with `cross-platform-smoke`, matrixed over:

- `ubuntu-latest`
- `windows-latest`

The job installs Rust and Node, then runs:

- `cargo clippy --workspace --exclude wapc-app --all-targets -- -D warnings`
- `cargo test --workspace --exclude wapc-app`
- `yarn --cwd ui test`
- `yarn --cwd ui build`

It intentionally does not run `cargo tauri build` or upload artifacts.

- [x] **Step 3: Verify**

Run: `cargo test -p wapc-app ci_workflow_has_non_release_cross_platform_smoke_gates -- --nocapture`
Result: PASS.

Run: `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci workflow yaml ok"'`
Result: PASS.

Run: `cargo clippy --workspace --exclude wapc-app --all-targets -- -D warnings`
Result: PASS.

Run: `cargo test --workspace --exclude wapc-app`
Result: PASS, 98 core tests, 0 doctests.

Run: `cd ui && yarn test`
Result: PASS, 22 tests.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size warning.

Run: `cargo test --workspace`
Result: PASS, 98 core tests, 23 Tauri tests, 0 main tests, 0 doctests.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `git diff --check`
Result: PASS.

## Task 52: WAPC Application PathResolver Foundation

**Files:**
- Create: `src/platform_paths.rs`
- Modify: `src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Start Phase 5 F2 path work by centralizing WAPC application paths.
- Preserve the current `~/.wapc` data location for compatibility.
- Do not claim Windows/Linux support; this is a foundation for later platform-specific directory policy and fixtures.
- Tauri command bootstrap must stop hand-rolling the db path and call the core resolver instead.

- [x] **Step 1: Add red PathResolver tests**

Added:

- `platform_paths::tests::resolves_wapc_paths_from_explicit_home_without_touching_filesystem`
- `tauri_commands_resolve_app_paths_through_core_path_resolver`

Run: `cargo test -p wapc platform_paths -- --nocapture`
Result: FAIL before implementation because `WapcPaths::from_home` did not exist.

Run: `cargo test -p wapc-app tauri_commands_resolve_app_paths_through_core_path_resolver -- --nocapture`
Result: FAIL before implementation because Tauri commands still hand-rolled `home.join(".wapc/wapc.db")`.

- [x] **Step 2: Implement core WAPC path resolver**

Created `src/platform_paths.rs` with:

- `WapcPaths`
- `WapcPaths::from_home(home)`
- `WapcPaths::from_platform_home()`

Current paths remain:

- app dir: `<home>/.wapc`
- db: `<home>/.wapc/wapc.db`
- backups: `<home>/.wapc/backups`
- settings: `<home>/.wapc/settings.json`

- [x] **Step 3: Use resolver from Tauri commands**

Updated `src-tauri/src/commands.rs` so `resolve_paths()` calls `WapcPaths::from_platform_home()` and returns `home_dir` / `db_path` from the core resolver.

- [x] **Step 4: Verify**

Run: `cargo test -p wapc platform_paths -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc-app tauri_commands_resolve_app_paths_through_core_path_resolver -- --nocapture`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 98 core tests, 22 Tauri tests, 0 main tests, 0 doctests.

Run: `cd ui && yarn test`
Result: PASS, 22 tests.

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings`
Result: PASS.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size warning.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS.

## Task 51: Production CI Desktop Gate Hardening

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- CI must prove the current Tauri desktop application can pass Rust, UI, and bundle gates.
- Do not keep stale root-only `cargo test` / `cargo build --release` gates as the primary proof.
- PR/push CI builds an unsigned local macOS app only; signing/notarization remains release workflow responsibility.

- [x] **Step 1: Add red CI workflow contract**

Added `ci_workflow_runs_workspace_ui_and_tauri_release_gates`.

The test requires `.github/workflows/ci.yml` to include:

- `actions/setup-node@v4`
- `cache-dependency-path: ui/yarn.lock`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `yarn --cwd ui install --frozen-lockfile`
- `yarn --cwd ui test`
- `yarn --cwd ui build`
- `cargo install tauri-cli --locked`
- `cargo tauri build`

It rejects stale root-only gates:

- `cargo test`
- `cargo build --release`

Run: `cargo test -p wapc-app ci_workflow_runs_workspace_ui_and_tauri_release_gates -- --nocapture`
Result: FAIL before implementation because CI did not install Node/UI dependencies or run Tauri build.

- [x] **Step 2: Replace stale CI workflow**

Updated `.github/workflows/ci.yml` to run a macOS desktop gate on push and pull request:

- checkout
- Node 20 with Yarn cache
- Rust stable with cargo cache
- `yarn --cwd ui install --frozen-lockfile`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `yarn --cwd ui test`
- `yarn --cwd ui build`
- `cargo install tauri-cli --locked`
- `cd src-tauri && cargo tauri build`

- [x] **Step 3: Verify**

Run: `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci workflow yaml ok"'`
Result: PASS.

Run: `cargo test -p wapc-app ci_workflow_runs_workspace_ui_and_tauri_release_gates -- --nocapture`
Result: PASS.

Run: `rg -n "actions/setup-node@v4|cargo test --workspace|yarn --cwd ui test|yarn --cwd ui build|cargo tauri build|cargo build --release|cargo test$" .github/workflows/ci.yml src-tauri/src/lib.rs docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
Result: PASS, expected CI gates are present and stale gates are absent from `.github/workflows/ci.yml`.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd ui && yarn test`
Result: PASS, 22 tests.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size warning.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Result: FAIL before lint cleanup, then PASS after fixing clippy findings in `src/export.rs`, `src/headless.rs`, `src/resources.rs`, and `src/store.rs`.

Run: `cargo test --workspace`
Result: PASS, 97 core tests, 21 Tauri tests, 0 main tests, 0 doctests.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS.

## Task 50: Phase 5 README Release Readiness Boundary

**Files:**
- Modify: `README.md`
- Modify: `src-tauri/src/lib.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 5 FR-E3 as honest release documentation, not as a fake "download ready" claim.
- README must explain the GitHub Release path, Apple Developer signing/notarization gate, and Gatekeeper clean-machine boundary.
- README must keep source build instructions available until a real signed/notarized release has been produced and verified.

- [x] **Step 1: Add red README release documentation test**

Added `readme_documents_release_gate_without_pretending_notarization_is_done`.

The test requires README to name:

- `macOS 签名与公证发布说明`
- `docs/release/macos-signing-notarization.md`
- `GitHub Release`
- `Apple Developer`
- `Gatekeeper`
- `源码构建`

It also forbids overclaiming text such as:

- `当前已完成 Gatekeeper 验收`
- `已经通过 Gatekeeper 验收`
- `无需源码构建`

Run: `cargo test -p wapc-app readme_documents_release_gate_without_pretending_notarization_is_done -- --nocapture`
Result: FAIL before README update because the release section did not link the macOS signing/notarization release notes.

- [x] **Step 2: Update README quick start release boundary**

Updated `README.md` quick start to:

- state macOS 12+ support
- define signed/notarized GitHub Release as the intended download path
- explain that the release job fails when Apple signing/notarization credentials are missing
- link `docs/release/macos-signing-notarization.md`
- keep source build instructions as the supported fallback before the first verified signed release

- [x] **Step 3: Verify**

Run: `cargo test -p wapc-app readme_documents_release_gate_without_pretending_notarization_is_done -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc-app release_workflow_uses_tauri_signing_and_notarization_path -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc-app macos_package_script_delegates_to_tauri_build -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc-app tauri_macos_release_minimum_matches_readme_support_policy -- --nocapture`
Result: PASS.

Run: `rg -n "GitHub Release|Apple Developer|Gatekeeper|源码构建|macOS 签名与公证发布说明|docs/release/macos-signing-notarization.md" README.md docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
Result: PASS.

Run: `cargo fmt --check`
Result: PASS.

Run: `git diff --check`
Result: PASS.

## Task 49: Phase 5 Cross-platform Feasibility Assessment

**Files:**
- Create: `docs/design/cross-platform-feasibility.md`
- Modify: `docs/design/tool-adapter-matrix.md`
- Modify: `docs/README.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 5 WP5.F as an assessment and matrix increment only.
- Do not claim Windows/Linux support or generate release artifacts.
- Keep non-macOS write/sync/template/deep-link apply flows explicitly `unsupported` until real platform fixtures and e2e rollback tests exist.
- Prefer a PathResolver/adapter path strategy over hardcoded platform paths.

- [x] **Step 1: Review WP5.F scope and platform facts**

Reviewed:

- `docs/prd/phase-5-advanced.md`
- `docs/design/resource-center-architecture.md`
- `docs/design/tool-adapter-matrix.md`
- `docs/cc-switch-reference-roadmap.md`
- Tauri v2 prerequisites/path/configuration docs
- Rust `directories` / `dirs` docs

Conclusion: Tauri and Rust path libraries provide cross-platform foundations, but WAPC's tool-specific config paths, command resolution, filesystem semantics, and safe-write pipeline need per-platform validation before any Windows/Linux write support can be enabled.

- [x] **Step 2: Add cross-platform feasibility document**

Created `docs/design/cross-platform-feasibility.md` covering:

- go/no-go conclusion
- reusable WAPC components
- platform path, command, filesystem, and release engineering blockers
- staged route F0-F4
- explicit `unsupported` boundary for non-macOS write flows
- references to Tauri and Rust platform directory docs

- [x] **Step 3: Extend tool adapter matrix**

Updated `docs/design/tool-adapter-matrix.md` with:

- Windows/Linux candidate path table for Claude Code, Codex, Gemini CLI, OpenCode, Cursor, Windsurf, VS Code, and Claude Desktop
- per-tool non-macOS write posture (`只读` or `unsupported`)
- cross-platform path strategy requirements
- Go/No-Go checklist
- expanded verification backlog

- [x] **Step 4: Update docs index**

Updated `docs/README.md` to link the cross-platform feasibility assessment.

- [x] **Step 5: Verify**

Run: `test -f docs/design/cross-platform-feasibility.md && test -f docs/design/tool-adapter-matrix.md && test -f docs/prd/phase-5-advanced.md`
Result: PASS.

Run: `rg -n "WP5\\.F|跨平台可行性|Windows/Linux|unsupported|PathResolver|Go / No-Go|非 macOS" docs/design/cross-platform-feasibility.md docs/design/tool-adapter-matrix.md docs/README.md docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`
Result: PASS, expected WP5.F scope, unsupported boundary, PathResolver plan, and Windows/Linux matrix entries are present.

Run: `git diff --check`
Result: PASS.

## Task 48: Phase 5 macOS Signing and Notarization Release Gate

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `scripts/package-macos-app.sh`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/lib.rs`
- Create: `docs/release/macos-signing-notarization.md`
- Modify: `docs/README.md`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 5 WP5.E as a real release engineering gate, not as a fake signed/notarized status.
- Missing Apple Developer signing/notarization credentials must fail the release job explicitly.
- Release builds must use Tauri's macOS bundle/signing/notarization path, not the stale manual `.app` assembly path.
- The local macOS packaging script must delegate to the same Tauri build path.
- macOS minimum version must match the README/PRD support policy of macOS 12+.
- AC-E final clean-machine Gatekeeper verification remains pending until real Apple Developer credentials are configured and a tag release is produced.

- [x] **Step 1: Add red release gate tests**

Added Tauri app contract tests:

- `tauri_macos_release_minimum_matches_readme_support_policy`
- `release_workflow_uses_tauri_signing_and_notarization_path`
- `macos_package_script_delegates_to_tauri_build`

Run: `cargo test -p wapc-app release_ -- --nocapture`
Result: FAIL before implementation because the release workflow still used `scripts/package-macos-app.sh` and copied `target/release/wapc`, and because `minimumSystemVersion` was still `10.13`.

- [x] **Step 2: Implement release workflow signing/notarization gate**

Updated `.github/workflows/release.yml` to:

- trigger on `v*` tags and `workflow_dispatch`
- build signed macOS releases for `aarch64-apple-darwin` and `x86_64-apple-darwin`
- fail fast when any required Apple signing/notarization secret is missing
- run workspace/UI release gates before packaging
- import the Developer ID `.p12` certificate into a CI keychain
- use `tauri-apps/tauri-action@v1` with `cargo tauri build`
- publish artifacts to a draft GitHub Release

- [x] **Step 3: Replace stale manual macOS package script**

Replaced `scripts/package-macos-app.sh` with a thin wrapper around:

```bash
cd src-tauri
cargo tauri build "$@"
```

The script no longer hand-creates `WAPC.app`, no longer copies the old CLI binary, and no longer ad-hoc signs a manually assembled bundle.

- [x] **Step 4: Align macOS support policy and release docs**

Updated `src-tauri/tauri.conf.json`:

- `bundle.macOS.minimumSystemVersion` is now `12.0`.

Added `docs/release/macos-signing-notarization.md` documenting:

- required GitHub Secrets
- CI release path
- local package path
- explicit no-fake-release boundary
- pending clean-machine Gatekeeper verification requirement

- [x] **Step 5: Verify**

During full-suite verification, `headless::tests::serves_read_only_summary_from_real_store_without_raw_fields` exposed a deterministic parallel-test failure on macOS: accepted streams from the nonblocking listener could return `Resource temporarily unavailable (os error 35)` on immediate read. Fixed `src/headless.rs` so each accepted stream is set back to blocking mode before request handling, and changed the test to assert the HTTP status before parsing the body.

Run: `cargo test -p wapc-app release_workflow_uses_tauri_signing_and_notarization_path -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc-app tauri_macos_release_minimum_matches_readme_support_policy -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc-app macos_package_script_delegates_to_tauri_build -- --nocapture`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 97 core tests, 19 Tauri tests, 0 main tests, 0 doctests.

Run: `cd ui && yarn test`
Result: PASS, 22 tests.

Run: `cd ui && yarn build`
Result: PASS, Vite build completed with the existing chunk-size warning.

Run: `scripts/package-macos-app.sh`
Result: PASS, produced `target/release/bundle/macos/WAPC.app` through Tauri.

Run: `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release.yml"); puts "release workflow yaml ok"'`
Result: PASS.

Run: `cargo fmt --check && git diff --check`
Result: PASS.

## Task 47: Phase 5 Resource Template Library Install Preview

**Files:**
- Create: `src/template_library.rs`
- Modify: `src/lib.rs`
- Modify: `src/model.rs`
- Modify: `src/store.rs`
- Modify: `src/cross_sync.rs`
- Modify: `src/privacy.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 5 FR-A1/FR-A2/FR-A3/FR-A4 as a real local template catalog and install preview/apply route.
- Persist template metadata in `resource_templates`; do not persist template-derived resources into `resources` merely to preview installation.
- Built-in templates must be canonical resources without real secret values.
- Template installation must route through the existing Sync Engine preview/apply path, with backup/write/rollback behavior inherited from Phase 4.
- Template source and content fingerprint must be visible in backend results and UI before write confirmation.
- Templates with env placeholders must require manual/reuse/skip env strategy just like ordinary Phase 4 sync, and manual values must not be stored.

- [x] **Step 1: Add red template catalog/store tests**

Added store tests:

- `persists_resource_templates_without_secret_payloads`
- `rejects_resource_templates_with_plain_secret_values`

Run: `cargo test -p wapc resource_templates -- --nocapture`
Result: FAIL before implementation because `ResourceTemplate`, `upsert_resource_templates`, `list_resource_templates`, and `get_resource_template` were missing; PASS after implementation.

- [x] **Step 2: Add red template install preview tests**

Added template library tests:

- `seeds_builtin_templates_with_source_fingerprint_and_env_keys`
- `plans_template_sync_without_persisting_template_as_resource`

The install preview test writes a real temporary MCP target config, seeds templates, calls `plan_template_sync`, and asserts:

- `source_resource_id` is `template:builtin:context7-mcp:<fingerprint>`
- generated plan includes `@upstash/context7-mcp`
- generated plan includes `<WAPC_MANUAL_ENV:CONTEXT7_API_KEY>`
- no raw env value appears
- `resources` remains empty during preview

Run: `cargo test -p wapc template_library -- --nocapture`
Result: FAIL before implementation because built-in/template sync APIs were missing; PASS after implementation.

- [x] **Step 3: Implement template model, built-ins, and store methods**

Added:

- `ResourceTemplate`
- `PlanTemplateSyncRequest`
- `resource_templates` table and CRUD
- `src/template_library.rs`
- built-in Context7 MCP template, sourced from Context7 MCP client docs
- `canonical_resource_from_template`
- `plan_template_sync` that routes a template-derived canonical resource directly into `cross_sync::plan_sync_from_resource`
- command/args MCP generation in `cross_sync` for safe curated templates

Production note: preview does not persist template-derived resources into `resources`; only template metadata is stored in `resource_templates`.

- [x] **Step 4: Add Tauri commands and Resource Center UI**

Added:

- `list_resource_templates`
- `plan_template_sync`
- Tauri invoke registration
- Resource Center `资源模板库` panel
- template source/fingerprint/env key display
- target selection and env strategy controls
- install preview button wired to `plan_template_sync`
- preview/apply reuse of existing Sync Engine dialog and `apply_sync`

Run: `cargo test -p wapc-app template_commands_seed_list_and_plan_without_writing_resources -- --nocapture`
Result: FAIL before helper/command implementation; PASS after implementation.

Run: `cd ui && yarn test --test-name-pattern template`
Result: FAIL before `buildTemplatePreviewResource`; PASS after helper/type implementation.

- [x] **Step 5: Privacy audit coverage**

Added `privacy_audit_names_phase_five_resource_template_boundary`.

The audit now discloses `resource_templates`, names source/fingerprint/no-secret fields, states that templates store structure/source metadata only, and adds forbidden fields for raw template secrets and template writes without preview.

Run: `cargo test -p wapc privacy_audit_names_phase_five_resource_template_boundary -- --nocapture`
Result: FAIL before privacy wording/table update; PASS after implementation.

- [x] **Step 6: Rendered UI QA**

Flow verified: Resource Center loads -> template library panel appears -> Context7 template details appear -> click `生成安装预览` -> Sync preview dialog shows template source id, env strategy, Context7 package, and `CONTEXT7_API_KEY` placeholder -> no write occurs before apply.

Result: PASS with local Playwright against `ui/dist` served by `yarn preview --host 127.0.0.1 --port 4174`.

Evidence:

- mocked only Tauri read/history commands plus `list_resource_templates` and `plan_template_sync`
- visible template: `Context7 MCP`
- visible fingerprint: `0123456789abcdef`
- visible env key: `CONTEXT7_API_KEY`
- visible preview package: `@upstash/context7-mcp`
- visible placeholder: `<WAPC_MANUAL_ENV:CONTEXT7_API_KEY>`
- called commands: `get_snapshot`, `list_changes`, `list_backups`, `list_sync_operations`, `list_sync_presets`, `list_resource_templates`, `plan_template_sync`
- forbidden write command calls before apply: none

Note: the in-app Browser plugin was read, but pre-load Tauri mock injection requires full Playwright; QA used the bundled local Playwright runtime.

- [x] **Step 7: Verify**

Run: `cargo test -p wapc template_library -- --nocapture`
Result: PASS, 2 tests.

Run: `cargo test -p wapc resource_templates -- --nocapture`
Result: PASS, 2 tests.

Run: `cargo test -p wapc privacy_audit_names_phase_five_resource_template_boundary -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc-app template_commands_seed_list_and_plan_without_writing_resources -- --nocapture`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 97 core tests, 16 Tauri tests, 0 main tests, 0 doctests.

Run: `cd ui && yarn test`
Result: PASS, 22 frontend helper tests.

Run: `cd ui && yarn build`
Result: PASS. Known warning: Vite chunk-size warning.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

Run: `git diff --check`
Result: PASS after final plan update.

## Task 46: Phase 5 Deep Link Import Safe Preview

**Files:**
- Create: `src/deep_link.rs`
- Modify: `src/lib.rs`
- Modify: `src/model.rs`
- Modify: `src/privacy.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 5 FR-B as a safe preview path for `wapc://import` deep links.
- Do not persist imported resources in this slice.
- Do not write or sync target tool files from a deep link.
- Require a visible source and content fingerprint before any later sync flow.
- Reject raw env values, Authorization headers, and token-like secrets in the link payload.
- Keep the next write step explicitly routed through the existing Sync Engine preview/confirmation flow.

- [x] **Step 1: Add red core parser tests**

Added `previews_wapc_import_link_as_canonical_resource_without_writing`.

The test builds a real `wapc://import` link with percent-encoded JSON:

- `source=https://example.test/templates/docs-mcp`
- `resource.kind=mcp`
- `resource.name=docs`
- `resource.scope=user`
- payload contains an HTTP MCP URL plus `env_keys`, but no env values

It asserts:

- schema is `wapc.deep_link_import_preview.v1`
- source is preserved and visible
- canonical resource uses `origin_tool=deep-link`
- origin locator is `wapc://import`
- resource is marked redacted because it references env placeholders
- content fingerprint is 16 characters
- no risks for HTTPS source
- no secret-looking raw values appear

Added `rejects_wapc_import_link_with_raw_env_secret` and `rejects_non_wapc_import_links`.

Run: `cargo test -p wapc deep_link -- --nocapture`
Result: FAIL before implementation because `preview_deep_link_import` was missing; PASS after implementation.

- [x] **Step 2: Implement safe deep link parser**

Added `src/deep_link.rs` with:

- `preview_deep_link_import`
- strict `wapc://import?source=...&resource=...` parsing
- percent-decoding
- canonical resource construction
- stable 16-character content fingerprint
- HTTPS source risk detection
- raw secret rejection for env values, Authorization headers, token-like values, and sensitive keys

The parser returns only `DeepLinkImportPreview`; it does not open SQLite, persist rows, or write target files.

- [x] **Step 3: Add Tauri preview command**

Added `preview_deep_link_import(url)` and registered it in the Tauri invoke handler.

Added `preview_deep_link_import_command_returns_safe_preview`.

Run: `cargo test -p wapc-app preview_deep_link_import_command_returns_safe_preview -- --nocapture`
Result: PASS after command implementation.

- [x] **Step 4: Expose Resource Center preview UI**

Added `DeepLinkImportPreview` to frontend types and a `深链导入预览` panel in Resource Center.

The UI:

- accepts a `wapc://import?...` link
- invokes `preview_deep_link_import`
- shows resource kind/name, scope, source, fingerprint, and risks
- states that preview does not save resources or write target tools

- [x] **Step 5: Privacy audit coverage**

Added `privacy_audit_names_phase_five_deep_link_import_boundary`.

The audit now states that `wapc://import` deep links are preview-only until explicitly routed through Sync Engine preview/confirmation, and that raw env values, Authorization headers, and token-like secrets are rejected.

Run: `cargo test -p wapc privacy_audit_names_phase_five_deep_link_import_boundary -- --nocapture`
Result: FAIL before privacy wording update; PASS after implementation.

- [x] **Step 6: Rendered UI QA**

Flow to verify: Resource Center loads -> deep link preview panel appears -> paste safe `wapc://import` link -> click `预览导入` -> preview shows source, resource, fingerprint -> no write/sync command is invoked.

Result: PASS with local Playwright against `ui/dist` served by `yarn preview --host 127.0.0.1 --port 4174`.

Evidence:

- mocked only Tauri read/history commands plus `preview_deep_link_import`
- visible preview text: `已生成安全预览，下一步需选择目标并走同步预览。`
- visible resource: `mcp · docs`
- visible source: `https://example.test/templates/docs-mcp`
- visible fingerprint: `0123456789abcdef`
- called commands: `get_snapshot`, `list_changes`, `list_backups`, `list_sync_operations`, `list_sync_presets`, `preview_deep_link_import`
- forbidden write/sync command calls: none

Note: the in-app Browser plugin was read and connected, but its page execution surface is read-only and cannot inject the required Tauri mock before page load; this QA used the bundled local Playwright runtime for the pre-load mock and browser interaction.

- [x] **Step 7: Verify**

Run: `cargo test -p wapc deep_link -- --nocapture`
Result: PASS, 6 deep-link/privacy-filtered tests.

Run: `cargo test -p wapc-app preview_deep_link_import_command_returns_safe_preview -- --nocapture`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 92 core tests, 15 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd ui && yarn test`
Result: PASS, 21 frontend helper tests.

Run: `cd ui && yarn build`
Result: PASS. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

Run: `git diff --check`
Result: PASS after final plan update.

## Task 45: Phase 5 Headless Read-Only Local Dashboard

**Files:**
- Create: `src/headless.rs`
- Modify: `src/lib.rs`
- Modify: `src/privacy.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/AutoScanPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 5 FR-D as a local-only, explicitly started, read-only headless dashboard.
- Default state must be off; nothing listens until the user starts it.
- Bind host must be exactly `127.0.0.1`; non-loopback bind attempts must be rejected.
- HTTP routes must be read-only. Non-GET requests return 405 and unknown write/sync paths return 404.
- Summary data must come from the real local SQLite store; no demo data.
- Headless output must avoid raw project paths, source paths, session ids, prompt/response bodies, and key material.

- [x] **Step 1: Add red core tests**

Added `rejects_non_loopback_bind_host`.

The test attempts to start the headless dashboard on `0.0.0.0` and requires an error mentioning `127.0.0.1`.

Added `serves_read_only_summary_from_real_store_without_raw_fields`.

The test writes a real `usage_records` row containing a raw `/Users/alice/work/secret-project` path, source path, session id, model, token buckets, and cost. It starts the dashboard on `127.0.0.1:0` and asserts:

- `GET /api/summary` returns HTTP 200
- schema is `wapc.headless_summary.v1`
- tool/project totals are based on the persisted SQLite row
- raw user path, project name, session id, and source file name are absent
- `POST /api/summary` returns 405
- `GET /api/sync` returns 404

Run: `cargo test -p wapc headless -- --nocapture`
Result: FAIL before implementation because `HeadlessDashboardConfig` and `start_headless_dashboard` were missing; PASS after implementation.

- [x] **Step 2: Implement read-only HTTP core**

Added `src/headless.rs` with:

- `HeadlessDashboardConfig`
- `HeadlessDashboardServer`
- `start_headless_dashboard`
- GET `/`
- GET `/health`
- GET `/api/summary`

The server uses only std `TcpListener`/`TcpStream`, validates `bind_host == 127.0.0.1`, and opens SQLite in read-query mode for summary aggregation. Project rows are represented by stable hashes only.

- [x] **Step 3: Add Tauri command contract**

Added `headless_dashboard_commands_start_disabled_and_stop_explicitly`.

The test verifies:

- `headless_dashboard_status` is off by default
- `start_headless_dashboard(None)` explicitly starts a `127.0.0.1` listener on an OS-selected port
- returned URL starts with `http://127.0.0.1:`
- `stop_headless_dashboard` turns it off

Run: `cargo test -p wapc-app headless_dashboard_commands_start_disabled_and_stop_explicitly -- --nocapture`
Result: FAIL before implementation because the Tauri commands were missing; PASS after implementation.

- [x] **Step 4: Expose desktop controls**

Added `HeadlessDashboardStatus` to frontend types and added a `只读 Headless Dashboard` section to the Auto Scan page. The UI:

- reads `headless_dashboard_status`
- shows disabled/running state
- starts and stops through real Tauri commands
- displays the local URL when running
- states the security boundary: default off, only `127.0.0.1`, read-only query page/API, no write or sync endpoints

- [x] **Step 5: Privacy audit coverage**

Added `privacy_audit_names_phase_five_headless_dashboard_boundary` and updated privacy audit wording/forbidden fields.

The audit now states that the Headless dashboard is disabled by default, binds only to `127.0.0.1`, serves read-only usage summaries only, and exposes no write/sync/import/resource mutation endpoints.

Run: `cargo test -p wapc privacy_audit_names_phase_five_headless_dashboard_boundary -- --nocapture`
Result: FAIL before audit wording update; PASS after implementation.

- [x] **Step 6: Rendered UI QA**

Flow to verify: Auto Scan page loads -> Headless section appears -> status starts closed -> click start -> request uses `start_headless_dashboard` -> URL appears with `http://127.0.0.1:` -> click stop -> status returns closed.

Result: PASS through Playwright fallback against the built `ui/dist` served by Vite preview. Evidence:

- Built `ui/dist` was served with `yarn preview --host 127.0.0.1 --port 4174`.
- A Tauri invoke mock was injected before app load to provide `get_snapshot`, `headless_dashboard_status`, `start_headless_dashboard`, and `stop_headless_dashboard`.
- Page rendered and navigated through `扫描设置`.
- `只读 Headless Dashboard` section appeared.
- Boundary copy included `默认关闭；开启后仅监听 127.0.0.1`.
- Clicking `开启只读 Dashboard` invoked `start_headless_dashboard` and displayed `已开启 http://127.0.0.1:49152`.
- Clicking `关闭只读 Dashboard` invoked `stop_headless_dashboard` and displayed `已关闭只读 Dashboard`.
- Console errors: none.
- Temporary Vite preview server was stopped after QA.

- [x] **Step 7: Verify**

Run: `cargo test -p wapc headless -- --nocapture`
Result: PASS, 3 headless/privacy-filtered tests.

Run: `cargo test -p wapc-app headless_dashboard_commands_start_disabled_and_stop_explicitly -- --nocapture`
Result: PASS.

Run: `cd ui && yarn build`
Result: PASS. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

Run: `cargo test --workspace`
Result: PASS, 86 core tests, 14 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd ui && yarn test`
Result: PASS, 21 frontend state tests. Known warning: Node `[DEP0205] module.register()` deprecation.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 44: Phase 5 Redacted Team Report Synthetic Fixture

**Files:**
- Modify: `src/model.rs`
- Modify: `src/export.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/ExportPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Complete Phase 5 FR-C3 by making redacted team reports optionally attach a reproducible synthetic fixture.
- The fixture is explicitly synthetic and must not use real project paths, project names, source paths, session ids, prompt/response bodies, or key material.
- Default redacted report exports must remain unchanged unless the user explicitly requests the fixture.
- Fixture generation must be derived from already-redacted aggregate metadata only; it must not become sample/demo runtime data or replace real report totals.

- [x] **Step 1: Add red export test**

Added `exports_redacted_team_report_with_synthetic_fixture_when_requested`.

The test writes a real usage record containing a raw `/Users/alice/work/secret-client` project path, source path, session id, model, timestamp, and secret-looking prompt fixture. It exports `view="redacted"` with `include_fixture=true` and asserts:

- report includes `fixture.schema = wapc.redacted_report_fixture.v1`
- `fixture.synthetic = true`
- fixture project key is `fixture-project-001`
- fixture carries tool/model/token bucket metadata
- raw `/Users/alice`, project name, session id, source file name, and secret-looking content are absent

Run: `cargo test -p wapc export -- --nocapture`
Result: FAIL before implementation because `ExportReportRequest` had no `include_fixture`; PASS after implementation.

- [x] **Step 2: Extend export request contract**

Added `include_fixture: bool` to `ExportReportRequest` with serde default `false`. Existing callers that omit the field keep the original behavior.

Updated the TypeScript `ExportReportRequest` type with optional `include_fixture`.

- [x] **Step 3: Generate synthetic fixture**

Updated `render_redacted_report` to build `fixture` only when requested. The fixture uses:

- schema `wapc.redacted_report_fixture.v1`
- `synthetic=true`
- stable seed label `wapc-redacted-report-fixture-v1`
- synthetic ids like `fixture-record-001-001`
- synthetic project keys like `fixture-project-001`
- aggregate tool/model/record/token/cost metadata from the redacted report path

The fixture does not serialize raw project paths, project names, source paths, session ids, file names, prompt/response bodies, or key material.

- [x] **Step 4: Expose fixture option in Export page**

When `团队脱敏报告` is selected, the page now shows `附带可复现合成 fixture`. The frontend passes `include_fixture=true` only when the checkbox is selected. Boundary copy now states that optional fixture output contains only synthetic project keys.

- [x] **Step 5: Rendered UI QA**

Flow to verify: Export page loads -> select `团队脱敏报告` -> fixture checkbox appears -> check it -> export request includes `include_fixture=true` with RFC3339 window fields when provided -> success notice appears.

Result: PASS through Playwright fallback against the built `ui/dist` served by Vite preview. Evidence:

- Built `ui/dist` was served with `yarn preview --host 127.0.0.1 --port 4174`.
- A Tauri invoke mock was injected before app load to provide `get_snapshot` and capture `export_report`.
- Page rendered and navigated to `导出`.
- Selecting `团队脱敏报告` displayed `附带可复现合成 fixture`.
- Checking the fixture option and exporting produced a request with `include_fixture=true`.
- Export request also included `view="redacted"`, `format="json"`, path `/tmp/team-redacted-fixture.json`, and UTC ISO/RFC3339 `from` / `to`.
- Boundary copy included `可选 fixture 仅包含合成项目 key`.
- Success notice displayed `已写入 /tmp/team-redacted-fixture.json`.
- Console errors: none.
- Temporary Vite preview server was stopped after QA.

- [x] **Step 6: Verify**

Run: `cargo test -p wapc export -- --nocapture`
Result: PASS, 11 export/model/privacy-related tests.

Run: `cargo test --workspace`
Result: PASS, 83 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd ui && yarn test`
Result: PASS, 21 frontend state tests. Known warning: Node `[DEP0205] module.register()` deprecation.

Run: `cd ui && yarn build`
Result: PASS. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 43: Phase 5 Redacted Team Report Time Window

**Files:**
- Modify: `src/model.rs`
- Modify: `src/store.rs`
- Modify: `src/export.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/ExportPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Extend Phase 5 redacted team report export with an optional time window.
- Window filtering must run against real persisted `usage_records` timestamps.
- Records outside the requested window must not contribute to project totals, model breakdown, model names, tools, token totals, or costs.
- Invalid time windows must fail fast in the backend.
- Keep redaction strict: no raw project paths, project names, source paths, session ids, prompt/response bodies, or secret-looking content in the redacted report.

- [x] **Step 1: Add red export tests**

Added `exports_redacted_team_report_with_time_window_only_includes_matching_records`.

The test writes two real usage records into SQLite:

- one inside `2026-06-05T00:00:00Z` to `2026-06-06T00:00:00Z`
- one outside that window with a different project name and model

It exports `view="redacted"` and asserts:

- report `window.from` and `window.to` are present
- only one project is reported
- the included model appears
- the outside model and raw project names are absent

Added `rejects_redacted_team_report_time_window_when_to_is_before_from` to assert backend rejection for an inverted time window.

Run: `cargo test -p wapc export -- --nocapture`
Result: FAIL before implementation because `ExportReportRequest` had no `from` / `to` fields; PASS after implementation.

- [x] **Step 2: Extend export request contract**

Added optional `from` and `to` fields to `ExportReportRequest` with serde defaults so older callers that only pass `view`, `format`, and `path` remain compatible.

Updated the TypeScript `ExportReportRequest` type with nullable optional `from` and `to` fields and added `redacted` to the known view union.

- [x] **Step 3: Add real SQLite window filtering**

Added `UsageStore::project_summaries_in_window` and `UsageStore::project_model_summaries_in_window`.

The existing full-summary methods now delegate to the window-aware methods with no bounds. The new methods apply fixed SQL `ts >= ?` and `ts <= ?` filters through bound parameters, then reuse the same path normalization and aggregation behavior as the full reports.

- [x] **Step 4: Wire redacted renderer to windowed summaries**

Updated `render_redacted_report` to:

- parse optional RFC3339 `from` and `to`
- reject `to < from`
- normalize bounds to UTC RFC3339 strings
- include the requested `window` in JSON and Markdown output
- read project totals and model breakdowns from the filtered store methods

- [x] **Step 5: Expose window controls in the Export page**

When `团队脱敏报告` is selected, the page now shows `开始时间` and `结束时间` controls. The frontend converts local `datetime-local` values to RFC3339 before invoking the real `export_report` command. The export boundary copy now states that team redacted reports can be exported by time window.

- [x] **Step 6: Rendered UI QA**

Flow to verify: Export page loads -> select `团队脱敏报告` -> `开始时间` and `结束时间` controls appear -> fill both values -> export request includes RFC3339 `from` and `to` -> success notice appears.

Result: PASS through Playwright fallback after the in-app Browser tab API timed out during locator reads. Evidence:

- Built `ui/dist` was served with `yarn preview --host 127.0.0.1 --port 4174`.
- A Tauri invoke mock was injected before app load to provide `get_snapshot` and capture `export_report`.
- Page rendered and navigated to `导出`.
- Selecting `团队脱敏报告` displayed `开始时间` and `结束时间`.
- Format selector options were exactly `JSON` and `Markdown`; `CSV` was hidden.
- Filled `2026-06-05T00:00` to `2026-06-06T00:00`.
- Export request included `view="redacted"`, `format="json"`, path `/tmp/team-redacted-window.json`, and UTC ISO/RFC3339 `from` / `to`.
- Success notice displayed `已写入 /tmp/team-redacted-window.json`.
- Console errors: none.
- Temporary Vite preview server was stopped after QA.

- [x] **Step 7: Verify**

Run: `cargo test -p wapc export -- --nocapture`
Result: PASS, 10 export/model/privacy-related tests.

Run: `cargo test --workspace`
Result: PASS, 82 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd ui && yarn test`
Result: PASS, 21 frontend state tests. Known warning: Node `[DEP0205] module.register()` deprecation.

Run: `cd ui && yarn build`
Result: PASS. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 42: Phase 5 Redacted Team Report Export

**Files:**
- Modify: `src/store.rs`
- Modify: `src/export.rs`
- Modify: `src/privacy.rs`
- Modify: `ui/src/pages/ExportPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 5 FR-C1/FR-C2/FR-C3 as a local, metadata-only redacted report export.
- Use real SQLite `usage_records` aggregation by project/tool/model; do not synthesize product outcomes.
- Default to strict redaction: project paths become stable hashes; source paths, session ids, project names, prompt/response bodies, and key material must not appear in the report.
- Support JSON and Markdown for the new `redacted` report view.
- Keep existing non-redacted exports unchanged for local personal use.

- [x] **Step 1: Add red export test**

Added `exports_redacted_team_report_without_paths_names_or_bodies`.

The test writes a real `UsageRecord` containing `/Users/alice/work/secret-client`, a source path, a session id, a model name, and a secret-looking body fixture. It exports `view="redacted"` and asserts:

- schema is `wapc.redacted_report.v1`
- project id is a 16-character hash
- records and model summary are present
- raw `/Users/alice`, `secret-client`, session id, source file name, and secret body text are absent

Run: `cargo test -p wapc exports_redacted_team_report_without_paths_names_or_bodies -- --nocapture`
Result: FAIL before implementation because `redacted` was unsupported; PASS after implementation.

- [x] **Step 2: Add real metadata aggregation**

Added `UsageStore::project_model_summaries`, a read-only SQLite aggregation over `usage_records` grouped by normalized project path, tool, and model. It returns token buckets, record counts, and cost only; it does not expose source paths, session ids, prompts, responses, or file bodies.

- [x] **Step 3: Implement redacted export renderer**

Added `render_redacted_report`:

- JSON output with schema `wapc.redacted_report.v1`
- Markdown output with hashed project rows
- stable path hash using a versioned WAPC prefix
- project-level totals and model breakdown
- no raw project path/name/session/source fields in the serialized report

- [x] **Step 4: Add privacy audit coverage**

Added `privacy_audit_names_phase_five_redacted_team_report_boundary` and updated privacy audit wording:

- redacted team reports hash project paths
- source paths, session ids, project names, prompt bodies, and key material are excluded
- forbidden fields include `redacted report raw project path`, `redacted report session id`, and `redacted report project name`

Run: `cargo test -p wapc privacy_audit_names_phase_five_redacted_team_report_boundary -- --nocapture`
Result: FAIL before privacy wording; PASS after implementation.

- [x] **Step 5: Expose in Export page**

Added `团队脱敏报告` to the Export page view selector. When selected, available formats are JSON and Markdown only; CSV is not offered for this report type. The page copy now states that team redacted reports hash project paths and exclude real paths, project names, session ids, bodies, and secrets.

- [x] **Step 6: Rendered UI QA**

Flow under test: Export page loads -> select `团队脱敏报告` -> format selector offers only JSON/Markdown -> export through the real `export_report` command bridge contract -> success notice appears.

Result: PASS through Playwright fallback after the in-app Browser runtime exposed an incompatible object without `newPage`. Evidence:

- Page rendered `导出本机报告`.
- `团队脱敏报告` was present in the view selector.
- Format selector options after choosing redacted view were exactly `JSON` and `Markdown`; `CSV` was hidden.
- Boundary copy included project-path hashing plus exclusion of session ids, bodies, and secrets.
- Export action displayed `已写入 /tmp/team-redacted.json`.
- Console errors: none.
- Known unrelated warning: existing Recharts width/height warning from the dashboard chart.
- Temporary Node static server was used for `ui/dist` and stopped after QA.

- [x] **Step 7: Verify**

Run: `cargo test -p wapc exports_redacted_team_report_without_paths_names_or_bodies -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc privacy_audit_names_phase_five_redacted_team_report_boundary -- --nocapture`
Result: PASS.

Run: `cargo test -p wapc export -- --nocapture`
Result: PASS, 8 export/model/privacy-related tests.

Run: `cargo test -p wapc privacy -- --nocapture`
Result: PASS, 9 privacy tests.

Run: `cd ui && yarn test`
Result: PASS, 21 frontend state tests. Known warning: Node `[DEP0205] module.register()` deprecation.

Run: `cd ui && yarn build`
Result: PASS. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

Run: `cargo test --workspace`
Result: PASS, 80 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS after `cargo fmt`.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 41: Phase 4 Sync Preset Exact Target Matching

**Files:**
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Preserve FR-33 safety after project targets were introduced.
- Applying a saved sync preset must select only the exact target paths stored in the preset.
- Do not select a project target merely because it shares the same tool id as a user target.
- Avoid legacy/fuzzy matching that could silently add cross-scope targets.

- [x] **Step 1: Add red frontend preset matching test**

Added `applies sync presets by exact target path when user and project targets share a tool`.

The test builds both user and project Cursor targets, then applies a preset containing only `/Users/example/.cursor/mcp.json`. Expected selection is `['cursor']`, not `['cursor', 'project:cursor']`.

Run: `cd ui && yarn test`
Result: FAIL before implementation because `selectedToolsFromSyncPreset` matched by `targetTools` as well as path; PASS after implementation.

- [x] **Step 2: Require exact path matching**

Updated `selectedToolsFromSyncPreset` to match available target options by `target_path` only. This keeps preset replay deterministic and prevents accidental cross-scope target selection when user and project options share the same tool name.

- [x] **Step 3: Verify**

Run: `cd ui && yarn test`
Result: PASS, 21 frontend state tests. Known warning: Node `[DEP0205] module.register()` deprecation.

Run: `cd ui && yarn build`
Result: PASS. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

Run: `cargo test --workspace`
Result: PASS, 78 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS before this plan update.

- [x] **Step 4: Rendered UI QA**

Flow under test: Resource Center page loads -> user enters `/Users/example/repo` as `项目目标路径` -> both user Cursor and project Cursor targets exist -> applying `Cursor user target` preset selects only the user Cursor target.

Result: PASS through Playwright fallback after in-app Browser runtime exposed an incompatible object without `newPage`. Evidence:

- Resource Center rendered `资源盘点` and `跨工具同步`.
- `Cursor user target` application notice was visible.
- User Cursor target `/Users/example/.cursor/mcp.json` was present and checked.
- Project Cursor target `/Users/example/repo/.cursor/mcp.json` was present and unchecked.
- Temporary Python `http.server` was not used because Python 3.14 `getfqdn` reverse lookup hung; a temporary Node static server was used and stopped after QA.

## Task 40: Phase 4 Explicit Project Sync Target Path

**Files:**
- Modify: `src/cross_sync.rs`
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement Phase 4 FR-33: project sync targets require an explicit user-provided project path.
- Do not generate fake project target rows. Project Claude/Cursor targets appear only after the user enters a project path.
- Backend must reject project targets that omit `project_path` or point outside the declared project directory.
- Cross-scope authorization remains mandatory when a user-scope source writes to a project-scope target.

- [x] **Step 1: Add backend project target safety tests**

Added tests proving:

- A project target without `project_path` returns `unsupported` and no fake plan.
- A project target whose `target_path` is outside `project_path` returns `unsupported`.
- A user MCP source can plan and apply a real project Cursor target only when `project_path` is explicit and the target file is inside it.

Run: `cargo test -p wapc project_target -- --nocapture`
Result: PASS, 3 project target tests.

- [x] **Step 2: Enforce backend project path boundary**

Added `validate_project_target_path` in `plan_target` before JSON/TOML target planning. It requires non-empty `project_path` for `scope="project"` and checks `target_path` is under that directory.

- [x] **Step 3: Add Resource Center project target controls**

Updated Resource Center state helpers and UI:

- `SyncTargetOption` now has a stable `id`, so user and project targets for the same tool can coexist.
- User target ids remain `codex`, `claude`, `gemini`, and `cursor` for preset compatibility.
- Project target ids are `project:claude` and `project:cursor`.
- The sync panel includes a `项目目标路径` input.
- Project targets are generated as real paths only after the input is filled:
  - `Claude Project` -> `<project>/.mcp.json`
  - `Cursor Project` -> `<project>/.cursor/mcp.json`
- Selecting a project target from a user source requires the existing `允许跨 Scope` checkbox before planning.

Run: `cd ui && yarn test`
Result: PASS, 20 frontend state tests.

Run: `cd ui && yarn build`
Result: PASS. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

- [x] **Step 4: Rendered UI QA**

Flow under test: Resource Center page loads -> user enters `/tmp/wapc-fr33-project` in `项目目标路径` -> project Claude/Cursor targets render -> selecting a project target requires `允许跨 Scope` -> enabling it opens `同步预览`.

Result: PASS through Playwright fallback after in-app Browser connection was unavailable (`iab` reported disconnected). Evidence:

- Page identity: `http://127.0.0.1:4177/`, title `WAPC — AI Coding Token Observer`.
- Not blank: page rendered `资源盘点` and `跨工具同步`.
- Framework overlay: none.
- Console errors: none.
- Project target rendering: `Claude Project`, `Cursor Project`, `/tmp/wapc-fr33-project/.mcp.json`, and `/tmp/wapc-fr33-project/.cursor/mcp.json` were visible.
- Interaction proof: after selecting `Claude Project`, plan button was disabled until `允许跨 Scope` was checked; then `同步预览` opened with the project target path.
- Known unrelated warning: Recharts width/height warning from existing dashboard/chart layout.

- [x] **Step 5: Verify**

Run: `cargo test --workspace`
Result: PASS, 78 core tests, 13 Tauri tests, 0 main tests, 0 doctests.

Run: `cargo fmt --check`
Result: PASS after `cargo fmt`.

Run: `cd ui && yarn test`
Result: PASS, 20 frontend state tests.

Run: `cd ui && yarn build`
Result: PASS. Known warnings: Node `[DEP0205] module.register()` deprecation and Vite chunk-size warning.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 33: Phase 4 Sync Presets Persistence and Resource Center UI

**Files:**
- Modify: `src/model.rs`
- Modify: `src/store.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement real local `sync_presets` SQLite persistence and real Tauri commands.
- Presets store resource ids and sync target metadata only; no env values, no key material, no fake preset rows.
- Resource Center may save, apply, and delete presets through real `invoke` commands; applying a preset only selects targets that are currently available for the selected resource.
- Do not implement preset JSON export, project-scope target selection, Codex TOML write UI, or fake built-in presets in this task.

- [x] **Step 1: Add red store sync preset tests**

Added `sync_presets_can_be_saved_listed_deleted_without_secret_values`.

Run: `cargo test -p wapc sync_presets_can_be_saved_listed_deleted_without_secret_values`
Result: FAIL before implementation because `SyncPreset`, `save_sync_preset`, `list_sync_presets`, and `delete_sync_preset` did not exist; PASS after implementation.

- [x] **Step 2: Implement model, schema, CRUD, and secret guard**

Added:

- `SyncPreset` model.
- `sync_presets` table migration.
- Store methods `save_sync_preset`, `list_sync_presets`, `delete_sync_preset`.
- Safety checks requiring `resources_json` and `targets_json` to be arrays.
- Safety checks rejecting preset JSON that contains `env`, `env_values`, `env_keys`, `env_fingerprints`, token/api_key/secret/authorization-like field names, or high-risk plain secret values.

- [x] **Step 3: Add red Tauri command helper test**

Added `sync_preset_command_helpers_persist_without_real_home`.

Run: `cargo test -p wapc-app sync_preset_command_helpers_persist_without_real_home`
Result: FAIL before implementation because helper functions did not exist; PASS after helper and command implementation.

- [x] **Step 4: Expose Tauri commands**

Added and registered:

- `save_sync_preset`
- `list_sync_presets`
- `delete_sync_preset`

- [x] **Step 5: Add red frontend preset helper tests**

Added tests for:

- `buildSyncPresetFromSelection` serializes current resource id and selected target metadata without env values.
- `buildSyncPresetSummary` parses real preset metadata and reports malformed JSON explicitly.
- `selectedToolsFromSyncPreset` selects only currently available target tools.

Run: `cd ui && yarn test`
Result: FAIL before helper implementation because preset helpers were not exported; PASS after implementation with 16 tests.

- [x] **Step 6: Add Resource Center preset UI**

Resource Center now:

- Calls real `list_sync_presets` while loading resource history.
- Saves current selected target set through `save_sync_preset`.
- Applies a preset by selecting only currently available targets.
- Deletes presets through `delete_sync_preset`.
- Refreshes presets from backend after save/delete.
- Keeps manual env values out of preset payloads.

- [x] **Step 7: Rendered UI QA**

Rendered QA evidence:

- Served production `ui/dist` from `/tmp/wapc-ui-qa` with a temporary `window.__TAURI_INTERNALS__.invoke` harness.
- Browser page identity: `http://localhost:4177/index.html`, title `WAPC — AI Coding Token Observer`.
- Not blank: DOM contains WAPC shell and Resource Center navigation.
- Resource Center renders `资源盘点`, `跨工具同步`, preset name input, `Existing Gemini preset`, and preset metadata `1 资源` / `1 目标`.
- Applying `Existing Gemini preset` shows `已应用同步预设 Existing Gemini preset` and keeps the Gemini target visible.
- Saving `QA Saved Preset` shows `已保存同步预设 QA Saved Preset` and renders the saved preset after backend refresh.
- Deleting `QA Saved Preset` shows `已删除同步预设 QA Saved Preset`; the delete button for that preset disappears while the existing preset remains.
- QA secret `qa-preset-secret-value` is not present in DOM at any verified state.
- Console health: no app errors; existing Recharts overview warning remains outside this Resource Center flow.
- Browser screenshot capture failed with `Page.captureScreenshot` timeout, so visual evidence is DOM/interaction based for this run.

- [x] **Step 8: Verify**

Run: `cargo test -p wapc sync_presets`
Result: PASS, 1 sync preset store test.

Run: `cargo test -p wapc-app sync_preset_command`
Result: PASS, 1 Tauri sync preset helper test.

Run: `cargo test -p wapc apply_sync`
Result: PASS, 4 apply-sync tests.

Run: `cd ui && yarn test`
Result: PASS, 16 tests, with Node `module.register()` deprecation warning.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cargo test --workspace`
Result: PASS, 71 core tests, 12 Tauri tests, 0 main tests, 0 doctests.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `cargo fmt --check`
Result: PASS.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 32: Phase 4 Sync Operation History UI and Strategy Labels

**Files:**
- Modify: `src/model.rs`
- Modify: `src/cross_sync.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Persist and display real `sync_operations` metadata from SQLite via `list_sync_operations`.
- Store only the env strategy label (`reuse`, `manual`, `skip`, or default `none`) and never store raw env values.
- Display selected-resource and recent global sync history from backend rows; malformed `targets_json` must remain explicit instead of being silently converted into fake target rows.
- Do not implement sync presets, project-scope target selection, Codex TOML write UI, or fake history entries in this task.

- [x] **Step 1: Add red backend env strategy persistence test**

Updated `env_strategy_apply_sync_manual_env_uses_memory_value_without_persisting_secret` to require `ApplySyncRequest.env_strategy = Some("manual")` and assert `sync_operations.env_strategy == "manual"` while `manual-secret` stays out of operation/change metadata.

Run: `cargo test -p wapc env_strategy_apply_sync_manual_env_uses_memory_value_without_persisting_secret`
Result: FAIL before implementation because `ApplySyncRequest` had no `env_strategy` field; PASS after implementation.

- [x] **Step 2: Persist strategy labels without persisting env values**

Added optional `ApplySyncRequest.env_strategy`; `apply_sync` persists the non-empty strategy label or `none` by default. Existing Rust tests and Tauri helper callers pass `None` unless they intentionally record a strategy.

- [x] **Step 3: Add red frontend sync history helper tests**

Added tests for:

- `buildSyncOperationSummary` parses persisted `targets_json` into target tools, paths, counts, and env strategy without exposing secrets.
- Malformed `targets_json` returns explicit `目标元数据解析失败`.
- `syncOperationMatchesResource` matches by source resource id or target path.

Run: `cd ui && yarn test`
Result: FAIL before helper implementation because `buildSyncOperationSummary` was not exported; PASS after implementation with 13 tests.

- [x] **Step 4: Add Resource Center sync history UI**

Resource Center now:

- Calls real `list_sync_operations` alongside `list_changes` and `list_backups`.
- Passes `env_strategy` to `apply_sync`.
- Shows selected-resource sync history in the detail panel.
- Shows recent global sync history in a full-width Resource Center section.
- Renders parsed target tool/kind/op/path metadata, source resource id, env strategy label, cross-scope label, and explicit target metadata parse errors.

- [x] **Step 5: Rendered UI QA**

Rendered QA evidence:

- Served production `ui/dist` from `/tmp/wapc-ui-qa` with a temporary `window.__TAURI_INTERNALS__.invoke` harness.
- Browser page identity: `http://localhost:4177/index.html`, title `WAPC — AI Coding Token Observer`.
- Not blank: DOM contains WAPC shell, overview content, and Resource Center navigation.
- Resource Center renders `资源盘点`, `跨工具同步`, `当前资源同步历史`, global `跨工具同步历史`, `sync:historyqa`, `手填 env`, `.gemini/settings.json`, and `.cursor/mcp.json`.
- QA secret `qa-history-secret-value` is not present in DOM.
- Manual env strategy + `同步到...` opens `同步预览`, shows `<WAPC_MANUAL_ENV:GITHUB_TOKEN>`, accepts a password input, and confirms sync with `同步完成 sync:manualqa，成功 1 个目标`.
- Entered secret `qa-manual-secret` is not present after confirmation.
- Console health: no app errors; existing Recharts overview warning remains outside this Resource Center flow.
- Browser screenshot capture failed with `Page.captureScreenshot` timeout, so visual evidence is DOM/interaction based for this run.

- [x] **Step 6: Verify**

Run: `cargo test -p wapc apply_sync`
Result: PASS, 4 apply-sync tests.

Run: `cargo test -p wapc-app apply_sync_command`
Result: PASS, 1 Tauri apply-sync helper test.

Run: `cd ui && yarn test`
Result: PASS, 13 tests, with Node `module.register()` deprecation warning.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cargo test --workspace`
Result: PASS, 70 core tests, 11 Tauri tests, 0 main tests, 0 doctests.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`.

Run: `cargo fmt --check`
Result: PASS.

Run: `git diff --check`
Result: PASS after this plan update.

## Task 27: Phase 3 Resource Management UI and Explicit Rollback Slice

**Files:**
- Modify: `src/sync_engine.rs`
- Modify: `src/store.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/pages/resourceCenterState.ts`
- Modify: `ui/src/pages/ResourcesPage.tsx`
- Modify: `ui/tests/resourceCenterState.test.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Add explicit rollback support before exposing rollback UI.
- Keep write controls narrow: only user-scoped, non-plugin, JSON MCP resources can generate a disable plan.
- Unsupported resources must remain visibly read-only with a reason; no fake enable/edit/delete controls.
- The preview dialog must show target path, before/after content, diff, backup requirement, risks, cancel, and explicit confirm.
- The UI may call real `plan_resource_change`, `apply_resource_change`, `list_changes`, `list_backups`, and `rollback_change`; it must not mutate resources locally as a substitute for backend state.

- [x] **Step 1: Add rollback red tests and core implementation**

Added:

- `rollback_resource_change_restores_backup_and_records_revert_change`

Result:

- RED: missing `rollback_resource_change`.
- GREEN: `rollback_resource_change(home, store, change_id)` restores the recorded backup, verifies file bytes, records a new rollback change with `reverts_change_id`, marks the original change `rolledback`, and backs up the pre-rollback state.

- [x] **Step 2: Add Tauri rollback command contract**

Added:

- `rollback_change_helper_restores_previous_resource_state`
- `rollback_change(change_id)` command

Result:

- RED: missing `rollback_change_for_paths`.
- GREEN: helper and command restore the previous resource state without touching real home in tests.

- [x] **Step 3: Add UI management gating tests**

Added:

- `enables disable action only for user json mcp resources without plugin ownership`
- `keeps unsupported resource management entries visibly read-only`

Result: PASS with `cd ui && yarn test`.

- [x] **Step 4: Implement Resource Center management UI**

Implemented:

- Management panel in resource detail.
- Disabled/read-only reasons for unsupported resources.
- `plan_resource_change` preview dialog with before/after and diff.
- `apply_resource_change` confirmation flow with drift confirmation state.
- Change log and backup metadata panel.
- `rollback_change` button only for committed, non-rollback changes with a backup.
- Refresh inventory and history after apply/rollback through backend commands.

- [x] **Step 5: Rendered UI QA**

Attempted:

- `cd src-tauri && cargo tauri dev`
- Computer Use against WAPC window

Initial result:

- BLOCKED for reliable visual evidence in this environment: macOS/Computer Use selected the already bundled release app with the same bundle identifier `com.wapc.app` instead of the dev `target/debug/wapc-app`; the selected release window stayed on `加载中…`.
- Follow-up: change the dev bundle identifier or add a deterministic local mock harness for rendered UI QA. Do not claim rendered UI QA complete from this attempt.

Follow-up implementation and QA:

- Added a failing config contract for macOS-safe bundle identifier, then changed identifier from `com.wapc.app` to `com.wapc.desktop`.
- Added a failing config contract for auto-created main window with explicit `/` URL, then changed the window config to `create: true` and `url: "/"`.
- Added explicit `ActivationPolicy::Regular` and post-create `show/focus` for the main window.
- Rebuilt with `cd src-tauri && cargo tauri build`; the previous Tauri warning about identifier ending in `.app` is gone.
- Direct Computer Use still cannot reliably expose the local Tauri WebView window in this environment, even though Tauri setup can create the `main` webview.
- Used a deterministic temporary QA harness outside the repo at `/tmp/wapc-ui-qa`, serving the production `ui/dist` with a preloaded `window.__TAURI_INTERNALS__.invoke` mock. This verifies UI rendering and interaction only; real command behavior remains covered by Rust/Tauri tests.

Rendered QA evidence:

- Browser page identity: `http://127.0.0.1:4177/`, title `WAPC — AI Coding Token Observer`.
- Not blank: DOM contains WAPC shell, sidebar, and Resource Center.
- Resource Center: `资源盘点`, management panel, change log, and backup panel are visible.
- Preview flow: clicking `禁用 MCP` opens `写入预览` with `修改前`, `修改后`, `Diff`, and `确认写入`.
- Read-only gating: selecting a `skill` resource shows `当前切片仅开放 JSON MCP 禁用`; `只读` button is disabled.
- Apply/rollback UI: confirm apply shows `已提交变更`; rollback shows `已回滚变更` and a `reverts chg:` record.
- Console health: no app errors; an existing Recharts overview warning about chart container width remains outside this Resource Center flow.
- Screenshot capture through Browser failed with `Page.captureScreenshot` timeout, so visual evidence is DOM/interaction based for this run.

- [x] **Step 6: Final verification**

Run: `cargo fmt --check`
Result: PASS.

Run: `cargo test --workspace`
Result: PASS, 60 core tests, 9 Tauri tests, 0 main tests, 0 doctests.

Run: `cd ui && yarn test`
Result: PASS, 7 tests, with Node `module.register()` deprecation warning.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, bundled `target/release/bundle/macos/WAPC.app`; the previous bundle identifier warning is gone.

Run: `git diff --check`
Result: PASS before this plan update; rerun after this plan update before final handoff.

## Task 24: Phase 3 Sync Engine Core Foundation

**Files:**
- Create: `src/sync_engine.rs`
- Modify: `src/model.rs`
- Modify: `src/store.rs`
- Modify: `src/lib.rs`
- Modify: `src/privacy.rs`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Implement the real local safety pipeline for one concrete single-tool operation: disabling one MCP entry in a JSON MCP file.
- The pipeline must be `plan -> preview data -> backup -> atomic write -> verify -> commit`, with automatic rollback on verify failure.
- No cross-tool sync, no fake plans, no demo resources, and no direct write path outside the Sync Engine.
- Backups may contain original tool secrets; backup files must be written under `~/.wapc/backups/<tool>/<timestamp>/...` with owner-only permissions where the platform supports it, and privacy audit must disclose this boundary.
- The plan must detect drift by comparing the target file fingerprint from plan time with the current file before apply.

- [x] **Step 1: Add red Sync Engine model and behavior tests**

Add tests in `src/sync_engine.rs` for:

- `plan_disable_mcp_json_entry_returns_diff_without_writing_file`
- `apply_disable_mcp_json_entry_backs_up_writes_verifies_and_commits_change`
- `apply_disable_mcp_json_entry_blocks_unconfirmed_drift_without_writing`
- `apply_disable_mcp_json_entry_rolls_back_when_verify_fails`
- `apply_disable_mcp_json_entry_rotates_old_tool_backups`

Run: `cargo test -p wapc sync_engine`
Result: FAIL before implementation because `sync_engine` did not exist, then PASS after implementation.

- [x] **Step 2: Implement Sync Engine core types**

Add serializable models in `src/model.rs`: `ResourceChangeRequest`, `WritePlan`, `WritePlanRisk`, `ApplyChangeRequest`, `ApplyChangeResult`, `ResourceChangeLog`, and `ResourceBackup`.

Add `src/sync_engine.rs` with pure plan/apply functions for JSON MCP disable:

- `plan_resource_change(home, request) -> WritePlan`
- `apply_resource_change(home, plan, confirm_drift, verify_override) -> ApplyChangeResult`
- atomic write helper using temp file + fsync + rename
- backup helper under `~/.wapc/backups`
- rollback-on-verify-failure helper
- backup rotation helper that keeps the latest 10 backup groups per tool and removes stale backup metadata

- [x] **Step 3: Persist changes, backups, and file fingerprints**

Add SQLite tables from Phase 3 PRD:

- `resource_changes`
- `resource_backups`
- `file_fingerprints`

Add store methods:

- `record_file_fingerprint`
- `get_file_fingerprint`
- `insert_resource_change`
- `list_resource_changes`
- `insert_resource_backup`
- `list_resource_backups`
- `delete_resource_backup`

- [x] **Step 4: Add privacy audit wording**

Update `privacy_audit` to disclose:

- `~/.wapc/backups` may contain original tool config contents, including secrets already present in the source tool file.
- `resource_changes` stores metadata, not secret values.
- `file_fingerprints` stores hashes only.

- [x] **Step 5: Verify**

Run: `cargo test -p wapc sync_engine`
Result: PASS, 5 Sync Engine tests.

Run: `cargo test -p wapc privacy`
Result: PASS through `cargo test --workspace`, including Phase 3 privacy audit coverage.

Run: `cargo test --workspace`
Result: PASS, 59 core tests + 6 Tauri helper tests + doc-tests.

Run: `git diff --check`
Result: PASS.

## Task 25: Phase 3 Tauri Command Contract for Safe Single-Tool Writes

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `ui/src/types/index.ts`
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Expose only plan/apply/list primitives backed by the Sync Engine.
- `plan_resource_change` must not write files.
- `apply_resource_change` must require a server-generated `WritePlan`; drift requires explicit `confirm_drift`.
- UI management controls remain a later task until command contract and tests are green.

- [x] **Step 1: Add command tests**

Add Tauri helper tests that plan a temp JSON MCP disable operation, verify no write during plan, apply it, and list changes/backups.

Run: `cargo test -p wapc-app plan_resource_change`
Result: FAIL before command helper implementation, then PASS with `plan_and_apply_resource_change_helpers_use_sync_engine_without_real_home`.

- [x] **Step 2: Add Tauri commands**

Expose:

- `plan_resource_change`
- `apply_resource_change`
- `list_changes`
- `list_backups`

- [x] **Step 3: Add frontend types only**

Mirror the command request/result structs in `ui/src/types/index.ts`; do not add write buttons yet.

- [x] **Step 4: Verify**

Run: `cargo test -p wapc-app`
Result: PASS, 6 tests.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `git diff --check`
Result: PASS.

## Task 26: Phase 3 Acceptance Audit Before UI Write Controls

**Files:**
- Modify: `docs/superpowers/plans/2026-06-05-wapc-docs-roadmap-iteration.md`

Production boundary for this slice:

- Audit AC-1 through AC-6 and AC-8 for backend/command readiness before adding visible write controls.
- Do not declare Phase 3 UI resource management complete until preview modal, backup/change page, rollback UI, Guide Center, and enterprise read-only gating are implemented and verified.

- [x] **Step 1: Record evidence matrix**

Map tests and runtime commands to Phase 3 AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, and AC-8. Leave AC-7 and AC-9 open unless Guide Center and enterprise gating are implemented.

Evidence matrix:

- AC-1 plan-only preview: `plan_disable_mcp_json_entry_returns_diff_without_writing_file`, `plan_and_apply_resource_change_helpers_use_sync_engine_without_real_home`.
- AC-2 backup before write: `apply_disable_mcp_json_entry_backs_up_writes_verifies_and_commits_change`.
- AC-3 atomic write and verify: `apply_disable_mcp_json_entry_backs_up_writes_verifies_and_commits_change`.
- AC-4 drift detection: `apply_disable_mcp_json_entry_blocks_unconfirmed_drift_without_writing`.
- AC-5 automatic rollback on verify failure: `apply_disable_mcp_json_entry_rolls_back_when_verify_fails`.
- AC-6 backup retention: `apply_disable_mcp_json_entry_rotates_old_tool_backups`.
- AC-8 privacy boundary: `privacy_audit_names_phase_three_backup_and_change_boundaries`.
- AC-7 restore UI remains open.
- AC-9 Guide Center and enterprise read-only gating remain open.

- [x] **Step 2: Verify**

Run: `cargo test --workspace`
Result: PASS, 59 core tests + 6 Tauri helper tests + doc-tests.

Run: `cd ui && yarn test`
Result: PASS, 5 tests.

Run: `cd ui && yarn build`
Result: PASS, with existing Vite chunk-size and Node `module.register()` deprecation warnings.

Run: `cd src-tauri && cargo tauri build`
Result: PASS, built `/Users/wumin/workspace/github/WAPC/target/release/bundle/macos/WAPC.app`; existing warning remains for bundle identifier `com.wapc.app` ending in `.app`.

Run: `git diff --check`
Result: PASS.
