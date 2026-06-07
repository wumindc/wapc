# Phase 3 PRD — 安全写入管线与资源管理(Sync Engine)

> 状态:草案 v1,待审核 · 最后更新:2026-06-05 · 写入风险:单工具写入
> 上游:[PRD 总览](README.md) · 关联:[资源中心架构](../design/resource-center-architecture.md)

## 1. 背景与问题

Phase 2 已能"看清"资源,但所有操作都是只读。用户的真实痛点还包括:在**某一个工具内**编辑/启停/删除一个 MCP、修改一份指令、备份当前配置。

直接改工具文件风险极高(改坏 JSON/TOML、覆盖手改、无法回退)。Phase 3 的核心是建一条**统一安全写入管线(Sync Engine)**,把所有写入收口,并据此提供"单工具内"的资源管理能力。**本阶段不做跨工具同步**(留给 Phase 4),先把"写得安全"这件事做到位。

## 2. 目标 / 非目标

**目标**
- G1 Sync Engine:plan → preview → backup → atomic-write → verify → commit/rollback。
- G2 单工具资源管理:在目标工具内 编辑 / 启用 / 禁用 / 删除 资源。
- G3 备份与回滚:备份轮转、变更日志、一键回滚。
- G4 回填保护:写入前比对工具现状,检测用户手改,绝不静默覆盖。
- G5 Guide Center:每个工具/资源关联使用说明与安全提醒。

**非目标**
- 不做"一份 canonical → 多工具"的跨工具同步(Phase 4)。
- 不托管密钥;含密钥写入沿用"目标现有值或用户手填",不持久化原文。

## 3. 用户与场景

| Persona | 诉求 |
| --- | --- |
| 工具使用者 | 想在 WAPC 里直接禁用某个 MCP,而不去翻配置文件 |
| 谨慎用户 | 改之前要能预览 diff、能一键回退 |
| 新手 | 想要每个资源旁边有"这是什么、怎么用、有什么风险"的说明 |

## 4. 用户故事

- **US-1** 作为用户,我在资源中心禁用某工具的一个 MCP,系统先展示将要修改的文件与 diff,我确认后才写入。（AC-1/2）
- **US-2** 作为用户,任何写入前系统都已自动备份原文件,我能在"备份"列表看到并一键回滚。（AC-3/4）
- **US-3** 作为用户,如果目标文件自上次识别后被我手改过,系统会标红警告并要求我确认,不会直接覆盖。（AC-5）
- **US-4** 作为用户,写入后系统会重新读取校验,若结果与预期不符自动回滚并报错。（AC-6）
- **US-5** 作为用户,每个资源详情里有"使用说明"页签,告诉我用途、配置要点和风险。（AC-7）

## 5. 功能需求

### 5.1 Sync Engine(FR-1x)
- **FR-11** `plan`:由 `ToolAdapter::plan_write` 产出 `WritePlan`(目标文件、before、after、diff、是否需备份、风险标记),**不落盘**。
- **FR-12** `preview`:UI 展示 diff;CLI 不存在,默认必须经过 UI 确认。
- **FR-13** `backup`:写入前复制目标文件到 `~/.wapc/backups/<tool>/<ts>/<file>`,轮转保留最近 N=10 份(Skills 单独目录保留 20)。
- **FR-14** `atomic-write`:写临时文件 → fsync → rename 覆盖,避免半写。
- **FR-15** `verify`:写后重新 `read`,确认 canonical 与预期一致;不一致触发 `rollback`。
- **FR-16** `commit/rollback`:成功写变更日志(change_id、文件、备份路径);失败自动从备份恢复。
- **FR-17** 全流程**幂等**:重复执行同一 plan 不产生副作用。

### 5.2 回填保护(FR-2x)
- **FR-21** 写入前对目标文件计算当前指纹,与 WAPC 上次识别记录比对。
- **FR-22** 若不一致(用户手改),标记 `drift` 并在预览中红色警告,必须显式确认才能继续。
- **FR-23** 提供"以工具现状为准重新识别"选项,避免基于过期状态写入。

### 5.3 单工具资源管理(FR-3x)
- **FR-31** MCP:在目标工具内 启用/禁用/编辑(非密钥字段)/删除。
- **FR-32** 指令文件:编辑正文(编辑期内存加载,不持久化到 db)、备份后写回。
- **FR-33** Skills/Plugins:启用/禁用/删除(按工具机制);Skills 删除前备份。
- **FR-34** Subagents:启用/禁用/删除。
- **FR-35** 所有写操作必须经 Sync Engine;适配器只产出 plan,不自行写盘。
- **FR-36** 含密钥字段(env 等)编辑:默认"保留目标现有值";如需改值由用户在预览界面手填,**不入库**。

### 5.4 备份与变更管理(FR-4x)
- **FR-41** 备份列表 UI:按工具/时间浏览备份,显示来源变更。
- **FR-42** 变更日志:每次 commit 一条(change_id、时间、工具、文件、操作、备份路径、可回滚标记)。
- **FR-43** 一键回滚:`rollback(change_id)` 从备份恢复并记录反向变更。
- **FR-44** 备份轮转策略可配置上限,超出自动清理最旧。

### 5.5 Guide Center(FR-5x)
- **FR-51** 内置说明库:每个工具与每类资源的用途、配置要点、常见问题、安全提醒。
- **FR-52** 资源详情页"使用说明"页签自动关联对应条目。
- **FR-53** 说明库可随版本更新;支持本地补充(只读展示,不写工具)。

## 6. 数据模型(SQLite,新增)

```sql
CREATE TABLE IF NOT EXISTS resource_changes (
  change_id TEXT PRIMARY KEY,
  tool TEXT NOT NULL,
  resource_id TEXT,
  kind TEXT NOT NULL,
  op TEXT NOT NULL,              -- enable|disable|edit|delete|create
  target_path TEXT NOT NULL,
  backup_path TEXT,
  status TEXT NOT NULL,          -- committed|rolledback|failed
  reverts_change_id TEXT,        -- 若本条是回滚
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS resource_backups (
  backup_path TEXT PRIMARY KEY,
  tool TEXT NOT NULL,
  original_path TEXT NOT NULL,
  change_id TEXT,
  created_at TEXT NOT NULL
);

-- 目标文件指纹,用于回填保护(drift 检测)
CREATE TABLE IF NOT EXISTS file_fingerprints (
  tool TEXT NOT NULL,
  path TEXT NOT NULL,
  fingerprint TEXT NOT NULL,     -- 内容哈希
  observed_at TEXT NOT NULL,
  PRIMARY KEY (tool, path)
);
```

## 7. API(Tauri command)

| command | 入参 | 返回 | 说明 |
| --- | --- | --- | --- |
| `plan_resource_change` | `ResourceChange` | `WritePlan` | 产出计划 + diff,不落盘 |
| `apply_resource_change` | `{plan, confirm_drift?}` | `{change_id}` | 经 Sync Engine 写入 |
| `list_changes` | `{tool?}` | `Change[]` | 变更日志 |
| `rollback_change` | `{change_id}` | `{change_id}` | 回滚 |
| `list_backups` | `{tool?}` | `Backup[]` | 备份列表 |
| `get_guide` | `{tool?, kind?, resource_id?}` | `Guide` | 使用说明 |

`apply_resource_change` 在检测到 drift 且 `confirm_drift!=true` 时返回特定错误,UI 弹确认。

## 8. 界面与状态

- **资源详情(扩展)**:加入"管理"区(启用/禁用/编辑/删除)与"使用说明"页签。
- **写入预览弹窗**:左右 diff、目标文件路径、是否需备份、drift 警告、确认/取消。
- **备份与变更页(新)**:变更日志列表 + 每条"回滚";备份浏览。
- **状态**:写入中(禁用按钮)、成功(toast + 变更入列)、失败(回滚提示 + 错误详情)、drift 警告。

## 9. 隐私与安全

- 写入只改"用户在 UI 明确选择"的资源字段,范围最小化。
- 密钥不入库;编辑密钥值仅在内存传递到写入,不落 db、不进日志。
- 备份文件可能含工具原配置(含密钥)→ 备份目录权限 600,`privacy-audit` 明确告知备份会包含原文件、存放位置与清理方式。
- enterprise/managed 级资源**只读**,不允许写入。

## 10. 验收标准

- **AC-1** 禁用一个 MCP 前必出现 diff 预览,取消则文件零改动。
- **AC-2** 确认后目标文件被正确修改且结构合法(JSON/TOML 可被工具重新解析)。
- **AC-3** 每次写入前在 `~/.wapc/backups/` 生成备份,记录入 `resource_backups`。
- **AC-4** 一键回滚后文件与备份一致,且产生一条 `reverts_change_id` 记录。
- **AC-5** 人为手改目标文件后再写入,系统报 drift 并要求确认。
- **AC-6** 注入一个会导致 verify 失败的场景,系统自动回滚且不留损坏文件。
- **AC-7** 资源详情"使用说明"能正确关联到对应条目。
- **AC-8** 写入密钥值后,db 与变更日志中均无该值(断言)。
- **AC-9** enterprise 级资源的写入入口被禁用。

## 11. 依赖与顺序

- 依赖 Phase 2 的 canonical 资源与适配器 read/capabilities。
- 为 Phase 4 提供 Sync Engine(跨工具同步复用同一条写入管线)。

## 12. 风险与对策

| 风险 | 对策 |
| --- | --- |
| 写坏工具配置导致工具无法启动 | 原子写 + verify + 自动回滚 + 备份;格式序列化稳定性测试 |
| 备份膨胀占空间 | 轮转上限 + 清理最旧 + UI 可见占用 |
| 回填保护误报/漏报 | 指纹覆盖整文件;drift 仅警告不阻断(用户确认即可) |
| 备份泄露密钥 | 目录权限 + 审计说明 + 可一键清空备份 |

## 13. 估时与拆分(WP)

| WP | 内容 | 粗估 |
| --- | --- | --- |
| WP3.1 | Sync Engine 核心(plan/backup/atomic/verify/rollback) | L |
| WP3.2 | 回填保护(指纹 + drift) | M |
| WP3.3 | 适配器 plan_write(4 工具,各 kind) | L |
| WP3.4 | 单工具资源管理 UI(管理区 + 预览弹窗) | L |
| WP3.5 | 备份与变更页 + 回滚 | M |
| WP3.6 | Guide Center(说明库 + 关联) | M |
| WP3.7 | privacy-audit 增量 + 端到端写入测试 | M |

## 14. 指标(本机匿名)

- 写入成功/失败/回滚次数。
- drift 命中率。
- 备份占用空间、回滚使用次数。
