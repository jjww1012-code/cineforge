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
2. Für einen Film: new_project (Ziel + Stil), dann update_plan mit Storyboard (Shots), dann Aufgaben anlegen und Shot für Shot bauen (je Shot eine Szene mit new_scene). Shots können auch fertige Video-Clips sein (clip, Ordner Clips/). Ton: soundtrack + audio je Shot (Ordner Audio/, Sprecher mit generate_voice). Einblendungen über overlays in der Szene. Vor dem Export check_film, dann export_film (Shot-Cache: Änderungen rendern nur betroffene Shots neu, auch bei 1-Stunden-Filmen).
3. list_objects / get_scene zeigen den aktuellen Stand der geöffneten Szene.
4. Ändern mit add_objects, update_object, remove_objects, update_scene_settings – oder set_scene für komplett neue Szenen.
5. Nach jeder sichtbaren Änderung screenshot aufrufen und das Bild kritisch prüfen (Komposition, Licht, Größenverhältnisse, Überschneidungen). Dann verbessern.
6. Fehler? undo stellt die vorherige Fassung wieder her. Fehlermeldungen nennen die genaue Stelle und meist die Lösung („Meintest du …?“).
7. „Das hier“, „dieses Objekt“: get_selection sagt, was der Mensch im Fenster angeklickt hat. Mit select_object kannst du ihm umgekehrt ein Objekt zeigen.
8. Schneller arbeiten: batch für viele Änderungen auf einmal, duplicate_object und repeat für Wiederholungen, Bibliothek (save_asset/instance) für wiederkehrende Objekte.
Schnell schöne Szenen: new_scene mit template (wiese, dorf, wald, strand, nacht, studio, weltraum, zimmer, cartoon_2d). Kamera ohne Rechnen: camera.frame {target, shot: wide|full|medium|close|extreme_close, angle, tilt}.
Fertige Objekte: list_assets (haus, auto, laterne, bank, zaun, tisch, stuhl, wolke, brunnen, blume, busch, felsen) und per type \"instance\" einsetzen.
Natur: type \"tree\" (oak/pine/palm/birch/lollipop, wind), Wald = scatter mit tree-Shape, type \"water\" mit Wellen; Nacht: environment.stars + environment.moon.
Dialoge: Werkzeug dialogue (Stimmen + Lippen-Sync + Untertitel in einem Schritt).
Figuren: type \"character\" erzeugt eine fertige Pixar-artige Figur (Frisur, Farben, Ausdruck, actions wie walk/wave/jump/talk, talk = Lippen-Sync). Laufen: animate.path (Beine bewegen sich automatisch). Kamera: camera.follow / camera.target / camera.orbit statt Keyframes rechnen.
Cartoon-Figuren leben lassen: Mund mit animate.talk (Lippen-Sync zur Stimme aus generate_voice), Augen mit animate.blink, Körper mit animate.breathe.
Qualität: Für Kino-/Pixar-Look den Abschnitt „Kino-Qualität“ aus get_guide befolgen (Licht, dof, motion_blur, light_rays, grade, rounded-Formen, Bewegung mit out_back). Detaillierte 2D-Figuren als SVG mit Verläufen, weichen Schatten und Konturen; in 3D-Welten mit billboard, 3D-Objekte mit material.outline passend zum Cartoon-Stil.
Medien: Bilder (PNG/JPG/SVG) in Bilder/, 3D-Modelle (glTF/GLB) in Modelle/ (type \"model\"). 2D-Grafiken zeichnest du selbst als SVG (type \"svg\"), Geräusche mit create_sound, Musik mit create_music. Cartoons: 2D-Kamera (camera.orthographic) + SVG-Figuren, siehe get_guide.
Einheiten: Meter und Grad, Y zeigt nach oben. Jedes Objekt braucht eine eindeutige, sprechende id.";

const GUIDE: &str = include_str!("../../../docs/SZENEN-FORMAT.md");

pub fn definitions() -> Value {
    let obj = json!({"type": "object"});
    json!([
        {
            "name": "batch",
            "description": "Mehrere Änderungen in EINEM Schritt (schneller, alles oder nichts): operations = [{\"op\":\"add\",\"objects\":[…],\"parent\":…}, {\"op\":\"update\",\"id\":…,\"changes\":{…}}, {\"op\":\"remove\",\"ids\":[…]}, {\"op\":\"settings\",\"changes\":{…}}, {\"op\":\"duplicate\",\"id\":…,\"new_id\":…,\"count\":3,\"offset\":[2,0,0]}]. Optional scene: andere Szene des Projekts bearbeiten, ohne sie zu öffnen.",
            "inputSchema": {"type": "object", "properties": {"operations": {"type": "array", "items": obj}, "scene": {"type": "string"}, "agent": {"type": "string"}}, "required": ["operations"]}
        },
        {
            "name": "duplicate_object",
            "description": "Kopiert ein Objekt (inkl. Kinder) count-mal, jeweils um offset versetzt. Neue ids: new_id, new_id_2 … (Kinder bekommen dieselbe Endung).",
            "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "new_id": {"type": "string"}, "count": {"type": "integer"}, "offset": {"type": "array", "items": {"type": "number"}}, "scene": {"type": "string"}}, "required": ["id"]}
        },
        {
            "name": "claim_task",
            "description": "Für mehrere Agenten gleichzeitig: übernimmt die nächste freie Aufgabe (status open, ohne Besitzer) für agent und setzt sie auf in_progress. So arbeiten nie zwei Agenten an derselben Aufgabe.",
            "inputSchema": {"type": "object", "properties": {"agent": {"type": "string"}}, "required": ["agent"]}
        },
        {
            "name": "dialogue",
            "description": "Dialog in einem Schritt: lines = [{\"who\": \"max\", \"text\": \"Hallo Lena!\", \"action\": \"wave\"}, {\"who\": \"lena\", \"text\": \"Hi Max!\", \"pause\": 0.3}]. Erzeugt die Stimmen (voices: {\"max\": \"Anna\"} oder voice je Zeile), legt die Sätze hintereinander (start, gap), macht je Sprecher eine Tonspur mit Lippen-Sync (character: talk + Sprech-Gesten; andere Objekte: animate.talk auf dem Mund), Untertitel (subtitles: true) und trägt den Ton im Storyboard-Shot ein. Ersetzt einen früheren Dialog derselben Szene.",
            "inputSchema": {"type": "object", "properties": {"lines": {"type": "array", "items": obj}, "start": {"type": "number"}, "gap": {"type": "number"}, "subtitles": {"type": "boolean"}, "voices": obj, "scene": {"type": "string"}, "agent": {"type": "string"}}, "required": ["lines"]}
        },
        {
            "name": "get_selection",
            "description": "Welches Objekt hat der Mensch im CineForge-Fenster angeklickt? Liefert id und alle Felder. Nutze das, wenn er „das hier“, „dieses Objekt“, „das Ausgewählte“ o. ä. sagt.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "select_object",
            "description": "Markiert ein Objekt im Fenster (Rahmen in der Vorschau, Eigenschaften rechts), um dem Menschen zu zeigen, was du meinst. Ohne id: Markierung aufheben.",
            "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}}}
        },
        {
            "name": "get_guide",
            "description": "Liefert die vollständige Beschreibung des Szenen-Formats (alle Objekt-Typen, Felder, Standardwerte) und Tipps. Vor dem ersten Bauen lesen.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "get_scene",
            "description": "Liefert die aktuelle Szene als vollständiges JSON-Dokument.",
            "inputSchema": {"type": "object", "properties": {"scene": {"type": "string", "description": "Optional: andere Szene des Projekts (Name) bearbeiten, ohne sie zu öffnen"}, "agent": {"type": "string"}}}
        },
        {
            "name": "list_objects",
            "description": "Kurze Übersicht der Szene: Kamera, Sonne und alle Objekte als Baum mit id, Typ und Position.",
            "inputSchema": {"type": "object", "properties": {"scene": {"type": "string", "description": "Optional: andere Szene des Projekts (Name) bearbeiten, ohne sie zu öffnen"}, "agent": {"type": "string"}}}
        },
        {
            "name": "set_scene",
            "description": "Ersetzt die komplette Szene. Für neue Szenen oder große Umbauten. Für kleine Änderungen besser update_object.",
            "inputSchema": {"type": "object", "properties": {"scene": {"type": "string", "description": "Optional: andere Szene des Projekts (Name) bearbeiten, ohne sie zu öffnen"}, "agent": {"type": "string"}, "scene": obj}, "required": ["scene"]}
        },
        {
            "name": "add_objects",
            "description": "Fügt Objekte hinzu (vollständige Objekt-Definitionen mit id und type). Existiert eine id schon, wird das Objekt ersetzt. Mit parent werden sie Kinder einer Gruppe.",
            "inputSchema": {
                "type": "object",
                "properties": {"scene": {"type": "string", "description": "Optional: andere Szene des Projekts (Name) bearbeiten, ohne sie zu öffnen"}, "agent": {"type": "string"}, 
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
                "properties": {"scene": {"type": "string", "description": "Optional: andere Szene des Projekts (Name) bearbeiten, ohne sie zu öffnen"}, "agent": {"type": "string"}, "id": {"type": "string"}, "changes": obj},
                "required": ["id", "changes"]
            }
        },
        {
            "name": "remove_objects",
            "description": "Entfernt Objekte (inkl. Kinder) anhand ihrer ids.",
            "inputSchema": {
                "type": "object",
                "properties": {"scene": {"type": "string", "description": "Optional: andere Szene des Projekts (Name) bearbeiten, ohne sie zu öffnen"}, "agent": {"type": "string"}, "ids": {"type": "array", "items": {"type": "string"}}},
                "required": ["ids"]
            }
        },
        {
            "name": "update_scene_settings",
            "description": "Ändert name, description, environment (Himmel, Sonne, Nebel, Belichtung, Bloom) oder camera per Merge-Patch. Beispiel: {\"changes\":{\"environment\":{\"sun\":{\"elevation\":5}}}}",
            "inputSchema": {"type": "object", "properties": {"scene": {"type": "string", "description": "Optional: andere Szene des Projekts (Name) bearbeiten, ohne sie zu öffnen"}, "agent": {"type": "string"}, "changes": obj}, "required": ["changes"]}
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
            "description": "Rendert den ganzen Film (auch 1 Stunde und mehr): alle Shots des Storyboards in Reihenfolge – Szenen (scene) oder Video-Clips (clip) – mit Ton (Soundtrack + audio je Shot) als ein MP4. Jeder Shot wird zwischengespeichert: unveränderte Shots werden nicht neu gerendert, abgebrochene Exporte gehen weiter. transition: cut (Standard) oder fade für alle Übergänge; einzelne Übergänge über transition im Shot.",
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
            "name": "check_film",
            "description": "Prüft den ganzen Film vor dem Export: alle Shots (Szene oder Clip), Ton-Dateien, Bibliotheks-Objekte; meldet Länge, fehlende Dateien und wie viele Shots schon im Cache fertig sind.",
            "inputSchema": {"type": "object", "properties": {"width": {"type": "integer"}, "height": {"type": "integer"}, "fps": {"type": "integer"}, "quality": {"type": "string"}, "transition": {"type": "string"}}}
        },
        {
            "name": "list_shots",
            "description": "Listet die Shots des Storyboards mit Startzeit, Dauer, Quelle und Ton (seitenweise für lange Filme).",
            "inputSchema": {"type": "object", "properties": {"from": {"type": "integer"}, "count": {"type": "integer"}}}
        },
        {
            "name": "list_media",
            "description": "Listet die Mediendateien des Projekts: Audio/ (Musik, Geräusche, Stimmen), Clips/ (Videos), Bilder/ (Logos, Fotos, SVG), Modelle/ (3D-Modelle glTF/GLB) mit Länge.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "generate_voice",
            "description": "Vertont einen Sprechertext mit einer Systemstimme (kostenlos, offline) und speichert ihn als WAV in Audio/. Danach im Shot über audio einsetzen. list_voices: true zeigt die verfügbaren Stimmen (Mac z. B. „Anna“ für Deutsch).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "file": {"type": "string", "description": "Dateiname ohne Endung"},
                    "voice": {"type": "string"},
                    "rate": {"type": "integer", "description": "Sprechtempo (Wörter/Minute, ca. 150–220)"},
                    "list_voices": {"type": "boolean"}
                }
            }
        },
        {
            "name": "create_sound",
            "description": "Erzeugt einen Klang durch echte Synthese (kein KI-Sound) und speichert ihn als WAV in Audio/. Entweder preset (click, tap, pop, whoosh, swoosh, impact, boom, riser, notification, confirm, error, haptic, heartbeat, footstep, wind, rain, fire, typing, shutter, glass, success, coin, swipe, rope, crowd_cheer, drone) mit pitch/length/intensity/brightness/reverb/seed – oder ein eigenes recipe: {\"layers\":[{\"wave\":\"sine|triangle|saw|square|noise|pink|brown\",\"start\":0,\"length\":0.2,\"freq\":[800,120],\"attack\":0.003,\"decay\":0.2,\"sustain\":0,\"release\":0.05,\"gain\":1,\"filter\":{\"type\":\"lowpass|highpass|bandpass\",\"cutoff\":[4000,300],\"q\":0.7},\"drive\":0,\"pan\":0,\"detune\":0,\"fm\":{\"ratio\":3.5,\"index\":[2,0]},\"vibrato\":[5,0.3],\"repeat\":{\"count\":3,\"interval\":0.1,\"decay\":0.7,\"pitch\":1,\"jitter\":0}}],\"reverb\":{\"size\":0.5,\"mix\":0.2},\"echo\":{\"time\":0.2,\"feedback\":0.3,\"mix\":0.2}}. Ohne preset/recipe: Liste der Klänge.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Dateiname ohne Endung"},
                    "preset": {"type": "string"},
                    "recipe": obj,
                    "pitch": {"type": "number"}, "length": {"type": "number"}, "intensity": {"type": "number"},
                    "brightness": {"type": "number"}, "reverb": {"type": "number"}, "seed": {"type": "integer"},
                    "list": {"type": "boolean"}
                }
            }
        },
        {
            "name": "create_music",
            "description": "Erzeugt einen Musik-Track (WAV in Audio/) aus Akkorden und Abschnitten – der Track kann über die Abschnitte wachsen. style: synthwave, edm, lofi, cinematic, ambient, hiphop. sections: [{\"bars\":4,\"parts\":[\"pulse\"],\"intensity\":0.5},{\"bars\":8,\"parts\":[\"pad\",\"bass\",\"drums\",\"arp\"]}] mit Instrumenten pulse, pad, bass, kick, snare, hats, drums, arp. Länge = Takte × 4 Schläge × 60/bpm.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "bpm": {"type": "number"},
                    "style": {"type": "string", "enum": ["synthwave", "edm", "lofi", "cinematic", "ambient", "hiphop"]},
                    "chords": {"type": "array", "items": {"type": "string"}},
                    "beats_per_chord": {"type": "integer"},
                    "sections": {"type": "array", "items": obj},
                    "seed": {"type": "integer"}
                },
                "required": ["sections"]
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
                    "storyboard": {"type": "array", "items": obj},
                    "soundtrack": {"type": "array", "items": obj, "description": "Ersetzt den Film-Ton: [{\"file\":\"musik.mp3\",\"start\":0,\"volume\":0.6,\"fade_in\":2,\"fade_out\":3}] – Dateien im Ordner Audio/ des Projekts"}
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
            "description": "Legt eine neue Szenen-Datei im aktuellen Projekt an und öffnet sie. Optional direkt mit Inhalt (scene) oder mit einer fertig ausgeleuchteten Vorlage (template): wiese, dorf, wald, strand, nacht, studio, weltraum, zimmer, cartoon_2d. Vorlagen sind der schnellste Weg zu schönen Szenen – danach Figuren und Handlung ergänzen.",
            "inputSchema": {
                "type": "object",
                "properties": {"name": {"type": "string"}, "scene": obj, "template": {"type": "string"}},
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
    /// Was der Mensch im Fenster angeklickt hat.
    pub selection: &'a mut crate::studio_ui::Selection,
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
        "batch" => format!("{} Änderungen in einem Schritt", a.get("operations").and_then(Value::as_array).map(|l| l.len()).unwrap_or(0)),
        "duplicate_object" => format!("Objekt dupliziert: {}", a.get("id").and_then(Value::as_str).unwrap_or("?")),
        "dialogue" => format!("Dialog mit {} Sätzen vertont", a.get("lines").and_then(Value::as_array).map(|l| l.len()).unwrap_or(0)),
        other => other.into(),
    }
}

/// Führt einen Aufruf aus. Screenshots antworten später (siehe capture_tick).
pub fn execute(call: ToolCall, cx: &mut Ctx) {
    let a = &call.args;
    let str_arg = |k: &str| a.get(k).and_then(Value::as_str);
    let scene_file = cx.src.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

    // Agent-Name fürs Protokoll (mehrere Claude-Sitzungen gleichzeitig).
    let actor = who(a);

    // Während eines Exports darf sich die Szene nicht ändern.
    let is_mutation = matches!(
        call.name.as_str(),
        "set_scene" | "add_objects" | "update_object" | "remove_objects" | "update_scene_settings" | "batch" | "duplicate_object"
            | "dialogue" | "undo" | "open_scene" | "new_scene" | "open_project" | "new_project"
    );
    if is_mutation && cx.jobs.export_running() {
        let _ = call.reply.send(ToolOutput::error(format!(
            "Gerade läuft ein Video-Export – Änderungen sind erst danach möglich.\n{}",
            cx.jobs.export_summary().unwrap_or_default()
        )));
        return;
    }

    // Ziel-Szene: die geöffnete, oder per "scene" eine andere Szene des Projekts
    // (so arbeiten mehrere Agenten parallel, ohne sich die Ansicht wegzunehmen).
    let target: Option<std::path::PathBuf> = match str_arg("scene").filter(|_| {
        matches!(call.name.as_str(), "set_scene" | "add_objects" | "update_object" | "remove_objects" | "update_scene_settings" | "batch" | "duplicate_object" | "list_objects" | "get_scene")
    }) {
        Some(name) if a.get("scene").is_some_and(Value::is_string) => match scene_path(cx.project, name) {
            Ok(p) if p == cx.src.path => None,
            Ok(p) if p.is_file() => Some(p),
            Ok(p) => {
                let _ = call.reply.send(ToolOutput::error(format!("Szene {} gibt es nicht (list_scenes)", p.display())));
                return;
            }
            Err(e) => {
                let _ = call.reply.send(ToolOutput::error(e));
                return;
            }
        },
        _ => None,
    };
    let base_scene: Scene = match &target {
        Some(p) => match std::fs::read_to_string(p).map_err(|e| e.to_string()).and_then(|t| Scene::from_json(&t).map_err(|e| e.to_string())) {
            Ok(s) => s,
            Err(e) => {
                let _ = call.reply.send(ToolOutput::error(format!("Szene nicht lesbar: {e}")));
                return;
            }
        },
        None => cx.src.scene.clone(),
    };
    let target_name = target
        .as_ref()
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| scene_file.clone());

    if target.is_some() {
        match call.name.as_str() {
            "list_objects" => {
                let _ = call.reply.send(ToolOutput::text(ops::outline(&base_scene)));
                return;
            }
            "get_scene" => {
                let _ = call.reply.send(ToolOutput::text(base_scene.to_json_pretty()));
                return;
            }
            _ => {}
        }
    }

    let mutation: Option<Result<Scene, String>> = match call.name.as_str() {
        "set_scene" => Some(
            serde_json::to_string(&a.get("scene").cloned().unwrap_or(Value::Null))
                .map_err(|e| e.to_string())
                .and_then(|t| Scene::from_json(&t).map_err(|e| format!("Ungültige Szene: {e}"))),
        ),
        "batch" => Some(run_batch(&base_scene, a)),
        "duplicate_object" => Some(match str_arg("id") {
            Some(id) => ops::duplicate(
                &base_scene,
                id,
                str_arg("new_id"),
                a.get("count").and_then(Value::as_u64).unwrap_or(1) as u32,
                vec3_arg(a, "offset").map(|v| v.to_array()).unwrap_or([0.0; 3]),
            ),
            None => Err("id fehlt".into()),
        }),
        "add_objects" => Some(match a.get("objects").and_then(Value::as_array) {
            Some(list) => ops::upsert_objects(&base_scene, list, str_arg("parent")),
            None => Err("objects muss eine Liste sein".into()),
        }),
        "update_object" => Some(match str_arg("id") {
            Some(id) => ops::update_object(&base_scene, id, a.get("changes").unwrap_or(&Value::Null)),
            None => Err("id fehlt".into()),
        }),
        "remove_objects" => Some(match a.get("ids").and_then(Value::as_array) {
            Some(ids) => {
                let ids: Vec<String> = ids.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                ops::remove_objects(&base_scene, &ids)
            }
            None => Err("ids muss eine Liste sein".into()),
        }),
        "update_scene_settings" => Some(ops::update_settings(&base_scene, a.get("changes").unwrap_or(&Value::Null))),
        "dialogue" => Some(dialogue(&base_scene, a, &target_name, cx)),
        _ => None,
    };

    if let Some(result) = mutation {
        // Alle Bibliotheks-Verweise müssen auflösbar sein, bevor übernommen wird.
        let result = result.and_then(|scene| {
            cx.library.check(cx.project, &scene.objects)?;
            Ok(scene)
        });
        let saved = result.and_then(|scene| match &target {
            Some(p) => {
                let tmp = p.with_extension("json.tmp");
                std::fs::write(&tmp, scene.to_json_pretty())
                    .and_then(|_| std::fs::rename(&tmp, p))
                    .map_err(|e| format!("Speichern fehlgeschlagen: {e}"))?;
                Ok(scene)
            }
            None => cx.src.commit(scene, cx.out).map(|_| cx.src.scene.clone()),
        });
        let reply = match saved {
            Ok(scene) => {
                cx.project.log(&actor, format!("[{target_name}] {}", describe(&call.name, a)));
                let where_ = if target.is_some() {
                    format!("in Szene „{target_name}“ gespeichert (nicht geöffnet – Ansicht der App bleibt unverändert)")
                } else {
                    "übernommen und gespeichert. Die App zeigt die Änderung jetzt an".into()
                };
                let missing = crate::media::check_files(cx.project, &scene);
                let warn = if missing.is_empty() {
                    String::new()
                } else {
                    format!("\n⚠ Achtung, Dateien fehlen oder sind fehlerhaft (Objekt bleibt bis dahin leer):\n- {}\n", missing.join("\n- "))
                };
                ToolOutput::text(format!(
                    "✓ {where_}.{warn}\nTipp: Mit screenshot das Ergebnis prüfen.\n\n{}",
                    ops::outline(&scene)
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
        "list_objects" => ToolOutput::text(format!("{}{}", ops::outline(&cx.src.scene), selection_line(cx))),
        "get_selection" => match cx.selection.id.clone() {
            None => ToolOutput::text("Im Fenster ist nichts ausgewählt."),
            Some(id) => {
                let detail = ops::find(&cx.src.scene, &id)
                    .and_then(|o| serde_json::to_string_pretty(o).ok())
                    .unwrap_or_else(|| "(Objekt gibt es in der geöffneten Szene nicht mehr)".into());
                ToolOutput::text(format!("Der Mensch hat „{id}“ ausgewählt. Wenn er „das hier“, „dieses Objekt“ o. ä. sagt, meint er dieses.\n\n{detail}"))
            }
        },
        "select_object" => {
            let id = str_arg("id").map(String::from);
            match &id {
                Some(i) if ops::find(&cx.src.scene, i).is_none() => ToolOutput::error(format!("Kein Objekt „{i}“ in der geöffneten Szene.")),
                _ => {
                    cx.selection.id = id.clone();
                    ToolOutput::text(match id {
                        Some(i) => format!("„{i}“ ist jetzt im Fenster markiert (Rahmen in der Vorschau, Eigenschaften rechts)."),
                        None => "Markierung aufgehoben.".into(),
                    })
                }
            }
        }
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
        "claim_task" => match cx.project.require() {
            Err(e) => ToolOutput::error(e),
            Ok(()) => {
                let agent = str_arg("agent").unwrap_or("claude").to_string();
                let now = crate::project::now();
                match cx.project.data.claim_task(&agent, &now) {
                    Some(t) => {
                        cx.project.log(&actor, format!("Aufgabe #{} übernommen", t.id));
                        ToolOutput::text(format!(
                            "Aufgabe #{} gehört jetzt dir ({agent}): {}{}\nWenn fertig: update_tasks mit status done. Szenen anderer Shots mit dem Parameter scene bearbeiten, damit sich Agenten nicht stören.",
                            t.id,
                            t.text,
                            if t.notes.is_empty() { String::new() } else { format!("\nNotizen: {}", t.notes) }
                        ))
                    }
                    None => ToolOutput::text("Keine freie Aufgabe mehr. resume zeigt den Stand."),
                }
            }
        },
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
        "check_film" => check_film(a, cx),
        "list_shots" => {
            let from = a.get("from").and_then(Value::as_u64).unwrap_or(1).max(1) as usize;
            let count = a.get("count").and_then(Value::as_u64).unwrap_or(50).clamp(1, 200) as usize;
            let sb = &cx.project.data.storyboard;
            let mut t = 0.0;
            let mut lines = Vec::new();
            for (i, sh) in sb.iter().enumerate() {
                let start = t;
                t += sh.duration;
                if i + 1 >= from && lines.len() < count {
                    let src = match (&sh.scene, &sh.clip) {
                        (Some(x), _) => format!("Szene {x}"),
                        (None, Some(c)) => format!("Clip {}", c.file),
                        _ => "–".into(),
                    };
                    lines.push(format!(
                        "{} {:>3}. [{}] {} – {} ({:.1} s, {src}{})",
                        sh.status.symbol(),
                        i + 1,
                        cf_scene::project::format_duration(start),
                        sh.id,
                        sh.title,
                        sh.duration,
                        if sh.audio.is_empty() { String::new() } else { format!(", {} Ton", sh.audio.len()) }
                    ));
                }
            }
            ToolOutput::text(format!(
                "Shots {from}–{} von {} (Gesamtlänge {}):\n{}",
                (from + lines.len()).saturating_sub(1),
                sb.len(),
                cf_scene::project::format_duration(t),
                lines.join("\n")
            ))
        }
        "list_media" => {
            let mut out = String::new();
            for folder in [crate::media::AUDIO, crate::media::CLIPS, crate::media::IMAGES, crate::media::MODELS] {
                let (dir, files) = crate::media::list(cx.project, folder);
                out += &format!("{folder}/ ({}):\n", dir.display());
                if files.is_empty() {
                    out += "  (leer)\n";
                }
                for f in files {
                    let info = if folder == crate::media::IMAGES || folder == crate::media::MODELS {
                        String::new()
                    } else {
                        crate::media::probe(&dir.join(&f))
                            .ok()
                            .and_then(|i| i.duration)
                            .map(|d| format!(" – {d:.1} s"))
                            .unwrap_or_default()
                    };
                    out += &format!("  {f}{info}\n");
                }
            }
            ToolOutput::text(out)
        }
        "generate_voice" => generate_voice(a, cx),
        "create_sound" => create_sound(a, cx),
        "create_music" => create_music(a, cx),
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

/// Mehrere Änderungen in einem Schritt – alle oder keine.
fn run_batch(scene: &Scene, a: &Value) -> Result<Scene, String> {
    let ops_list = a.get("operations").and_then(Value::as_array).ok_or("operations muss eine Liste sein")?;
    if ops_list.is_empty() {
        return Err("operations ist leer".into());
    }
    let mut cur = scene.clone();
    for (i, op) in ops_list.iter().enumerate() {
        let s = |k: &str| op.get(k).and_then(Value::as_str);
        let kind = s("op").ok_or_else(|| format!("operations[{i}]: \"op\" fehlt (add, update, remove, settings, duplicate)"))?;
        let r = match kind {
            "add" => match op.get("objects").and_then(Value::as_array) {
                Some(list) => ops::upsert_objects(&cur, list, s("parent")),
                None => Err("objects muss eine Liste sein".into()),
            },
            "update" => match s("id") {
                Some(id) => ops::update_object(&cur, id, op.get("changes").unwrap_or(&Value::Null)),
                None => Err("id fehlt".into()),
            },
            "remove" => match op.get("ids").and_then(Value::as_array) {
                Some(ids) => ops::remove_objects(&cur, &ids.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>()),
                None => Err("ids muss eine Liste sein".into()),
            },
            "settings" => ops::update_settings(&cur, op.get("changes").unwrap_or(&Value::Null)),
            "duplicate" => match s("id") {
                Some(id) => ops::duplicate(
                    &cur,
                    id,
                    s("new_id"),
                    op.get("count").and_then(Value::as_u64).unwrap_or(1) as u32,
                    vec3_arg(op, "offset").map(|v| v.to_array()).unwrap_or([0.0; 3]),
                ),
                None => Err("id fehlt".into()),
            },
            other => {
                let hint = cf_scene::diagnose::suggest(other, ["add", "update", "remove", "settings", "duplicate"])
                    .map(|x| format!(" → Meintest du „{x}“?"))
                    .unwrap_or_default();
                Err(format!("unbekannte Operation „{other}“{hint}"))
            }
        };
        cur = r.map_err(|e| format!("operations[{i}] ({kind}): {e}\nKeine der Änderungen wurde übernommen."))?;
    }
    Ok(cur)
}

fn vec3_arg(a: &Value, k: &str) -> Option<Vec3> {
    let arr = a.get(k)?.as_array()?;
    if arr.len() != 3 {
        return None;
    }
    Some(Vec3::new(arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32))
}

/// Wer handelt? Claude, ein benannter Agent (claude/agent-2) oder der Mensch über die Oberfläche.
fn who(a: &Value) -> String {
    match a.get("agent").and_then(Value::as_str) {
        Some("mensch") => "mensch".into(),
        Some(name) => format!("claude/{name}"),
        None => "claude".into(),
    }
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
    cx.project.log(&who(a), format!("Video-Export gestartet: {} ({width}×{height}, {fps} fps, {frames} Bilder)", output.display()));
    ToolOutput::text(format!(
        "Export gestartet: {frames} Bilder ({width}×{height}, {fps} fps, {:.1}–{:.1} s) → {}\nEncoder: {encoder}\nFortschritt mit export_status abfragen. Solange der Export läuft, ist die Szene gesperrt.",
        start,
        end,
        output.display()
    ))
}

fn audio_out(cx: &Ctx, name: &str) -> Result<std::path::PathBuf, String> {
    let name = safe_name(name)?;
    let dir = cx.project.dir.as_ref().map(|d| d.join(crate::media::AUDIO)).ok_or("Kein Projekt geöffnet")?;
    Ok(dir.join(format!("{}.wav", name.trim_end_matches(".wav"))))
}

fn create_sound(a: &Value, cx: &mut Ctx) -> ToolOutput {
    if a.get("list").and_then(Value::as_bool) == Some(true) || (a.get("preset").is_none() && a.get("recipe").is_none()) {
        let list: Vec<String> = cf_audio::presets::NAMES.iter().map(|(n, d)| format!("- {n}: {d}")).collect();
        return ToolOutput::text(format!(
            "Fertige Klänge (preset), anpassbar mit pitch, length, intensity (0..1), brightness, reverb (0..1), seed:\n{}\n\nOder eigenes Rezept (recipe) aus Schichten – siehe Beschreibung von create_sound.",
            list.join("\n")
        ));
    }
    let out = match audio_out(cx, a.get("name").and_then(Value::as_str).unwrap_or("klang")) {
        Ok(p) => p,
        Err(e) => return ToolOutput::error(e),
    };
    let recipe = match (a.get("recipe"), a.get("preset").and_then(Value::as_str)) {
        (Some(r), _) => match serde_json::from_value::<cf_audio::synth::Recipe>(r.clone()) {
            Ok(r) => r,
            Err(e) => return ToolOutput::error(format!("Ungültiges Rezept: {}", cf_scene::diagnose::explain(&e.to_string()))),
        },
        (None, Some(name)) => {
            let tweak = cf_audio::presets::Tweak {
                pitch: a.get("pitch").and_then(Value::as_f64).unwrap_or(1.0) as f32,
                length: a.get("length").and_then(Value::as_f64).unwrap_or(1.0) as f32,
                intensity: a.get("intensity").and_then(Value::as_f64).unwrap_or(0.5) as f32,
                brightness: a.get("brightness").and_then(Value::as_f64).unwrap_or(1.0) as f32,
                reverb: a.get("reverb").and_then(Value::as_f64).map(|v| v as f32),
                seed: a.get("seed").and_then(Value::as_u64).unwrap_or(0) as u32,
            };
            match cf_audio::presets::preset(name, &tweak) {
                Ok(r) => r,
                Err(e) => return ToolOutput::error(e),
            }
        }
        _ => unreachable!(),
    };
    if let Err(e) = recipe.validate() {
        return ToolOutput::error(e);
    }
    let sound = recipe.render();
    if let Err(e) = cf_audio::write_wav(&out, &sound) {
        return ToolOutput::error(format!("Speichern fehlgeschlagen: {e}"));
    }
    let file = out.file_name().unwrap_or_default().to_string_lossy().to_string();
    cx.project.log("claude", format!("Klang erzeugt: {file}"));
    ToolOutput::text(format!(
        "✓ Audio/{file} – {}\nIm Film verwenden: im Shot \"audio\": [{{\"file\": \"{file}\", \"start\": 1.2}}]",
        cf_audio::describe(&sound)
    ))
}

fn create_music(a: &Value, cx: &mut Ctx) -> ToolOutput {
    let out = match audio_out(cx, a.get("name").and_then(Value::as_str).unwrap_or("musik")) {
        Ok(p) => p,
        Err(e) => return ToolOutput::error(e),
    };
    let mut spec_json = a.clone();
    if let Some(m) = spec_json.as_object_mut() {
        m.remove("name");
        m.remove("agent");
    }
    let spec: cf_audio::music::MusicSpec = match serde_json::from_value(spec_json) {
        Ok(s) => s,
        Err(e) => return ToolOutput::error(format!("Ungültige Angaben: {}", cf_scene::diagnose::explain(&e.to_string()))),
    };
    if let Err(e) = spec.validate() {
        return ToolOutput::error(e);
    }
    let track = spec.render();
    if let Err(e) = cf_audio::write_wav(&out, &track) {
        return ToolOutput::error(format!("Speichern fehlgeschlagen: {e}"));
    }
    let file = out.file_name().unwrap_or_default().to_string_lossy().to_string();
    cx.project.log("claude", format!("Musik erzeugt: {file}"));
    ToolOutput::text(format!(
        "✓ Audio/{file} – {} (Musik {:.1} s + Nachhall)\nAls Film-Musik: update_plan mit \"soundtrack\": [{{\"file\": \"{file}\", \"volume\": 0.6, \"fade_out\": 3}}]",
        cf_audio::describe(&track),
        spec.duration()
    ))
}

fn generate_voice(a: &Value, cx: &mut Ctx) -> ToolOutput {
    if a.get("list_voices").and_then(Value::as_bool) == Some(true) {
        return ToolOutput::text(format!("Verfügbare Stimmen:\n{}", crate::media::list_voices()));
    }
    let Some(text) = a.get("text").and_then(Value::as_str).filter(|t| !t.trim().is_empty()) else {
        return ToolOutput::error("text fehlt");
    };
    let name = match safe_name(a.get("file").and_then(Value::as_str).unwrap_or("sprecher")) {
        Ok(n) => n,
        Err(e) => return ToolOutput::error(e),
    };
    let dir = match &cx.project.dir {
        Some(d) => d.join(crate::media::AUDIO),
        None => return ToolOutput::error("Kein Projekt geöffnet"),
    };
    let out = dir.join(format!("{}.wav", name.trim_end_matches(".wav")));
    let rate = a.get("rate").and_then(Value::as_u64).map(|r| r as u32);
    match crate::media::synthesize(text, a.get("voice").and_then(Value::as_str), rate, &out) {
        Ok(()) => {
            let dur = crate::media::probe(&out).ok().and_then(|i| i.duration);
            cx.project.log("claude", format!("Sprecher-Ton erzeugt: {}", out.file_name().unwrap_or_default().to_string_lossy()));
            ToolOutput::text(format!(
                "✓ Gespeichert: Audio/{}{}\nIm Film verwenden, z. B. im Shot: \"audio\": [{{\"file\": \"{}\", \"start\": 0.5}}]",
                out.file_name().unwrap_or_default().to_string_lossy(),
                dur.map(|d| format!(" ({d:.1} s)")).unwrap_or_default(),
                out.file_name().unwrap_or_default().to_string_lossy()
            ))
        }
        Err(e) => ToolOutput::error(e),
    }
}

fn film_options(a: &Value, cx: &Ctx) -> Result<crate::film::PlanOptions, String> {
    let quality = crate::video::Quality::parse(a.get("quality").and_then(Value::as_str))?;
    let fade_all = match a.get("transition").and_then(Value::as_str).unwrap_or("cut") {
        "cut" | "schnitt" => false,
        "fade" | "blende" => true,
        other => return Err(format!("Unbekannter Übergang \"{other}\" (cut oder fade)")),
    };
    let even = |v: u64| ((v as u32).clamp(64, 3840) / 2) * 2;
    let width = even(a.get("width").and_then(Value::as_u64).unwrap_or(1920));
    let height = even(a.get("height").and_then(Value::as_u64).unwrap_or(1080).min(2160));
    let base_name = match a.get("name").and_then(Value::as_str) {
        Some(n) => safe_name(n)?,
        None => safe_name(&cx.project.data.name)?,
    };
    let dir = cx.project.export_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut output = dir.join(format!("{base_name}.mp4"));
    let mut n = 2;
    while output.exists() {
        output = dir.join(format!("{base_name}_{n}.mp4"));
        n += 1;
    }
    Ok(crate::film::PlanOptions {
        width,
        height,
        fps: a.get("fps").and_then(Value::as_u64).map(|v| v as u32),
        quality,
        fade_all,
        output,
    })
}

fn check_film(a: &Value, cx: &mut Ctx) -> ToolOutput {
    if let Err(e) = cx.project.require() {
        return ToolOutput::error(e);
    }
    let opt = match film_options(a, cx) {
        Ok(o) => o,
        Err(e) => return ToolOutput::error(e),
    };
    match crate::film::build_plan(cx.project, cx.library, opt) {
        Ok(plan) => ToolOutput::text(format!("✓ Film ist exportierbar.\n{}", crate::film::describe_plan(&plan))),
        Err(problems) => ToolOutput::error(format!("Noch nicht exportierbar:\n- {}", problems.join("\n- "))),
    }
}

fn export_film(a: &Value, cx: &mut Ctx) -> ToolOutput {
    if let Err(e) = cx.project.require() {
        return ToolOutput::error(e);
    }
    // Mit ffmpeg: Shot für Shot mit Cache, Clips und Ton (auch für sehr lange Filme).
    if crate::video::find_ffmpeg().is_some() {
        if cx.jobs.export_running() {
            return ToolOutput::error(format!("Es läuft schon ein Export.\n{}", cx.jobs.export_summary().unwrap_or_default()));
        }
        let opt = match film_options(a, cx) {
            Ok(o) => o,
            Err(e) => return ToolOutput::error(e),
        };
        return match crate::film::build_plan(cx.project, cx.library, opt) {
            Ok(plan) => {
                let text = crate::film::describe_plan(&plan);
                cx.project.log(&who(a), format!("Film-Export gestartet: {}", plan.output.display()));
                cx.jobs.film = Some(plan);
                ToolOutput::text(format!(
                    "Film-Export gestartet.\n{text}\nJeder Shot wird einzeln gerendert und zwischengespeichert; bei Abbruch geht ein neuer export_film dort weiter. Fortschritt mit export_status."
                ))
            }
            Err(problems) => ToolOutput::error(format!("Film kann noch nicht exportiert werden:\n- {}", problems.join("\n- "))),
        };
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
    cx.project.log(&who(a), format!("Film-Export gestartet: {count} Shots, {total:.1} s → {}", output.display()));
    ToolOutput::text(format!(
        "Film-Export gestartet: {count} Shots, {total:.1} s, {width}×{height}, {fps} fps → {}\nÜbergänge: {}. Fortschritt mit export_status.",
        output.display(),
        if fade_all { "Abblenden über Schwarz" } else { "harte Schnitte (Ein-/Ausblende am Anfang und Ende)" }
    ))
}

fn selection_line(cx: &Ctx) -> String {
    cx.selection.id.as_ref().map(|id| format!("\nIm Fenster ausgewählt: „{id}“ (meint der Mensch mit „das hier“)\n")).unwrap_or_default()
}

fn resume_text(cx: &Ctx) -> String {
    let text = resume_inner(cx);
    let notice = cx.src.notice.as_ref().map(|n| format!("⚠ {n}\n\n")).unwrap_or_default();
    format!("{notice}{text}{}", selection_line(cx))
}

fn resume_inner(cx: &Ctx) -> String {
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
    if let Some(v) = a.get("soundtrack") {
        match serde_json::from_value::<Vec<cf_scene::project::AudioClip>>(v.clone()) {
            Ok(list) => {
                report.push(format!("Soundtrack: {} Spur(en)", list.len()));
                data.soundtrack = list;
            }
            Err(e) => return ToolOutput::error(format!("Ungültiger Soundtrack: {e}")),
        }
    }
    if let Some(list) = a.get("storyboard").and_then(Value::as_array) {
        match data.update_storyboard(list) {
            Ok(r) => report.extend(r),
            Err(e) => return ToolOutput::error(format!("Nicht übernommen: {e}")),
        }
    }
    if report.is_empty() {
        return ToolOutput::error("Nichts zu ändern (name, goal, style, next_step, storyboard oder soundtrack angeben)");
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
        if let Some(owner) = u.get("owner").and_then(Value::as_str) {
            if let Some(t) = data.tasks.iter_mut().find(|t| t.id == id as u32) {
                t.owner = owner.to_string();
            }
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
    let template = a.get("template").and_then(Value::as_str);
    let scene_value = match (a.get("scene"), template) {
        (Some(v), _) => Some(v.clone()),
        (None, Some(t)) => match cf_scene::templates::template(t) {
            Some(v) => Some(v),
            None => {
                let names: Vec<&str> = cf_scene::templates::TEMPLATES.iter().map(|(n, _)| *n).collect();
                let hint = cf_scene::diagnose::suggest(t, names.iter().copied()).map(|s| format!(" Meintest du „{s}“?")).unwrap_or_default();
                return ToolOutput::error(format!("Vorlage „{t}“ gibt es nicht.{hint} Vorhanden: {}", names.join(", ")));
            }
        },
        (None, None) => None,
    };
    let scene = match scene_value.as_ref() {
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

/// Dialog: Stimmen erzeugen, Sätze hintereinander legen, je Sprecher eine Tonspur (für Lippen-Sync),
/// Sprech-Gesten, Untertitel, Shot-Ton im Storyboard. Liefert die geänderte Szene.
fn dialogue(base: &Scene, a: &Value, scene_name: &str, cx: &mut Ctx) -> Result<Scene, String> {
    cx.project.require()?;
    let lines = a.get("lines").and_then(Value::as_array).filter(|l| !l.is_empty()).ok_or("lines fehlt: [{\"who\": \"max\", \"text\": \"Hallo!\"}, …]")?;
    let start = a.get("start").and_then(Value::as_f64).unwrap_or(0.5) as f32;
    let gap = a.get("gap").and_then(Value::as_f64).unwrap_or(0.35) as f32;
    let subtitles = a.get("subtitles").and_then(Value::as_bool).unwrap_or(true);
    let voices = a.get("voices").cloned().unwrap_or(Value::Null);
    let audio_dir = cx.project.dir.as_ref().map(|d| d.join(crate::media::AUDIO)).ok_or("Kein Projekt")?;
    std::fs::create_dir_all(&audio_dir).map_err(|e| e.to_string())?;
    let scene_key = safe_name(scene_name).unwrap_or_else(|_| "szene".into());
    const SR: u32 = 48_000;
    // 1. Jede Zeile vertonen und zeitlich einordnen
    struct Line {
        who: String,
        text: String,
        at: f32,
        samples: Vec<f32>,
        action: Option<String>,
    }
    let mut out: Vec<Line> = Vec::new();
    let mut t = start;
    for (i, l) in lines.iter().enumerate() {
        let who = l.get("who").and_then(Value::as_str).ok_or(format!("lines[{i}]: who fehlt (id der Figur)"))?.to_string();
        let text = l.get("text").and_then(Value::as_str).filter(|s| !s.trim().is_empty()).ok_or(format!("lines[{i}]: text fehlt"))?.to_string();
        if ops::find(base, &who).is_none() {
            return Err(format!("lines[{i}]: Objekt „{who}“ gibt es nicht in der Szene"));
        }
        let voice = l.get("voice").and_then(Value::as_str).or_else(|| voices.get(&who).and_then(Value::as_str));
        let rate = l.get("rate").and_then(Value::as_u64).map(|r| r as u32);
        let file = audio_dir.join(format!("dlg_{scene_key}_{i:02}.wav"));
        crate::media::synthesize(&text, voice, rate, &file)?;
        let bytes = std::fs::read(&file).map_err(|e| e.to_string())?;
        let (sr, mono) = cf_audio::read_wav(&bytes)?;
        // auf 48 kHz umrechnen (linear)
        let n = (mono.len() as f64 * SR as f64 / sr as f64) as usize;
        let samples: Vec<f32> = (0..n)
            .map(|k| {
                let x = k as f64 * sr as f64 / SR as f64;
                let i0 = x.floor() as usize;
                let f = (x - i0 as f64) as f32;
                let a0 = mono.get(i0).copied().unwrap_or(0.0);
                let a1 = mono.get(i0 + 1).copied().unwrap_or(a0);
                a0 + (a1 - a0) * f
            })
            .collect();
        t += l.get("pause").and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let dur = samples.len() as f32 / SR as f32;
        out.push(Line { who, text, at: t, samples, action: l.get("action").and_then(Value::as_str).map(String::from) });
        t += dur + gap;
    }
    let end = t;
    // 2. Je Sprecher eine Tonspur
    let mut speakers: Vec<String> = out.iter().map(|l| l.who.clone()).collect();
    speakers.dedup();
    speakers.sort();
    speakers.dedup();
    let mut scene = base.clone();
    let mut tracks = Vec::new();
    for who in &speakers {
        let len = ((end + 0.5) * SR as f32) as usize;
        let mut track = vec![0.0f32; len];
        for l in out.iter().filter(|l| &l.who == who) {
            let off = (l.at * SR as f32) as usize;
            for (k, s) in l.samples.iter().enumerate() {
                if let Some(v) = track.get_mut(off + k) {
                    *v += s;
                }
            }
        }
        let fname = format!("dlg_{scene_key}_{}.wav", safe_name(who).unwrap_or_else(|_| who.clone()));
        cf_audio::write_wav(&audio_dir.join(&fname), &(track.clone(), track)).map_err(|e| e.to_string())?;
        tracks.push(fname.clone());
        // 3. Figur: Lippen-Sync + Sprech-Gesten
        let obj = ops::find(&scene, who).ok_or("Objekt verschwunden")?.clone();
        let talk = serde_json::json!({"audio": fname, "start": 0.0, "gain": 1.3});
        let changes = match &obj.kind {
            cf_scene::Kind::Character(c) => {
                let mut actions: Vec<Value> = c
                    .actions
                    .iter()
                    .filter(|x| x.kind != cf_scene::character::ActionKind::Talk)
                    .map(|x| serde_json::to_value(x).unwrap_or(Value::Null))
                    .collect();
                for l in out.iter().filter(|l| &l.who == who) {
                    let d = l.samples.len() as f32 / SR as f32;
                    actions.push(serde_json::json!({"type": "talk", "start": l.at, "end": l.at + d}));
                    if let Some(act) = &l.action {
                        actions.push(serde_json::json!({"type": act, "start": l.at, "end": l.at + d.min(2.5)}));
                    }
                }
                serde_json::json!({"talk": talk, "actions": actions})
            }
            _ => serde_json::json!({"animate": {"talk": talk}}),
        };
        scene = ops::update_object(&scene, who, &changes)?;
    }
    // 4. Untertitel (ersetzt frühere Dialog-Untertitel) und Szenenlänge
    let mut overlays: Vec<Value> = scene
        .overlays
        .iter()
        .filter(|o| !(o.background.is_some() && (o.y + 0.82).abs() < 1e-3))
        .map(|o| serde_json::to_value(o).unwrap_or(Value::Null))
        .collect();
    if subtitles {
        for l in &out {
            let d = l.samples.len() as f32 / SR as f32;
            let name = ops::find(&scene, &l.who).and_then(|o| o.name.clone()).unwrap_or_else(|| {
                let mut c = l.who.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            });
            overlays.push(serde_json::json!({"text": format!("{name}: {}", l.text), "y": -0.82, "size": 0.042,
                "background": "#000000", "background_opacity": 0.55, "start": l.at, "end": l.at + d + 0.25, "fade": 0.12}));
        }
    }
    let duration = scene.duration.max(end + 0.8);
    scene = ops::update_settings(&scene, &serde_json::json!({"overlays": overlays, "duration": duration}))?;
    // 5. Storyboard: Ton und Länge des Shots
    if let Some(shot) = cx.project.data.storyboard.iter_mut().find(|s| s.scene.as_deref() == Some(scene_name)) {
        shot.audio.retain(|c| !c.file.starts_with(&format!("dlg_{scene_key}_")));
        for f in &tracks {
            shot.audio.push(serde_json::from_value(serde_json::json!({"file": f, "start": 0.0})).map_err(|e: serde_json::Error| e.to_string())?);
        }
        shot.duration = shot.duration.max(duration);
        let _ = cx.project.save();
    }
    Ok(scene)
}

/// Hilfs-Query für die aktuelle Fenster-Kamera.
pub type UserViewQuery<'w, 's> = Query<'w, 's, &'static Transform, With<MainCamera>>;
