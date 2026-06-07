# 跨平台可行性评估

状态: 草案 v1,待审核 + 待真机核验  
最后更新: 2026-06-06  
关联: [Phase 5 PRD](../prd/phase-5-advanced.md) · [工具适配矩阵](tool-adapter-matrix.md) · [资源中心架构设计](resource-center-architecture.md)

## 1. 结论

WAPC 具备跨平台基础,但不应直接进入 Windows/Linux 的写入能力实现。

建议结论:

- **可以推进**:跨平台编译检查、只读本机数据库/报告/headless dashboard、Tauri UI 基础适配、路径解析抽象。
- **谨慎推进**:工具识别与只读资源扫描。必须先做 Windows/Linux fixture 与真机核验。
- **暂不开放**:跨工具写入、模板安装、深链导入后写入、MCP/指令文件同步。缺少逐工具真实路径和字段核验前,这些能力在非 macOS 平台应显示 `unsupported`。

这满足 Phase 5 WP5.F 的范围:产出可行性评估文档与适配矩阵增量,本阶段不承诺三端可用。

## 2. 当前实现可复用部分

| 能力 | 可复用度 | 说明 |
| --- | --- | --- |
| SQLite store | 高 | 业务表与隐私边界可复用,但 db 默认目录应经统一 PathResolver 解析。 |
| 报告导出 | 高 | Markdown/JSON/CSV 生成逻辑与平台无关;导出路径选择需平台化。 |
| Headless read-only dashboard | 中高 | HTTP 逻辑可复用;监听、端口、浏览器打开行为需 Windows/Linux 验证。 |
| Resource canonical model | 高 | canonical 结构与脱敏策略平台无关。 |
| Sync Engine | 中 | diff/backup/rollback 可复用;真实 target path、原子 rename、权限和换行策略需平台验证。 |
| Tauri UI | 中 | React UI 可复用;Tauri 配置、bundle、权限、签名和系统 API 需按平台拆分。 |

## 3. 主要阻塞点

### 3.1 平台路径不能硬编码

当前 WAPC 大量路径来自 macOS/Unix 习惯,例如 `~/.claude`、`~/.codex`、`~/.config/...`、`~/Library/Application Support/...`。Windows/Linux 必须通过平台目录策略与工具自身约定组合解析:

- WAPC 自身配置/数据库目录:优先使用 Tauri path API 或 Rust `directories`/`dirs` 这类平台目录库。
- 工具 user 级配置:每个工具单独定义候选路径列表,并保留 `待核验` 状态。
- project 级配置:优先以显式 project root 为基准,不能从当前工作目录隐式推断。

### 3.2 命令与 shell 差异

MCP stdio 配置里常见 `npx`、`node`、`uvx`、`python`。Windows 需要额外处理:

- `.exe` / `.cmd` / `.bat` 查找。
- 参数 quoting 与路径空格。
- `PATH` 分隔符差异。
- `which` 与 `where.exe` 差异。

非 macOS 写入预览必须展示解析后的候选命令,不能只展示原字符串。

### 3.3 文件系统语义差异

- Windows 路径有盘符、UNC、大小写规则与保留文件名。
- Linux/macOS 的 symlink 策略不能直接等价到 Windows 普通用户权限。
- 原子写入必须保证 temp file 与目标文件在同一目录/同一卷。
- 换行符、文件权限和可执行位要按目标平台处理。

### 3.4 发布工程差异

macOS 的 Developer ID + notarization 不能覆盖 Windows/Linux:

- Windows 需要 Authenticode/code signing 与 installer 策略。
- Linux 需要 AppImage/deb/rpm 或其他包格式取舍。
- 自动更新通道和 artifact 命名需要三端矩阵。

## 4. 分阶段路线

### F0: 本评估与矩阵增量

状态: 当前任务范围。

交付:

- `docs/design/cross-platform-feasibility.md`
- `docs/design/tool-adapter-matrix.md` 增加 Windows/Linux 适配列
- 明确非 macOS 写入能力仍为 unsupported

### F1: 跨平台编译与只读 smoke CI

目标:

- cross-platform core smoke CI 在 `ubuntu-latest`、`windows-latest` 上运行 `cargo clippy --workspace --exclude wapc-app --all-targets -- -D warnings` 与 `cargo test --workspace --exclude wapc-app`,覆盖非 Tauri GUI app 的 Rust core。
- UI `yarn --cwd ui lint` / `yarn --cwd ui test` / `yarn --cwd ui build` 在 cross-platform core smoke CI 中通过。
- Tauri bundle 先保持 macOS release 主线,Windows/Linux 只做 compile/smoke,不构建 Tauri GUI bundle。

验收:

- CI matrix 不访问真实用户目录。
- 失败信息明确标注 platform。
- 不生成对外可发布的 Windows/Linux artifact。

### F2: PathResolver 抽象

目标:

- 新增平台目录解析层,集中处理 home/config/data/project root。
- 所有工具 adapter 从 PathResolver 获取候选路径,不在业务逻辑里拼平台绝对路径。
- 测试覆盖 Windows/Linux 路径样例,包括 drive letter、XDG、AppData、空格路径。

验收:

- macOS 现有路径行为不回退。
- Windows/Linux 候选路径仅进入 read-only scan,写入状态仍 unsupported。

### F3: 只读工具识别真机核验

目标:

- 先选 2 个低风险工具:Codex + Gemini CLI。
- 分别在 Windows/Linux 上核验安装检测、配置路径、MCP 读取、会话数据读取。
- 增加脱敏 fixture,不读取 prompt/response/source body。

验收:

- `privacy-audit` 覆盖新增平台路径。
- Detector 失败不阻断整次 inventory。
- UI 对未核验工具显示 `待核验` / `unsupported`。

### F4: 写入能力分平台开放

目标:

- 仅对已有真机 fixture 和回滚测试的平台/工具开放写入。
- 每个工具单独 feature gate。
- 同步预览必须展示平台路径、backup 路径和 rollback 状态。

验收:

- 非 macOS 写入默认关闭。
- 每个平台至少有一个 end-to-end fixture: plan -> backup -> write -> verify -> rollback。

## 5. Go / No-Go 门禁

| 门禁 | Go 条件 | No-Go 条件 |
| --- | --- | --- |
| 编译 | 三端 Rust/UI 基础测试通过 | 任一平台基础测试只能靠跳过核心模块通过 |
| 路径 | PathResolver 有单元测试与真机样例 | adapter 内继续散落平台路径拼接 |
| 隐私 | 新平台 fixture 无正文/密钥 | 需要落库真实配置正文才能工作 |
| 写入 | 同平台同工具有备份/回滚/e2e 测试 | 只有手工测试或只验证 happy path |
| 发布 | 平台签名/包格式有明确策略 | unsigned artifact 被包装为正式发布 |

## 6. 适配器矩阵增量摘要

详细候选路径见 [工具适配矩阵](tool-adapter-matrix.md) 第 9 节。摘要:

- Codex/Gemini/OpenCode 这类 CLI 工具最适合作为首批只读跨平台对象。
- Cursor/VS Code 需要同时处理 project `.vscode`/`.cursor` 与 user application data 目录。
- Claude Desktop 这类 GUI 应用在 Windows/Linux 的配置路径与安装形态必须先单独核验。
- Skills symlink 安装策略不应默认跨平台开放;Windows 优先 copy,symlink 只作为高级选项。

## 7. 参考资料

- [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri v2 path API](https://v2.tauri.app/reference/javascript/api/namespacepath/)
- [Tauri v2 configuration](https://v2.tauri.app/develop/configuration-files/)
- [directories crate docs](https://docs.rs/directories)
- [dirs crate docs](https://docs.rs/dirs/latest/dirs/)
