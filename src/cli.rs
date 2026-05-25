//! Command-line interface for the local WAPC token observer.
//! @author codex

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use chrono::Local;
use clap::{Parser, Subcommand};

use crate::{scanner, store::UsageStore};

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
        #[arg(long)]
        tool: Option<String>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    PrivacyAudit {
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long)]
        db: Option<PathBuf>,
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
        Command::Report { period, tool, db } => {
            if let Some(period) = period.as_deref()
                && period != "today"
            {
                bail!("unsupported report period: {period}");
            }
            let db = resolve_db(db)?;
            let store = UsageStore::open(&db)?;
            let day_prefix = match period.as_deref() {
                Some("today") => Some(Local::now().format("%Y-%m-%d").to_string()),
                _ => None,
            };
            let summaries =
                store.summary_by_tool_filtered(tool.as_deref(), day_prefix.as_deref())?;
            println!(
                "tool,records,input,output,cache_read,cache_write,reasoning,tool_tokens,total,cost_usd"
            );
            for summary in summaries {
                println!(
                    "{},{},{},{},{},{},{},{},{},{}",
                    summary.tool,
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
            Ok(())
        }
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
