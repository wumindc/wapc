# WAPC CLI Token Observer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an installable macOS-first Rust CLI that passively indexes local AI coding tool token usage without wrapping commands, installing proxies, or modifying the tools.

**Architecture:** WAPC uses tool-specific collectors to read existing local session files, normalize token usage into one model, and persist it in SQLite. The CLI exposes scan, report, and privacy-audit commands so the first release works without a desktop UI.

**Tech Stack:** Rust 2024, clap, serde_json, rusqlite with bundled SQLite, walkdir, chrono.

---

### Task 1: Project Skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `src/lib.rs`
- Modify: `src/main.rs`

- [x] Add runtime dependencies for CLI parsing, JSON parsing, walking local directories, SQLite, and timestamps.
- [x] Split the crate into a reusable library plus a thin binary entrypoint.
- [x] Keep comments privacy-oriented and include `@author codex` in generated comments.

### Task 2: Unified Usage Model

**Files:**
- Create: `src/model.rs`
- Test: `src/model.rs`

- [x] Add `TokenUsage`, `UsageRecord`, `SourcePrecision`, and `ToolKind`.
- [x] Test total token arithmetic including cache and reasoning buckets.

### Task 3: Passive Collectors

**Files:**
- Create: `src/collectors/mod.rs`
- Create: `src/collectors/claude.rs`
- Create: `src/collectors/codex.rs`
- Create: `src/collectors/gemini.rs`
- Create: `src/collectors/opencode.rs`

- [x] Parse Claude Code JSONL `message.usage`.
- [x] Parse Codex rollout JSONL `last_token_usage`.
- [x] Parse Gemini CLI chat JSON `messages.*.tokens`.
- [x] Parse OpenCode storage part JSON `tokens.*` and `cost`.
- [x] Never persist prompt or response text.

### Task 4: SQLite Store

**Files:**
- Create: `src/store.rs`

- [x] Create schema for usage records.
- [x] Upsert by stable event id.
- [x] Add daily and tool summaries.

### Task 5: CLI

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`

- [x] Implement `wapc scan`.
- [x] Implement `wapc report today`.
- [x] Implement `wapc report --tool <name>`.
- [x] Implement `wapc privacy-audit`.
- [x] Support `--home` and `--db` for tests and advanced users.

### Task 6: Verification and Install

**Files:**
- Create: `README.md`

- [x] Run unit tests.
- [x] Run `wapc scan --dry-run` against local AI tool directories.
- [x] Run real `wapc scan` into local SQLite.
- [x] Verify `cargo install --path .` installs the binary.
- [x] Document install and uninstall commands.

### Task 7: Completion Features

**Files:**
- Create: `src/launchd.rs`
- Modify: `src/cli.rs`
- Modify: `src/store.rs`
- Modify: `README.md`

- [x] Add `wapc doctor` for install and source self-checks.
- [x] Add `wapc service install|uninstall|status` for macOS LaunchAgent periodic scans.
- [x] Add `wapc report --group project`.
- [x] Add `wapc report --json`.
- [x] Verify service commands with the installed binary.
