# CC Switch 深度研究与 WAPC 路线图

> 状态:草案 v1，待审核
> 最后更新:2026-06-05
> 关联文档:[资源中心架构设计](design/resource-center-architecture.md)、[工具适配矩阵](design/tool-adapter-matrix.md)

本文是对开源项目 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 的系统性研究,目的是回答两个问题:

1. CC Switch 把"AI 编程工具统一管理"这件事做到了什么程度,哪些做法值得 WAPC 学习。
2. WAPC 在"观测 + 资源管理 + 使用说明"的定位下,应该如何差异化推进,以及近/中/长期具体做什么。

---

## 1. CC Switch 是什么

CC Switch 是一个跨平台(Windows / macOS / Linux)桌面应用,用一个可视化界面统一管理多个 AI 编程工具的 Provider、API Key、MCP、Prompt、Skills 和会话。它的核心主张是:**开发者不应该手动编辑每个工具各不相同的 JSON/TOML 配置文件**。

关键事实(截至研究时):

- GitHub ~92.4k stars、6k forks、41 个 release(最新 v3.16.1),社区运营非常成熟。
- 技术栈:Tauri 2.8 + Rust 后端(serde / tokio / thiserror),React 18 + TypeScript + Vite + TailwindCSS + TanStack Query + shadcn/ui 前端。代码构成 Rust 61% / TypeScript 37%。
- 支持 7 个工具:Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes Agent。

## 2. CC Switch 的核心能力

| 能力 | 说明 |
| --- | --- |
| Provider 管理 | 50+ 内置 Provider 预设(AWS Bedrock、NVIDIA NIM、社区中转等),一键导入/切换/拖拽排序/导入导出,系统托盘即时切换 |
| Universal Providers | 一个 Provider 同时同步到 Claude Code、Codex、Gemini CLI |
| 代理与故障切换 | 本地代理热切换、格式转换、自动故障切换、熔断、Provider 健康监控、按 App 独立接管 |
| MCP 面板 | 跨多个应用管理 MCP Server,**双向同步** |
| Prompts | Markdown 编辑器,跨应用同步 + 回填保护(backfill protection) |
| Skills | 从 GitHub 仓库或 ZIP 一键安装,支持自定义仓库,默认通过 **symlink** 装到各应用 |
| 用量与成本 | 花费/请求监控面板、token 趋势图、请求日志、按模型自定义价格 |
| 会话管理 | 跨会话源浏览/搜索历史会话、恢复会话;OpenClaw 工作区编辑 AGENTS.md / SOUL.md |
| 云同步 | 自定义配置目录(Dropbox / OneDrive / iCloud / NAS)、WebDAV 同步、跨设备 |
| 深链 | `ccswitch://` 协议,可通过 URL 导入 Provider / MCP / Prompt / Skill |

## 3. CC Switch 的架构与工程做法(最值得学习的部分)

CC Switch 在工程上有几条值得 WAPC 直接借鉴的设计原则:

1. **SSOT(单一数据源)**:所有可同步数据落在 SQLite(`~/.cc-switch/cc-switch.db`),设备级偏好放 JSON(`settings.json`)。这是"双层存储"——可同步数据 vs 设备本地数据分离。
2. **原子写入**:写配置时先写临时文件再 rename,避免写一半导致工具配置损坏。这是改动外部工具配置文件时的安全底线。
3. **双向同步**:切换 Provider 时写入工具的真实配置文件;编辑时从当前激活的 Provider 回填,避免覆盖用户手改。
4. **备份轮转**:`~/.cc-switch/backups/` 保留最近 10 份;Skills 备份 `~/.cc-switch/skill-backups/` 保留最近 20 份;卸载前自动备份。
5. **并发安全**:Mutex 保护数据库连接,避免竞态。
6. **分层架构**:前端 Components / Hooks / TanStack Query;后端 Commands(API 层)/ Services(业务)/ DAO(数据)/ Models(结构)。
7. **深链协议**:`ccswitch://` 让资源分发可以"点链接即导入"。

## 4. CC Switch 与 WAPC 的定位差异

| 维度 | CC Switch | WAPC |
| --- | --- | --- |
| 一句话定位 | AI 编程工具**配置中枢**:Provider 切换、MCP、Prompt、Skills、代理、故障切换 | AI 编程工具**本机观测与管理工具**:工具识别、token/成本观测、资源管理、使用说明、隐私审计 |
| 首要价值 | 帮你**配置和切换**工具(尤其是换 Provider / 换 Key) | 帮你**看清**本机装了什么、各自怎么用、花了多少 token、归到哪些项目、是否可审计 |
| 对工具的介入 | 主动接管配置,甚至插入本地代理转发流量 | 无侵入:不替换命令、不包代理、不装证书;先只读识别,再"写入必备份"地安全管理 |
| 隐私姿态 | 以配置管理为主,涉及 Key/Provider | 只存元数据,从不落库 prompt/response/源码/输出正文 |
| Provider/Key | 核心功能 | **刻意不做**密钥托管和流量代理(避免成为密钥风险点) |
| 切入顺序 | 配置 → 用量 | 观测 → 资源管理 → 同步注入 |

**结论**:CC Switch 解决"怎么配置和切换 AI 工具",WAPC 解决"本机有哪些 AI 工具、各自怎么用、装了哪些 Skills/MCP/Plugins/指令文件、用了多少 token、花在哪些项目、是否安全可审计"。两者互补,不正面竞争。WAPC 不打算复刻 Provider 代理/故障切换/密钥托管——那既偏离无侵入原则,也是 CC Switch 的强项。

## 5. WAPC 可以从 CC Switch 直接借鉴的做法

不复刻产品,但复用工程方法论:

- ✅ **SSOT + 双层存储**:可同步资源进 SQLite,设备级偏好进 JSON。WAPC 已有 `~/.wapc/wapc.db`,后续资源清单沿用同一库。
- ✅ **原子写入 + 备份轮转 + 回滚**:这是 WAPC "写入必备份"原则的具体实现标准。详见架构设计文档。
- ✅ **双向同步 + 回填保护**:注入资源到工具时不能盲覆盖用户手改,要先读取现状、diff、回填。
- ✅ **适配器分层**:每个工具一个 Adapter,把工具差异收敛到边界。详见[工具适配矩阵](design/tool-adapter-matrix.md)。
- ✅ **深链/可分享资源**:长期可做 `wapc://` 深链,但优先做本地安全管理。
- ⚠️ **代理/故障切换/密钥托管**:**明确不做**,与无侵入和隐私原则冲突。
- ⚠️ **跨平台**:CC Switch 三端;WAPC 坚持 macOS 先做稳,再谈跨平台。

## 6. WAPC 路线图

实现节奏坚持"先只读识别 → 再安全管理 → 再自动同步"。

### 近期(只读识别与观测加固)

- **Tool Registry**:自动识别本机 AI 编程工具、配置目录、数据目录、版本、使用状态。
- **Collector Registry**:把每个工具的数据源、解析器、置信度、健康状态注册化。
- **Data Source Doctor**:目录是否存在、可读文件数、解析记录数、失败文件数、最新事件时间。
- **Pricing Rules**:模型/Provider 价格表、本地覆盖、按历史记录重算费用。
- **Instruction Inventory**:只读识别 `AGENTS.md` / `CLAUDE.md` / `GEMINI.md` / Cursor rules。
- **MCP Inventory**:只读识别各工具配置中的 MCP Server,展示协议、命令、URL、启用范围。
- **Skills / Plugin Inventory**:扫描本机 Skills、Plugins、Prompt 模板、工具扩展。
- **Session Browser**:搜索/分组本机会话,不暴露 prompt/response 正文。
- **Project Attribution**:项目路径归一、别名、跨工具项目聚合。
- **Export**:CSV / JSON / Markdown 使用报告。

### 中期(安全管理与桌面产品化)

- **桌面版产品化**:每个菜单都有真实数据、加载/空/错误态和可用操作。
- **Resource Center**:Skills / MCP / Plugins / Instruction Files 统一列表、详情、备份、启停。
- **写入管线**:预览 → 备份 → 写入(原子)→ 校验 → 回滚,所有写操作走同一条管线。
- **Guide Center**:每个工具/资源自动关联安装说明、配置说明、FAQ、安全提醒。
- **后台服务 UX**:安装/更新/卸载/立即扫描/最后运行结果/日志路径。
- **菜单栏/托盘状态**:今日 token、服务状态、立即扫描、打开面板。
- **本机配置备份**:WAPC 设置、价格规则、资源清单、指令文件、导出模板。

### 长期(自动同步与团队场景)

- **一键资源同步**:把选中的 Skills / MCP / Plugins / 指令模板安全同步到指定工具或项目(详见架构设计的 Sync Engine)。
- **跨工具规范化层**:一份 canonical 资源,自动适配到各工具的真实格式。
- **与配置类工具互补**:在不读密钥的前提下检测当前工具配置上下文,叠加成本/资源/使用说明。
- **Headless / Web 模式**:SSH/server 场景的本地只读 dashboard。
- **团队安全报告**:脱敏路径、项目别名、时间窗口汇总、可复现 fixture。
- **macOS 签名与 notarization**,稳定后再推进跨平台。

## 7. 风险与边界

- **不碰密钥**:WAPC 不读取、不存储、不转发 API Key。识别 MCP/Provider 时只记录"存在性与结构",对疑似密钥字段做脱敏(只记长度/前缀指纹)。
- **不盲写**:任何写入都必须经过预览 + 备份 + 原子写 + 回滚。
- **可复现**:对外文档与 issue 只用脱敏元数据或合成 fixture。
- **平台聚焦**:macOS 优先,避免过早跨平台稀释质量。

## 8. 验收口径

每一阶段都要满足:

1. CLI 有对应只读命令,输出可被 `--json` 消费。
2. 桌面版有对应页面,真实数据 + 四态(加载/空/错误/正常)。
3. 写入类能力必须有 dry-run、备份路径和回滚命令。
4. 隐私审计命令 `wapc privacy-audit` 覆盖新增数据源。
