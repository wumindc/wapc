# VS Code Copilot 资源官方核验

> 状态:官方资料核验 + 本机 metadata 补充
> 核验日期:2026-06-06
> 关联:[工具适配矩阵](tool-adapter-matrix.md)

本文记录 VS Code Copilot workspace 级 MCP 与 instructions 资源的官方资料结论和 WAPC 当前支持边界。它不代表 VS Code user profile、组织级 instructions、OAuth/header 运行态连接或写入流程已经完成。

## 官方结论

| 能力 | 官方来源 | 结论 | WAPC 当前处理 |
| --- | --- | --- | --- |
| Workspace MCP | https://code.visualstudio.com/docs/agents/reference/mcp-configuration | MCP 配置可放在 workspace `.vscode/mcp.json`;顶层 `servers`;支持 stdio、http、sse、headers、oauth、inputs、sandbox | 只读扫描 project `.vscode/mcp.json` 的 `servers`;沿用 MCP payload 脱敏;不开放写入 |
| Copilot instructions | https://code.visualstudio.com/docs/copilot/customization/custom-instructions | VS Code 自动检测 workspace root 的 `.github/copilot-instructions.md`,并将其作为 workspace always-on instructions 应用于 chat requests | 只读扫描 project `.github/copilot-instructions.md`;保存标题树、段落 hash、字节数;不保存正文;不开放写入 |

## 本机 metadata 补充

| 工具 | scope | kind | 路径 | exists | type |
| --- | --- | --- | --- | --- | --- |
| VS Code Copilot | project | project_mcp_config | `<project>/.vscode/mcp.json` | no | missing |
| VS Code Copilot | project | project_instruction_file | `<project>/.github/copilot-instructions.md` | no | missing |

## 边界

- 当前只支持 workspace/project scope 的 read-only inventory。
- VS Code user profile `mcp.json`、user instructions、组织级 instructions、`.instructions.md` 文件集合和 AGENTS/CLAUDE fallback 的运行态优先级仍待核验。
- MCP 运行态连接、OAuth/header 展开、VS Code 版本兼容和任何写入/同步目标仍 unsupported。
- checked-in fixture 只使用安全占位与非敏感正文,用于证明扫描路径和脱敏边界。
