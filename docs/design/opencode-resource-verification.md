# OpenCode 指令与 Skills 官方核验

> 状态:官方资料核验 + 本机 metadata 补充
> 核验日期:2026-06-07
> 关联:[工具适配矩阵](tool-adapter-matrix.md)

本文记录 OpenCode MCP、指令文件与 skills 机制的官方资料结论、WAPC 当前 PathResolver 的候选路径落点,以及 Resource Center 当前已支持的只读扫描边界。它不代表 OpenCode MCP 运行态 auth/OAuth 状态、skill 安装、同步、权限策略或写入回滚已经完成。

## 官方结论

| 能力 | 官方来源 | 结论 | WAPC 当前处理 |
| --- | --- | --- | --- |
| MCP servers | https://dev.opencode.ai/docs/mcp-servers | OpenCode 在 `opencode.json` 顶层 `mcp` 下配置 MCP;local 使用 `type: "local"` + `command` 数组 + `environment`;remote 使用 `type: "remote"` + `url` + `headers` / `oauth`;可通过 `enabled` 临时禁用 | Resource Center 只读扫描 user/project `opencode.json` 的 `mcp`;local 映射为 `stdio`,remote 映射为 `http`;`enabled:false` 资源仍进入清单但不计入 `enabled_in`;`environment`/headers 敏感值只保存 key 与指纹;不读取或声明运行态 auth/OAuth 状态 |
| Rules / instructions | https://dev.opencode.ai/docs/rules | OpenCode 使用 `AGENTS.md` 作为项目规则;全局规则可放在 `~/.config/opencode/AGENTS.md`;也兼容 `CLAUDE.md` / `~/.claude/CLAUDE.md` fallback;`opencode.json` 支持 `instructions` 列表引用额外文件 | `AGENTS.md` 作为 `agents-md` 方言只读识别;不解析远程 instruction URL;不开放写入 |
| Skills | https://dev.opencode.ai/docs/skills | OpenCode 发现 `.opencode/skills/<name>/SKILL.md`、`~/.config/opencode/skills/<name>/SKILL.md`,并兼容 `.claude/skills` 与 `.agents/skills`;frontmatter 识别 `name`、`description`、`license`、`compatibility`、`metadata` | Resource Center 只读扫描 OpenCode 原生 user/project skill 目录;只保存文件元数据、内容 hash、frontmatter keys 和 description 指纹;OpenCode skill 安装/同步/写入仍 unsupported |

## 本机 metadata 补充

| 工具 | scope | kind | 路径 | exists | type |
| --- | --- | --- | --- | --- | --- |
| OpenCode | user | instruction_file | `~/.config/opencode/AGENTS.md` | no | missing |
| OpenCode | user | mcp_config | `~/.config/opencode/opencode.json` | yes | file |
| OpenCode | user | skill_dir | `~/.config/opencode/skills` | yes | dir |
| OpenCode | project | project_mcp_config | `<project>/opencode.json` | no | missing |
| OpenCode | project | project_instruction_file | `<project>/AGENTS.md` | no | missing |
| OpenCode | project | project_skill_dir | `<project>/.opencode/skills` | no | missing |

## 边界

- 当前只确认官方路径/机制、本机 metadata、OpenCode MCP 只读 scanner 和 OpenCode 原生 user/project skill 的只读 scanner,不持久化 OpenCode 配置正文或 skill 正文。
- `enabled:false` 只作为配置状态 metadata 保存;WAPC 不把该 MCP 标记为已在 OpenCode 启用。
- OpenCode MCP 运行态连接、auth/OAuth 状态、`~/.local/share/opencode/mcp-auth.json` 凭据存储、`opencode mcp auth/debug/list` 命令结果和 Resource Center 写入/禁用动作不进入当前支持范围。
- `AGENTS.md` 可同时被 Codex 与 OpenCode 消费;WAPC 以工具归属分别记录,但不复制正文。
- OpenCode skill 模板安装、symlink/copy 策略、权限处理和写入回滚尚未进入支持范围。
- `.agents/skills` 与 Claude-compatible skill fallback 已记录为官方机制,但 WAPC 当前仅先暴露 OpenCode 原生 `.opencode` / `~/.config/opencode` 候选。
