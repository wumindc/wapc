# macOS 工具路径本机核验证据

> 状态:本机 metadata 证据
> 核验日期:2026-06-07
> 关联:[工具适配矩阵](tool-adapter-matrix.md)

本文记录 WAPC 当前 macOS 工作站上对 `PlatformPathContext::macos` / `tool_path_candidates` 候选路径的只读核验结果。核验只调用文件系统 metadata 判断 `exists` / `file` / `dir`,不读取任何配置文件正文、会话内容、prompt、response、源码正文或密钥。

路径脱敏规则:

- user home 统一显示为 `~`。
- 当前项目根统一显示为 `<project>`。
- 缺失路径同样保留在表中,用于证明候选列表覆盖而不是只记录已存在项。

## 本机结果

| 工具 | scope | kind | 路径 | exists | type |
| --- | --- | --- | --- | --- | --- |
| Claude Code | user | config_dir | `~/.claude` | yes | dir |
| Claude Code | user | data_dir | `~/.claude/projects` | yes | dir |
| Claude Code | user | session_data | `~/.claude/projects` | yes | dir |
| Claude Code | user | skill_dir | `~/.claude/skills` | yes | dir |
| Claude Code | user | plugin_dir | `~/.claude/plugins` | yes | dir |
| Claude Code | user | subagent_dir | `~/.claude/agents` | no | missing |
| Claude Code | user | instruction_file | `~/.claude/CLAUDE.md` | no | missing |
| Claude Code | user | mcp_config | `~/.claude.json` | yes | file |
| Codex | user | config_dir | `~/.codex` | yes | dir |
| Codex | user | data_dir | `~/.codex/sessions` | yes | dir |
| Codex | user | session_data | `~/.codex/sessions` | yes | dir |
| Codex | user | session_data | `~/.codex/archived_sessions` | yes | dir |
| Codex | user | mcp_config | `~/.codex/config.toml` | yes | file |
| Codex | user | instruction_file | `~/.codex/AGENTS.md` | yes | file |
| Gemini CLI | user | config_dir | `~/.gemini` | yes | dir |
| Gemini CLI | user | data_dir | `~/.gemini/tmp` | yes | dir |
| Gemini CLI | user | session_data | `~/.gemini/tmp` | yes | dir |
| Gemini CLI | user | mcp_config | `~/.gemini/settings.json` | yes | file |
| Gemini CLI | user | instruction_file | `~/.gemini/GEMINI.md` | yes | file |
| OpenCode | user | config_dir | `~/.config/opencode` | yes | dir |
| OpenCode | user | data_dir | `~/.local/share/opencode/storage` | yes | dir |
| OpenCode | user | session_data | `~/.local/share/opencode/storage` | yes | dir |
| OpenCode | user | mcp_config | `~/.config/opencode/opencode.json` | yes | file |
| OpenCode | user | instruction_file | `~/.config/opencode/AGENTS.md` | no | missing |
| OpenCode | user | skill_dir | `~/.config/opencode/skills` | yes | dir |
| Cursor | user | mcp_config | `~/.cursor/mcp.json` | yes | file |
| Cursor | user | instruction_file | `~/.cursorrules` | no | missing |
| Cursor | user | instruction_dir | `~/.cursor/rules` | no | missing |
| Claude Code | project | project_mcp_config | `<project>/.mcp.json` | no | missing |
| Cursor | project | project_mcp_config | `<project>/.cursor/mcp.json` | no | missing |
| VS Code Copilot | project | project_mcp_config | `<project>/.vscode/mcp.json` | no | missing |
| OpenCode | project | project_mcp_config | `<project>/opencode.json` | no | missing |
| Claude Code | project | project_skill_dir | `<project>/.claude/skills` | no | missing |
| OpenCode | project | project_skill_dir | `<project>/.opencode/skills` | no | missing |
| Claude Code | project | project_subagent_dir | `<project>/.claude/agents` | no | missing |
| Claude Code | project | project_instruction_file | `<project>/CLAUDE.md` | no | missing |
| Codex | project | project_instruction_file | `<project>/AGENTS.md` | no | missing |
| OpenCode | project | project_instruction_file | `<project>/AGENTS.md` | no | missing |
| Gemini CLI | project | project_instruction_file | `<project>/GEMINI.md` | no | missing |
| VS Code Copilot | project | project_instruction_file | `<project>/.github/copilot-instructions.md` | no | missing |
| Cursor | project | project_instruction_file | `<project>/.cursorrules` | no | missing |
| Cursor | project | project_instruction_dir | `<project>/.cursor/rules` | no | missing |

## 工程化验证

新增 `verify_tool_path_candidates` 作为只读候选路径核验函数:

- 输入来自 `PlatformPathContext` 和 `tool_path_candidates`,不在业务层拼接平台绝对路径。
- 输出只包含工具、平台、scope、kind、脱敏路径、候选核验标记、存在性、file/dir 类型、read-only 与 write-supported 元数据。
- 单测 `verifies_macos_candidates_from_filesystem_without_reading_contents` 使用临时 home/project 创建真实文件和目录,并把文件内容设为 `SHOULD_NOT_BE_READ_SECRET`;序列化报告不得包含该字符串。

## 边界

本证据只说明当前 macOS 机器上的候选路径 metadata 已核验。它不代表:

- 配置文件内容已经解析成功。
- MCP server 已经真实连接、OAuth/header 已经运行态验证。
- Windows/Linux 路径已在真机核验。
- 非 macOS 写入、instruction/frontmatter 写入、skills/plugins/subagents 写入已经开放。
