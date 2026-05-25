# WAPC

WAPC 是一个 macOS-first 的本机 AI 编程工具 Token 观测器。第一版以 Rust CLI 形态交付，通过旁路读取本机已有 usage/session 文件来统计 token，不替换命令、不修改工具、不安装代理证书、不上传数据。

## 已支持工具

- Claude Code: `~/.claude/projects/**/*.jsonl`
- Codex: `~/.codex/sessions/**/*.jsonl`、`~/.codex/archived_sessions/*.jsonl`
- Gemini CLI: `~/.gemini/tmp/**/chats/*.json`
- OpenCode: `~/.local/share/opencode/storage/**/*.json`

## 隐私边界

WAPC 只落库这些字段：工具名、来源文件路径、会话 ID、时间、项目路径、模型、token 分桶、费用、精度。

WAPC 不落库 prompt 正文、response 正文、文件内容或工具输出正文。

## 本地运行

```bash
cargo run -- privacy-audit
cargo run -- scan --dry-run
cargo run -- scan
cargo run -- report today
cargo run -- report --tool claude
```

默认数据库位置：

```bash
~/.wapc/wapc.db
```

可以覆盖 home 和 db，便于测试：

```bash
cargo run -- scan --home /tmp/wapc-home --db /tmp/wapc.db
```

## 安装

```bash
cargo install --path .
```

如果 `~/.cargo/bin` 不在 PATH 中，可以创建软链接：

```bash
ln -sf ~/.cargo/bin/wapc /opt/homebrew/bin/wapc
```

安装后：

```bash
wapc privacy-audit
wapc doctor
wapc scan
wapc report today
wapc report --group project
wapc report --tool claude
wapc report today --json
```

## 后台定时扫描

安装 LaunchAgent，每 15 分钟自动执行一次 `wapc scan`：

```bash
wapc service install --binary /opt/homebrew/bin/wapc --interval-minutes 15
wapc service status
```

卸载后台扫描：

```bash
wapc service uninstall
```

卸载：

```bash
cargo uninstall wapc
```

删除本机索引库：

```bash
rm -f ~/.wapc/wapc.db
```

## 验证

```bash
cargo fmt --check
cargo test
cargo build --release
```

## 发布

推送 tag 会触发 GitHub Actions 构建 macOS release artifact：

```bash
git tag v0.1.0
git push origin v0.1.0
```
