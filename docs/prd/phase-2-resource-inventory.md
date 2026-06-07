# Phase 2 PRD — 资源只读识别(Resource Inventory)

> 状态:草案 v1,待审核 · 最后更新:2026-06-05 · 写入风险:只读
> 上游:[PRD 总览](README.md) · 关联:[资源中心架构](../design/resource-center-architecture.md)、[工具适配矩阵](../design/tool-adapter-matrix.md)

## 1. 背景与问题

用户本机散落着大量 AI 资源:MCP Server、Skills、Plugins、指令文件(`AGENTS.md`/`CLAUDE.md`/`GEMINI.md`/Cursor rules)、Subagents。它们分布在各工具不同位置、不同格式,用户**看不全、对不齐、不知道哪些重复**。

Phase 2 的目标是:把这些资源**只读识别 + 规范化入库**,在桌面端提供统一的"资源中心(只读)"。这是整个资源管理引擎的地基,**不做任何写入**。

## 2. 目标 / 非目标

**目标**
- G1 实现 Canonical Resource 模型与 Canonical Store(SSOT)。
- G2 实现各资源 Detector:MCP / Skills / Plugins / Instructions / Subagents。
- G3 为优先工具实现 `ToolAdapter::read`(Claude Code / Codex / Gemini CLI / Cursor)。
- G4 跨工具去重(内容指纹)+ 解析置信度 + 敏感字段脱敏。
- G5 桌面"资源中心"只读:列表 + 详情 + 四态 + 过滤/搜索。
- G6 Session Browser:按工具/项目/时间浏览会话元数据(不含正文)。

**非目标**
- 不写入、不编辑、不同步任何资源 → Phase 3/4。
- 不存任何指令正文 / Skill 文件内容 / 密钥原文。

## 3. 用户与场景

| Persona | 诉求 |
| --- | --- |
| 多工具用户 | 想一次看清本机所有 MCP/Skills/指令文件,以及谁在哪个工具启用 |
| 安全审计者 | 想知道哪些 MCP 携带密钥、是否有可疑配置(只看结构,不看值) |
| 整理控 | 想发现重复/冗余资源 |

## 4. 用户故事

- **US-1** 作为用户,我打开"资源中心",按类型(MCP/Skill/Plugin/指令/Subagent)看到本机全部资源及其所属工具。（AC-1）
- **US-2** 作为用户,同一个 MCP 在多个工具里只显示一条,并标注"启用于:Claude Code、Cursor"。（AC-2）
- **US-3** 作为用户,我点开某 MCP 详情,看到传输方式、命令、URL、env **key 名**,密钥值显示为指纹而非原文。（AC-3/4）
- **US-4** 作为用户,我能看到每条资源的解析置信度,低置信度有标注。（AC-5）
- **US-5** 作为用户,我在 Session Browser 按项目筛选会话,看到会话 ID/工具/时间/消息数,但**看不到任何对话内容**。（AC-6）

## 5. 功能需求

### 5.1 Canonical 模型与 Store(FR-1x)
- **FR-11** 实现公共信封 + 五类资源结构(见架构文档 §3),Rust struct + serde。
- **FR-12** 内容指纹 `id`:对资源规范化序列化后取哈希,跨工具去重的依据。
- **FR-13** Canonical Store 落库 `resources` 表,支持按 kind/tool/scope 查询。
- **FR-14** 重新识别采用 upsert:同 `id` 更新 `last_seen` 与 `enabled_in`,不产生重复。

### 5.2 Detectors(FR-2x)
- **FR-21 MCP**:从各工具配置读取 server 列表,产出 canonical MCP;`env` 仅记 key 名 + 值指纹(len/prefix/sha256 前 8)。
- **FR-22 Skills**:扫描技能目录,记录名称、来源、文件清单、content_hash、安装位置。
- **FR-23 Plugins**:扫描插件目录,记录名称、marketplace、版本、包含组件;标注其内含的 MCP/agents 与独立资源的 `provided_by_plugin` 关系,避免重复计数。
- **FR-24 Instructions**:识别 `AGENTS.md`/`CLAUDE.md`/`GEMINI.md`/Cursor rules;**只存结构指纹**(标题树 + 段 hash + 字节数),不存正文。
- **FR-25 Subagents**:识别 `*.md` agent 定义(名称、allowed tools、model、scope)。
- **FR-26** 所有 Detector 只读;解析失败计入失败列表,不中断。

### 5.3 Tool Adapter read()(FR-3x)
- **FR-31** 为 4 个优先工具实现 `ToolAdapter::detect` 与 `read(kind)`。
- **FR-32** 适配器封装格式差异(JSON / TOML / Markdown frontmatter)。
- **FR-33** 适配器声明 capabilities(支持的 kind/scope/transport),供后续阶段与 UI 使用。
- **FR-34** 适配器读取的字段、坑点以 [工具适配矩阵](../design/tool-adapter-matrix.md) 为准,落地前逐项真机核验。

### 5.4 去重 / 置信度 / 脱敏(FR-4x)
- **FR-41** 跨工具同资源按 `id` 合并,聚合 `enabled_in`。
- **FR-42** 每条资源带 `confidence`(0–1),解析不完整或推断字段降低置信度。
- **FR-43** 敏感字段统一走脱敏器:env 值、args 中疑似 token、headers 的 Authorization → 指纹化。
- **FR-44** `redacted` 标记是否发生脱敏,UI 显式提示。

### 5.5 资源中心 UI(FR-5x)
- **FR-51** 左侧按 kind 分组,右侧资源列表(名称、类型、所属工具、scope、置信度、是否脱敏)。
- **FR-52** 详情抽屉:展示 canonical 字段(脱敏后)+ 来源(工具/文件/定位)。
- **FR-53** 搜索与过滤:按名称、kind、tool、scope、是否脱敏。
- **FR-54** 四态:加载/空(无资源)/错误(解析失败汇总)/正常。
- **FR-55** 全页只读,无任何"写入/同步"入口(Phase 3 才加)。

### 5.6 Session Browser(FR-6x)
- **FR-61** 复用 `usage_records` 的 session 元数据,按工具/项目/时间分组浏览。
- **FR-62** 展示 session_id、工具、时间范围、记录(消息)数、关联项目;**不展示任何正文**。
- **FR-63** 支持搜索 session_id / 项目;支持时间窗口过滤。

## 6. 数据模型(SQLite,新增)

```sql
CREATE TABLE IF NOT EXISTS resources (
  id TEXT PRIMARY KEY,            -- 内容指纹
  kind TEXT NOT NULL,            -- mcp|skill|plugin|instruction|subagent
  name TEXT NOT NULL,
  scope TEXT NOT NULL,           -- user|project|enterprise
  origin_tool TEXT NOT NULL,
  origin_path TEXT NOT NULL,
  origin_locator TEXT,
  enabled_in TEXT NOT NULL,      -- JSON 数组:工具 id 列表
  confidence REAL NOT NULL,
  redacted INTEGER NOT NULL,
  payload_json TEXT NOT NULL,    -- 脱敏后的 canonical 资源(各 kind 专有字段)
  provided_by_plugin TEXT,       -- 若来自插件
  last_seen TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_res_kind ON resources(kind);
CREATE INDEX IF NOT EXISTS idx_res_tool ON resources(origin_tool);

-- 解析失败留痕(只记路径与原因,不记内容)
CREATE TABLE IF NOT EXISTS resource_parse_failures (
  path TEXT NOT NULL,
  tool TEXT NOT NULL,
  kind TEXT,
  reason TEXT NOT NULL,
  seen_at TEXT NOT NULL,
  PRIMARY KEY (path, seen_at)
);
```

> `payload_json` 内对 instruction 只存结构指纹;对 mcp 只存 env 指纹;对 skill 只存 content_hash + 文件清单。

## 7. API(Tauri command)

| command | 入参 | 返回 | 说明 |
| --- | --- | --- | --- |
| `inventory_scan` | `{kinds?: string[]}` | `{counts}` | 触发只读识别并 upsert |
| `list_resources` | `{kind?, tool?, scope?, query?}` | `Resource[]` | 资源列表(脱敏) |
| `get_resource` | `{id}` | `Resource` | 详情(脱敏) |
| `list_parse_failures` | — | `ParseFailure[]` | 解析失败汇总 |
| `list_sessions` | `{tool?, project?, from?, to?, query?}` | `SessionMeta[]` | 会话元数据 |
| `adapter_capabilities` | — | `AdapterCap[]` | 各工具能力声明 |

## 8. 界面与状态

- **资源中心(新页面)**:kind 分组 + 列表 + 详情抽屉 + 搜索过滤 + 四态。
- **解析失败入口**:资源中心顶部"N 个文件解析失败"可展开查看路径与原因。
- **Session Browser(新页面)**:分组浏览 + 过滤;每行明确标注"仅元数据,无正文"。
- 详情中所有脱敏字段加锁形图标与"已脱敏"标签。

## 9. 隐私与安全

- 这是隐私最敏感的阶段,规则从严:
  - 指令正文、Skill 内容、密钥原文**一律不入库**。
  - `payload_json` 入库前必须经过脱敏器;脱敏器有单测覆盖各敏感模式。
- `privacy-audit` 增量:列出每类资源读取的目录、入库字段、脱敏策略;明确声明"不存正文/不存密钥原文"。
- 失败留痕只记路径与原因。

## 10. 验收标准

- **AC-1** 资源中心能列出 5 类资源,数量与真机一致(对照人工核验的 fixture)。
- **AC-2** 跨工具同一 MCP 合并为一条且 `enabled_in` 正确。
- **AC-3** MCP 详情中 env 显示 key 名 + 指纹,**无任何原文**(代码层断言 + 单测)。
- **AC-4** 指令资源 db 内只有结构指纹,grep 数据库无正文片段。
- **AC-5** 低置信度资源被正确标注。
- **AC-6** Session Browser 任何视图/接口都不返回正文字段。
- **AC-7** 坏 fixture 进入 `resource_parse_failures` 且不影响其他资源识别。
- **AC-8** `privacy-audit` 覆盖本阶段全部新增读取与字段。

## 11. 依赖与顺序

- 依赖 Phase 1 的 Tool Registry(需知道各工具配置目录)。
- 产出 capabilities 与 canonical 资源,供 Phase 3 写入、Phase 4 跨工具同步使用。

## 12. 风险与对策

| 风险 | 对策 |
| --- | --- |
| 各工具格式/路径随版本漂移 | 适配矩阵逐项核验 + 脱敏 fixture + 版本探测(Phase 1)联动 |
| 脱敏遗漏导致密钥入库 | 集中脱敏器 + 黑名单字段 + 单测覆盖 + "入库前断言无高熵明文" |
| 指纹去重误合并不同资源 | 指纹纳入关键标识字段;提供"按来源展开"核对 |

## 13. 估时与拆分(WP)

| WP | 内容 | 粗估 |
| --- | --- | --- |
| WP2.1 | Canonical 模型 + Store + upsert | M |
| WP2.2 | 脱敏器(集中)+ 单测 | M |
| WP2.3 | MCP/Skills/Plugins/Instructions/Subagents Detector | L |
| WP2.4 | 4 工具 ToolAdapter::read + capabilities | L |
| WP2.5 | 资源中心 UI(列表/详情/过滤/四态) | L |
| WP2.6 | Session Browser | M |
| WP2.7 | privacy-audit 增量 + fixtures | S |

## 14. 指标(本机匿名)

- 各 kind 资源数、跨工具去重命中数。
- 平均置信度、脱敏发生比例。
- 解析失败文件数。
