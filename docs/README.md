# WAPC 文档索引

WAPC 的研究、规划与设计文档集中在这里。所有设计文档默认状态为"草案,待审核"。

## 研究与规划

- [CC Switch 深度研究与 WAPC 路线图](cc-switch-reference-roadmap.md)
  对开源项目 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 的系统研究、定位差异、可借鉴工程做法,以及 WAPC 近/中/长期路线图。

## 设计文档

- [资源中心架构设计:统一识别 · 适配 · 同步 · 注入](design/resource-center-architecture.md)
  Skills / MCP / Plugins / 指令文件 / Subagents 的统一识别、规范化、跨工具适配与安全注入引擎设计。
- [工具适配矩阵](design/tool-adapter-matrix.md)
  各 AI 编程工具的真实配置位置、字段映射与适配坑点 —— 适配器层的"事实库"。
- [macOS 工具路径本机核验证据](design/macos-path-verification.md)
  当前 macOS 工作站上 user/project 候选路径的只读 metadata 核验结果,不读取配置正文或密钥。
- [OpenCode 指令与 Skills 官方核验](design/opencode-resource-verification.md)
  OpenCode `AGENTS.md`、`~/.config/opencode/AGENTS.md`、`.opencode/skills` 与 `~/.config/opencode/skills` 的官方机制和当前支持边界。
- [VS Code Copilot 资源官方核验](design/vscode-copilot-resource-verification.md)
  VS Code workspace `.vscode/mcp.json` 与 `.github/copilot-instructions.md` 的官方机制、只读扫描证据和未支持边界。
- [跨平台可行性评估](design/cross-platform-feasibility.md)
  Phase 5 WP5.F 的 Windows/Linux 评估、Go/No-Go 门禁与后续路径抽象路线。

## 分阶段 PRD(供研发团队)

- [PRD 总览与统一模板](prd/README.md)
- [Phase 1 — 观测台加固与工具识别](prd/phase-1-observatory.md)
- [Phase 2 — 资源只读识别](prd/phase-2-resource-inventory.md)
- [Phase 3 — 安全写入管线与资源管理](prd/phase-3-sync-engine.md)
- [Phase 4 — 跨工具同步注入](prd/phase-4-cross-tool-sync.md)
- [Phase 5 — 进阶与发布](prd/phase-5-advanced.md)

## 发布工程

- [macOS 签名与公证发布说明](release/macos-signing-notarization.md)
  Phase 5 WP5.E 的签名/公证 CI、GitHub Secrets、本地打包路径与验收边界。

## 阅读顺序建议

1. 先读路线图,理解定位与节奏。
2. 再读资源中心架构,理解"自动识别适配同步注入"的整体引擎。
3. 落地某个工具时,查工具适配矩阵的对应条目。

## 贡献文档

- 设计文档改动请保留顶部"状态/最后更新/关联"元信息。
- 涉及真机路径/字段的内容,务必标注是否已核验,并配脱敏 fixture。
- 不要在文档或 fixture 中包含真实密钥、prompt、response 或源码正文。
