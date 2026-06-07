# MCP 字段与 Transport 官方核验

> 本文只记录字段名与 transport 取值的官方资料核验,不代表 WAPC 已经完成所有工具的真机连接、OAuth、网络可达性或版本兼容验证。写入/同步仍只开放已在 Sync Engine 中有测试证据的路径。

## 核验范围

| 工具 | 官方来源 | 配置位置 / 字段 | transport 结论 | WAPC 当前处理 |
| --- | --- | --- | --- | --- |
| Claude Code | https://code.claude.com/docs/en/mcp | `~/.claude.json`、项目 `.mcp.json`; `mcpServers.<name>.type` + `url`; stdio 使用 `command` / `args` / `env` | CLI 使用 `--transport http` / `stdio`; JSON `type` 支持 `http`,并说明 `streamable-http` 可作为 `http` alias | 可读取 `mcpServers`; 写入仅限当前 Sync Engine 已支持的 JSON MCP 路径 |
| Codex | https://developers.openai.com/learn/docs-mcp | `~/.codex/config.toml`; `[mcp_servers.<name>]`; remote 示例使用 `url` | OpenAI 官方 Docs MCP 示例使用 `codex mcp add <name> --url ...`; 直接 TOML 示例不要求 `type` 字段 | Codex TOML sync preview 生成 `[mcp_servers.<name>]` + `url`; 不伪造 `type` |
| Gemini CLI | https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md | `settings.json`; top-level `mcpServers`; stdio 使用 `command` / `args` / `env`; remote 使用 `url` 或 `httpUrl`; headers 通过 CLI `--header` | 官方文档列出 Stdio、SSE、Streamable HTTP; `gemini mcp add --transport` 支持 `stdio` / `sse` / `http` | 读取 `mcpServers`; remote `url` / `httpUrl` 需要保持字段差异,不统一伪造成一个字段 |
| OpenCode | https://dev.opencode.ai/docs/mcp-servers | user/project `opencode.json`; top-level `mcp`; local 使用 `type: "local"` + `command` 数组 + `environment`; remote 使用 `type: "remote"` + `url` + `headers` / `oauth` | 官方文档区分 local 和 remote; WAPC canonical 将 local 映射为 `stdio`,remote 映射为 `http`; `enabled` 可表达是否启用 | WAPC 已只读识别 user/project `opencode.json` 的 `mcp`;`enabled:false` 不计入 `enabled_in`;运行态 auth/OAuth 状态、凭据存储、连接调试和写入仍不开放 |
| Cursor | https://docs.cursor.com/context/model-context-protocol | global `~/.cursor/mcp.json`; project `.cursor/mcp.json`; top-level `mcpServers`; stdio 用 `command` / `args` / `env`; remote 用 `url` / `headers` | 官方 docs 描述 `stdio`、`SSE`、`Streamable HTTP` 三类 transport; Cursor 配置对 `command`、`args`、`env`、`url`、`headers` 支持变量插值 | 读取 user/project `mcp.json`; 只在已支持 JSON MCP sync 路径写入 |
| VS Code Copilot | https://code.visualstudio.com/docs/agents/reference/mcp-configuration | workspace `.vscode/mcp.json` 或 user profile `mcp.json`; top-level `servers`; 可选 top-level `inputs` 与 `sandbox`; stdio 使用 `type: "stdio"`、`command`、`args`、`env`; remote 使用 `type: "http"` / `"sse"` + `url`、`headers`、`oauth` | VS Code reference 明确支持 stdio、HTTP、SSE,并说明 HTTP 会优先尝试 streamable HTTP,再 fallback 到 SSE; sandbox 仅 macOS/Linux | WAPC 已只读识别 workspace `.vscode/mcp.json` 的 `servers`; user profile 真机路径、运行态连接、OAuth/header 行为与写入仍不开放 |

## 仍未完成

- 以上只证明字段名与 transport 取值来自官方文档,不证明本机所有版本已经连接成功。
- OAuth、headers、环境变量展开、项目/用户 scope 的实际写入副作用仍需要逐工具真机验收。
- Windows/Linux 路径与命令 quoting 仍未真机核验。
- OpenCode、VS Code Copilot、Windsurf、Claude Desktop 仍未进入 WAPC 当前写入目标。
