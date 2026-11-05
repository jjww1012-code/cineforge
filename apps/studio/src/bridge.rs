//! `cineforge mcp` – Brücke für Claude Desktop.
//!
//! Claude Desktop startet lokale MCP-Server als Programm und spricht über
//! stdin/stdout mit ihnen. Diese Brücke leitet jede Nachricht an die laufende
//! App weiter. Läuft die App noch nicht, wird sie automatisch gestartet.
//!
//! Wichtig: Auf stdout darf nur JSON-RPC stehen. Meldungen gehen nach stderr.

use std::{
    io::{BufRead, Write},
    time::{Duration, Instant},
};

pub fn run(port: u16) -> std::process::ExitCode {
    let base = format!("http://127.0.0.1:{port}");
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match forward(&base, &line) {
            Ok(r) => r,
            Err(_) => {
                eprintln!("CineForge läuft nicht – starte die App …");
                launch_app();
                if wait_until_ready(&base, Duration::from_secs(45)) {
                    forward(&base, &line).unwrap_or_else(|e| error_reply(&line, &e))
                } else {
                    error_reply(&line, "CineForge Studio konnte nicht gestartet werden")
                }
            }
        };
        if let Some(r) = reply {
            if writeln!(stdout, "{r}").and_then(|_| stdout.flush()).is_err() {
                break;
            }
        }
    }
    std::process::ExitCode::SUCCESS
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(620)))
        .build()
        .into()
}

/// Liefert `Ok(None)` bei Notifications (HTTP 202 ohne Inhalt).
fn forward(base: &str, line: &str) -> Result<Option<String>, String> {
    let mut resp = agent()
        .post(format!("{base}/mcp"))
        .header("Content-Type", "application/json")
        .send(line)
        .map_err(|e| e.to_string())?;
    let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    Ok(if body.trim().is_empty() { None } else { Some(body) })
}

fn wait_until_ready(base: &str, max: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < max {
        if agent().get(format!("{base}/health")).call().is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

fn launch_app() {
    let Ok(exe) = std::env::current_exe() else { return };
    // Ausgaben der App: normalerweise verworfen; mit CINEFORGE_APP_LOG=<Datei> zur Fehlersuche gespeichert.
    let log = std::env::var_os("CINEFORGE_APP_LOG").and_then(|p| std::fs::File::create(p).ok());
    let (out, err) = match log.and_then(|f| f.try_clone().ok().map(|g| (f, g))) {
        Some((f, g)) => (std::process::Stdio::from(f), std::process::Stdio::from(g)),
        None => (std::process::Stdio::null(), std::process::Stdio::null()),
    };
    let _ = std::process::Command::new(exe).stdin(std::process::Stdio::null()).stdout(out).stderr(err).spawn();
}

fn error_reply(line: &str, msg: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let id = v.get("id")?.clone();
    Some(
        serde_json::json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": msg}})
            .to_string(),
    )
}
