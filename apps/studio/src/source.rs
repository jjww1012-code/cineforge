//! Woher die Szene kommt und wann sie sich ändert.
//!
//! Die Datei wird alle 250 ms auf Änderungen geprüft. Ist die neue Fassung
//! fehlerhaft, bleibt die alte Szene sichtbar und der Fehler wird angezeigt –
//! so kann Claude (oder ein Mensch) gefahrlos live editieren.

use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use bevy::{prelude::*, window::FileDragAndDrop};
use cf_scene::Scene;

#[derive(Resource)]
pub struct SceneSource {
    pub path: PathBuf,
    modified: Option<SystemTime>,
    poll: Timer,
    /// Zuletzt erfolgreich geladene Szene.
    pub scene: Scene,
    /// Fehler des letzten Ladeversuchs (None = alles ok).
    pub error: Option<String>,
    /// Hinweis für den Menschen (z. B. „letzte Szene nicht ladbar, Demo geöffnet“).
    pub notice: Option<String>,
    /// Frühere Fassungen für „Rückgängig“ (neueste zuletzt).
    history: Vec<Scene>,
}

const MAX_HISTORY: usize = 100;

/// Wird gesendet, wenn eine neue gültige Szene geladen wurde.
#[derive(Message)]
pub struct SceneLoaded {
    /// true beim ersten Laden oder beim Öffnen einer anderen Datei.
    pub new_file: bool,
}

impl SceneSource {
    pub fn open(path: Option<PathBuf>) -> Result<Self, String> {
        let path = match path {
            Some(p) => p,
            None => crate::project::startup_scene()?,
        };
        let (scene, modified) = read_scene(&path)?;
        Ok(Self {
            path,
            modified,
            poll: Timer::new(Duration::from_millis(250), TimerMode::Repeating),
            scene,
            error: None,
            notice: None,
            history: Vec::new(),
        })
    }

    /// Ersatz, wenn die gewünschte Szene nicht ladbar ist: Demo-Szene, sonst eine leere Notfall-Szene.
    pub fn fallback(reason: &str) -> Self {
        let notice = format!("Die zuletzt geöffnete Szene konnte nicht geladen werden – stattdessen Demo geöffnet. Grund: {reason}");
        let mut s = crate::project::demo_scene()
            .ok()
            .and_then(|p| Self::open(Some(p)).ok())
            .unwrap_or_else(|| {
                let path = std::env::temp_dir().join("cineforge-notfall.json");
                let scene = Scene::from_json("{}").expect("leere Szene ist gültig");
                let _ = std::fs::write(&path, scene.to_json_pretty());
                Self {
                    modified: std::fs::metadata(&path).and_then(|m| m.modified()).ok(),
                    path,
                    poll: Timer::new(Duration::from_millis(250), TimerMode::Repeating),
                    scene,
                    error: None,
                    notice: None,
                    history: Vec::new(),
                }
            });
        s.notice = Some(notice);
        s
    }

    /// Übernimmt eine neue (bereits geprüfte) Szene: merkt die alte für
    /// „Rückgängig“, speichert die Datei und meldet die Änderung.
    pub fn commit(&mut self, scene: Scene, out: &mut MessageWriter<SceneLoaded>) -> Result<(), String> {
        self.write_file(&scene)?;
        let old = std::mem::replace(&mut self.scene, scene);
        self.push_history(old);
        self.error = None;
        out.write(SceneLoaded { new_file: false });
        Ok(())
    }

    /// Stellt die vorherige Fassung wieder her. Liefert false, wenn es keine gibt.
    pub fn undo(&mut self, out: &mut MessageWriter<SceneLoaded>) -> Result<bool, String> {
        let Some(prev) = self.history.pop() else { return Ok(false) };
        self.write_file(&prev)?;
        self.scene = prev;
        self.error = None;
        out.write(SceneLoaded { new_file: false });
        Ok(true)
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Öffnet eine andere Szenen-Datei (Verlauf beginnt neu).
    pub fn open_file(&mut self, path: PathBuf, out: &mut MessageWriter<SceneLoaded>) -> Result<(), String> {
        let (scene, modified) = read_scene(&path)?;
        self.path = path;
        self.scene = scene;
        self.modified = modified;
        self.error = None;
        self.notice = None;
        self.history.clear();
        out.write(SceneLoaded { new_file: true });
        Ok(())
    }

    fn push_history(&mut self, old: Scene) {
        if self.history.last() != Some(&old) {
            self.history.push(old);
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }
    }

    fn write_file(&mut self, scene: &Scene) -> Result<(), String> {
        // Erst in eine Hilfsdatei schreiben, dann umbenennen: so entsteht
        // nie eine halb geschriebene Szene, falls etwas abbricht.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, scene.to_json_pretty())
            .and_then(|_| std::fs::rename(&tmp, &self.path))
            .map_err(|e| format!("Speichern fehlgeschlagen ({}): {e}", self.path.display()))?;
        // Eigene Änderung nicht noch einmal als „Datei geändert“ einlesen.
        self.modified = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        Ok(())
    }
}

fn read_scene(path: &Path) -> Result<(Scene, Option<SystemTime>), String> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Datei {} nicht lesbar: {e}", path.display()))?;
    let scene = Scene::from_json(&text).map_err(|e| e.to_string())?;
    Ok((scene, modified))
}

pub struct SourcePlugin;

impl Plugin for SourcePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SceneLoaded>()
            .add_systems(Startup, |mut w: MessageWriter<SceneLoaded>| {
                w.write(SceneLoaded { new_file: true });
            })
            .add_systems(Update, (poll_file, manual_reload, drop_file, undo_key));
    }
}

fn reload(src: &mut SceneSource, out: &mut MessageWriter<SceneLoaded>, new_file: bool) {
    match read_scene(&src.path) {
        Ok((scene, modified)) => {
            if !new_file {
                let old = std::mem::replace(&mut src.scene, scene);
                src.push_history(old);
            } else {
                src.scene = scene;
                src.history.clear();
            }
            src.modified = modified;
            src.error = None;
            out.write(SceneLoaded { new_file });
            info!("Szene geladen: {}", src.path.display());
        }
        Err(e) => {
            // Zeitstempel trotzdem merken, sonst wird der Fehler 4x pro Sekunde wiederholt.
            src.modified = std::fs::metadata(&src.path).and_then(|m| m.modified()).ok();
            warn!("{e}");
            src.error = Some(e);
        }
    }
}

fn poll_file(time: Res<Time>, mut src: ResMut<SceneSource>, mut out: MessageWriter<SceneLoaded>) {
    if !src.poll.tick(time.delta()).just_finished() {
        return;
    }
    let now = std::fs::metadata(&src.path).and_then(|m| m.modified()).ok();
    if now.is_some() && now != src.modified {
        reload(&mut src, &mut out, false);
    }
}

fn manual_reload(
    keys: Res<ButtonInput<KeyCode>>,
    mut src: ResMut<SceneSource>,
    mut out: MessageWriter<SceneLoaded>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        reload(&mut src, &mut out, false);
    }
}

fn drop_file(
    mut drops: MessageReader<FileDragAndDrop>,
    mut src: ResMut<SceneSource>,
    mut out: MessageWriter<SceneLoaded>,
) {
    for d in drops.read() {
        if let FileDragAndDrop::DroppedFile { path_buf, .. } = d {
            if path_buf.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")) {
                src.path = path_buf.clone();
                reload(&mut src, &mut out, true);
            }
        }
    }
}

fn undo_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut src: ResMut<SceneSource>,
    mut out: MessageWriter<SceneLoaded>,
) {
    let modifier = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    if modifier && keys.just_pressed(KeyCode::KeyZ) {
        match src.undo(&mut out) {
            Ok(true) => info!("Rückgängig"),
            Ok(false) => info!("Nichts mehr rückgängig zu machen"),
            Err(e) => src.error = Some(e),
        }
    }
}
