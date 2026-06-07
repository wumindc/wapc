# Changelog

## [0.2.2] - 2026-06-07
### Changed
- **MCP/Skills/Agents 页面接入真实同步下发能力**：创建通用 `SyncDialog` 组件，完整实现"选择目标工具 → plan_sync 获取 diff 预览 → apply_sync 确认写入"三步流程，替换了之前所有页面中无效的"即将上线"占位提示。
- **MCP 页面新增"添加 MCP" 功能**：创建 `AddMcpDialog` 组件，支持 stdio/http/sse 三种传输协议配置，可手动填写命令参数后直接注入到 Claude/Cursor/Gemini 的配置文件，使用 `plan_resource_change` + `apply_sync` 真实写入。
- **MCP 页面分发矩阵可操作化**：详情页头部新增"同步下发"按钮，替换之前纯静态展示的分发矩阵；支持用户级（~/.claude.json 等）和项目级（选择项目目录）两种 scope 下发。
- **Agents 页面重构**：移除之前错误使用 `plan_ids` 的手动流程，改为统一使用 `SyncDialog`；扩展了规范文件识别范围（AGENTS.md、CLAUDE.md、GEMINI.md、.cursorrules 等）；增加空状态引导文案。
- **AgentsPage SyncDialog**：规范下发现在正确传递完整 WritePlan 对象到 apply_sync，而非之前错误的 plan_ids 参数。
- **应用内自动更新机制 (Tauri Updater)**：集成 `tauri-plugin-updater`，应用启动 30 秒后在后台静默检查 GitHub Releases 获取最新版本。
- **自动更新 UI 提示**：前端新增 `useAppUpdater` Hook 和 `UpdateBadge` 组件，当检测到新版本时，侧边栏左上角 Logo 旁会显示下载图标（带有红点呼吸灯动画），点击后展示下载进度环并在完成后自动重启安装。
- **CI DMG 打包与 Release 流程**：修改 GitHub Action Release 流程（`release.yml`），将 Tauri 打包目标由单一的 `app` 扩展为 `dmg,app`，并配置了 Updater 签名私钥相关的安全环境变量，实现 `latest.json` 和签名更新包的自动发布。
- **后端 cross_sync.rs**：instruction 类型现在全面支持 plan_sync 流程，plan_instruction_target 实现源文件实时读取和差异计算。
- **后端 sync_engine.rs**：atomic_write 写入前自动 create_dir_all，避免目标目录不存在时报错。


### Changed
- **UI 全面自适应与响应式重构**：基于产品原型重写了全部页面的布局，引入 `PageHeader.tsx` 作为独立模块页头；重构了 `Sidebar.tsx` 使其在移动端通过 Drawer/Overlay 呈现；移除旧的全局 Topbar，使每个页面能够独立掌控自身内容区的滚动和响应式折叠。
- **亮暗色模式适配规范化**：移除了组件中硬编码的颜色代码（如纯黑/纯白），全面接入基于 TailwindCSS 的 `.dark` 主题变量系统（`bg-surface`、`text-heading`、`border-border` 等），实现了跨组件统一的 Dark/Light 切换。
- **数据同步功能重组**：将原先分散的 Export/Import 页面整合为统一的 `SyncPage.tsx` 页面；对接了 `@tauri-apps/plugin-dialog` 实现原生的系统文件存储交互（例如在导出时直接调起本地另存为窗口）。
- **工具与数据详情区视觉增强**：改进了 `McpPage` 和 `SkillsPage`，采用 Master-Detail 两段式侧边栏拆分布局，并增加了流畅的高光卡片效果与过渡动画。
- **自动扫描引擎对接**：为 Sidebar 增加了真实有效的自动扫描控制勾选框，将其与底层的 `useAutoScan` React Hook 状态完成联调。

### Docs
- **UI 全面重设计 PRD**：创建 `docs/20260607-001-WAPC-UI全面重设计PRD.md`，详细规划导航重构、响应式全局规范、MCP/Skills/Plugin/支持工具/数据导入等模块的独立需求设计，包含 4 个待审核的设计决策问题。
- **开发规范 AGENTS.md**：创建 `AGENTS.md`，定义了前端界面的响应式、主题切换、动画规范，以及组件拆分和 `CHANGELOG` 登记规则。

## [0.2.2] - 2026-06-07
### Performance
- **get_snapshot 启动加载优化**：`get_snapshot` 命令不再在每次调用时触发全量文件扫描（`scanner::scan_home`、`scanner::source_health`、`resources::scan_inventory_with_project_roots`），改为直接从 SQLite DB 读取已持久化的缓存数据（`count_records`、`latest_source_health`、`list_resources`），窗口重开后加载时间从数秒降至毫秒级。全量扫描仍由用户主动触发的 `scan_now` 命令负责。
- `UsageStore` 新增 `count_records()` 方法，执行 `SELECT COUNT(*)` 查询返回已索引记录总数。

## [Unreleased] - 2026-05-26
### Added
- **Resource Center**: Added local metadata-only inventory for MCP servers, skills, plugins, subagents, and instruction files across Claude Code, Codex, Gemini CLI, OpenCode, Cursor, and VS Code workspace resources.
- **Safe Sync Engine**: Added preview, backup, atomic write, verify, rollback, drift detection, backup rotation, and idempotent no-op handling for supported write paths.
- **Cross-tool MCP Sync**: Added Phase 4 planning and apply flow for JSON MCP targets and Codex TOML targets, including per-target result isolation, sync history, presets, explicit cross-scope authorization, and memory-only env strategies.
- **Template and Deep-link Flows**: Added resource template install preview/apply routing and `wapc://import` deep-link preview/target-selection flow with secret rejection and verified apply boundaries.
- **Privacy and Export Surfaces**: Added privacy-audit coverage, redacted team reports, time-windowed exports, synthetic fixtures, and user-selected export destinations.
- **Headless Read-only Dashboard**: Added explicit-start, loopback-only read-only dashboard support with no write or sync endpoints.
- **Release Readiness Gates**: Added macOS Tauri release, signing, notarization, and CI gate documentation and enforcement paths.
- **UI/UX Re-design**: Migrated from a simple eframe/egui GUI to a premium React+TypeScript modern UI, styled with TailwindCSS and Lucide-react.
- **Tauri Integration**: Replaced eframe with Tauri v2 to serve as the application shell and backend bridge for system APIs.
- **Project & Service Management**: Added dedicated views for real-time project statistics, AI token usage, and background service status.
- **Animations & Micro-interactions**: Integrated `framer-motion` (via CSS transition / standard React state) and SVG-based Sparklines for dynamic data visualization.
- **Frontend Architecture**: Established a modular frontend architecture (`ui/src/`) using Vite, with strict separation of views, components, and data hooks.

### Changed
- Refactored `Cargo.toml` into a Cargo Workspace layout, separating core Rust logic (`wapc`) and the Tauri wrapper (`wapc-app`).
- Restructured Rust CLI entry points into `cli` module and `wapc` bin while adapting internal APIs (`model`, `store`, `scanner`, `launchd`) to export to Tauri Commands.
- Deprecated legacy egui Desktop entrypoint in favor of the new Tauri UI.
- Clarified README roadmap to distinguish implemented local capabilities, macOS release gates, and non-macOS unsupported boundaries.

### Fixed
- Fixed Tauri icons generation by synthesizing missing macOS app icons required during the build phase.
- Fixed Tauri snapshot tests so they use isolated temporary app paths instead of the user's default local WAPC database.
- Fixed adapter-matrix wording so Codex TOML cross-tool sync is documented as supported while unsupported write categories remain explicit.
