# Phase 1 PRD — 观测台加固与工具识别

> 状态:草案 v1,待审核 · 最后更新:2026-06-05 · 写入风险:只读
> 上游:[PRD 总览](README.md) · 关联:[路线图](../cc-switch-reference-roadmap.md)

## 1. 背景与问题

WAPC 已能采集 4 个工具的用量并展示仪表盘,但观测层还不够扎实:

- 工具是"硬编码"的,没有**本机工具清单**(装了什么、版本、配置目录)。
- 费用在采集时一次性算好(`usage_records.cost_usd`),价格表无法配置,历史无法重算。
- 数据源是否健康只能在"自动扫描"页看到静态文案,没有真实体检。
- 项目维度按 `project_path` 原样分组,跨工具同一项目无法合并,长路径难读。
- 无法导出报告。

Phase 1 把这些补齐,且**全部只读**,不触碰任何工具文件。

## 2. 目标 / 非目标

**目标**
- G1 本机 AI 编程工具自动识别(Tool Registry)。
- G2 采集器注册化(Collector Registry)+ 数据源体检(Data Source Doctor)。
- G3 可配置价格规则 + 历史费用重算(Pricing Rules)。
- G4 项目归因:路径归一、别名、跨工具合并(Project Attribution)。
- G5 报告导出:CSV / JSON / Markdown。

**非目标**
- 不做资源(MCP/Skills/...)识别 → Phase 2。
- 不做任何写入工具文件的能力。
- 不读取/不展示 prompt、response 正文。

## 3. 用户与场景

| Persona | 诉求 |
| --- | --- |
| 多工具重度用户 | 想知道本机到底装了哪些 AI 工具、各花了多少钱 |
| 团队负责人 | 想按项目看 AI 成本,并导出给团队 |
| 隐私敏感用户 | 想确认数据源健康、且只读不上传 |

## 4. 用户故事

- **US-1** 作为用户,我打开"工具"页,能看到本机识别到的工具及版本与配置目录,无需手动配置。（AC-1）
- **US-2** 作为用户,我能在"数据源"页看到每个数据源是否存在、可读文件数、解析成功/失败数、最新事件时间。（AC-2/3）
- **US-3** 作为用户,我能编辑某模型的单价,保存后历史费用按新价重算并刷新所有汇总。（AC-4/5）
- **US-4** 作为用户,我在"项目"页看到的是合并后的项目(跨工具同一仓库算一个),长路径有可读别名。（AC-6/7）
- **US-5** 作为用户,我能把当前视图导出为 CSV/JSON/Markdown。（AC-8）

## 5. 功能需求

### 5.1 Tool Registry(FR-1x)
- **FR-11** 内置工具定义表:工具 id、显示名、可执行探测方式、配置目录、数据目录、官网/文档链接。
- **FR-12** 识别逻辑:配置目录或可执行存在即判定"已安装";记录 `installed`、`config_dir_exists`、`data_dir_exists`。
- **FR-13** 版本探测(尽力而为):对支持 `--version` 的工具读取版本号;失败不报错,记 `unknown`。
- **FR-14** 识别结果落库 `tools` 表,带 `last_detected_at`。
- **FR-15** 识别**只读**:只 stat 目录与(可选)执行 `--version`,不修改任何文件。

### 5.2 Collector Registry + Data Source Doctor(FR-2x)
- **FR-21** 每个采集器注册:工具 id、glob 模式、解析器、置信度策略。
- **FR-22** 体检逐数据源产出:`exists`、`readable_files`、`parsed_records`、`failed_files`、`latest_event_ts`。
- **FR-23** 解析失败的文件路径计入 `failed_files` 列表(只记路径,不记内容),不中断整体扫描。
- **FR-24** 体检结果可在桌面"数据源"页展示,并支持"重新体检"。

### 5.3 Pricing Rules(FR-3x)
- **FR-31** 价格规则表:`model`(或前缀匹配)、`provider`(可选)、各 token 桶单价(input/output/cache_read/cache_write/reasoning/tool)、币种、生效区间(可选)。
- **FR-32** 内置一份默认价格表;用户可新增/编辑/删除本地覆盖规则。
- **FR-33** 费用计算从"采集时固化"改为"按当前规则可重算":`recompute_costs` 命令遍历 `usage_records` 按规则重算 `cost_usd`。
- **FR-34** 匹配优先级:精确 model > 前缀 > 默认;命中规则与否记录在 `precision`/或新增 `cost_source` 字段,便于审计。
- **FR-35** 无匹配规则时 `cost_usd = NULL` 并在 UI 标注"价格未知",不臆造。

### 5.4 Project Attribution(FR-4x)
- **FR-41** 路径归一:去尾斜杠、`~` 展开、大小写按 macOS 规则、软链解析(尽力)。
- **FR-42** 项目别名表:`canonical_path` → `alias`;UI 优先显示别名。
- **FR-43** 跨工具合并:同一 `canonical_path` 的不同工具记录在"项目"页合并为一个项目,内部可按工具下钻。
- **FR-44** 归一是**展示/聚合层**行为,不改写 `usage_records.project_path` 原值(保留可追溯)。

### 5.5 Export(FR-5x)
- **FR-51** 导出当前视图(工具/项目/按天)为 CSV、JSON、Markdown 三种格式。
- **FR-52** 导出文件默认存到用户选择目录;文件名含视图名 + 日期。
- **FR-53** 导出内容只含已落库的元数据字段,绝不含正文。

## 6. 数据模型(SQLite,新增/变更)

现有:`usage_records`(见 `src/store.rs`)。本阶段新增:

```sql
-- 工具识别结果
CREATE TABLE IF NOT EXISTS tools (
  id TEXT PRIMARY KEY,           -- 'claude-code'
  display_name TEXT NOT NULL,
  installed INTEGER NOT NULL,    -- 0/1
  version TEXT,                  -- 'unknown' 允许
  config_dir TEXT,
  data_dir TEXT,
  last_detected_at TEXT NOT NULL
);

-- 价格规则(用户可覆盖)
CREATE TABLE IF NOT EXISTS pricing_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  model_match TEXT NOT NULL,     -- 精确或前缀,如 'claude-opus' / 'gpt-'
  match_kind TEXT NOT NULL,      -- 'exact' | 'prefix'
  provider TEXT,
  currency TEXT NOT NULL DEFAULT 'USD',
  price_input REAL, price_output REAL,
  price_cache_read REAL, price_cache_write REAL,
  price_reasoning REAL, price_tool REAL,
  source TEXT NOT NULL,          -- 'builtin' | 'user'
  updated_at TEXT NOT NULL
);

-- 项目别名 / 归一
CREATE TABLE IF NOT EXISTS project_aliases (
  canonical_path TEXT PRIMARY KEY,
  alias TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- 数据源体检快照(可选持久化,便于趋势)
CREATE TABLE IF NOT EXISTS source_health (
  tool TEXT NOT NULL,
  source_glob TEXT NOT NULL,
  exists_flag INTEGER NOT NULL,
  readable_files INTEGER NOT NULL,
  parsed_records INTEGER NOT NULL,
  failed_files INTEGER NOT NULL,
  latest_event_ts TEXT,
  checked_at TEXT NOT NULL,
  PRIMARY KEY (tool, source_glob, checked_at)
);
```

`usage_records` 变更(向后兼容,迁移加列):新增 `cost_source TEXT`(`exact|prefix|none`),记录费用来自哪条规则。

## 7. API(Tauri command)

| command | 入参 | 返回 | 说明 |
| --- | --- | --- | --- |
| `list_tools` | — | `Tool[]` | Tool Registry 结果 |
| `detect_tools` | — | `Tool[]` | 触发一次识别并落库 |
| `source_health` | — | `SourceHealth[]` | 数据源体检 |
| `list_pricing_rules` | — | `PricingRule[]` | 价格规则 |
| `upsert_pricing_rule` | `PricingRule` | `PricingRule` | 新增/编辑 |
| `delete_pricing_rule` | `{id}` | `void` | 删除 |
| `recompute_costs` | — | `{updated:number}` | 按规则重算历史费用 |
| `list_project_aliases` | — | `ProjectAlias[]` | 别名 |
| `set_project_alias` | `{canonical_path, alias}` | `ProjectAlias` | 设别名 |
| `export_report` | `{view, format, path}` | `{path}` | 导出 |

错误以 `Result<T, String>` 返回,UI 展示错误态。

## 8. 界面与状态

- **工具页(改造)**:表格列 = 工具 / 状态(已安装/未识别)/ 版本 / 配置目录 / 数据目录 / 操作(重新识别)。四态齐全。
- **数据源页(由"自动扫描"页扩展)**:每个数据源一行体检结果 + "重新体检"。
- **设置 → 价格规则(新)**:规则列表 + 编辑弹窗 + "重算历史费用"按钮(带进度与结果提示)。
- **项目页(改造)**:显示别名,支持"设置别名";同一项目跨工具合并,可下钻。
- **导出**:各列表页右上角"导出"菜单(CSV/JSON/Markdown)。

## 9. 隐私与安全

- 全部只读:Tool Registry 仅 stat 目录 + 可选 `--version`;体检只读数据源。
- 版本探测执行外部命令前需在内置白名单内(仅 `--version` 类只读参数)。
- `privacy-audit` 增量:列出 Tool Registry 探测的目录、是否执行版本命令、新增表存了哪些字段。
- 导出文件不含正文;路径可由用户选择脱敏目录。

## 10. 验收标准

- **AC-1** 在装有 ≥2 个受支持工具的机器上,`detect_tools` 能正确标记 installed 并填出配置目录。
- **AC-2** 数据源体检对存在/缺失目录分别给出正确 `exists` 与计数。
- **AC-3** 故意放一个坏 fixture,`failed_files` +1 且扫描不中断。
- **AC-4** 编辑某 model 单价并 `recompute_costs` 后,该 model 的历史 `cost_usd` 按新价更新。
- **AC-5** 无匹配规则的记录 `cost_usd=NULL` 且 UI 显示"价格未知"。
- **AC-6** 两个工具记录同一仓库路径,项目页合并为一条。
- **AC-7** 设置别名后项目页优先显示别名,原始路径仍可见(hover/下钻)。
- **AC-8** 三种格式导出成功,内容与界面一致且不含正文。
- **AC-9** `privacy-audit` 覆盖本阶段全部新增读取与字段。

## 11. 依赖与顺序

- 无前置阶段(基于现有采集与 store)。
- 为 Phase 2 提供 Tool Registry 基础(资源识别要知道工具配置目录在哪)。

## 12. 风险与对策

| 风险 | 对策 |
| --- | --- |
| 版本探测执行外部命令的安全性 | 白名单 + 仅 `--version` + 超时 + 失败降级 unknown |
| 价格重算误改数据 | 重算只更新 `cost_usd`/`cost_source`,可重复执行;提供"恢复默认价格表" |
| 路径归一在软链/大小写上出错 | 归一只作用于展示层,保留原值可回溯 |

## 13. 估时与拆分(WP)

| WP | 内容 | 粗估 |
| --- | --- | --- |
| WP1.1 | Tool Registry(表 + 识别 + 工具页改造) | M |
| WP1.2 | Collector Registry + 数据源体检 + 页面 | M |
| WP1.3 | Pricing Rules(表 + CRUD + 重算 + 设置页) | L |
| WP1.4 | Project Attribution(归一 + 别名 + 合并 + 项目页改造) | M |
| WP1.5 | Export(CSV/JSON/Markdown) | S |
| WP1.6 | privacy-audit 增量 + fixtures + 测试 | S |

(S≈1–2d,M≈3–5d,L≈1–2w,粗估待团队校准)

## 14. 指标(本机匿名,不外传)

- 识别到的工具数、有数据的工具数。
- 体检失败文件占比。
- 命中价格规则的记录占比(衡量价格表覆盖度)。
