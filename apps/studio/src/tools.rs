//! Die Werkzeuge, mit denen Claude die App bedient.
//!
//! Definitionen (Name, Beschreibung, Parameter-Schema) sind reine Daten und
//! werden vom Server-Thread ausgeliefert. Ausgeführt wird in der Bevy-Welt,
//! weil dort Szene, Kamera und Renderer leben.

use std::sync::mpsc::Sender;

use bevy::prelude::*;
use cf_scene::{
    ops,
    project::{parse_status, safe_name},
    Scene,
};
use serde_json::{json, Value};

use crate::{
    project::{list_projects, ActiveProject},
    source::{SceneLoaded, SceneSource},
    MainCamera,
};

// ---------------------------------------------------------------------------
// Ergebnis-Typen
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Image { base64: String, mime: &'static str },
}

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub content: Vec<Content>,
    pub is_error: bool,
}

impl ToolOutput {
    pub fn text(t: impl Into<String>) -> Self {
        Self { content: vec![Content::Text(t.into())], is_error: false }
    }
    pub fn error(t: impl Into<String>) -> Self {
        Self { content: vec![Content::Text(t.into())], is_error: true }
    }
    pub fn to_mcp(&self) -> Value {
        let content: Vec<Value> = self
            .content
            .iter()
            .map(|c| match c {
                Content::Text(t) => json!({"type": "text", "text": t}),
                Content::Image { base64, mime } => json!({"type": "image", "data": base64, "mimeType": mime}),
            })
            .collect();
        json!({"content": content, "isError": self.is_error})
    }
}

/// Ein Werkzeug-Aufruf, der aus dem Server-Thread in die Bevy-Welt wandert.
pub struct ToolCall {
    pub name: String,
    pub args: Value,
    pub reply: Sender<ToolOutput>,
}

// ---------------------------------------------------------------------------
// Anleitung & Definitionen
// ---------------------------------------------------------------------------

pub const INSTRUCTIONS: &str = "\
CineForge Studio ist eine 3D-Film-Engine. Du baust Szenen als JSON-Dokument; die App rendert sie live.

WICHTIG – Fortsetzung nach Unterbrechungen:
- Beginne JEDE Unterhaltung mit `resume`. Dort steht der komplette Stand des Projekts (Ziel, Stil, Storyboard, Aufgaben, Notizen, letzte Aktionen, nächster Schritt).
- Halte das Logbuch aktuell, denn die Unterhaltung kann jederzeit abbrechen (Nutzungslimit): Aufgaben mit update_tasks anlegen und abhaken, Entscheidungen mit add_note festhalten, und bevor du einen längeren Arbeitsschritt beginnst, mit update_plan einen next_step setzen, der genau beschreibt, wo es weitergeht.
- Alle Szenen-Änderungen werden sofort gespeichert und automatisch protokolliert.

Arbeitsweise:
1. Beim ersten Mal get_guide lesen (Szenen-Format mit allen Objekt-Typen und Feldern).
2. Für einen Film: new_project (Ziel + Stil), dann update_plan mit Storyboard (Shots), dann Aufgaben anlegen und Shot für Shot bauen (je Shot eine Szene mit new_scene).
3. list_objects / get_scene zeigen den aktuellen Stand der geöffneten Szene.
4. Ändern mit add_objects, update_object, remove_objects, update_scene_settings – oder set_scene für komplett neue Szenen.
5. Nach jeder sichtbaren Änderung screenshot aufrufen und das Bild kritisch prüfen (Komposition, Licht, Größenverhältnisse, Überschneidungen). Dann verbessern.
6. Fehler? undo stellt die vorherige Fassung wieder her.
Einheiten: Meter und Grad, Y zeigt nach oben. Jedes Objekt braucht eine eindeutige, sprechende id.";

const GUIDE: &str = include_str!("../../../docs/SZENEN-FORMAT.md");

pub fn definitions() -> Value {
    let obj = json!({"type": "object"});
    json!([
        {
            "name": "get_guide",
            "description": "Liefert die vollständige Beschreibung des Szenen-Formats (alle Objekt-Typen, Felder, Standardwerte) und Tipps. Vor dem ersten Bauen lesen.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "get_scene",
            "description": "Liefert die aktuelle Szene als vollständiges JSON-Dokument.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "list_objects",
            "description": "Kurze Übersicht der Szene: Kamera, Sonne und alle Objekte als Baum mit id, Typ und Position.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "set_scene",
            "description": "Ersetzt die komplette Szene. Für neue Szenen oder große Umbauten. Für kleine Änderungen besser update_object.",
            "inputSchema": {"type": "object", "properties": {"scene": obj}, "required": ["scene"]}
        },
        {
            "name": "add_objects",
            "description": "Fügt Objekte hinzu (vollständige Objekt-Definitionen mit id und type). Existiert eine id schon, wird das Objekt ersetzt. Mit parent werden sie Kinder einer Gruppe.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "objects": {"type": "array", "items": obj},
                    "parent": {"type": "string", "description": "id einer Gruppe (optional)"}
                },
                "required": ["objects"]
            }
        },
        {
            "name": "update_object",
            "description": "Ändert einzelne Eigenschaften eines Objekts per Merge-Patch. Beispiel: {\"id\":\"tower\",\"changes\":{\"position\":[0,5,0],\"material\":{\"color\":\"#ff0000\"}}}. null entfernt ein Feld (zurück auf Standard).",
            "inputSchema": {
                "type": "object",
                "properties": {"id": {"type": "string"}, "changes": obj},
                "required": ["id", "changes"]
            }
        },
        {
            "name": "remove_objects",
            "description": "Entfernt Objekte (inkl. Kinder) anhand ihrer ids.",
            "inputSchema": {
                "type": "object",
                "properties": {"ids": {"type": "array", "items": {"type": "string"}}},
                "required": ["ids"]
            }
        },
        {
            "name": "update_scene_settings",
            "description": "Ändert name, description, environment (Himmel, Sonne, Nebel, Belichtung, Bloom) oder camera per Merge-Patch. Beispiel: {\"changes\":{\"environment\":{\"sun\":{\"elevation\":5}}}}",
            "inputSchema": {"type": "object", "properties": {"changes": obj}, "required": ["changes"]}
        },
        {
            "name": "screenshot",
            "description": "Rendert ein Bild der Szene und gibt es zurück, damit du das Ergebnis prüfen kannst. Standard: aus Sicht der Szenen-Kamera zur aktuellen Zeit, 1280×720. Mit time einen bestimmten Zeitpunkt der Animation ansehen. view=\"user_view\" zeigt, was der Mensch gerade im Fenster sieht. Mit position/look_at kannst du von einem beliebigen Punkt aus schauen (z. B. Nahaufnahme eines Objekts).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "width": {"type": "integer", "minimum": 64, "maximum": 3840},
                    "height": {"type": "integer", "minimum": 64, "maximum": 2160},
                    "view": {"type": "string", "enum": ["scene_camera", "user_view"]},
                    "position": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                    "look_at": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                    "fov": {"type": "number"},
                    "time": {"type": "number", "description": "Zeitpunkt in Sekunden (Standard: aktuelle Zeit der App)"}
                }
            }
        },
        {
            "name": "contact_sheet",
            "description": "Kontaktabzug: mehrere Zeitpunkte der Animation als Raster in EINEM Bild (aus Sicht der Szenen-Kamera). Ideal, um Bewegungen, Kamerafahrten und Timing zu prüfen. Standard: 6 gleichmäßig verteilte Zeitpunkte.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "times": {"type": "array", "items": {"type": "number"}, "description": "Zeitpunkte in Sekunden (max. 24)"},
                    "count": {"type": "integer", "minimum": 2, "maximum": 24},
                    "columns": {"type": "integer", "minimum": 1, "maximum": 6},
                    "width": {"type": "integer", "description": "Breite eines Einzelbilds (160–960, Standard 480)"}
                }
            }
        },
        {
            "name": "export_video",
            "description": "Rendert die Szene als MP4-Video (bis 4K) in den Export-Ordner des Projekts. Läuft im Hintergrund; Fortschritt mit export_status. Nutzt automatisch den Hardware-Encoder des Rechners.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Dateiname ohne Endung (Standard: Szenenname)"},
                    "width": {"type": "integer", "description": "Standard 1920 (4K: 3840)"},
                    "height": {"type": "integer", "description": "Standard 1080 (4K: 2160); für Hochformat z. B. 1080×1920"},
                    "fps": {"type": "integer"},
                    "start": {"type": "number"},
                    "end": {"type": "number"},
                    "quality": {"type": "string", "enum": ["draft", "standard", "high"]}
                }
            }
        },
        {
            "name": "export_film",
            "description": "Rendert den ganzen Film: alle Shots des Storyboards in Reihenfolge (jeder Shot braucht scene = Name einer Szenen-Datei; Dauer = duration des Shots oder der Szene) als ein MP4. transition: cut (Standard) oder fade (Abblende über Schwarz zwischen Shots).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "width": {"type": "integer"},
                    "height": {"type": "integer"},
                    "fps": {"type": "integer"},
                    "quality": {"type": "string", "enum": ["draft", "standard", "high"]},
                    "transition": {"type": "string", "enum": ["cut", "fade"]}
                }
            }
        },
        {
            "name": "export_status",
            "description": "Fortschritt bzw. Ergebnis des Video-Exports.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "cancel_export",
            "description": "Bricht den laufenden Video-Export ab.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "undo",
            "description": "Macht die letzte Änderung rückgängig (auch Änderungen des Menschen).",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "list_scenes",
            "description": "Listet die Szenen-Dateien des aktuellen Projekts.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "open_scene",
            "description": "Öffnet eine Szene des aktuellen Projekts (Name ohne .json) in der App.",
            "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}
        },
        {
            "name": "save_asset",
            "description": "Speichert ein Objekt (meist eine Gruppe aus vielen Teilen) in der Bibliothek, damit es in allen Szenen per {\"type\":\"instance\",\"asset\":NAME} wiederverwendet werden kann – z. B. Hauptfiguren, Fahrzeuge, Gebäude. Entweder object (Definition) oder from_id (Objekt aus der geöffneten Szene; mit replace_with_instance wird es dort gleich durch eine Instanz ersetzt). Ursprung des Objekts = Bezugspunkt (am besten unten in der Mitte). global: true speichert projektübergreifend.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "description": {"type": "string"},
                    "object": obj,
                    "from_id": {"type": "string"},
                    "replace_with_instance": {"type": "boolean"},
                    "global": {"type": "boolean"}
                },
                "required": ["name"]
            }
        },
        {
            "name": "list_assets",
            "description": "Listet die Objekte der Bibliothek (Projekt und global) mit Beschreibung und ungefährer Größe.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "get_asset",
            "description": "Liefert die Definition eines Bibliotheks-Objekts (zum Ansehen oder als Vorlage für Varianten).",
            "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}
        },
        {
            "name": "resume",
            "description": "Zeigt den kompletten Stand des aktuellen Projekts zum Weitermachen: Ziel, Stil, Storyboard mit Status, offene Aufgaben, Notizen, letzte Aktionen, nächster Schritt und die geöffnete Szene. Zu Beginn JEDER Unterhaltung aufrufen.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "update_plan",
            "description": "Ändert das Projekt-Logbuch: name, goal (Ziel/Idee), style (Stil-Vorgaben), next_step (Übergabe-Notiz: wo geht es weiter?) und storyboard. Storyboard-Einträge (Shots) werden per id zusammengeführt: {\"id\":\"s01\",\"title\":…,\"description\":…,\"scene\":\"s01\",\"duration\":6,\"status\":\"open|in_progress|done|blocked\"}; neue ids werden angehängt (oder mit position eingefügt), {\"id\":…,\"remove\":true} löscht.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "goal": {"type": "string"},
                    "style": {"type": "string"},
                    "next_step": {"type": "string"},
                    "storyboard": {"type": "array", "items": obj}
                }
            }
        },
        {
            "name": "update_tasks",
            "description": "Aufgabenliste pflegen. add: neue Aufgaben (Texte). update: [{\"id\":3,\"status\":\"done\",\"notes\":\"…\"}] mit status open|in_progress|done|blocked.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "add": {"type": "array", "items": {"type": "string"}},
                    "update": {"type": "array", "items": obj}
                }
            }
        },
        {
            "name": "add_note",
            "description": "Hält eine wichtige Entscheidung oder Erkenntnis im Projekt-Logbuch fest (z. B. Maßstab, Farbpalette, Namenskonventionen).",
            "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}
        },
        {
            "name": "list_projects",
            "description": "Listet alle Projekte.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "open_project",
            "description": "Öffnet ein Projekt (und dessen zuletzt bearbeitete Szene) und zeigt den Stand wie resume.",
            "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}
        },
        {
            "name": "new_project",
            "description": "Legt ein neues Film-Projekt an (Ordner mit Logbuch, Szenen, Objekten, Export) und öffnet es.",
            "inputSchema": {
                "type": "object",
                "properties": {"name": {"type": "string"}, "goal": {"type": "string"}, "style": {"type": "string"}},
                "required": ["name", "goal"]
            }
        },
        {
            "name": "new_scene",
            "description": "Legt eine neue Szenen-Datei im aktuellen Projekt an und öffnet sie. Optional direkt mit Inhalt (scene).",
            "inputSchema": {
                "type": "object",
                "properties": {"name": {"type": "string"}, "scene": obj},
                "required": ["name"]
            }
        }
    ])
}

// ---------------------------------------------------------------------------
// Ausführung
// ---------------------------------------------------------------------------

/// Alles, was ein Werkzeug zum Arbeiten braucht.
pub struct Ctx<'a, 'm> {
    pub src: &'a mut SceneSource,
    pub project: &'a mut ActiveProject,
    pub library: &'a mut crate::library::Library,
    pub jobs: &'a mut crate::render::RenderQueue,
    pub out: &'a mut MessageWriter<'m, SceneLoaded>,
    pub user_view: Option<Transform>,
}

/// Kurzbeschreibung einer Änderung fürs Protokoll.
fn describe(name: &str, a: &Value) -> String {
    let ids = |k: &str| {
        a.get(k)
            .and_then(Value::as_array)
            .map(|l| {
                l.iter()
                    .filter_map(|o| o.get("id").and_then(Value::as_str).or(o.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default()
    };
    match name {
        "add_objects" => format!("Objekte hinzugefügt: {}", ids("objects")),
        "update_object" => format!(
            "Objekt geändert: {} ({})",
            a.get("id").and_then(Value::as_str).unwrap_or("?"),
            a.get("changes").and_then(Value::as_object).map(|m| m.keys().cloned().collect::<Vec<_>>().join(", ")).unwrap_or_default()
        ),
        "remove_objects" => format!("Objekte entfernt: {}", ids("ids")),
        "update_scene_settings" => format!(
            "Szenen-Einstellungen: {}",
            a.get("changes").and_then(Value::as_object).map(|m| m.keys().cloned().collect::<Vec<_>>().join(", ")).unwrap_or_default()
        ),
        "set_scene" => "Szene komplett neu gesetzt".into(),
        other => other.into(),
    }
}

/// Führt einen Aufruf aus. Screenshots antworten später (siehe capture_tick).
pub fn execute(call: ToolCall, cx: &mut Ctx) {
    let a = &call.args;
    let str_arg = |k: &str| a.get(k).and_then(Value::as_str);
    let scene_file = cx.src.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

    // Während eines Exports darf sich die Szene nicht ändern.
    let is_mutation = matches!(
        call.name.as_str(),
        "set_scene" | "add_objects" | "update_object" | "remove_objects" | "update_scene_settings" | "undo" | "open_scene" | "new_scene" | "open_project" | "new_project"
    );
    if is_mutation && cx.jobs.export_running() {
        let _ = call.reply.send(ToolOutput::error(format!(
            "Gerade läuft ein Video-Export – Änderungen sind erst danach möglich.\n{}",
            cx.jobs.export_summary().unwrap_or_default()
        )));
        return;
    }

    let mutation: Option<Result<Scene, String>> = match call.name.as_str() {
        "set_scene" => Some(
            serde_json::from_value::<Scene>(a.get("scene").cloned().unwrap_or(Value::Null))
                .map_err(|e| format!("Ungültige Szene: {e}"))
                .and_then(|s| {
                    let p = s.validate();
                    if p.is_empty() { Ok(s) } else { Err(format!("Szene ist ungültig:\n- {}", p.join("\n- "))) }
                }),
        ),
        "add_objects" => Some(match a.get("objects").and_then(Value::as_array) {
            Some(list) => ops::upsert_objects(&cx.src.scene, list, str_arg("parent")),
            None => Err("objects muss eine Liste sein".into()),
        }),
        "update_object" => Some(match str_arg("id") {
            Some(id) => ops::update_object(&cx.src.scene, id, a.get("changes").unwrap_or(&Value::Null)),
            None => Err("id fehlt".into()),
        }),
        "remove_objects" => Some(match a.get("ids").and_then(Value::as_array) {
            Some(ids) => {
                let ids: Vec<String> = ids.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                ops::remove_objects(&cx.src.scene, &ids)
            }
            None => Err("ids muss eine Liste sein".into()),
        }),
        "update_scene_settings" => {
            Some(ops::update_settings(&cx.src.scene, a.get("changes").unwrap_or(&Value::Null)))
        }
        _ => None,
    };

    if let Some(result) = mutation {
        // Alle Bibliotheks-Verweise müssen auflösbar sein, bevor übernommen wird.
        let result = result.and_then(|scene| {
            cx.library.check(cx.project, &scene.objects)?;
            Ok(scene)
        });
        let reply = match result.and_then(|scene| cx.src.commit(scene, cx.out)) {
            Ok(()) => {
                cx.project.log("claude", format!("[{scene_file}] {}", describe(&call.name, a)));
                ToolOutput::text(format!(
                    "✓ Übernommen und gespeichert. Die App zeigt die Änderung jetzt an.\n\
                     Tipp: Mit screenshot das Ergebnis prüfen.\n\n{}",
                    ops::outline(&cx.src.scene)
                ))
            }
            Err(e) => ToolOutput::error(format!("Nicht übernommen – die Szene bleibt unverändert.\n{e}")),
        };
        let _ = call.reply.send(reply);
        return;
    }

    let reply = match call.name.as_str() {
        "get_guide" => ToolOutput::text(format!("{INSTRUCTIONS}\n\n{GUIDE}")),
        "get_scene" => ToolOutput::text(cx.src.scene.to_json_pretty()),
        "list_objects" => ToolOutput::text(ops::outline(&cx.src.scene)),
        "undo" => match cx.src.undo(cx.out) {
            Ok(true) => {
                cx.project.log("claude", format!("[{scene_file}] rückgängig"));
                ToolOutput::text(format!("↶ Rückgängig gemacht.\n\n{}", ops::outline(&cx.src.scene)))
            }
            Ok(false) => ToolOutput::text("Es gibt nichts mehr rückgängig zu machen."),
            Err(e) => ToolOutput::error(e),
        },
        "resume" => ToolOutput::text(resume_text(cx)),
        "update_plan" => update_plan(a, cx.project),
        "update_tasks" => update_tasks(a, cx.project),
        "add_note" => match str_arg("text").filter(|t| !t.trim().is_empty()) {
            Some(t) => match cx.project.require() {
                Ok(()) => {
                    cx.project.data.notes.push(cf_scene::project::Note { time: crate::project::now(), text: t.into() });
                    match cx.project.save() {
                        Ok(()) => ToolOutput::text("Notiz gespeichert."),
                        Err(e) => ToolOutput::error(e),
                    }
                }
                Err(e) => ToolOutput::error(e),
            },
            None => ToolOutput::error("text fehlt"),
        },
        "list_projects" => {
            let current = cx.project.name().map(str::to_string);
            let list: Vec<String> = list_projects()
                .into_iter()
                .map(|p| if Some(&p) == current.as_ref() { format!("{p}  ← geöffnet") } else { p })
                .collect();
            ToolOutput::text(if list.is_empty() { "Noch keine Projekte.".into() } else { list.join("\n") })
        }
        "open_project" => match str_arg("name").map(ActiveProject::open) {
            Some(Ok((p, scene))) => {
                *cx.project = p;
                match cx.src.open_file(scene, cx.out) {
                    Ok(()) => {
                        cx.project.log("claude", "Projekt geöffnet");
                        ToolOutput::text(resume_text(cx))
                    }
                    Err(e) => ToolOutput::error(e),
                }
            }
            Some(Err(e)) => ToolOutput::error(e),
            None => ToolOutput::error("name fehlt"),
        },
        "new_project" => match ActiveProject::create(
            str_arg("name").unwrap_or(""),
            str_arg("goal").unwrap_or(""),
            str_arg("style").unwrap_or(""),
        ) {
            Ok((p, scene)) => {
                *cx.project = p;
                match cx.src.open_file(scene, cx.out) {
                    Ok(()) => ToolOutput::text(format!(
                        "Projekt angelegt und geöffnet: {}\nNächste Schritte: update_plan mit Storyboard, update_tasks mit Aufgaben, dann Szenen bauen (new_scene).",
                        cx.project.dir.as_ref().map(|d| d.display().to_string()).unwrap_or_default()
                    )),
                    Err(e) => ToolOutput::error(e),
                }
            }
            Err(e) => ToolOutput::error(e),
        },
        "list_scenes" => list_scenes(cx),
        "open_scene" => match str_arg("name").map(|n| scene_path(cx.project, n)) {
            Some(Ok(path)) if path.is_file() => match cx.src.open_file(path, cx.out) {
                Ok(()) => {
                    cx.project.log("claude", format!("Szene geöffnet: {}", str_arg("name").unwrap_or("")));
                    ToolOutput::text(format!("Geöffnet.\n\n{}", ops::outline(&cx.src.scene)))
                }
                Err(e) => ToolOutput::error(e),
            },
            Some(Ok(path)) => ToolOutput::error(format!("{} gibt es nicht. list_scenes zeigt alle Szenen.", path.display())),
            Some(Err(e)) => ToolOutput::error(e),
            None => ToolOutput::error("name fehlt"),
        },
        "new_scene" => new_scene(a, cx),
        "save_asset" => save_asset(a, cx),
        "list_assets" => {
            let list = crate::library::Library::list(cx.project);
            if list.is_empty() {
                ToolOutput::text("Die Bibliothek ist leer. Objekte mit save_asset speichern.")
            } else {
                let lines: Vec<String> = list
                    .iter()
                    .map(|a| {
                        let size = a
                            .extent
                            .map(|(lo, hi)| format!(" · ca. {:.1}×{:.1}×{:.1} m", hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]))
                            .unwrap_or_default();
                        format!("- {} ({}){size}: {}", a.name, a.scope, a.description)
                    })
                    .collect();
                ToolOutput::text(format!(
                    "Bibliothek (Verwendung: {{\"id\":\"…\",\"type\":\"instance\",\"asset\":\"NAME\",\"position\":[…]}}):\n{}",
                    lines.join("\n")
                ))
            }
        }
        "get_asset" => match str_arg("name").map(|n| cx.library.load(cx.project, n)) {
            Some(Ok(asset)) => ToolOutput::text(asset.to_json_pretty()),
            Some(Err(e)) => ToolOutput::error(e),
            None => ToolOutput::error("name fehlt"),
        },
        "screenshot" | "contact_sheet" if cx.jobs.export_running() => ToolOutput::error(format!(
            "Gerade läuft ein Video-Export.\n{}",
            cx.jobs.export_summary().unwrap_or_default()
        )),
        "screenshot" => {
            let dim = |k: &str, d: u32, max: u32| a.get(k).and_then(Value::as_u64).map(|v| v as u32).unwrap_or(d).clamp(64, max);
            let view = crate::render::ViewSpec {
                user_view: (str_arg("view") == Some("user_view")).then_some(cx.user_view).flatten(),
                position: vec3_arg(a, "position"),
                look_at: vec3_arg(a, "look_at"),
                fov: a.get("fov").and_then(Value::as_f64).map(|v| v as f32),
            };
            cx.jobs.push(crate::render::JobRequest::Still {
                reply: call.reply,
                time: a.get("time").and_then(Value::as_f64).map(|t| (t as f32).clamp(0.0, cx.src.scene.duration)),
                view,
                width: dim("width", 1280, 3840),
                height: dim("height", 720, 2160),
            });
            return;
        }
        "contact_sheet" => {
            let dur = cx.src.scene.duration;
            let times: Vec<f32> = match a.get("times").and_then(Value::as_array) {
                Some(list) => list.iter().filter_map(Value::as_f64).map(|t| (t as f32).clamp(0.0, dur)).take(24).collect(),
                None => {
                    let n = a.get("count").and_then(Value::as_u64).unwrap_or(6).clamp(2, 24) as usize;
                    (0..n).map(|i| dur * i as f32 / (n - 1) as f32).collect()
                }
            };
            if times.is_empty() {
                ToolOutput::error("times ist leer")
            } else {
                let w = a.get("width").and_then(Value::as_u64).unwrap_or(480).clamp(160, 960) as u32;
                cx.jobs.push(crate::render::JobRequest::Sheet {
                    reply: call.reply,
                    columns: a.get("columns").and_then(Value::as_u64).unwrap_or(3).clamp(1, 6) as u32,
                    width: w,
                    height: (w as f32 * 9.0 / 16.0) as u32,
                    times,
                });
                return;
            }
        }
        "export_video" => export_video(a, cx),
        "export_film" => export_film(a, cx),
        "export_status" => ToolOutput::text(cx.jobs.export_summary().unwrap_or_else(|| "Es wurde noch kein Export gestartet.".into())),
        "cancel_export" => {
            if cx.jobs.export_running() {
                cx.jobs.cancel_export();
                ToolOutput::text("Export wird abgebrochen.")
            } else {
                ToolOutput::text("Es läuft kein Export.")
            }
        }
        other => ToolOutput::error(format!("Unbekanntes Werkzeug: {other}")),
    };
    let _ = call.reply.send(reply);
}

fn save_asset(a: &Value, cx: &mut Ctx) -> ToolOutput {
    let name = match a.get("name").and_then(Value::as_str).map(safe_name) {
        Some(Ok(n)) => n,
        Some(Err(e)) => return ToolOutput::error(e),
        None => return ToolOutput::error("name fehlt"),
    };
    let from_id = a.get("from_id").and_then(Value::as_str);
    let mut object: cf_scene::Object = match (a.get("object"), from_id) {
        (Some(v), _) => match serde_json::from_value(v.clone()) {
            Ok(o) => o,
            Err(e) => return ToolOutput::error(format!("Ungültiges Objekt: {e}")),
        },
        (None, Some(id)) => match cx.src.scene.find(id) {
            Some(o) => o.clone(),
            None => return ToolOutput::error(format!("Kein Objekt mit id \"{id}\" in der Szene")),
        },
        (None, None) => return ToolOutput::error("object oder from_id angeben"),
    };
    // Bezugspunkt: Ursprung des Objekts; Position/Drehung gehören zur Platzierung, nicht zum Objekt.
    let placed_at = (object.position, object.rotation);
    object.position = [0.0; 3];
    object.rotation = [0.0; 3];
    let asset = cf_scene::library::AssetFile {
        name: name.clone(),
        description: a.get("description").and_then(Value::as_str).unwrap_or("").to_string(),
        object,
    };
    let global = a.get("global").and_then(Value::as_bool).unwrap_or(false);
    let path = match cx.library.save(cx.project, &asset, global) {
        Ok(p) => p,
        Err(e) => return ToolOutput::error(e),
    };
    cx.project.log("claude", format!("Bibliothek: \"{name}\" gespeichert"));
    let mut msg = format!("✓ \"{name}\" in der Bibliothek gespeichert: {}", path.display());
    // Optional: Original in der Szene durch eine Instanz ersetzen.
    if let (Some(id), true) = (from_id, a.get("replace_with_instance").and_then(Value::as_bool).unwrap_or(false)) {
        let inst = json!({"id": id, "type": "instance", "asset": name, "position": placed_at.0, "rotation": placed_at.1});
        match ops::upsert_objects(&cx.src.scene, &[inst], None).and_then(|s| cx.src.commit(s, cx.out)) {
            Ok(()) => msg.push_str(&format!("\n„{id}“ in der Szene ist jetzt eine Instanz davon.")),
            Err(e) => msg.push_str(&format!("\nErsetzen in der Szene fehlgeschlagen: {e}")),
        }
    }
    msg.push_str(&format!(
        "\nVerwenden: {{\"id\":\"…\",\"type\":\"instance\",\"asset\":\"{name}\",\"position\":[x,y,z]}}"
    ));
    ToolOutput::text(msg)
}

fn vec3_arg(a: &Value, k: &str) -> Option<Vec3> {
    let arr = a.get(k)?.as_array()?;
    if arr.len() != 3 {
        return None;
    }
    Some(Vec3::new(arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32))
}

fn export_video(a: &Value, cx: &mut Ctx) -> ToolOutput {
    if cx.jobs.export_running() {
        return ToolOutput::error(format!("Es läuft schon ein Export.\n{}", cx.jobs.export_summary().unwrap_or_default()));
    }
    let quality = match crate::video::Quality::parse(a.get("quality").and_then(Value::as_str)) {
        Ok(q) => q,
        Err(e) => return ToolOutput::error(e),
    };
    // Gerade Maße (nötig für H.264), Standard Full HD.
    let even = |v: u64| ((v as u32).clamp(64, 3840) / 2) * 2;
    let width = even(a.get("width").and_then(Value::as_u64).unwrap_or(1920));
    let height = even(a.get("height").and_then(Value::as_u64).unwrap_or(1080).min(2160));
    let scene = &cx.src.scene;
    let fps = a.get("fps").and_then(Value::as_u64).map(|v| v as u32).unwrap_or(scene.fps).clamp(1, 120);
    let start = a.get("start").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(0.0).clamp(0.0, scene.duration);
    let end = a.get("end").and_then(Value::as_f64).map(|v| v as f32).unwrap_or(scene.duration).clamp(start, scene.duration);
    if end - start < 1.0 / fps as f32 {
        return ToolOutput::error("Der Zeitbereich ist zu kurz");
    }
    let base_name = match a.get("name").and_then(Value::as_str) {
        Some(n) => match safe_name(n) {
            Ok(n) => n,
            Err(e) => return ToolOutput::error(e),
        },
        None => cx.src.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "video".into()),
    };
    let dir = match cx.project.export_dir() {
        Ok(d) => d,
        Err(e) => return ToolOutput::error(e),
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return ToolOutput::error(format!("{}: {e}", dir.display()));
    }
    let mut output = dir.join(format!("{base_name}.mp4"));
    let mut n = 2;
    while output.exists() {
        output = dir.join(format!("{base_name}_{n}.mp4"));
        n += 1;
    }
    let frames = ((end - start) * fps as f32).round() as u32;
    let encoder = crate::video::find_ffmpeg()
        .map(|ff| crate::video::best_encoder(&ff))
        .unwrap_or_else(|| "keiner – ffmpeg nicht gefunden, es werden PNG-Bilder gespeichert".into());
    let segments = vec![crate::render::Segment { path: None, start, duration: end - start, fade_in: false, fade_out: false }];
    cx.jobs.push(crate::render::JobRequest::Export { output: output.clone(), width, height, fps, segments, quality });
    cx.project.log("claude", format!("Video-Export gestartet: {} ({width}×{height}, {fps} fps, {frames} Bilder)", output.display()));
    ToolOutput::text(format!(
        "Export gestartet: {frames} Bilder ({width}×{height}, {fps} fps, {:.1}–{:.1} s) → {}\nEncoder: {encoder}\nFortschritt mit export_status abfragen. Solange der Export läuft, ist die Szene gesperrt.",
        start,
        end,
        output.display()
    ))
}

fn export_film(a: &Value, cx: &mut Ctx) -> ToolOutput {
    if let Err(e) = cx.project.require() {
        return ToolOutput::error(e);
    }
    if cx.jobs.export_running() {
        return ToolOutput::error(format!("Es läuft schon ein Export.\n{}", cx.jobs.export_summary().unwrap_or_default()));
    }
    let shots = cx.project.data.storyboard.clone();
    if shots.is_empty() {
        return ToolOutput::error("Das Storyboard ist leer – erst mit update_plan Shots anlegen.");
    }
    let quality = match crate::video::Quality::parse(a.get("quality").and_then(Value::as_str)) {
        Ok(q) => q,
        Err(e) => return ToolOutput::error(e),
    };
    let fade_all = match a.get("transition").and_then(Value::as_str).unwrap_or("cut") {
        "cut" | "schnitt" => false,
        "fade" | "blende" => true,
        other => return ToolOutput::error(format!("Unbekannter Übergang \"{other}\" (cut oder fade)")),
    };
    let scenes_dir = match cx.project.scenes_dir() {
        Ok(d) => d,
        Err(e) => return ToolOutput::error(e),
    };
    let mut segments = Vec::new();
    let mut problems = Vec::new();
    let mut first_fps = None;
    for (i, shot) in shots.iter().enumerate() {
        let Some(name) = &shot.scene else {
            problems.push(format!("Shot {} „{}“ hat keine Szene (Feld scene im Storyboard)", shot.id, shot.title));
            continue;
        };
        let path = scenes_dir.join(format!("{name}.json"));
        let scene = match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| Scene::from_json(&t).map_err(|e| e.to_string())) {
            Ok(s) => s,
            Err(e) => {
                problems.push(format!("Shot {}: Szene \"{name}\" nicht ladbar ({e})", shot.id));
                continue;
            }
        };
        if let Err(e) = cx.library.check(cx.project, &scene.objects) {
            problems.push(format!("Shot {}: {e}", shot.id));
            continue;
        }
        first_fps.get_or_insert(scene.fps);
        let duration = if shot.duration > 0.0 { shot.duration } else { scene.duration };
        let last = i + 1 == shots.len();
        segments.push(crate::render::Segment {
            path: Some(path),
            start: 0.0,
            duration,
            fade_in: i == 0 || fade_all,
            fade_out: last || fade_all,
        });
    }
    if !problems.is_empty() {
        return ToolOutput::error(format!("Film kann noch nicht exportiert werden:\n- {}", problems.join("\n- ")));
    }
    let even = |v: u64| ((v as u32).clamp(64, 3840) / 2) * 2;
    let width = even(a.get("width").and_then(Value::as_u64).unwrap_or(1920));
    let height = even(a.get("height").and_then(Value::as_u64).unwrap_or(1080).min(2160));
    let fps = a.get("fps").and_then(Value::as_u64).map(|v| v as u32).unwrap_or(first_fps.unwrap_or(30)).clamp(1, 120);
    let base_name = match a.get("name").and_then(Value::as_str).map(safe_name).unwrap_or_else(|| safe_name(&cx.project.data.name)) {
        Ok(n) => n,
        Err(e) => return ToolOutput::error(e),
    };
    let dir = match cx.project.export_dir() {
        Ok(d) => d,
        Err(e) => return ToolOutput::error(e),
    };
    let _ = std::fs::create_dir_all(&dir);
    let mut output = dir.join(format!("{base_name}.mp4"));
    let mut n = 2;
    while output.exists() {
        output = dir.join(format!("{base_name}_{n}.mp4"));
        n += 1;
    }
    let total: f32 = segments.iter().map(|s| s.duration).sum();
    let count = segments.len();
    cx.jobs.push(crate::render::JobRequest::Export { output: output.clone(), width, height, fps, segments, quality });
    cx.project.log("claude", format!("Film-Export gestartet: {count} Shots, {total:.1} s → {}", output.display()));
    ToolOutput::text(format!(
        "Film-Export gestartet: {count} Shots, {total:.1} s, {width}×{height}, {fps} fps → {}\nÜbergänge: {}. Fortschritt mit export_status.",
        output.display(),
        if fade_all { "Abblenden über Schwarz" } else { "harte Schnitte (Ein-/Ausblende am Anfang und Ende)" }
    ))
}

fn resume_text(cx: &Ctx) -> String {
    let scene = format!("Datei: {}\n{}", cx.src.path.display(), ops::outline(&cx.src.scene));
    if cx.project.dir.is_none() {
        return format!(
            "Kein Projekt geöffnet (die Szene liegt außerhalb eines Projekts).\nProjekte: {}\n\n## Geöffnete Szene\n{scene}",
            list_projects().join(", ")
        );
    }
    format!(
        "{}\nSo geht es weiter: Arbeite am „Nächsten Schritt“ bzw. an der ersten offenen Aufgabe. Halte Aufgaben und next_step aktuell.",
        cx.project.data.digest(Some(&scene))
    )
}

fn update_plan(a: &Value, project: &mut ActiveProject) -> ToolOutput {
    if let Err(e) = project.require() {
        return ToolOutput::error(e);
    }
    let mut report = Vec::new();
    let mut data = project.data.clone();
    for (key, field) in [("name", &mut data.name), ("goal", &mut data.goal), ("style", &mut data.style), ("next_step", &mut data.next_step)] {
        if let Some(v) = a.get(key).and_then(Value::as_str) {
            *field = v.to_string();
            report.push(format!("{key} gesetzt"));
        }
    }
    if let Some(list) = a.get("storyboard").and_then(Value::as_array) {
        match data.update_storyboard(list) {
            Ok(r) => report.extend(r),
            Err(e) => return ToolOutput::error(format!("Nicht übernommen: {e}")),
        }
    }
    if report.is_empty() {
        return ToolOutput::error("Nichts zu ändern (name, goal, style, next_step oder storyboard angeben)");
    }
    project.data = data;
    project.log("claude", format!("Plan: {}", report.join("; ")));
    ToolOutput::text(format!("✓ Logbuch aktualisiert: {}", report.join("; ")))
}

fn update_tasks(a: &Value, project: &mut ActiveProject) -> ToolOutput {
    if let Err(e) = project.require() {
        return ToolOutput::error(e);
    }
    let now = crate::project::now();
    let mut data = project.data.clone();
    let mut report = Vec::new();
    for t in a.get("add").and_then(Value::as_array).into_iter().flatten() {
        if let Some(text) = t.as_str().filter(|x| !x.trim().is_empty()) {
            let id = data.add_task(text, &now);
            report.push(format!("#{id} angelegt"));
        }
    }
    for u in a.get("update").and_then(Value::as_array).into_iter().flatten() {
        let Some(id) = u.get("id").and_then(Value::as_u64) else {
            return ToolOutput::error("Jede Änderung braucht eine numerische id");
        };
        let status = match u.get("status").and_then(Value::as_str).map(parse_status) {
            Some(Ok(s)) => Some(s),
            Some(Err(e)) => return ToolOutput::error(e),
            None => None,
        };
        if let Err(e) = data.update_task(
            id as u32,
            status,
            u.get("text").and_then(Value::as_str),
            u.get("notes").and_then(Value::as_str),
            &now,
        ) {
            return ToolOutput::error(e);
        }
        report.push(format!("#{id} geändert"));
    }
    if report.is_empty() {
        return ToolOutput::error("Nichts zu ändern (add oder update angeben)");
    }
    project.data = data;
    project.log("claude", format!("Aufgaben: {}", report.join(", ")));
    let open: Vec<String> = project
        .data
        .tasks
        .iter()
        .filter(|t| t.status != cf_scene::project::Status::Done)
        .map(|t| format!("{} #{} {}", t.status.symbol(), t.id, t.text))
        .collect();
    ToolOutput::text(format!("✓ {}\nOffen:\n{}", report.join(", "), open.join("\n")))
}

/// Nur einfache Dateinamen im Szenen-Ordner des Projekts (kein Zugriff auf andere Ordner).
fn scene_path(project: &ActiveProject, name: &str) -> Result<std::path::PathBuf, String> {
    let name = safe_name(name)?;
    Ok(project.scenes_dir()?.join(format!("{name}.json")))
}

fn list_scenes(cx: &Ctx) -> ToolOutput {
    let dir = match cx.project.scenes_dir() {
        Ok(d) => d,
        Err(e) => return ToolOutput::error(e),
    };
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .filter_map(|p| {
                    let stem = p.file_stem()?.to_string_lossy().to_string();
                    Some(if p == cx.src.path { format!("{stem}  ← geöffnet") } else { stem })
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    ToolOutput::text(format!("Szenen in {}:\n{}", dir.display(), names.join("\n")))
}

fn new_scene(a: &Value, cx: &mut Ctx) -> ToolOutput {
    let path = match a.get("name").and_then(Value::as_str).map(|n| scene_path(cx.project, n)) {
        Some(Ok(p)) => p,
        Some(Err(e)) => return ToolOutput::error(e),
        None => return ToolOutput::error("name fehlt"),
    };
    if path.exists() {
        return ToolOutput::error(format!("{} existiert schon – open_scene benutzen oder anderen Namen wählen", path.display()));
    }
    let scene = match a.get("scene") {
        Some(v) => match serde_json::from_value::<Scene>(v.clone()) {
            Ok(s) if s.validate().is_empty() => s,
            Ok(s) => return ToolOutput::error(format!("Szene ist ungültig:\n- {}", s.validate().join("\n- "))),
            Err(e) => return ToolOutput::error(format!("Ungültige Szene: {e}")),
        },
        None => Scene::from_json(r#"{"name":"Neue Szene"}"#).expect("leere Szene ist gültig"),
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, scene.to_json_pretty()) {
        return ToolOutput::error(format!("Konnte {} nicht anlegen: {e}", path.display()));
    }
    match cx.src.open_file(path.clone(), cx.out) {
        Ok(()) => {
            cx.project.log("claude", format!("Neue Szene: {}", path.file_stem().unwrap_or_default().to_string_lossy()));
            ToolOutput::text(format!("Neue Szene angelegt und geöffnet: {}", path.display()))
        }
        Err(e) => ToolOutput::error(e),
    }
}

/// Hilfs-Query für die aktuelle Fenster-Kamera.
pub type UserViewQuery<'w, 's> = Query<'w, 's, &'static Transform, With<MainCamera>>;
