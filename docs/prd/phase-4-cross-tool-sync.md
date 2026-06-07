# Phase 4 PRD — 跨工具同步注入(Cross-tool Sync)

> 状态:草案 v1,待审核 · 最后更新:2026-06-05 · 写入风险:跨工具写入
> 上游:[PRD 总览](README.md) · 关联:[资源中心架构](../design/resource-center-architecture.md)、[工具适配矩阵](../design/tool-adapter-matrix.md)

## 1. 背景与问题

到 Phase 3 为止,用户能在**单个工具内**安全管理资源。但最大的价值主张还没兑现:**一份资源定义,适配并同步到多个工具/项目**。

例如:用户在 Claude Code 配好了 `github` MCP,想同步到 Codex 和 Cursor —— 今天只能手动改三种格式。Phase 4 用 canonical 资源 + 适配器 + Sync Engine,把这件事做成"勾选目标 → 预览多份 diff → 一键安全写入"。

## 2. 目标 / 非目标

**目标**
- G1 跨工具同步:canonical 资源 → 多个目标工具/项目的格式化写入。
- G2 含密钥资源(MCP env)的同步流程:目标现有值 / 预览手填 / 拒绝同步。
- G3 作用域(scope)规则:默认不跨 scope,显式开启。
- G4 批量预览 + 逐目标备份/写入/校验/回滚(复用 Phase 3 Sync Engine)。
- G5 同步结果可分别回滚。

**非目标**
- 不引入云同步/WebDAV(那是另一个方向,且涉及上传,暂不做)。
- 不托管密钥;不在任何环节持久化密钥原文。

## 3. 用户与场景

| Persona | 诉求 |
| --- | --- |
| 多工具用户 | 配一次,推到所有工具,不再手动对齐格式 |
| 项目负责人 | 把一组项目级指令/MCP 同步到团队成员都在用的工具 |
| 安全意识强者 | 同步含密钥资源时,绝不希望密钥被 WAPC 存下来 |

## 4. 用户故事

- **US-1** 作为用户,我选中一个 MCP,勾选目标"Codex + Cursor",系统生成两份 diff 供我预览。（AC-1）
- **US-2** 作为用户,该 MCP 含 `GITHUB_TOKEN`,系统不持有原值,提示我选择"沿用目标现有值"或"手动填入";我手填的值不被保存。（AC-2/3）
- **US-3** 作为用户,确认后系统对每个目标分别备份、写入、校验,任一目标失败只影响该目标。（AC-4/5）
- **US-4** 作为用户,我想把一个 project 级资源同步到 user 级,系统默认拒绝并提示需显式开启跨 scope。（AC-6）
- **US-5** 作为用户,同步后我能对其中某个目标单独回滚。（AC-7）

## 5. 功能需求

### 5.1 跨工具同步引擎(FR-1x)
- **FR-11** 选择"源资源 + 目标集合(工具/项目)",对每个目标调用对应 `ToolAdapter::plan_write` 产出 WritePlan。
- **FR-12** 目标若不支持该 kind/transport(capabilities 不满足),灰置并说明原因,不生成 plan。
- **FR-13** 批量预览:一屏汇总 N 份 diff,逐条可展开。
- **FR-14** 逐目标执行 Sync Engine(备份/原子写/校验),目标间相互独立,失败隔离。
- **FR-15** 同步产生 N 条 `resource_changes`(每目标一条),可分别回滚。

### 5.2 密钥(env)处理(FR-2x)
- **FR-21** 同步含 env 的 MCP 时,WAPC 不持有原值;提供三种策略:
  - **沿用目标现有值**(若目标已有同名 env)
  - **预览时手填**(值仅内存,不入库、不进日志、不进备份索引)
  - **拒绝同步该字段**(写入占位/留空,由用户后续在工具内补)
- **FR-22** 默认策略:**沿用目标现有值,缺失则要求手填**。
- **FR-23** UI 明确标注哪些字段是密钥、当前采用哪种策略。

### 5.3 作用域规则(FR-3x)
- **FR-31** 默认**禁止跨 scope**(project ↔ user)同步;需用户在该次操作显式勾选"允许跨 scope"。
- **FR-32** enterprise/managed 级:只能作为**源**(读),不能作为**目标**(写)。
- **FR-33** project 目标需用户指定具体项目路径。

### 5.4 同步预设(可选,FR-4x)
- **FR-41** 支持把"一组资源 → 一组目标"存为本地预设,便于重复同步(不含密钥值)。
- **FR-42** 预设仅本机存储,可导出为不含密钥的 JSON。

## 6. 数据模型(SQLite,新增/复用)

复用 Phase 3 的 `resource_changes` / `resource_backups`(每目标一条 change)。新增:

```sql
-- 同步操作(一次跨工具同步,聚合多条 change)
CREATE TABLE IF NOT EXISTS sync_operations (
  sync_id TEXT PRIMARY KEY,
  source_resource_id TEXT NOT NULL,
  targets_json TEXT NOT NULL,     -- [{tool, scope, project_path?}]
  allow_cross_scope INTEGER NOT NULL,
  env_strategy TEXT NOT NULL,     -- reuse|manual|skip
  created_at TEXT NOT NULL
);
ALTER TABLE resource_changes ADD COLUMN sync_id TEXT;  -- 关联到同步操作

-- 同步预设(不含密钥)
CREATE TABLE IF NOT EXISTS sync_presets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  resources_json TEXT NOT NULL,
  targets_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

## 7. API(Tauri command)

| command | 入参 | 返回 | 说明 |
| --- | --- | --- | --- |
| `plan_sync` | `{resource_id, targets[], allow_cross_scope?, env_strategy}` | `WritePlan[]` | 批量计划 + 各 diff |
| `apply_sync` | `{plans, env_values?, confirm_drift?}` | `{sync_id, changes[]}` | 逐目标执行 |
| `rollback_change` | `{change_id}` | `{change_id}` | 复用 Phase 3,单目标回滚 |
| `list_sync_operations` | — | `SyncOp[]` | 同步历史 |
| `save_sync_preset` / `list_sync_presets` / `delete_sync_preset` | … | … | 预设管理 |

`env_values` 仅在 `apply_sync` 调用内存中使用,后端用完即弃,不写库不写日志。

## 8. 界面与状态

- **资源详情 → "同步到…"**:目标多选(工具 + scope + 项目)、capabilities 不满足项灰置说明、跨 scope 开关、env 策略选择。
- **批量预览页**:N 份 diff 汇总,逐条展开;密钥字段高亮 + 策略标注;drift 警告。
- **同步结果**:每目标成功/失败状态,失败项可重试或查看原因,成功项可单独回滚。
- **同步历史/预设页**。
- 状态:计划中、待确认、写入中、部分成功(明确列出失败目标)、全部成功。

## 9. 隐私与安全

- 密钥**全程不落库**:`env_strategy=manual` 时值仅在 `apply_sync` 内存传递。
- 备份可能含目标原文件(含其原密钥)→ 沿用 Phase 3 的备份权限与审计说明。
- `privacy-audit` 增量:声明跨工具同步会写哪些目标文件、密钥如何处理、备份在哪。
- 失败隔离:任一目标失败不影响其他目标已成功的写入与备份。

## 10. 验收标准

- **AC-1** 选 1 个 MCP + 2 个目标,生成 2 份正确格式的 diff(如 Codex=TOML、Cursor=JSON)。
- **AC-2** 含密钥时按默认策略走"沿用现有值",缺失则要求手填。
- **AC-3** 手填的密钥值在 db、变更日志、备份索引中均不可见(断言)。
- **AC-4** 逐目标分别备份并写入;目标 A 失败不影响目标 B 成功。
- **AC-5** 写入后各目标 verify 通过;失败目标自动回滚。
- **AC-6** 跨 scope 同步默认被拒,开启开关后可执行。
- **AC-7** 可对单个目标的 change 回滚,且只影响该目标。
- **AC-8** `privacy-audit` 覆盖跨工具写入与密钥处理说明。

## 11. 依赖与顺序

- 依赖 Phase 2(canonical + capabilities)与 Phase 3(Sync Engine + 回填保护 + 回滚)。
- 完成后,WAPC 的核心价值主张(统一管理 + 安全同步注入)闭环。

## 12. 风险与对策

| 风险 | 对策 |
| --- | --- |
| 格式转换语义丢失(TOML↔JSON↔frontmatter) | 适配器单测覆盖往返转换;预览强制人工确认 |
| 部分目标失败导致状态不一致 | 目标间独立事务 + 失败隔离 + 各自可回滚 |
| 用户误把密钥同步到不该去的工具 | 密钥字段高亮 + 默认不带原值 + 二次确认 |
| 跨 scope 误操作 | 默认禁止 + 显式开关 + enterprise 只读 |

## 13. 估时与拆分(WP)

| WP | 内容 | 粗估 |
| --- | --- | --- |
| WP4.1 | 跨工具 plan(多目标)+ capabilities 校验 | M |
| WP4.2 | env 策略流程(reuse/manual/skip)+ 内存传值 | M |
| WP4.3 | scope 规则 + enterprise 只读约束 | S |
| WP4.4 | 批量预览 + 逐目标执行(复用 Sync Engine) | L |
| WP4.5 | 同步历史 / 预设 | M |
| WP4.6 | privacy-audit 增量 + 端到端跨工具测试 | M |

## 14. 指标(本机匿名)

- 同步操作数、平均目标数、部分失败率。
- 各 env 策略使用占比。
- 跨 scope 开启次数。
