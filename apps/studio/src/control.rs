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
    project::ActiveProject,
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

/// Sender für Befehle aus der Oberfläche (gleiche Werkzeuge wie Claude, Logbuch: „mensch“).
#[derive(Resource)]
pub struct UiCommands(Mutex<Sender<ToolCall>>);

impl UiCommands {
    pub fn send(&self, name: &str, mut args: Value) {
        args["agent"] = Value::String("mensch".into());
        let (reply, _) = mpsc::channel();
        if let Ok(tx) = self.0.lock() {
            let _ = tx.send(ToolCall { name: name.into(), args, reply });
        }
    }
}

pub struct ControlPlugin {
    pub port: u16,
}

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = mpsc::channel::<ToolCall>();
        // Die Oberfläche schickt ihre Befehle (z. B. Export-Knopf) auf demselben Weg wie Claude.
        app.insert_resource(UiCommands(Mutex::new(tx.clone())));
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
        app.init_resource::<crate::studio_ui::Selection>()
            .insert_resource(status)
            .insert_resource(Inbox(Mutex::new(rx)))
            .add_systems(Update, process_calls);
    }
}

fn process_calls(
    inbox: Res<Inbox>,
    mut status: ResMut<ControlStatus>,
    mut src: ResMut<SceneSource>,
    mut project: ResMut<ActiveProject>,
    mut library: ResMut<crate::library::Library>,
    mut jobs: ResMut<crate::render::RenderQueue>,
    mut out: MessageWriter<SceneLoaded>,
    user_view: UserViewQuery,
    mut selection: ResMut<crate::studio_ui::Selection>,
) {
    let Ok(rx) = inbox.0.lock() else { return };
    // Mehrere Aufrufe pro Frame abarbeiten, aber nicht unbegrenzt.
    for _ in 0..16 {
        let Ok(call) = rx.try_recv() else { break };
        status.calls += 1;
        let target = call.args.get("id").and_then(Value::as_str).map(|id| format!(" „{id}“")).unwrap_or_default();
        status.last_action = Some(format!("{}{}", call.name, target));
        info!("Claude: {}{}", call.name, target);
        let mut cx = tools::Ctx {
            src: &mut src,
            project: &mut project,
            library: &mut library,
            jobs: &mut jobs,
            out: &mut out,
            user_view: user_view.single().ok().copied(),
            selection: &mut selection,
        };
        tools::execute(call, &mut cx);
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
                    "capabilities": {"tools": {}, "prompts": {}},
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
            // Film-Übersicht öffnet nacheinander alle Szenen: braucht mehr Zeit
            let wait = if name == "contact_sheet" && args.get("film").and_then(Value::as_bool).unwrap_or(false) { 590 } else { 120 };
            if tx.send(ToolCall { name, args, reply: reply_tx }).is_err() {
                return Some(rpc_error(id, -32603, "App wird gerade beendet"));
            }
            let output = reply_rx
                .recv_timeout(Duration::from_secs(wait))
                .unwrap_or_else(|_| ToolOutput::error("Zeitüberschreitung: Die App hat nicht rechtzeitig geantwortet (ist das Fenster minimiert?)"));
            rpc_ok(id, output.to_mcp())
        }
        "resources/list" => rpc_ok(id, json!({"resources": []})),
        "prompts/list" => rpc_ok(id, json!({"prompts": prompts_list()})),
        "prompts/get" => match prompt_get(&params) {
            Ok(v) => rpc_ok(id, v),
            Err(e) => rpc_error(id, -32602, &e),
        },
        other => rpc_error(id, -32601, &format!("Unbekannte Methode: {other}")),
    })
}

// ---------------------------------------------------------------------------
// Vorlagen (in Claude Desktop unter „+“ → cineforge auswählbar)
// ---------------------------------------------------------------------------

fn prompts_list() -> Value {
    json!([
        {
            "name": "weitermachen",
            "title": "Weitermachen",
            "description": "Setzt die Arbeit am aktuellen CineForge-Projekt genau dort fort, wo zuletzt aufgehört wurde."
        },
        {
            "name": "mitarbeiten",
            "title": "Als weiterer Agent mitarbeiten",
            "description": "Für ein zusätzliches Claude-Fenster: übernimmt freie Aufgaben des Projekts und baut parallel, ohne die anderen zu stören.",
            "arguments": [{"name": "name", "description": "Name dieses Agenten, z. B. agent-2", "required": true}]
        },
        {
            "name": "neuer_film",
            "title": "Neuer Film",
            "description": "Startet ein neues Film-Projekt aus einer Idee: Logbuch, Storyboard, Aufgaben, dann Szene für Szene bauen.",
            "arguments": [{"name": "idee", "description": "Worum geht es im Film? (Handlung, Länge, Stimmung)", "required": true}]
        }
    ])
}

fn prompt_get(params: &Value) -> Result<Value, String> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let text = match name {
        "weitermachen" => "Mach in CineForge weiter, wo du zuletzt aufgehört hast: Rufe zuerst `resume` auf,              lies den Stand (nächster Schritt, offene Aufgaben, letzte Aktionen) und arbeite dann selbstständig weiter.              Halte dabei Aufgaben und next_step aktuell, damit es nach einer Unterbrechung wieder nahtlos weitergeht."
            .to_string(),
        "mitarbeiten" => {
            let name = params.pointer("/arguments/name").and_then(Value::as_str).filter(|s| !s.trim().is_empty()).unwrap_or("agent-2");
            format!(
                "Du arbeitest als Agent „{name}“ zusammen mit anderen Claude-Agenten im selben CineForge-Projekt. Regeln: \
                 1) resume lesen. 2) claim_task mit agent=\"{name}\" – nur an übernommenen Aufgaben arbeiten. \
                 3) Szenen immer mit dem Parameter scene bearbeiten (add_objects, update_object, batch …) und agent=\"{name}\" mitschicken – \
                 so bleibt die Ansicht der anderen unberührt. 4) screenshot mit scene prüfen. \
                 5) Aufgabe mit update_tasks auf done setzen, dann die nächste mit claim_task holen, bis keine mehr frei ist."
            )
        }
        "neuer_film" => {
            let idee = params
                .pointer("/arguments/idee")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .ok_or("Argument \"idee\" fehlt")?;
            format!(
                "Erstelle in CineForge einen Film zu dieser Idee:\n\n{idee}\n\n\
                 Arbeite wie ein Regie-Team (Pixar-Qualität), Schritt für Schritt:\n\
                 1) get_guide lesen (besonders „Figuren“, „Kino-Qualität“, „Schnellstart mit Vorlagen“).\n\
                 2) Geschichte: kurze Handlung mit Anfang, Problem, Höhepunkt, Ende; Figuren mit Namen, Aussehen, Charakter.\n\
                 3) new_project mit Ziel und Stil (Farben, Stimmung, Look film oder cartoon).\n\
                 4) update_plan: Storyboard – je Shot Titel, Beschreibung, Dauer (3–8 s), transition; mit template (wiese, dorf, wald, strand, nacht, studio, \
                 weltraum, zimmer, cartoon_2d, schloss, unterwasser, stadt, bauernhof, winter) bzw. copy_scene (gleicher Drehort) entstehen alle Szenen in EINEM Schritt; \
                 update_tasks mit Aufgaben.\n\
                 5) Je Shot die Szene öffnen (open_scene) und ausbauen.\n\
                 6) Figuren erst mit cast in die Besetzung (Aussehen einmal festlegen), dann in jeder Szene {{\"id\": \"max\", \"cast\": \"max\", \"position\": …}} – \
                 so bleiben sie in allen Shots gleich; Tiere ebenso (type \"animal\", auch Kuh, Schwein, Schaf, Pferd, Huhn); \
                 Handlung mit actions (walk, wave, jump, clap, bow, laugh, scared, shrug, sad, excited …), Wege mit animate.path, Mimik mit expressions \
                 (auch laugh, excited, sleepy, wink), look_at zum Gesprächspartner.\n\
                 7) Dialoge mit dialogue (Stimmen, Lippen-Sync, Mimik je Zeile mit expression, Untertitel; camera_cuts: true für Zweier-Einstellung + Schuss/Gegenschuss).\n\
                 8) Kamera wie im Film: camera.frame (wide zum Einführen, medium/close für Gefühle), Kamerafahrten mit frame.to (Heranfahren, Bogen, Kran, Push-in), \
                 follow für Bewegung, cuts innerhalb der Szene, dof mit focus auf die Hauptfigur; Zeitlupe für große Momente (speed 0.5 im Storyboard).\n\
                 9) Licht und Look: Tageszeit passend zur Stimmung, light_rays/fog für Atmosphäre, grade (saturation 1.1, vignette 0.3).\n\
                 10) Jede Szene mit screenshot oder contact_sheet prüfen und verbessern (Komposition, Größenverhältnisse, Lesbarkeit). \
                 Danach contact_sheet mit film: true (ganzer Film in einem Bild) und check_film (rechnerische Bildkontrolle) – Hinweise beheben.\n\
                 11) Ton: je Szene foley (Geräusche zur Handlung + Umgebung wie Vögel, Meer, Grillen); Musik mit create_music mood (happy, sad, adventure, tense, magical, \
                 calm, epic, funny, night) und film: true – passt sich der Filmlänge an und wird als Soundtrack eingetragen.\n\
                 12) Titel mit overlays (effect pop/typewriter, shadow), Abspann als letzter Shot mit effect scroll.\n\
                 13) check_film, dann export_film (zum Prüfen quality draft, für das fertige Video quality best). Shots und Aufgaben abhaken.\n\
                 Setze vor jedem größeren Schritt next_step, damit nach einer Unterbrechung nahtlos weitergearbeitet werden kann."
            )
        }
        other => return Err(format!("Unbekannte Vorlage: {other}")),
    };
    Ok(json!({"messages": [{"role": "user", "content": {"type": "text", "text": text}}]}))
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
    fn prompts_are_listed_and_filled() {
        let (tx, _rx) = mpsc::channel();
        let r = handle_rpc(r#"{"jsonrpc":"2.0","id":1,"method":"prompts/list"}"#, &tx).unwrap();
        assert_eq!(r["result"]["prompts"][0]["name"], "weitermachen");
        let r = handle_rpc(r#"{"jsonrpc":"2.0","id":2,"method":"prompts/get","params":{"name":"neuer_film","arguments":{"idee":"Raumstation"}}}"#, &tx).unwrap();
        assert!(r["result"]["messages"][0]["content"]["text"].as_str().unwrap().contains("Raumstation"));
        let r = handle_rpc(r#"{"jsonrpc":"2.0","id":3,"method":"prompts/get","params":{"name":"neuer_film"}}"#, &tx).unwrap();
        assert!(r["error"].is_object());
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
