# macOS 签名与公证发布说明

状态: 草案,待真实 Apple Developer 凭据验证  
最后更新: 2026-06-06  
关联: Phase 5 WP5.E / `.github/workflows/release.yml` / `scripts/package-macos-app.sh`

## 目标

Phase 5 要求 macOS 发布产物使用 Developer ID 签名并通过 Apple 公证。WAPC 当前实现的是一条真实 CI 门禁链路:有完整凭据时发布 tag 会构建签名/公证产物,缺少任一凭据时 release job 直接失败,不会把 unsigned 或 ad-hoc signed 产物声明为正式发布。

## GitHub Secrets

`Release` workflow 需要以下 GitHub Secrets:

- `APPLE_CERTIFICATE`: Developer ID Application `.p12` 证书的 base64 内容。
- `APPLE_CERTIFICATE_PASSWORD`: 导出 `.p12` 时设置的密码。
- `APPLE_SIGNING_IDENTITY`: `security find-identity -v -p codesigning` 中的 Developer ID Application identity。
- `APPLE_ID`: Apple ID 邮箱。
- `APPLE_PASSWORD`: Apple app-specific password。
- `APPLE_TEAM_ID`: Apple Developer Team ID。
- `KEYCHAIN_PASSWORD`: CI 临时 keychain 密码。

证书来源与导出方式遵循 Tauri v2 macOS signing 文档:在 Keychain Access 中导出 `.p12`,再用 `openssl base64 -A -in certificate.p12 -out certificate-base64.txt` 生成 `APPLE_CERTIFICATE`。

## CI 发布路径

发布入口:

- push `v*` tag。
- 手动触发 `workflow_dispatch`。

CI 步骤:

- 校验所有 signing/notarization secrets;缺失即失败。
- 安装 Rust、macOS target、Node 与 UI 依赖。
- 运行 `cargo test --workspace`、`yarn --cwd ui lint`、`yarn --cwd ui test`、`yarn --cwd ui build`。
- 安装 Tauri CLI 并校验 `cargo tauri build` 可用。
- 导入 Developer ID `.p12` 到临时 keychain,并校验证书 identity。
- 使用 `tauri-apps/tauri-action@v1` 调用 `cargo tauri build --target <arch> --bundles app`,生成 draft GitHub Release 产物。

## 本地打包路径

本地只保留一个 thin wrapper:

```bash
scripts/package-macos-app.sh
```

该脚本进入 `src-tauri` 后委托 `cargo tauri build`。它不再手工创建 `.app`,也不再复制旧 CLI sidecar。缺少正式签名身份时,本地构建只能作为开发验收产物,不能作为 Phase 5 正式发布验收。

## 验收边界

- `src-tauri/tauri.conf.json` 的 `minimumSystemVersion` 必须保持为 `12.0`,与 README/PRD 的 macOS 12+ 策略一致。
- release workflow 必须使用 Tauri build/release 路径,不能回退到 `scripts/package-macos-app.sh` 或直接复制 `target/release/wapc`。
- CI 缺少 Apple Developer 凭据时必须失败,不能降级为 unsigned/ad-hoc 正式发布。
- AC-E 的最终验收仍需要一次真实 Apple Developer 账号下的 tag release,并在干净 macOS 机器上下载启动确认无 Gatekeeper 拦截。
