//! macOS LaunchAgent support for periodic passive scans.
//! @author codex

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

pub const LABEL: &str = "com.wapc.scan";

pub fn agent_path(home: &Path) -> PathBuf {
    home.join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

pub fn plist_contents(binary: &Path, interval_seconds: u64) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>scan</string>
  </array>
  <key>StartInterval</key>
  <integer>{interval_seconds}</integer>
  <key>RunAtLoad</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/wapc.launchd.out.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/wapc.launchd.err.log</string>
</dict>
</plist>
"#,
        escape_xml(&binary.display().to_string())
    )
}

pub fn install(home: &Path, binary: &Path, interval_minutes: u64) -> Result<PathBuf> {
    if interval_minutes == 0 {
        bail!("interval_minutes must be greater than 0");
    }
    let path = agent_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, plist_contents(binary, interval_minutes * 60))?;
    quiet_launchctl(&["unload", path.to_string_lossy().as_ref()]);
    launchctl(&["load", path.to_string_lossy().as_ref()])
        .with_context(|| format!("load LaunchAgent {}", path.display()))?;
    Ok(path)
}

pub fn uninstall(home: &Path) -> Result<PathBuf> {
    let path = agent_path(home);
    if path.exists() {
        quiet_launchctl(&["unload", path.to_string_lossy().as_ref()]);
        fs::remove_file(&path)?;
    }
    Ok(path)
}

pub fn is_installed(home: &Path) -> bool {
    agent_path(home).exists()
}

pub fn is_loaded() -> bool {
    let Ok(output) = Command::new("launchctl").arg("list").output() else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).contains(LABEL)
}

fn launchctl(args: &[&str]) -> Result<()> {
    let status = Command::new("launchctl").args(args).status()?;
    if !status.success() {
        bail!("launchctl {:?} exited with {status}", args);
    }
    Ok(())
}

fn quiet_launchctl(args: &[&str]) {
    let _ = Command::new("launchctl").args(args).output();
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_contains_scan_command_and_interval() {
        let plist = plist_contents(Path::new("/opt/homebrew/bin/wapc"), 900);

        assert!(plist.contains("<string>com.wapc.scan</string>"));
        assert!(plist.contains("<string>/opt/homebrew/bin/wapc</string>"));
        assert!(plist.contains("<string>scan</string>"));
        assert!(plist.contains("<integer>900</integer>"));
    }

    #[test]
    fn agent_path_uses_user_launch_agents_directory() {
        let path = agent_path(Path::new("/Users/example"));

        assert_eq!(
            path,
            Path::new("/Users/example/Library/LaunchAgents/com.wapc.scan.plist")
        );
    }
}
