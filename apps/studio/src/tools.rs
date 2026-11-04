//! Die Werkzeuge, mit denen Claude die App bedient.
//!
//! Definitionen (Name, Beschreibung, Parameter-Schema) sind reine Daten und
//! werden vom Server-Thread ausgeliefert. Ausgeführt wird in der Bevy-Welt,
//! weil dort Szene, Kamera und Renderer leben.

use std::sync::mpsc::Sender;

use base64::Engine as _;
use bevy::{
    camera::RenderTarget,
    prelude::*,
    render::{
        render_resource::TextureFormat,
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
};
use cf_scene::{ops, Scene};
use serde_json::{json, Value};

use crate::{
    environment::{configure_view, scene_camera_transform, SceneView},
    source::{scenes_dir, SceneLoaded, SceneSource},
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
CineForge Studio ist eine 3D-Video-Engine. Du baust Szenen als JSON-Dokument; die App rendert sie live.
Arbeitsweise:
1. Beim ersten Mal get_guide lesen (Szenen-Format mit allen Objekt-Typen und Feldern).
2. list_objects / get_scene zeigen den aktuellen Stand.
3. Ändern mit add_objects, update_object, remove_objects, update_scene_settings – oder set_scene für komplett neue Szenen.
4. Nach jeder sichtbaren Änderung screenshot aufrufen und das Bild kritisch prüfen (Komposition, Licht, Größenverhältnisse, Überschneidungen). Dann verbessern.
5. Fehler? undo stellt die vorherige Fassung wieder her.
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
            "description": "Rendert ein Bild der Szene und gibt es zurück, damit du das Ergebnis prüfen kannst. Standard: aus Sicht der Szenen-Kamera, 1280×720. view=\"user_view\" zeigt, was der Mensch gerade im Fenster sieht. Mit position/look_at kannst du von einem beliebigen Punkt aus schauen (z. B. Nahaufnahme eines Objekts).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "width": {"type": "integer", "minimum": 64, "maximum": 3840},
                    "height": {"type": "integer", "minimum": 64, "maximum": 2160},
                    "view": {"type": "string", "enum": ["scene_camera", "user_view"]},
                    "position": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                    "look_at": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                    "fov": {"type": "number"}
                }
            }
        },
        {
            "name": "undo",
            "description": "Macht die letzte Änderung rückgängig (auch Änderungen des Menschen).",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "list_scenes",
            "description": "Listet die Szenen-Dateien im Ordner Dokumente/CineForge/Szenen.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "open_scene",
            "description": "Öffnet eine Szene aus dem Szenen-Ordner (Name ohne .json) in der App.",
            "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}
        },
        {
            "name": "new_scene",
            "description": "Legt eine neue Szenen-Datei im Szenen-Ordner an und öffnet sie. Optional direkt mit Inhalt (scene).",
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

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, capture_tick);
    }
}

/// Führt einen Aufruf aus. Liefert `None`, wenn die Antwort später kommt (Screenshot).
#[allow(clippy::too_many_arguments)]
pub fn execute(
    call: ToolCall,
    src: &mut SceneSource,
    out: &mut MessageWriter<SceneLoaded>,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    user_view: Option<Transform>,
) {
    let a = &call.args;
    let str_arg = |k: &str| a.get(k).and_then(Value::as_str);

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
            Some(list) => ops::upsert_objects(&src.scene, list, str_arg("parent")),
            None => Err("objects muss eine Liste sein".into()),
        }),
        "update_object" => Some(match str_arg("id") {
            Some(id) => ops::update_object(&src.scene, id, a.get("changes").unwrap_or(&Value::Null)),
            None => Err("id fehlt".into()),
        }),
        "remove_objects" => Some(match a.get("ids").and_then(Value::as_array) {
            Some(ids) => {
                let ids: Vec<String> = ids.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                ops::remove_objects(&src.scene, &ids)
            }
            None => Err("ids muss eine Liste sein".into()),
        }),
        "update_scene_settings" => {
            Some(ops::update_settings(&src.scene, a.get("changes").unwrap_or(&Value::Null)))
        }
        _ => None,
    };

    if let Some(result) = mutation {
        let reply = match result.and_then(|scene| src.commit(scene, out)) {
            Ok(()) => ToolOutput::text(format!(
                "✓ Übernommen und gespeichert. Die App zeigt die Änderung jetzt an.\n\
                 Tipp: Mit screenshot das Ergebnis prüfen.\n\n{}",
                ops::outline(&src.scene)
            )),
            Err(e) => ToolOutput::error(format!("Nicht übernommen – die Szene bleibt unverändert.\n{e}")),
        };
        let _ = call.reply.send(reply);
        return;
    }

    let reply = match call.name.as_str() {
        "get_guide" => ToolOutput::text(format!("{INSTRUCTIONS}\n\n{GUIDE}")),
        "get_scene" => ToolOutput::text(src.scene.to_json_pretty()),
        "list_objects" => ToolOutput::text(ops::outline(&src.scene)),
        "undo" => match src.undo(out) {
            Ok(true) => ToolOutput::text(format!("↶ Rückgängig gemacht.\n\n{}", ops::outline(&src.scene))),
            Ok(false) => ToolOutput::text("Es gibt nichts mehr rückgängig zu machen."),
            Err(e) => ToolOutput::error(e),
        },
        "list_scenes" => list_scenes(&src.path),
        "open_scene" => match str_arg("name").map(scene_path) {
            Some(Ok(path)) => match src.open_file(path, out) {
                Ok(()) => ToolOutput::text(format!("Geöffnet.\n\n{}", ops::outline(&src.scene))),
                Err(e) => ToolOutput::error(e),
            },
            Some(Err(e)) => ToolOutput::error(e),
            None => ToolOutput::error("name fehlt"),
        },
        "new_scene" => new_scene(a, src, out),
        "screenshot" => {
            start_capture(call, src, commands, images, user_view);
            return;
        }
        other => ToolOutput::error(format!("Unbekanntes Werkzeug: {other}")),
    };
    let _ = call.reply.send(reply);
}

/// Erlaubt nur einfache Dateinamen im Szenen-Ordner (kein Zugriff auf andere Ordner).
fn scene_path(name: &str) -> Result<std::path::PathBuf, String> {
    let name = name.trim().trim_end_matches(".json");
    let valid = !name.is_empty()
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '.'))
        && !name.starts_with('.');
    if !valid {
        return Err(format!("Ungültiger Szenen-Name \"{name}\" (erlaubt: Buchstaben, Zahlen, - _ Leerzeichen)"));
    }
    Ok(scenes_dir()?.join(format!("{name}.json")))
}

fn list_scenes(current: &std::path::Path) -> ToolOutput {
    let dir = match scenes_dir() {
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
                    Some(if p == current { format!("{stem}  ← geöffnet") } else { stem })
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    ToolOutput::text(format!("Szenen in {}:\n{}", dir.display(), names.join("\n")))
}

fn new_scene(a: &Value, src: &mut SceneSource, out: &mut MessageWriter<SceneLoaded>) -> ToolOutput {
    let path = match a.get("name").and_then(Value::as_str).map(scene_path) {
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
    match src.open_file(path.clone(), out) {
        Ok(()) => ToolOutput::text(format!("Neue Szene angelegt und geöffnet: {}", path.display())),
        Err(e) => ToolOutput::error(e),
    }
}

// ---------------------------------------------------------------------------
// Screenshots über eine unsichtbare Kamera
// ---------------------------------------------------------------------------

/// Unsichtbare Kamera, die in ein Bild rendert. Das Fenster des Menschen
/// bleibt dabei unverändert. (Gleiche Technik wird später für den Export genutzt.)
#[derive(Component)]
struct CaptureJob {
    reply: Sender<ToolOutput>,
    frames_left: u32,
    image: Handle<Image>,
    requested: bool,
}

fn capture_frames() -> u32 {
    std::env::var("CINEFORGE_CAPTURE_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(8)
}

fn vec3_arg(a: &Value, k: &str) -> Option<Vec3> {
    let arr = a.get(k)?.as_array()?;
    if arr.len() != 3 {
        return None;
    }
    Some(Vec3::new(arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32))
}

fn start_capture(
    call: ToolCall,
    src: &SceneSource,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    user_view: Option<Transform>,
) {
    let a = &call.args;
    let dim = |k: &str, d: u32, max: u32| a.get(k).and_then(Value::as_u64).map(|v| v as u32).unwrap_or(d).clamp(64, max);
    let (w, h) = (dim("width", 1280, 3840), dim("height", 720, 2160));

    let mut transform = match a.get("view").and_then(Value::as_str) {
        Some("user_view") => user_view.unwrap_or_else(|| scene_camera_transform(&src.scene.camera)),
        _ => scene_camera_transform(&src.scene.camera),
    };
    if let Some(p) = vec3_arg(a, "position") {
        transform.translation = p;
    }
    if let Some(t) = vec3_arg(a, "look_at") {
        if (t - transform.translation).length_squared() > 1e-6 {
            transform.look_at(t, Vec3::Y);
        }
    }

    let image = images.add(Image::new_target_texture(w, h, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb)));
    let mut cam = commands.spawn((
        Name::new("Aufnahme-Kamera"),
        SceneView,
        Camera3d::default(),
        Camera { order: -1, ..default() },
        RenderTarget::Image(image.clone().into()),
        Msaa::Sample4,
        transform,
        CaptureJob { reply: call.reply, frames_left: capture_frames(), image, requested: false },
    ));
    configure_view(&mut cam, &src.scene);
    if let Some(fov) = a.get("fov").and_then(Value::as_f64) {
        cam.insert(Projection::Perspective(PerspectiveProjection {
            fov: (fov as f32).clamp(5.0, 150.0).to_radians(),
            near: 0.1,
            far: 20_000.0,
            ..default()
        }));
    }
}

fn capture_tick(mut commands: Commands, mut jobs: Query<(Entity, &mut CaptureJob)>) {
    for (entity, mut job) in &mut jobs {
        if job.requested {
            continue;
        }
        if job.frames_left > 0 {
            job.frames_left -= 1;
            continue;
        }
        job.requested = true;
        let reply = job.reply.clone();
        commands.spawn(Screenshot::image(job.image.clone())).observe(
            move |captured: On<ScreenshotCaptured>, mut commands: Commands, mut images: ResMut<Assets<Image>>, jobs: Query<&CaptureJob>| {
                let out = match encode_jpeg(&captured.image) {
                    Ok((b64, w, h)) => ToolOutput {
                        content: vec![
                            Content::Image { base64: b64, mime: "image/jpeg" },
                            Content::Text(format!("Aufnahme {w}×{h}")),
                        ],
                        is_error: false,
                    },
                    Err(e) => ToolOutput::error(format!("Bild konnte nicht erstellt werden: {e}")),
                };
                let _ = reply.send(out);
                if let Ok(job) = jobs.get(entity) {
                    images.remove(&job.image);
                }
                commands.entity(entity).despawn();
            },
        );
    }
}

pub fn encode_jpeg(img: &Image) -> Result<(String, u32, u32), String> {
    let dynamic = img.clone().try_into_dynamic().map_err(|e| e.to_string())?;
    let rgb = dynamic.to_rgb8();
    let (w, h) = rgb.dimensions();
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 88)
        .encode_image(&rgb)
        .map_err(|e| e.to_string())?;
    Ok((base64::engine::general_purpose::STANDARD.encode(bytes), w, h))
}

/// Hilfs-Query für die aktuelle Fenster-Kamera.
pub type UserViewQuery<'w, 's> = Query<'w, 's, &'static Transform, With<MainCamera>>;
