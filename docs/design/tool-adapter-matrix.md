# 工具适配矩阵(Tool Adapter Matrix)

> 状态:草案 v1,待审核 + 待逐项核验
> 最后更新:2026-06-07
> 关联:[资源中心架构设计](resource-center-architecture.md)

本文是 WAPC 适配器层的"事实库":每个 AI 编程工具,各类资源**存在哪个文件、用什么格式、字段怎么映射、有什么坑**。这是 `ToolAdapter` 的实现依据。

> ⚠️ 路径与字段名会随工具版本变化。落地实现前,每一格都要在真机上核验,并写入脱敏 fixture。标 `(待核验)` 的项是基于公开格式的推断,需确认。

---

## 1. 配置文件总览

| 工具 | 主配置 | MCP 位置 | 指令文件 | Skills | 数据源(已支持) |
| --- | --- | --- | --- | --- | --- |
| Claude Code | `~/.claude/settings.json`、`~/.claude.json` | `~/.claude.json` → `mcpServers`(也支持 `.mcp.json` 项目级) | `CLAUDE.md`(项目)、`~/.claude/CLAUDE.md`(user) | `~/.claude/skills/`、`<project>/.claude/skills/` | `~/.claude/projects/**/*.jsonl` |
| Codex | `~/.codex/config.toml` | `~/.codex/config.toml` → `[mcp_servers.*]` | `AGENTS.md`(项目)、`~/.codex/AGENTS.md` | 无原生 skills(待核验) | `~/.codex/sessions/**/*.jsonl` |
| Gemini CLI | `~/.gemini/settings.json` | `~/.gemini/settings.json` → `mcpServers` | `GEMINI.md` | 无原生 skills(待核验) | `~/.gemini/tmp/**/chats/*.json` |
| OpenCode | `~/.config/opencode/opencode.json`、`<project>/opencode.json` | `opencode.json` → `mcp`;WAPC 当前只读扫描 user/project config | `AGENTS.md`、`~/.config/opencode/AGENTS.md`;也可通过 `opencode.json` 的 `instructions` 引用额外文件 | `.opencode/skills/`、`~/.config/opencode/skills/`;兼容 `.claude/skills` / `.agents/skills`;WAPC 当前只读扫描原生 user/project skill 目录 | `~/.local/share/opencode/storage/**/*.json` |
| Cursor | `~/.cursor/`、`<project>/.cursor/` | `~/.cursor/mcp.json`、`<project>/.cursor/mcp.json` → `mcpServers` | `.cursor/rules/*.mdc`、`.cursorrules`(legacy) | 不适用 | (IDE,后续识别) |
| Windsurf | `~/.codeium/windsurf/` | `~/.codeium/windsurf/mcp_config.json` → `mcpServers` | `.windsurfrules`(待核验) | 不适用 | 后续识别 |
| VS Code (Copilot) | `.vscode/` | `.vscode/mcp.json` → `servers`;user profile `mcp.json` 路径待核验 | `.github/copilot-instructions.md` | 不适用 | 后续识别 |
| Claude Desktop | `~/Library/Application Support/Claude/claude_desktop_config.json` | 同文件 → `mcpServers` | 不适用 | 不适用 | 桌面应用 |

## 2. MCP 字段映射

不同工具的 MCP 配置结构相似但不完全一致。WAPC canonical MCP(见架构文档 §3.2)与各工具映射:

### 2.1 Claude Code / Claude Desktop / Cursor / Gemini(JSON 系,`mcpServers`)

形态接近,均为 JSON 对象,key 是 server 名:

```jsonc
// stdio
"mcpServers": {
  "github": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-github"],
    "env": { "GITHUB_TOKEN": "..." }      // ← 敏感,canonical 只存指纹
  }
}
// http / sse / streamable HTTP(字段名随工具;详见 mcp-field-verification.md)
"mcpServers": {
  "remote": { "type": "sse", "url": "https://..." }   // Claude/Cursor JSON 系示例;Codex/Gemini remote 字段不同
}
```

映射:

| canonical | JSON 系字段 |
| --- | --- |
| `transport=stdio` | 有 `command` |
| `transport=http/sse` | 工具差异:Claude/Cursor 可含 `type` + `url`;Gemini 使用 `url` / `httpUrl`;Codex TOML remote 示例使用 `url` |
| `command` / `args` | 同名 |
| `url` | `url` |
| `env_keys` / `env_fingerprints` | 由 `env` 推导,**原值不落库** |

### 2.2 Codex(TOML 系,`[mcp_servers.*]`)

```toml
[mcp_servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "..." }     # ← 敏感
```

坑点:

- TOML ↔ JSON 转换由 CodexAdapter 负责;数组/内联表的格式化要稳定(便于 diff)。
- Codex 官方 Docs MCP 示例使用 `[mcp_servers.<name>]` + `url`;运行态连接与版本兼容仍需单独验收。

### 2.3 转换矩阵

| 源 → 目标 | 主要转换 |
| --- | --- |
| Claude Code(JSON)→ Codex(TOML) | JSON 对象 → TOML table;`env` 脱敏后由用户提供真值 |
| Codex(TOML)→ Gemini(JSON) | TOML table → JSON;注意布尔/数字类型保真 |
| 任意 → Cursor(JSON) | 直接 `mcpServers` 注入 `~/.cursor/mcp.json` 或项目 `.cursor/mcp.json` |

### 2.4 OpenCode(JSON 系,`mcp`)

OpenCode MCP 配置位于 user/project `opencode.json` 顶层 `mcp`:

```jsonc
{
  "mcp": {
    "context7": {
      "type": "remote",
      "url": "https://mcp.context7.com/mcp",
      "headers": { "CONTEXT7_API_KEY": "{env:CONTEXT7_API_KEY}" }
    },
    "local-docs": {
      "type": "local",
      "command": ["npx", "-y", "@upstash/context7-mcp"],
      "environment": { "CONTEXT7_API_KEY": "{env:CONTEXT7_API_KEY}" }
    }
  }
}
```

映射:

| canonical | OpenCode 字段 |
| --- | --- |
| `transport=stdio` | `type="local"` |
| `transport=http` | `type="remote"` |
| `command` / `args` | `command` 数组首项为命令,后续为 args |
| `url` | `url` |
| `env_keys` / `env_fingerprints` | 由 `environment` 推导,原值不落库 |
| `header_keys` / `header_fingerprints` | 由 `headers` 推导,敏感 header 原值不落库 |
| `enabled` | 由 `enabled` 推导;`false` 时资源仍进入清单,但不计入 `enabled_in` |

当前只读扫描配置 metadata。OpenCode auth/OAuth 运行态状态、`mcp-auth.json` 凭据存储、`opencode mcp auth/list/debug` 命令结果和写入仍不进入支持范围。

## 3. 指令文件方言(Instruction Dialects)

| 方言 | 文件 | 特点 |
| --- | --- | --- |
| `agents-md` | `AGENTS.md` | 通用约定,Codex/OpenCode 等读取;纯 Markdown |
| `claude-md` | `CLAUDE.md` | Claude Code 读取;支持 `@import` 其他文件、user/project/enterprise 分层 |
| `gemini-md` | `GEMINI.md` | Gemini CLI 读取 |
| `cursor-rules` | `.cursor/rules/*.mdc` | 带 frontmatter(`description`、`globs`、`alwaysApply`),按文件拆分规则;旧版 `.cursorrules` 单文件 |
| `copilot` | `.github/copilot-instructions.md` | VS Code Copilot workspace always-on instructions;WAPC 只读扫描结构指纹 |

适配要点:

- **同一份指令,多方言落地**:WAPC canonical instruction 以 Markdown 段落树为中心。写入 Cursor `.mdc` 时需要生成 frontmatter;写入 `CLAUDE.md` 时保留 `@import`。
- **不盲目合并**:不同工具的指令语义可能不同,默认"按工具分别管理 + 可选同步",不强行统一成一份。
- **正文不落库**:只存标题树 + 段 hash + 大小(见架构文档 §3.5)。

## 4. Skills

| 工具 | 机制 | 位置 |
| --- | --- | --- |
| Claude Code | `SKILL.md` + 资源文件,放技能目录;支持 user/project/plugin 来源 | `~/.claude/skills/<name>/`、`<project>/.claude/skills/<name>/` |
| OpenCode | `SKILL.md` + YAML frontmatter;官方识别 `name`、`description`、`license`、`compatibility`、`metadata` | `.opencode/skills/<name>/SKILL.md`、`~/.config/opencode/skills/<name>/SKILL.md`;兼容 `.claude/skills/` 与 `.agents/skills/` |
| 其他 CLI | 多数无原生 skills 概念(待核验) | — |

安装策略(借鉴 CC Switch):

- `symlink`:WAPC 在 `~/.wapc/skills/<name>/` 维护真身,软链到各工具技能目录。便于集中更新、统一卸载。
- `copy`:直接复制(适合不支持 symlink 的场景)。
- 备份:同步/覆盖前备份到 `~/.wapc/skill-backups/`,保留最近 20 份。

## 5. Plugins / Subagents(Claude Code 生态为主)

| 资源 | 位置 | canonical 关注点 |
| --- | --- | --- |
| Plugins | `~/.claude/plugins/`(含 marketplace 安装的插件) | 插件名、来源 marketplace、版本、包含组件(commands/agents/hooks/mcp) |
| Subagents | `~/.claude/agents/*.md`、`<project>/.claude/agents/*.md` | 名称、allowed tools、model、scope;定义为带 frontmatter 的 Markdown |

注意:Plugin 可能**自带** MCP/commands/agents,识别时要避免与独立 MCP 资源重复计数 —— canonical 用内容指纹去重,并记录 `provided_by_plugin` 关系。

## 6. 作用域(Scope)解析规则

| scope | 典型位置 | 优先级 |
| --- | --- | --- |
| enterprise | 系统级托管目录(各工具不同,待核验) | 最高,WAPC 默认**只读不写** |
| user | `~/.<tool>/...` | 中 |
| project | repo 根 / `<project>/.<tool>/...` | 最低但最常用 |

WAPC 同步默认**不跨 scope**(project ↔ user 需显式开启),enterprise 级一律只读。

## 7. 适配器实现核对清单(每个工具落地前逐项打勾)

- [x] 配置文件真实路径(user / project)在 macOS 上核验
- [x] MCP 字段名与 transport 取值核验(尤其 http/sse 的 `type`/`url`)
- [x] 格式化稳定(TOML/JSON 序列化结果可重复,利于 diff)
- [x] 敏感字段识别规则(env、token、apiKey、headers 里的 Authorization)
- [x] 指令方言 frontmatter 生成/解析
- [x] 写入路径的备份与原子写验证
- [x] 脱敏 fixture 入库,补单测

## 8. 跨平台路径策略

Phase 5 WP5.F 不直接开放 Windows/Linux 写入能力。本节只记录候选路径与核验状态,用于后续 PathResolver 和只读 detector 设计。所有标记为 `待核验` 的项,在没有真机 fixture 前不得进入写入目标列表。

| 工具 | macOS 当前路径 | Windows 候选路径 | Linux 候选路径 | 状态 | 写入策略 |
| --- | --- | --- | --- | --- | --- |
| Claude Code | `~/.claude/settings.json`, `~/.claude.json`, `~/.claude/projects/**/*.jsonl` | `%USERPROFILE%\.claude\...`(待核验) | `~/.claude/...`(待核验) | macOS metadata 已核验;Win/Linux 待核验 | 非 macOS 只读 |
| Codex | `~/.codex/config.toml`, `~/.codex/sessions/**/*.jsonl` | `%USERPROFILE%\.codex\config.toml` 或 AppData 派生目录(待核验) | `~/.codex/config.toml`, `~/.codex/sessions/**/*.jsonl`(待核验) | macOS metadata 已核验;Win/Linux 待核验 | 非 macOS 只读 |
| Gemini CLI | `~/.gemini/settings.json`, `~/.gemini/tmp/**/chats/*.json` | `%USERPROFILE%\.gemini\settings.json`(待核验) | `~/.gemini/settings.json`(待核验) | macOS metadata 已核验;Win/Linux 待核验 | 非 macOS 只读 |
| OpenCode | `~/.config/opencode/opencode.json`, `~/.config/opencode/AGENTS.md`, `~/.config/opencode/skills`, `~/.local/share/opencode/storage/**/*.json`, `<project>/AGENTS.md`, `<project>/.opencode/skills` | `%APPDATA%\opencode\...` 或 `%LOCALAPPDATA%\opencode\...`(待核验) | `$XDG_CONFIG_HOME/opencode/opencode.json`, `$XDG_CONFIG_HOME/opencode/AGENTS.md`, `$XDG_CONFIG_HOME/opencode/skills`, `$XDG_DATA_HOME/opencode/storage/**/*.json`(待核验) | instructions/skills 官方机制已核验;macOS metadata 已核验;Linux 候选优先 | 非 macOS 只读 |
| Cursor | `~/.cursor/mcp.json`, `<project>/.cursor/mcp.json`, `.cursor/rules/*.mdc` | `%USERPROFILE%\.cursor\mcp.json`, `%APPDATA%\Cursor\User\...`(待核验) | `~/.cursor/mcp.json`, `$XDG_CONFIG_HOME/Cursor/User/...`(待核验) | macOS metadata 已核验;多路径风险高 | 非 macOS unsupported |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | `%APPDATA%\Codeium\Windsurf\...`(待核验) | `$XDG_CONFIG_HOME/Codeium/Windsurf/...`(待核验) | 待公开格式核验 | 非 macOS unsupported |
| VS Code (Copilot) | `<project>/.vscode/mcp.json`, `<project>/.github/copilot-instructions.md`, user profile `mcp.json`(路径待核验) | `%APPDATA%\Code\User\...`, `<project>\.vscode\mcp.json`, `<project>\.github\copilot-instructions.md`(待核验) | `$XDG_CONFIG_HOME/Code/User/...`, `<project>/.vscode/mcp.json`, `<project>/.github/copilot-instructions.md`(待核验) | workspace MCP `servers` 与 Copilot instructions 已只读识别;user profile 真机路径待核验 | 非 macOS 只读候选 |
| Claude Desktop | `~/Library/Application Support/Claude/claude_desktop_config.json` | `%APPDATA%\Claude\claude_desktop_config.json`(待核验) | 无稳定支持结论(待核验) | GUI 应用单独核验 | 非 macOS unsupported |

路径解析要求:

- WAPC 自身 config/data/cache 目录使用 Tauri path API 或 Rust `directories`/`dirs` 的平台目录结果,不要手写 `~/Library`、`%APPDATA%` 或 `$XDG_*`。
- 工具路径由 `ToolAdapter` 暴露候选列表,每个候选带 `platform`、`scope`、`verified`、`read_only`。
- Project 级路径必须由显式 project root 派生,不能用进程当前目录隐式推断。
- Windows 命令解析必须识别 `.exe`/`.cmd`/`.bat`,并保留原始 command 字段用于预览。
- Symlink 安装策略默认只在 macOS/Linux 开放;Windows 先使用 copy,symlink 需用户显式启用并通过权限检测。

## 9. 跨平台 Go / No-Go 清单

- [x] cross-platform core smoke CI 在 `ubuntu-latest` 和 `windows-latest` 上通过不访问真实用户目录的基础测试:`cargo clippy --workspace --exclude wapc-app --all-targets -- -D warnings`、`cargo test --workspace --exclude wapc-app`、`yarn --cwd ui lint`、`yarn --cwd ui test`、`yarn --cwd ui build`;不构建 Tauri GUI bundle。
- [x] PathResolver 覆盖 macOS/Windows/Linux 的 home/config/data/project root 样例。
- [ ] 至少 Codex + Gemini CLI 完成 Windows/Linux 只读路径真机核验。
- [ ] 新增平台 fixture 进入 `privacy-audit`,且不包含 prompt/response/source body/密钥。
- [ ] 每个非 macOS 写入目标都有 plan/backup/write/verify/rollback e2e 测试后,才从 `unsupported` 改为可选。

说明:当前勾选项仅代表 `PathResolver` 单元样例覆盖 drive letter、XDG/AppData 与带空格 project root,以及 GitHub Actions 的 cross-platform core smoke CI 覆盖非 Tauri GUI 的 Rust core 与 UI lint/test/build。`privacy-audit` 已列出 Windows/Linux 候选路径的只读、待核验、写入 unsupported 边界,但 Windows/Linux 真机路径核验、只读 fixture、Tauri GUI bundle 与任何写入支持仍未完成。

配置文件真实路径 macOS 勾选项仅代表 `docs/design/macos-path-verification.md` 已在当前 macOS 工作站对 `tool_path_candidates` 输出的 user/project 候选做 metadata 核验,并新增 `verify_tool_path_candidates` 单测保证报告不读取配置正文、不泄露真实 home/project 绝对路径或密钥。该勾选不代表配置内容解析、MCP 运行态连接、OAuth/header 行为或 Windows/Linux 真机路径已完成验收。

MCP 字段名与 transport 取值勾选项仅代表 `docs/design/mcp-field-verification.md` 已基于官方文档核验 Claude Code、Codex、Gemini CLI、Cursor 与 VS Code Copilot 的主要配置键、remote URL 字段和 transport 名称;VS Code workspace `.vscode/mcp.json` 顶层 key 已确认为 `servers`,并已进入只读 Resource Inventory 扫描。该勾选不代表这些工具在当前机器、所有版本、OAuth/header 场景或 Windows/Linux 平台已完成运行态连接验收。

敏感字段识别规则勾选项仅代表本地 canonical payload 入库前脱敏规则已有单测覆盖:env 原值、args 中疑似 token、top-level `apiKey`/token 类字段、`headers.Authorization` 与 `X-Api-Key` 均只保留 key 与指纹,不保存原值。该勾选不代表各工具远程 MCP 的 OAuth、headers 或连接行为已经运行态真机验收。

格式化稳定勾选项仅代表 WAPC 生成的跨工具 MCP sync preview 已统一走 canonical JSON/TOML pretty serializer,并有单测验证生成结果再次解析/序列化后字节不变。该勾选不代表保留原配置文件注释、手写排版或所有未来工具方言。

写入路径的备份与原子写验证勾选项仅代表当前已开放写入路径:单工具 JSON MCP disable、跨工具 JSON/TOML MCP sync、Resource Center/Tauri command helper 均走 Sync Engine 的 backup -> atomic write -> verify -> commit/rollback 流程,并有单测覆盖 backup 记录、drift 阻断、verify 失败回滚、备份轮转、Tauri helper 和 rollback。Resource Center `disable_mcp` 管理动作当前仅对 user-scope Claude/Cursor JSON `mcpServers.*` 资源开放;跨工具同步目标当前支持 Claude/Cursor/Gemini JSON MCP 与 Codex TOML MCP 的预览和写入。OpenCode `mcp`、VS Code `servers`、enterprise、非 macOS、instruction/frontmatter、skill/plugin/subagent 写入仍保持 unsupported。

指令方言 frontmatter 勾选项仅代表 Cursor `.mdc` 的内存生成器与 scanner 解析链路已有单测覆盖:生成器输出 `description`、`globs`、`alwaysApply` frontmatter;scanner 只持久化 description 指纹、globs、always_apply 和正文结构指纹,不保存正文或 description 原文。该勾选不代表 instruction/frontmatter 写入已经开放;当前写入仍保持 unsupported,后续必须单独接入 Sync Engine preview/backup/write/verify/rollback。

OpenCode MCP、指令与 skills 机制结论来自 `docs/design/opencode-resource-verification.md`:官方文档确认 `opencode.json` 顶层 `mcp`、`AGENTS.md`、`~/.config/opencode/AGENTS.md`、`.opencode/skills/<name>/SKILL.md`、`~/.config/opencode/skills/<name>/SKILL.md` 以及 Claude / agents 兼容 skill fallback。WAPC 当前已支持 OpenCode user/project MCP 只读识别、`AGENTS.md` 只读识别,并只读扫描 OpenCode 原生 user/project skill 目录;只保存 redacted MCP metadata、文件元数据、内容 hash、frontmatter keys 和 description 指纹。OpenCode `enabled:false` MCP 会进入清单但不计入 `enabled_in`;auth/OAuth 运行态状态、skill 安装/同步/写入、权限策略和 rollback 仍保持 unsupported。

VS Code Copilot 当前仅支持 project/workspace scope 的 `.vscode/mcp.json` 只读扫描,读取 top-level `servers` 并沿用 MCP payload 脱敏规则。user profile `mcp.json` 的真实路径、VS Code 运行态读取行为、OAuth/header 连接行为和任何写入仍待核验。

VS Code Copilot instructions 当前仅支持 project/workspace scope 的 `.github/copilot-instructions.md` 只读扫描,作为 `copilot` 方言保存标题树、段落 hash 与字节数,不保存正文。`.instructions.md`、AGENTS/CLAUDE fallback 的 VS Code 运行态优先级、user profile instructions、组织级 instructions 和任何写入仍待核验。

脱敏 fixture 勾选项对应仓库内 `tests/fixtures/resource_inventory/redacted-home`,覆盖 user/project scope 的 MCP、instruction、skill、plugin、subagent,以及 plugin-provided MCP/subagent。单测会扫描该 fixture,核对资源数量,并断言 fixture 与扫描结果不包含真实密钥形态、真实用户路径或正文原文。

## 10. 待核验清单(汇总)

以下项基于公开格式推断,落地前必须真机确认:

1. Codex `config.toml`、Gemini CLI `settings.json`、Cursor/Claude remote MCP 字段的运行态连接与版本兼容。
2. OAuth、headers、环境变量展开在各工具中的真实行为。
3. OpenCode MCP auth/OAuth 运行态状态、skills 权限策略、模板安装与写入回滚。
4. Windsurf `.windsurfrules` 与企业级目录。
5. VS Code Copilot user profile `mcp.json` 的真实路径与读取行为。
6. 各工具 enterprise/managed 级配置目录位置。
7. Cursor `.mdc` frontmatter 的完整字段集合。
8. Windows/Linux 上各工具 user config 与 data dir 的真实位置。
9. Windows MCP command 的 `.cmd`/`.exe` 解析与 quoting 规则。
10. Linux AppImage/deb/rpm 打包后数据目录和 WebKit 依赖行为。
