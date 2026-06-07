//! Local read-only headless dashboard server.
//! @author codex

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Result, bail};
use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{model::TokenUsage, store::UsageStore};

#[derive(Clone, Debug)]
pub struct HeadlessDashboardConfig {
    pub bind_host: String,
    pub port: u16,
    pub db_path: PathBuf,
}

pub struct HeadlessDashboardServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for HeadlessDashboardServer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HeadlessDashboardServer")
            .field("addr", &self.addr)
            .finish_non_exhaustive()
    }
}

impl HeadlessDashboardServer {
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for HeadlessDashboardServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn start_headless_dashboard(
    config: HeadlessDashboardConfig,
) -> Result<HeadlessDashboardServer> {
    if config.bind_host != "127.0.0.1" {
        bail!("headless dashboard may only bind to 127.0.0.1");
    }
    let listener = TcpListener::bind((config.bind_host.as_str(), config.port))?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let db_path = config.db_path;
    let handle = thread::spawn(move || {
        while !thread_stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = handle_connection(stream, &db_path);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });
    Ok(HeadlessDashboardServer {
        addr,
        stop,
        handle: Some(handle),
    })
}

fn handle_connection(mut stream: TcpStream, db_path: &Path) -> Result<()> {
    stream.set_nonblocking(false)?;
    if let Err(error) = handle_request(&mut stream, db_path) {
        let body = serde_json::json!({
            "error": "internal_server_error",
            "message": error.to_string()
        })
        .to_string();
        write_response(
            &mut stream,
            "500 Internal Server Error",
            "application/json; charset=utf-8",
            &body,
        )?;
        return Err(error);
    }
    Ok(())
}

fn handle_request(stream: &mut TcpStream, db_path: &Path) -> Result<()> {
    let mut buffer = [0_u8; 8192];
    let read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let Some(request_line) = request.lines().next() else {
        return write_response(
            stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            "bad request",
        );
    };
    let parts = request_line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 {
        return write_response(
            stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            "bad request",
        );
    }
    let method = parts[0];
    let path = parts[1];
    if method != "GET" {
        return write_response(
            stream,
            "405 Method Not Allowed",
            "application/json; charset=utf-8",
            r#"{"error":"read_only"}"#,
        );
    }
    match path {
        "/" => {
            let summary = headless_summary(db_path)?;
            write_response(
                stream,
                "200 OK",
                "text/html; charset=utf-8",
                &render_dashboard_html(&summary),
            )
        }
        "/health" => write_response(
            stream,
            "200 OK",
            "application/json; charset=utf-8",
            r#"{"ok":true,"read_only":true}"#,
        ),
        "/api/summary" => {
            let body = serde_json::to_string(&headless_summary(db_path)?)?;
            write_response(stream, "200 OK", "application/json; charset=utf-8", &body)
        }
        _ => write_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            r#"{"error":"not_found"}"#,
        ),
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn headless_summary(db_path: &Path) -> Result<HeadlessSummary> {
    let store = UsageStore::open(db_path)?;
    let tools = store.summary_by_tool(None)?;
    let daily = store.summary_by_day()?;
    let projects = store
        .project_summaries()?
        .into_iter()
        .map(|project| HeadlessProjectSummary {
            project_hash: stable_hash_16(&project.canonical_path),
            records: project.records,
            usage: project.usage,
            cost_usd: project.cost_usd,
            tools: project.tools,
        })
        .collect();
    Ok(HeadlessSummary {
        schema: "wapc.headless_summary.v1",
        generated_at: Utc::now().to_rfc3339(),
        read_only: true,
        tools,
        projects,
        daily,
    })
}

fn render_dashboard_html(summary: &HeadlessSummary) -> String {
    let total_tokens = summary
        .tools
        .iter()
        .map(|row| row.usage.total())
        .sum::<u64>();
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>WAPC Headless Dashboard</title></head><body><h1>WAPC Headless Dashboard</h1><p>Read only: true</p><p>Total tokens: {total_tokens}</p><p>Tools: {}</p><p>Projects: {}</p></body></html>",
        summary.tools.len(),
        summary.projects.len()
    )
}

fn stable_hash_16(value: &str) -> String {
    let digest = Sha256::digest(format!("wapc-headless-dashboard-v1:{value}").as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[derive(Serialize)]
struct HeadlessSummary {
    schema: &'static str,
    generated_at: String,
    read_only: bool,
    tools: Vec<crate::store::UsageSummary>,
    projects: Vec<HeadlessProjectSummary>,
    daily: Vec<crate::store::UsageSummary>,
}

#[derive(Serialize)]
struct HeadlessProjectSummary {
    project_hash: String,
    records: u64,
    usage: TokenUsage,
    cost_usd: f64,
    tools: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpStream,
    };

    use tempfile::tempdir;

    use crate::{
        model::{SourcePrecision, TokenUsage, ToolKind, UsageRecord},
        store::UsageStore,
    };

    use super::*;

    #[test]
    fn rejects_non_loopback_bind_host() {
        let dir = tempdir().unwrap();
        let config = HeadlessDashboardConfig {
            bind_host: "0.0.0.0".to_string(),
            port: 0,
            db_path: dir.path().join("wapc.db"),
        };

        let error = start_headless_dashboard(config).unwrap_err();

        assert!(error.to_string().contains("127.0.0.1"));
    }

    #[test]
    fn serves_read_only_summary_from_real_store_without_raw_fields() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("wapc.db");
        let store = UsageStore::open(&db).unwrap();
        store
            .upsert_records(&[UsageRecord {
                id: "r1".to_string(),
                tool: ToolKind::Claude,
                source_path: "/Users/alice/.claude/projects/secret/session.jsonl".to_string(),
                session_id: Some("secret-session".to_string()),
                timestamp: Some("2026-06-06T01:00:00Z".parse().unwrap()),
                project_path: Some("/Users/alice/work/secret-project".to_string()),
                model: Some("claude-opus".to_string()),
                usage: TokenUsage {
                    input: 11,
                    output: 7,
                    ..TokenUsage::default()
                },
                cost_usd: Some(0.42),
                precision: SourcePrecision::Exact,
            }])
            .unwrap();

        let server = start_headless_dashboard(HeadlessDashboardConfig {
            bind_host: "127.0.0.1".to_string(),
            port: 0,
            db_path: db,
        })
        .unwrap();

        let response = http_request(
            server.port(),
            "GET /api/summary HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        );
        assert!(
            response.starts_with("HTTP/1.1 200 OK"),
            "unexpected response: {response}"
        );
        let body = response.split("\r\n\r\n").nth(1).unwrap();
        let json: serde_json::Value = serde_json::from_str(body).unwrap();

        assert_eq!(json["schema"], "wapc.headless_summary.v1");
        assert_eq!(json["tools"][0]["name"], "claude");
        assert_eq!(json["tools"][0]["records"], 1);
        assert_eq!(json["tools"][0]["usage"]["input"], 11);
        assert_eq!(json["projects"][0]["records"], 1);
        assert!(!body.contains("/Users/alice"));
        assert!(!body.contains("secret-project"));
        assert!(!body.contains("secret-session"));
        assert!(!body.contains("session.jsonl"));

        let post_response = http_request(
            server.port(),
            "POST /api/summary HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\n\r\n",
        );
        assert!(post_response.starts_with("HTTP/1.1 405 Method Not Allowed"));

        let missing_response = http_request(
            server.port(),
            "GET /api/sync HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        );
        assert!(missing_response.starts_with("HTTP/1.1 404 Not Found"));
    }

    fn http_request(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }
}
