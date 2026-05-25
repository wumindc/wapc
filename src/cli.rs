//! Command-line interface for the local WAPC token observer.
//! @author codex

use std::{env, path::PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Local;
use clap::{Parser, Subcommand};

use crate::{launchd, scanner, store::UsageStore};

#[derive(Debug, Parser)]
#[command(
    name = "wapc",
    version,
    about = "Passive local AI coding token observer"
)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Scan {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    Report {
        period: Option<String>,
        #[arg(long, default_value = "tool")]
        group: String,
        #[arg(long)]
        tool: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    Doctor {
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    PrivacyAudit {
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ServiceCommand {
    Install {
        #[arg(long, default_value_t = 15)]
        interval_minutes: u64,
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long)]
        binary: Option<PathBuf>,
    },
    Uninstall {
        #[arg(long)]
        home: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        home: Option<PathBuf>,
    },
}

pub fn run() -> Result<()> {
    run_with_args(Args::parse())
}

fn run_with_args(args: Args) -> Result<()> {
    match args.command {
        Command::Scan { dry_run, home, db } => {
            let home = resolve_home(home)?;
            let records = scanner::scan_home(&home)?;
            if dry_run {
                println!("found {} usage records", records.len());
                return Ok(());
            }
            let db = resolve_db(db)?;
            let store = UsageStore::open(&db)?;
            let changed = store.upsert_records(&records)?;
            println!("indexed {changed} usage records into {}", db.display());
            Ok(())
        }
        Command::Report {
            period,
            group,
            tool,
            project,
            json,
            db,
        } => {
            if let Some(period) = period.as_deref()
                && period != "today"
            {
                bail!("unsupported report period: {period}");
            }
            if tool.is_some() && project.is_some() {
                bail!("--tool and --project cannot be used together");
            }
            let db = resolve_db(db)?;
            let store = UsageStore::open(&db)?;
            let day_prefix = match period.as_deref() {
                Some("today") => Some(Local::now().format("%Y-%m-%d").to_string()),
                _ => None,
            };
            let summaries = match group.as_str() {
                "tool" => store.summary_by_tool_filtered(tool.as_deref(), day_prefix.as_deref())?,
                "project" => {
                    store.summary_by_project_filtered(project.as_deref(), day_prefix.as_deref())?
                }
                other => bail!("unsupported report group: {other}"),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&summaries)?);
            } else {
                print_summaries(&group, &summaries);
            }
            Ok(())
        }
        Command::Doctor { home, db } => {
            let home = resolve_home(home)?;
            let db = resolve_db(db)?;
            println!("wapc: {}", env!("CARGO_PKG_VERSION"));
            println!("home: {}", home.display());
            println!("db: {}", db.display());
            println!("db_exists: {}", db.exists());
            println!("sources:");
            for path in scanner::audit_paths(&home) {
                println!("- {} exists={}", path.display(), path.exists());
            }
            let records = scanner::scan_home(&home)?;
            println!("scan_records: {}", records.len());
            println!("service_installed: {}", launchd::is_installed(&home));
            println!("service_loaded: {}", launchd::is_loaded());
            Ok(())
        }
        Command::Service { command } => match command {
            ServiceCommand::Install {
                interval_minutes,
                home,
                binary,
            } => {
                let home = resolve_home(home)?;
                let binary = match binary {
                    Some(path) => path,
                    None => env::current_exe().context("resolve current executable")?,
                };
                let path = launchd::install(&home, &binary, interval_minutes)?;
                println!("installed LaunchAgent {}", path.display());
                Ok(())
            }
            ServiceCommand::Uninstall { home } => {
                let home = resolve_home(home)?;
                let path = launchd::uninstall(&home)?;
                println!("uninstalled LaunchAgent {}", path.display());
                Ok(())
            }
            ServiceCommand::Status { home } => {
                let home = resolve_home(home)?;
                let path = launchd::agent_path(&home);
                println!("installed: {}", path.exists());
                println!("loaded: {}", launchd::is_loaded());
                println!("path: {}", path.display());
                Ok(())
            }
        },
        Command::PrivacyAudit { home, db } => {
            let home = resolve_home(home)?;
            let db = resolve_db(db)?;
            println!("WAPC reads these local directories if they exist:");
            for path in scanner::audit_paths(&home) {
                println!("- {}", path.display());
            }
            println!("WAPC stores token metadata only in:");
            println!("- {}", db.display());
            println!(
                "Stored fields: tool, source_path, session_id, timestamp, project_path, model, token buckets, cost, precision."
            );
            println!("Not stored: prompt text, response text, file contents, tool output bodies.");
            Ok(())
        }
    }
}

fn print_summaries(group: &str, summaries: &[crate::store::UsageSummary]) {
    println!(
        "{group},records,input,output,cache_read,cache_write,reasoning,tool_tokens,total,cost_usd"
    );
    for summary in summaries {
        println!(
            "{},{},{},{},{},{},{},{},{},{}",
            summary.name,
            summary.records,
            summary.usage.input,
            summary.usage.output,
            summary.usage.cache_read,
            summary.usage.cache_write,
            summary.usage.reasoning,
            summary.usage.tool,
            summary.usage.total(),
            summary.cost_usd
        );
    }
}

fn resolve_home(home: Option<PathBuf>) -> Result<PathBuf> {
    home.or_else(dirs_next::home_dir)
        .context("cannot resolve home directory")
}

fn resolve_db(db: Option<PathBuf>) -> Result<PathBuf> {
    match db {
        Some(path) => Ok(path),
        None => {
            let home = dirs_next::home_dir().context("cannot resolve home directory")?;
            Ok(home.join(".wapc/wapc.db"))
        }
    }
}
