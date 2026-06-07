# WAPC 分阶段 PRD 总览

> 状态:草案 v1,待审核
> 最后更新:2026-06-05
> 关联:[路线图](../cc-switch-reference-roadmap.md)、[资源中心架构](../design/resource-center-architecture.md)、[工具适配矩阵](../design/tool-adapter-matrix.md)

本目录是 WAPC 完整路线的产品需求文档(PRD),按阶段拆分,供研发团队直接据此排期与实现。每份 PRD 自包含,但共享下面的统一约定与术语。

## 形态约束(全阶段适用)

- **产品形态**:Rust Core(库)+ Tauri/React 桌面应用。**没有 CLI**(已下线)。所有"命令"指 Tauri command,不是终端命令。
- **平台**:macOS 优先(12+)。跨平台在路线最后再考虑。
- **三条铁律**:无侵入 · 只存元数据(密钥只记指纹)· 写入必备份(预览/备份/原子写/校验/回滚)。
- **数据库**:`~/.wapc/wapc.db`(SQLite);设备级偏好走 `localStorage` 或 `~/.wapc/settings.json`。

## 阶段地图

| 阶段 | PRD | 一句话目标 | 写入风险 |
| --- | --- | --- | --- |
| Phase 1 | [观测台加固与工具识别](phase-1-observatory.md) | 把现有用量观测做扎实:工具识别、采集器注册化、数据源体检、价格规则、项目归因、导出 | 只读 |
| Phase 2 | [资源只读识别](phase-2-resource-inventory.md) | 识别 Skills/MCP/Plugins/指令文件/Subagents/会话,规范化入库,桌面资源中心(只读) | 只读 |
| Phase 3 | [安全写入与资源管理](phase-3-sync-engine.md) | 统一安全写入管线 + 单工具内资源编辑/启停/备份/回滚 + 使用说明中心 | 单工具写入 |
| Phase 4 | [跨工具同步注入](phase-4-cross-tool-sync.md) | 一份 canonical 资源,适配并安全同步到多个工具/项目;密钥指纹流程 | 跨工具写入 |
| Phase 5 | [进阶与发布](phase-5-advanced.md) | 深链、资源模板库、团队脱敏报告、Headless、macOS 签名与公证 | 混合 |

阶段映射:资源中心架构文档的 P0–P3 ≈ 本目录 Phase 2–4。

## PRD 统一模板

每份 PRD 都按如下结构组织,研发可逐节对照实现:

1. **背景与问题** — 为什么做、当前缺口。
2. **目标 / 非目标** — 本阶段做什么、明确不做什么。
3. **用户与场景** — 目标用户(Persona)与典型使用场景。
4. **用户故事** — `US-x`,带验收口径。
5. **功能需求** — `FR-x`,可实现、可测试的需求条目。
6. **数据模型** — 新增/变更的表与字段(SQLite)。
7. **API(Tauri command)** — 前后端契约:命令名、入参、返回、错误。
8. **界面与状态** — 涉及的页面、四态(加载/空/错误/正常)、关键交互。
9. **隐私与安全** — 本阶段读写边界、脱敏规则、`privacy-audit` 增量。
10. **验收标准** — `AC-x`,逐条可勾选。
11. **依赖与顺序** — 前置阶段、外部依赖。
12. **风险与对策**。
13. **估时与拆分** — 粒度到可分配的工作包(WP)。
14. **埋点/指标(可选)** — 本机匿名计数,不外传。

## 术语表

| 术语 | 含义 |
| --- | --- |
| Canonical Resource | 规范化资源模型,跨工具统一中间层(见架构文档 §3) |
| Detector | 只读识别器,扫描本机产出 canonical 资源 |
| Tool Adapter | 工具适配器,负责 `read`(工具格式→canonical)与 `plan_write`(canonical→写入计划) |
| Sync Engine | 安全写入管线:plan → preview → backup → atomic-write → verify → commit/rollback |
| SSOT | 单一数据源,WAPC 以 `~/.wapc/wapc.db` 为准 |
| WP | Work Package,可分配的工作包 |

## 评审与变更

- 每份 PRD 顶部维护"状态/最后更新/关联"。
- 重大范围变更必须更新对应 PRD 与本总览的阶段地图。
- 任何写入类需求,验收标准里必须包含"预览、备份、回滚"三项。
