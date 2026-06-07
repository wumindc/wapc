//! Tauri application library — registers commands and plugins.
//! @author Claude Sonnet 4.6 (Thinking)

mod commands;
mod daemon;
mod updater;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            app.set_activation_policy(tauri::ActivationPolicy::Regular);
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
                window.set_focus()?;
            }
            daemon::start_auto_scan_daemon(app.handle().clone());
            // 延迟 30 秒后在后台检查更新，避免影响启动性能
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                updater::check_and_notify(handle).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::scan_now,
            commands::get_auto_scan_config,
            commands::set_auto_scan_config,
            commands::get_trend,
            commands::detect_tools,
            commands::list_tools,
            commands::source_health,
            commands::list_pricing_rules,
            commands::upsert_pricing_rule,
            commands::delete_pricing_rule,
            commands::recompute_costs,
            commands::list_project_aliases,
            commands::set_project_alias,
            commands::export_report,
            commands::export_backup,
            commands::import_backup,
            commands::privacy_audit,
            commands::preview_deep_link_import,
            commands::plan_deep_link_import,
            commands::headless_dashboard_status,
            commands::start_headless_dashboard,
            commands::stop_headless_dashboard,
            commands::inventory_scan,
            commands::list_resources,
            commands::get_resource,
            commands::list_parse_failures,
            commands::get_guide,
            commands::adapter_capabilities,
            commands::list_sessions,
            commands::plan_resource_change,
            commands::plan_sync,
            commands::list_resource_templates,
            commands::plan_template_sync,
            commands::apply_sync,
            commands::list_sync_operations,
            commands::save_sync_preset,
            commands::list_sync_presets,
            commands::delete_sync_preset,
            commands::export_sync_presets,
            commands::apply_resource_change,
            commands::list_changes,
            commands::list_backups,
            commands::rollback_change,
            commands::check_update,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    //! Tauri desktop configuration contract tests.
    //! @author codex

    use serde_json::Value;

    #[test]
    fn tauri_frontend_commands_use_workspace_ui_directory() {
        let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config = std::fs::read_to_string(config_path).unwrap();
        let value: Value = serde_json::from_str(&config).unwrap();
        let build = value.get("build").unwrap();

        assert_eq!(
            build.get("beforeBuildCommand").and_then(Value::as_str),
            Some("yarn --cwd ui build")
        );
        assert_eq!(
            build.get("beforeDevCommand").and_then(Value::as_str),
            Some("yarn --cwd ui dev")
        );
    }

    #[test]
    fn tauri_commands_resolve_app_paths_through_core_path_resolver() {
        let commands_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands.rs");
        let commands = std::fs::read_to_string(commands_path).unwrap();

        assert!(commands.contains("platform_paths::{"));
        assert!(commands.contains("WapcPaths"));
        assert!(commands.contains("PlatformPathContext"));
        assert!(commands.contains("WapcPaths::from_platform_home()"));
        assert!(
            !commands.contains("home.join(\".wapc/wapc.db\")"),
            "Tauri commands must not hand-roll WAPC db paths"
        );
    }

    #[test]
    fn tauri_phase_two_bundle_target_is_app_only() {
        let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config = std::fs::read_to_string(config_path).unwrap();
        let value: Value = serde_json::from_str(&config).unwrap();
        let targets = value
            .pointer("/bundle/targets")
            .and_then(Value::as_array)
            .unwrap();

        assert_eq!(
            targets,
            &[
                Value::String("dmg".to_string()),
                Value::String("app".to_string())
            ]
        );
    }

    #[test]
    fn tauri_registers_wapc_deep_link_scheme_statically() {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let manifest = std::fs::read_to_string(manifest_path).unwrap();
        let lib_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
        let lib = std::fs::read_to_string(lib_path).unwrap();
        let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config = std::fs::read_to_string(config_path).unwrap();
        let value: Value = serde_json::from_str(&config).unwrap();
        let schemes = value
            .pointer("/plugins/deep-link/desktop/schemes")
            .and_then(Value::as_array)
            .unwrap();

        assert!(
            manifest.contains("tauri-plugin-deep-link"),
            "Tauri app must include the deep-link plugin dependency"
        );
        assert!(
            lib.contains("tauri_plugin_deep_link::init()"),
            "Tauri app must initialize the deep-link plugin"
        );
        assert!(
            schemes.iter().any(|scheme| scheme.as_str() == Some("wapc")),
            "Tauri config must statically register the wapc:// desktop scheme"
        );
    }

    #[test]
    fn tauri_registers_dialog_plugin_for_user_selected_exports() {
        let lib_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
        let lib = std::fs::read_to_string(lib_path).unwrap();

        assert!(
            lib.contains("tauri_plugin_dialog::init()"),
            "desktop export flow must use the real Tauri dialog plugin for user-selected directories"
        );
    }

    #[test]
    fn tauri_macos_release_minimum_matches_readme_support_policy() {
        let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config = std::fs::read_to_string(config_path).unwrap();
        let value: Value = serde_json::from_str(&config).unwrap();

        assert_eq!(
            value
                .pointer("/bundle/macOS/minimumSystemVersion")
                .and_then(Value::as_str),
            Some("12.0")
        );
    }

    #[test]
    fn release_workflow_uses_tauri_signing_and_notarization_path() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let workflow = std::fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();

        for expected in [
            "tauri-apps/tauri-action@v0",
            "APPLE_CERTIFICATE",
            "APPLE_CERTIFICATE_PASSWORD",
            "APPLE_SIGNING_IDENTITY",
            "APPLE_ID",
            "APPLE_PASSWORD",
            "APPLE_TEAM_ID",
            "KEYCHAIN_PASSWORD",
            "security import certificate.p12",
            "yarn --cwd ui lint",
            "cargo tauri build",
        ] {
            assert!(
                workflow.contains(expected),
                "release workflow must contain {expected}"
            );
        }
        for forbidden in ["scripts/package-macos-app.sh", "target/release/wapc "] {
            assert!(
                !workflow.contains(forbidden),
                "release workflow must not use old CLI packaging path: {forbidden}"
            );
        }
    }

    #[test]
    fn ci_workflow_runs_workspace_ui_and_tauri_release_gates() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let workflow = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();

        for expected in [
            "actions/setup-node@v4",
            "cache-dependency-path: ui/yarn.lock",
            "cargo fmt --check",
            "cargo clippy --workspace --all-targets -- -D warnings",
            "cargo test --workspace",
            "yarn --cwd ui install --frozen-lockfile",
            "yarn --cwd ui lint",
            "yarn --cwd ui test",
            "yarn --cwd ui build",
            "cargo install tauri-cli --locked",
            "cargo tauri build",
        ] {
            assert!(
                workflow.contains(expected),
                "CI workflow must contain {expected}"
            );
        }
        for forbidden in ["cargo test\n", "cargo build --release"] {
            assert!(
                !workflow.contains(forbidden),
                "CI workflow must not use stale root-only gate: {forbidden}"
            );
        }
    }

    #[test]
    fn ci_workflow_has_non_release_cross_platform_smoke_gates() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let workflow = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();

        for expected in [
            "cross-platform-smoke",
            "runs-on: ${{ matrix.os }}",
            "ubuntu-latest",
            "windows-latest",
            "cargo clippy --workspace --exclude wapc-app --all-targets -- -D warnings",
            "cargo test --workspace --exclude wapc-app",
            "yarn --cwd ui lint",
            "yarn --cwd ui test",
            "yarn --cwd ui build",
        ] {
            assert!(
                workflow.contains(expected),
                "CI cross-platform smoke job must contain {expected}"
            );
        }
        let smoke_job = workflow
            .split("cross-platform-smoke:")
            .nth(1)
            .expect("CI workflow must define cross-platform-smoke job");
        let smoke_job = smoke_job.split("\n  desktop:").next().unwrap_or(smoke_job);
        for forbidden in ["cargo tauri build", "actions/upload-artifact"] {
            assert!(
                !smoke_job.contains(forbidden),
                "cross-platform smoke must not produce release artifacts: {forbidden}"
            );
        }
    }

    #[test]
    fn cross_platform_docs_match_core_smoke_ci_scope() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let matrix =
            std::fs::read_to_string(root.join("docs/design/tool-adapter-matrix.md")).unwrap();
        let feasibility =
            std::fs::read_to_string(root.join("docs/design/cross-platform-feasibility.md"))
                .unwrap();

        for expected in [
            "cross-platform core smoke CI",
            "cargo clippy --workspace --exclude wapc-app --all-targets -- -D warnings",
            "cargo test --workspace --exclude wapc-app",
            "yarn --cwd ui lint",
            "yarn --cwd ui test",
            "yarn --cwd ui build",
            "不构建 Tauri GUI bundle",
        ] {
            assert!(
                matrix.contains(expected),
                "tool adapter matrix must document cross-platform smoke scope: {expected}"
            );
            assert!(
                feasibility.contains(expected),
                "cross-platform feasibility doc must document cross-platform smoke scope: {expected}"
            );
        }

        let matrix_no_go = matrix
            .split("## 9. 跨平台 Go / No-Go 清单")
            .nth(1)
            .unwrap_or("");
        assert!(
            !matrix_no_go
                .contains("cargo test --workspace` 在 `ubuntu-latest` 和 `windows-latest`"),
            "tool adapter matrix must not overclaim full workspace tests on non-macOS"
        );
    }

    #[test]
    fn macos_package_script_delegates_to_tauri_build() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let script = std::fs::read_to_string(root.join("scripts/package-macos-app.sh")).unwrap();

        assert!(script.contains("cargo tauri build"));
        assert!(!script.contains("--bin wapc"));
        assert!(!script.contains("wapc-cli"));
        assert!(!script.contains("Contents/MacOS"));
    }

    #[test]
    fn readme_documents_release_gate_without_pretending_notarization_is_done() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let readme = std::fs::read_to_string(root.join("README.md")).unwrap();

        for expected in [
            "macOS 签名与公证发布说明",
            "docs/release/macos-signing-notarization.md",
            "GitHub Release",
            "Apple Developer",
            "Gatekeeper",
            "源码构建",
            "本地验收",
            "cargo fmt --check",
            "cargo clippy --workspace --all-targets -- -D warnings",
            "cargo test --workspace",
            "yarn --cwd ui lint",
            "yarn --cwd ui test",
            "yarn --cwd ui build",
            "cargo tauri build --manifest-path src-tauri/Cargo.toml",
        ] {
            assert!(
                readme.contains(expected),
                "README release section must contain {expected}"
            );
        }
        for forbidden in [
            "当前已完成 Gatekeeper 验收",
            "已经通过 Gatekeeper 验收",
            "无需源码构建",
        ] {
            assert!(
                !readme.contains(forbidden),
                "README must not overclaim release readiness: {forbidden}"
            );
        }
    }

    #[test]
    fn tauri_bundle_identifier_does_not_conflict_with_macos_app_extension() {
        let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config = std::fs::read_to_string(config_path).unwrap();
        let value: Value = serde_json::from_str(&config).unwrap();
        let identifier = value.get("identifier").and_then(Value::as_str).unwrap();

        assert_eq!(identifier, "com.wapc.desktop");
        assert!(
            !identifier.ends_with(".app"),
            "macOS bundle identifiers must not end with .app"
        );
    }

    #[test]
    fn tauri_run_sets_regular_macos_activation_policy() {
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
        let source = std::fs::read_to_string(source_path).unwrap();
        let production_source = source.split("#[cfg(test)]").next().unwrap();

        assert!(
            production_source
                .contains("app.set_activation_policy(tauri::ActivationPolicy::Regular)")
        );
    }

    #[test]
    fn tauri_main_window_is_auto_created_from_config() {
        let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config = std::fs::read_to_string(config_path).unwrap();
        let value: Value = serde_json::from_str(&config).unwrap();
        let window = value.pointer("/app/windows/0").unwrap();

        assert_eq!(window.get("label").and_then(Value::as_str), Some("main"));
        assert_eq!(window.get("url").and_then(Value::as_str), Some("/"));
        assert_eq!(window.get("create").and_then(Value::as_bool), Some(true));
        assert_eq!(window.get("visible").and_then(Value::as_bool), Some(true));
        assert_eq!(window.get("focus").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn get_snapshot_returns_desktop_bootstrap_data() {
        let temp_home =
            std::env::temp_dir().join(format!("wapc-snapshot-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_home);
        std::fs::create_dir_all(&temp_home).unwrap();
        let temp_db = temp_home.join(".wapc/wapc.db");

        let snapshot = crate::commands::get_snapshot_for_paths(&temp_home, &temp_db).unwrap();

        assert_eq!(snapshot.home_path, temp_home.display().to_string());
        assert_eq!(snapshot.db_path, temp_db.display().to_string());
        assert!(snapshot.db_exists);
        assert!(!snapshot.version.is_empty());

        std::fs::remove_dir_all(&temp_home).unwrap();
    }
}
