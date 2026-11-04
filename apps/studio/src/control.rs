//! Lokaler Steuerungs-Server (MCP über HTTP).
//!
//! Claude verbindet sich hierüber mit der laufenden App:
//! - Claude Code:   `claude mcp add --transport http cineforge http://127.0.0.1:47800/mcp`
//! - Claude Desktop: über die Brücke `cineforge mcp` (stdio), siehe bridge.rs
//!
//! Sicherheit: Der Server lauscht nur auf 127.0.0.1 (nicht im Netzwerk
//! erreichbar) und lehnt Anfragen von fremden Webseiten (Origin-Header) ab.

use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    time::Duration,
};

use bevy::prelude::*;
use serde_json::{json, Value};

use crate::{
    source::{SceneLoaded, SceneSource},
    tools::{self, ToolCall, ToolOutput, UserViewQuery},
};

pub const DEFAULT_PORT: u16 = 47800;
const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// Zustand der Claude-Verbindung (für die Anzeige).
#[derive(Resource, Default)]
pub struct ControlStatus {
    pub port: Option<u16>,
    pub error: Option<String>,
    pub calls: u32,
    pub last_action: Option<String>,
}

#[derive(Resource)]
struct Inbox(Mutex<Receiver<ToolCall>>);

pub struct ControlPlugin {
    pub port: u16,
}

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = mpsc::channel::<ToolCall>();
        let mut status = ControlStatus::default();
        match tiny_http::Server::http(("127.0.0.1", self.port)) {
            Ok(server) => {
                status.port = Some(self.port);
                info!("Claude-Verbindung bereit: http://127.0.0.1:{}/mcp", self.port);
                std::thread::Builder::new()
                    .name("cineforge-control".into())
                    .spawn(move || serve(server, tx))
                    .expect("Thread starten");
            }
            Err(e) => {
                let msg = format!("Port {} belegt ({e}) – läuft CineForge schon?", self.port);
                warn!("{msg}");
                status.error = Some(msg);
            }
        }
        app.insert_resource(status)
            .insert_resource(Inbox(Mutex::new(rx)))
            .add_plugins(tools::ToolsPlugin)
            .add_systems(Update, process_calls);
    }
}

fn process_calls(
    inbox: Res<Inbox>,
    mut status: ResMut<ControlStatus>,
    mut src: ResMut<SceneSource>,
    mut out: MessageWriter<SceneLoaded>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    user_view: UserViewQuery,
) {
    let Ok(rx) = inbox.0.lock() else { return };
    // Mehrere Aufrufe pro Frame abarbeiten, aber nicht unbegrenzt.
    for _ in 0..16 {
        let Ok(call) = rx.try_recv() else { break };
        status.calls += 1;
        let target = call.args.get("id").and_then(Value::as_str).map(|id| format!(" „{id}“")).unwrap_or_default();
        status.last_action = Some(format!("{}{}", call.name, target));
        info!("Claude: {}{}", call.name, target);
        tools::execute(call, &mut src, &mut out, &mut commands, &mut images, user_view.single().ok().copied());
    }
}

// ---------------------------------------------------------------------------
// HTTP / JSON-RPC (läuft im eigenen Thread)
// ---------------------------------------------------------------------------

fn serve(server: tiny_http::Server, tx: Sender<ToolCall>) {
    for mut req in server.incoming_requests() {
        let respond = |req: tiny_http::Request, status: u16, body: String| {
            let resp = tiny_http::Response::from_string(body).with_status_code(status).with_header(
                "Content-Type: application/json".parse::<tiny_http::Header>().expect("gültiger Header"),
            );
            let _ = req.respond(resp);
        };

        if let Some(origin) = req.headers().iter().find(|h| h.field.equiv("Origin")) {
            let o = origin.value.as_str();
            let local = ["http://localhost", "http://127.0.0.1", "https://localhost", "https://127.0.0.1"]
                .iter()
                .any(|p| o.starts_with(p));
            if !local {
                respond(req, 403, json!({"error": "Fremde Herkunft abgelehnt"}).to_string());
                continue;
            }
        }

        match (req.method(), req.url()) {
            (tiny_http::Method::Get, "/health") => {
                respond(req, 200, json!({"ok": true, "app": "cineforge"}).to_string());
            }
            (tiny_http::Method::Post, "/mcp") => {
                let mut body = String::new();
                if req.as_reader().read_to_string(&mut body).is_err() {
                    respond(req, 400, rpc_error(Value::Null, -32700, "Body nicht lesbar").to_string());
                    continue;
                }
                match handle_rpc(&body, &tx) {
                    Some(resp) => respond(req, 200, resp.to_string()),
                    None => {
                        let _ = req.respond(tiny_http::Response::empty(202));
                    }
                }
            }
            (_, "/mcp") => respond(req, 405, json!({"error": "Nur POST"}).to_string()),
            _ => respond(req, 404, json!({"error": "Nicht gefunden"}).to_string()),
        }
    }
}

fn rpc_error(id: Value, code: i64, msg: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": msg}})
}

fn rpc_ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Bearbeitet eine JSON-RPC-Nachricht (oder einen Batch). `None` = keine Antwort (Notification).
pub fn handle_rpc(body: &str, tx: &Sender<ToolCall>) -> Option<Value> {
    let msg: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Some(rpc_error(Value::Null, -32700, &format!("Kein gültiges JSON: {e}"))),
    };
    if let Value::Array(batch) = msg {
        let replies: Vec<Value> = batch.iter().filter_map(|m| handle_one(m, tx)).collect();
        return if replies.is_empty() { None } else { Some(Value::Array(replies)) };
    }
    handle_one(&msg, tx)
}

fn handle_one(msg: &Value, tx: &Sender<ToolCall>) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let Some(id) = id else {
        // Notification (z. B. notifications/initialized) – keine Antwort.
        return None;
    };
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    Some(match method {
        "initialize" => {
            let requested = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
            let version = if PROTOCOL_VERSIONS.contains(&requested) { requested } else { PROTOCOL_VERSIONS[0] };
            rpc_ok(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "cineforge", "title": "CineForge Studio", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": tools::INSTRUCTIONS
                }),
            )
        }
        "ping" => rpc_ok(id, json!({})),
        "tools/list" => rpc_ok(id, json!({"tools": tools::definitions()})),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let (reply_tx, reply_rx) = mpsc::channel();
            if tx.send(ToolCall { name, args, reply: reply_tx }).is_err() {
                return Some(rpc_error(id, -32603, "App wird gerade beendet"));
            }
            let output = reply_rx
                .recv_timeout(Duration::from_secs(120))
                .unwrap_or_else(|_| ToolOutput::error("Zeitüberschreitung: Die App hat nicht rechtzeitig geantwortet (ist das Fenster minimiert?)"));
            rpc_ok(id, output.to_mcp())
        }
        "resources/list" => rpc_ok(id, json!({"resources": []})),
        "prompts/list" => rpc_ok(id, json!({"prompts": []})),
        other => rpc_error(id, -32601, &format!("Unbekannte Methode: {other}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_and_list_tools() {
        let (tx, _rx) = mpsc::channel();
        let r = handle_rpc(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#, &tx).unwrap();
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
        assert!(handle_rpc(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#, &tx).is_none());
        let r = handle_rpc(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#, &tx).unwrap();
        let names: Vec<_> = r["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"screenshot") && names.contains(&"update_object"));
        let r = handle_rpc(r#"{"jsonrpc":"2.0","id":3,"method":"nope"}"#, &tx).unwrap();
        assert_eq!(r["error"]["code"], -32601);
        let r = handle_rpc("{kaputt", &tx).unwrap();
        assert_eq!(r["error"]["code"], -32700);
    }

    #[test]
    fn tool_call_is_forwarded_and_answered() {
        let (tx, rx) = mpsc::channel::<ToolCall>();
        std::thread::spawn(move || {
            let call = rx.recv().unwrap();
            assert_eq!(call.name, "list_objects");
            call.reply.send(ToolOutput::text("ok")).unwrap();
        });
        let r = handle_rpc(r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"list_objects","arguments":{}}}"#, &tx).unwrap();
        assert_eq!(r["id"], 7);
        assert_eq!(r["result"]["content"][0]["text"], "ok");
        assert_eq!(r["result"]["isError"], false);
    }
}
