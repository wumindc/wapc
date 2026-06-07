# WAPC 资源中心架构设计:统一识别 · 适配 · 同步 · 注入

> 状态:草案 v1,待审核
> 最后更新:2026-06-05
> 关联:[CC Switch 研究与路线图](../cc-switch-reference-roadmap.md)、[工具适配矩阵](tool-adapter-matrix.md)

本文设计 WAPC 的 **Resource Center**:把 Skills、MCP、Plugins、指令文件(AGENTS.md / CLAUDE.md / GEMINI.md / Cursor rules)、Subagents 等资源,做到"**自动识别 → 规范化 → 跨工具适配 → 安全同步注入**"。这是 WAPC 从"token 观测器"走向"统一观测与管理工具"的核心引擎。

设计目标:在**无侵入、只存元数据、写入必备份**三条原则下,让用户用一份资源定义,统一管理并安全分发到本机各种 AI 编程工具。

---

## 1. 问题陈述

本机的 AI 编程工具各自为政:

- **同一种资源,N 种格式**:MCP 在 Claude Code 是 `~/.claude.json` 的 `mcpServers`(JSON),在 Codex 是 `~/.codex/config.toml` 的 `[mcp_servers]`(TOML),在 Gemini 是 `~/.gemini/settings.json`,在 Cursor 是 `~/.cursor/mcp.json`。
- **同一种意图,N 个文件**:项目指令在 Claude Code 是 `CLAUDE.md`,Codex/通用是 `AGENTS.md`,Gemini 是 `GEMINI.md`,Cursor 是 `.cursor/rules/*.mdc`。
- **作用域分散**:有 user 级(`~`)、project 级(repo 根)、甚至 enterprise 级。
- **手动同步易错**:用户在一个工具里配好的 MCP/Skill,想复制到另一个工具,只能手动改格式,容易写坏。

WAPC 要把这些差异收敛到一个**规范化中间层(Canonical Model)**,并通过**适配器(Adapter)**双向转换。

## 2. 总体架构

```text
┌──────────────────────────────────────────────────────────────┐
│                        Resource Center                         │
│                                                                │
│   ┌────────────┐   ┌─────────────┐   ┌──────────────────┐      │
│   │ Detectors  │──▶│  Canonical  │◀──│   Sync Engine    │      │
│   │ (只读识别) │   │   Store     │   │ preview/backup/  │      │
│   └─────┬──────┘   │  (SSOT)     │   │ atomic-write/    │      │
│         │          └──────┬──────┘   │ verify/rollback  │      │
│         │                 │          └────────┬─────────┘      │
│         ▼                 ▼                   ▼                 │
│   ┌──────────────────────────────────────────────────┐        │
│   │              Tool Adapter Layer                    │        │
│   │  ClaudeCode │ Codex │ Gemini │ Cursor │ OpenCode … │        │
│   │  read(): Canonical    write(Canonical): Plan       │        │
│   └──────────────────────────┬─────────────────────────┘        │
└──────────────────────────────┼────────────────────────────────┘
                               ▼
        本机真实文件:~/.claude.json / ~/.codex/config.toml /
        ~/.gemini/settings.json / ~/.cursor/mcp.json / *.md …
```

四个层次:

1. **Detectors(只读识别)**:扫描本机,发现工具及其资源,产出只读清单。永不写入。
2. **Canonical Store(规范化 SSOT)**:把异构资源规范成统一模型,落 `~/.wapc/wapc.db`。这是单一数据源。
3. **Tool Adapter Layer(适配器)**:每个工具一个适配器,负责 `read`(工具格式 → canonical)和 `write`(canonical → 工具格式的写入计划)。工具差异只活在这一层。
4. **Sync Engine(同步引擎)**:统一的安全写入管线,负责预览 / 备份 / 原子写 / 校验 / 回滚。所有写操作必须经过它。

## 3. 资源类型模型(Canonical Model)

WAPC 识别五类资源,每类有统一的 canonical 结构。下面用伪结构表示(实际为 Rust struct + serde)。

### 3.1 公共信封

```jsonc
{
  "id": "stable-content-hash",      // 内容指纹,跨工具去重
  "kind": "mcp | skill | plugin | instruction | subagent",
  "name": "github",
  "scope": "user | project | enterprise",
  "origin": {                        // 在哪个工具/文件被发现
    "tool": "claude-code",
    "path": "~/.claude.json",
    "locator": "mcpServers.github"   // 文件内定位
  },
  "confidence": 0.0,                 // 解析置信度
  "redacted": true,                  // 是否对敏感字段做了脱敏
  "last_seen": "2026-06-05T..."
}
```

### 3.2 MCP Server

```jsonc
{
  "kind": "mcp",
  "name": "github",
  "transport": "stdio | http | sse",
  "command": "npx",                  // stdio
  "args": ["-y", "@modelcontextprotocol/server-github"],
  "url": null,                       // http/sse
  "env_keys": ["GITHUB_TOKEN"],      // 只记 key 名,不记值
  "env_fingerprints": {              // 值做指纹,绝不落原文
    "GITHUB_TOKEN": { "len": 40, "prefix": "ghp_", "sha256_8": "a1b2c3d4" }
  },
  "enabled_in": ["claude-code", "cursor"]
}
```

> 隐私关键点:MCP 的 `env` 往往含密钥。WAPC **只记录 key 名 + 值指纹(长度/前缀/哈希前 8 位)**,从不存原文。注入时若目标需要真实值,由用户在写入预览界面显式提供或选择"沿用目标工具现有值"。

### 3.3 Skill

```jsonc
{
  "kind": "skill",
  "name": "pdf",
  "source": "github:owner/repo@ref | zip | local",
  "files": ["SKILL.md", "scripts/..."],
  "content_hash": "...",
  "install_strategy": "symlink | copy",
  "installed_in": ["claude-code"]
}
```

### 3.4 Plugin

```jsonc
{
  "kind": "plugin",
  "name": "my-plugin",
  "marketplace": "github:owner/marketplace",
  "version": "1.2.0",
  "components": ["commands", "agents", "hooks", "mcp"],
  "enabled_in": ["claude-code"]
}
```

### 3.5 Instruction(指令文件)

```jsonc
{
  "kind": "instruction",
  "name": "AGENTS.md",
  "scope": "project",
  "project_path": "/Users/.../repo",
  "dialect": "agents-md | claude-md | gemini-md | cursor-rules | copilot",
  "sections": [ { "heading": "...", "hash": "..." } ],  // 只存结构指纹
  "byte_size": 2048,
  "applies_to": ["codex", "claude-code"]
}
```

> 指令文件正文是否落库?**默认不落正文**,只落结构指纹(标题树 + 各段 hash + 大小)。当用户在 Resource Center 主动编辑/同步某份指令时,才在内存中加载正文用于 diff 与写入,不持久化到 db。

### 3.6 Subagent(Agents)

```jsonc
{
  "kind": "subagent",
  "name": "code-reviewer",
  "definition_path": "~/.claude/agents/code-reviewer.md",
  "tools": ["Read", "Grep", "Bash"],
  "model": "inherit",
  "scope": "user | project"
}
```

## 4. Detectors(只读识别)

每个工具的 Detector 负责回答:这个工具装了吗?配置在哪?有哪些资源?Detector **只读**,产出 `Vec<CanonicalResource>` 与 `DataSourceHealth`。

识别策略(分层):

1. **存在性探测**:配置目录/可执行文件是否存在(如 `~/.claude/`、`which codex`)。
2. **资源解析**:解析各资源文件,转 canonical,记录 `confidence`。解析失败的文件计入 `failed_files`,不中断。
3. **健康度**:目录存在、可读文件数、解析记录数、失败文件数、最新事件时间 —— 复用现有 `doctor` 的口径。

CLI 形态(只读,默认 dry/print):

```bash
wapc inventory tools                 # 本机工具清单 + 版本 + 配置目录
wapc inventory mcp                   # 跨工具 MCP 清单(脱敏)
wapc inventory skills
wapc inventory plugins
wapc inventory instructions [--project PATH]
wapc inventory subagents
wapc inventory all --json
```

## 5. Tool Adapter Layer

适配器是整个设计的"收口点"。接口(Rust trait 草案):

```rust
pub trait ToolAdapter {
    fn tool_id(&self) -> &'static str;            // "claude-code"
    fn detect(&self, home: &Path) -> DetectResult; // 存在性 + 配置目录 + 版本

    /// 只读:把工具真实配置读成 canonical 资源
    fn read(&self, kind: ResourceKind, ctx: &ScanCtx) -> Result<Vec<CanonicalResource>>;

    /// 规划写入:给定目标 canonical 资源,产出"写入计划"(不落盘)
    /// 计划里包含:目标文件、变更前后内容、diff、是否需要备份、风险标记
    fn plan_write(&self, change: &ResourceChange, ctx: &ScanCtx) -> Result<WritePlan>;

    /// 能力声明:本工具支持哪些 kind / scope / transport
    fn capabilities(&self) -> AdapterCapabilities;
}
```

关键点:

- **适配器不直接写盘**。它只产出 `WritePlan`,交给 Sync Engine 执行。这样备份/原子写/回滚逻辑只实现一次。
- **能力声明驱动 UI**:某工具不支持 sse 传输,UI 就灰掉对应选项,而不是写坏。
- **格式转换在适配器内**:JSON ↔ TOML ↔ Markdown frontmatter 的差异都封装在各自适配器。

各工具的真实文件位置、字段映射、坑点见 [工具适配矩阵](tool-adapter-matrix.md)。

## 6. Sync Engine(安全写入管线)

这是"写入必备份"的工程落地。**任何修改外部工具文件的操作都走这条管线,无例外。**

```text
ResourceChange
     │
     ▼
[1] plan      ── adapter.plan_write() → WritePlan(目标文件/before/after/diff/风险)
     │
     ▼
[2] preview   ── 展示 diff,等待用户确认(CLI: --dry-run 默认; Desktop: diff 视图)
     │
     ▼
[3] backup    ── 复制目标文件到 ~/.wapc/backups/<tool>/<ts>/<file>,轮转保留 N 份
     │
     ▼
[4] write     ── 原子写:写临时文件 → fsync → rename 覆盖目标
     │
     ▼
[5] verify    ── 重新 read 目标,确认 canonical 与预期一致;失败则触发 rollback
     │
     ▼
[6] commit/rollback ── 记录变更日志(变更 id、备份路径、可回滚)
```

设计约束:

- **默认 dry-run**:CLI 写命令默认只打印计划;必须显式 `--apply` 才真正写。
- **回填保护(借鉴 CC Switch)**:写入前先 `read` 目标现状,若目标已被用户手改且与 WAPC 已知状态不一致,标红并要求确认,绝不静默覆盖。
- **原子写**:temp + rename,避免半写损坏。
- **备份轮转**:`~/.wapc/backups/` 按工具/时间组织,保留最近 N 份(默认 10),Skills 单独目录保留 20 份。
- **一键回滚**:`wapc resource rollback <change-id>` 从备份恢复。
- **变更日志**:每次 commit 写一条审计记录(谁、何时、改了哪个文件、备份在哪)。

CLI 形态:

```bash
wapc resource diff   --kind mcp --name github --to codex      # 看会怎么写,不落盘
wapc resource sync   --kind mcp --name github --to codex --apply
wapc resource backup list
wapc resource rollback <change-id>
```

## 7. 跨工具同步的典型流程

场景:用户在 Claude Code 配好了 `github` MCP,想同步到 Codex 和 Cursor。

```text
1. detect  : ClaudeCodeAdapter.read(mcp) → canonical{github}
2. select  : 用户在 Resource Center 选 github,目标勾选 Codex + Cursor
3. env     : github 含 GITHUB_TOKEN。WAPC 不持有原值 →
             选项 A:沿用目标工具现有同名 env(若存在)
             选项 B:用户在预览界面手动填入(不落库)
4. plan    : CodexAdapter.plan_write() → 写 ~/.codex/config.toml [mcp_servers.github](TOML)
             CursorAdapter.plan_write() → 写 ~/.cursor/mcp.json mcpServers.github(JSON)
5. preview : 展示两份 diff
6. apply   : Sync Engine 对每个目标 backup → atomic-write → verify
7. log     : 写两条变更记录,可分别 rollback
```

## 8. 隐私与安全设计

延续 WAPC 隐私边界,并针对资源管理新增规则:

| 数据 | 是否落库 | 处理方式 |
| --- | --- | --- |
| 工具名 / 配置路径 / 资源名 | 是 | 直接落库 |
| MCP command/args/url | 是 | 落库(注意 args 里可能夹带 token,做模式脱敏) |
| MCP/Provider env 值 | **否** | 只记 key 名 + 值指纹(len/prefix/sha256 前 8) |
| 指令文件正文 | **否(默认)** | 只记结构指纹;编辑时内存加载,不持久化 |
| Skill/Plugin 文件内容 | **否** | 只记 content_hash + 文件清单 |
| prompt / response / 源码 / 输出正文 | **否** | 从不读取、从不落库 |

`wapc privacy-audit` 必须同步列出资源管理新增读取的目录与字段,保持"读什么、存什么"可审计。

## 9. 与现有代码的衔接

当前结构(`src/`):`cli.rs` / `collectors/` / `scanner.rs` / `store.rs` / `model.rs` / `launchd.rs`。建议演进:

- 新增 `src/resource/`:`mod.rs`、`canonical.rs`(模型)、`detect.rs`、`sync.rs`(管线)、`backup.rs`。
- 新增 `src/adapters/`:每个工具一个文件,实现 `ToolAdapter`。复用 `collectors/` 已有的工具路径常量。
- `store.rs` 扩展资源表:`resources`、`resource_changes`、`resource_backups`。
- `cli.rs` 增加 `inventory` 与 `resource` 两个子命令族(只读优先,写入默认 dry-run)。
- 桌面侧 `src-tauri/commands.rs` 暴露对应只读查询 + plan/preview;apply 必须二次确认。

## 10. 分阶段交付(与路线图对齐)

| 阶段 | 交付 | 验收 |
| --- | --- | --- |
| P0 只读识别 | Detectors + Canonical Store + `inventory *` CLI | 各工具资源清单可 `--json`,隐私审计覆盖 |
| P1 适配只读 | 全部工具 `ToolAdapter::read` + 桌面 Resource Center 列表/详情 | 跨工具去重、置信度、四态 UI |
| P2 安全写入 | Sync Engine(preview/backup/atomic/verify/rollback)+ `resource diff/sync/rollback` | 默认 dry-run、回填保护、可回滚 |
| P3 跨工具同步 | 一份 canonical → 多工具适配写入 | 端到端同步一个 MCP/Skill,含 env 脱敏流程 |
| P4 进阶 | 深链 `wapc://`、模板库、团队脱敏报告 | 后续单独设计 |

## 11. 待决问题(请审核时拍板)

1. **指令文件正文**:确认"默认只存结构指纹、编辑时内存加载"是否可接受,还是允许用户显式开启正文索引?
2. **env 真实值**:同步含密钥的 MCP 时,采用"沿用目标现有值 / 预览时手填 / 拒绝同步"三选一的默认策略?建议默认"沿用目标现有值,缺失则要求手填"。
3. **作用域优先级**:project 级资源是否允许同步到 user 级(反之亦然)?建议默认禁止跨 scope,需显式开启。
4. **支持工具的优先级**:P0 先覆盖哪几个工具?建议 Claude Code / Codex / Gemini CLI / Cursor 四个先行。
5. **是否引入 `wapc://` 深链**:放长期还是中期?
