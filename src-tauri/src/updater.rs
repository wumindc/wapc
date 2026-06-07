//! 自动更新检查与安装逻辑
//! 使用 tauri-plugin-updater v2 检查 GitHub Releases 上的新版本。
//! 发现新版本时发出 "update-available" 事件通知前端，
//! 前端触发 install_update 命令后执行下载、进度上报、重启。
//! @author Claude Sonnet 4.6 (Thinking)

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// 发给前端的更新信息事件载荷
#[derive(Clone, Serialize)]
pub struct UpdateAvailablePayload {
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
}

/// 下载进度事件载荷（0–100）
#[derive(Clone, Serialize)]
pub struct UpdateProgressPayload {
    pub percent: u64,
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// 检查是否有新版本，若有则向前端发出 `update-available` 事件。
/// 应在应用启动后在后台任务中调用（已延迟 30 秒），避免影响启动性能。
pub async fn check_and_notify(app: AppHandle) {
    // 若 pubkey 是占位符，则跳过（开发阶段）
    let conf = match app.config().plugins.0.get("updater") {
        Some(v) => v.clone(),
        None => {
            println!("[updater] updater plugin not configured, skipping check");
            return;
        }
    };

    let pubkey = conf
        .get("pubkey")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    if pubkey.starts_with("PLACEHOLDER") || pubkey.is_empty() {
        println!("[updater] pubkey is placeholder, skipping update check (dev mode)");
        return;
    }

    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("[updater] failed to get updater: {e}");
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            let payload = UpdateAvailablePayload {
                version: update.version.clone(),
                notes: update.body.clone(),
                pub_date: update.date.map(|d| d.to_string()),
            };
            println!("[updater] new version available: {}", &payload.version);
            let _ = app.emit("update-available", payload);
        }
        Ok(None) => {
            println!("[updater] app is up to date");
        }
        Err(e) => {
            eprintln!("[updater] update check failed: {e}");
        }
    }
}

/// 执行下载并安装，期间向前端发送进度事件，完成后重启应用。
pub async fn download_and_install(app: AppHandle) -> Result<(), String> {
    let pubkey = app
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|v| v.get("pubkey"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    if pubkey.starts_with("PLACEHOLDER") || pubkey.is_empty() {
        return Err("updater not configured (placeholder pubkey)".into());
    }

    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no update available".to_string())?;

    let app_clone = app.clone();
    update
        .download_and_install(
            move |downloaded, total| {
                let downloaded_u64 = downloaded as u64;
                let percent = total
                    .and_then(|t| (downloaded_u64 * 100).checked_div(t))
                    .unwrap_or(0);
                let _ = app_clone.emit(
                    "update-progress",
                    UpdateProgressPayload {
                        percent,
                        downloaded: downloaded_u64,
                        total,
                    },
                );
            },
            || {
                println!("[updater] download complete, preparing to install");
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    app.restart();
}
