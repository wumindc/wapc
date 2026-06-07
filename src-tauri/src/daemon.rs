use std::time::Duration;
use tauri::AppHandle;
use tokio::time::sleep;

use crate::commands::{get_auto_scan_config, scan_now};

pub fn start_auto_scan_daemon(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Track the last time we successfully scanned
        let mut last_scan_time: Option<std::time::Instant> = None;

        loop {
            // Wake up every minute to check the configuration
            sleep(Duration::from_secs(60)).await;

            // Fetch the latest config from the database
            match get_auto_scan_config() {
                Ok(config) => {
                    if config.enabled {
                        let interval = Duration::from_secs(config.interval_minutes * 60);
                        let should_scan = match last_scan_time {
                            Some(last) => last.elapsed() >= interval,
                            None => true, // Run immediately on first cycle if enabled
                        };

                        if should_scan {
                            println!("Daemon: Triggering auto scan...");
                            match scan_now(app.clone()).await {
                                Ok(records) => {
                                    println!("Daemon: Auto scan finished, inserted {} records", records);
                                    last_scan_time = Some(std::time::Instant::now());
                                }
                                Err(e) => {
                                    eprintln!("Daemon: Auto scan failed: {}", e);
                                    // Optionally set last_scan_time to avoid rapid retries on persistent errors
                                    // last_scan_time = Some(std::time::Instant::now());
                                }
                            }
                        }
                    } else {
                        // If disabled, reset the timer so it scans immediately when re-enabled
                        last_scan_time = None;
                    }
                }
                Err(e) => {
                    eprintln!("Daemon: Failed to load auto-scan config: {}", e);
                }
            }
        }
    });
}
