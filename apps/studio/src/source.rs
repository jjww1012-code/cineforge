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

const DEMO_SCENE: &str = include_str!("../../../scenes/wueste.json");
const DEMO_FILE_NAME: &str = "wueste.json";

#[derive(Resource)]
pub struct SceneSource {
    pub path: PathBuf,
    modified: Option<SystemTime>,
    poll: Timer,
    /// Zuletzt erfolgreich geladene Szene.
    pub scene: Scene,
    /// Fehler der letzten Ladeversuchs (None = alles ok).
    pub error: Option<String>,
}

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
            None => demo_path()?,
        };
        let (scene, modified) = read_scene(&path)?;
        Ok(Self {
            path,
            modified,
            poll: Timer::new(Duration::from_millis(250), TimerMode::Repeating),
            scene,
            error: None,
        })
    }
}

fn read_scene(path: &Path) -> Result<(Scene, Option<SystemTime>), String> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Datei {} nicht lesbar: {e}", path.display()))?;
    let scene = Scene::from_json(&text).map_err(|e| e.to_string())?;
    Ok((scene, modified))
}

/// Dokumente/CineForge/Szenen/wueste.json – wird beim ersten Start angelegt,
/// damit man die Szene sofort bearbeiten und Live-Reload ausprobieren kann.
fn demo_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or("Benutzerordner nicht gefunden")?;
    let dir = home.join("Documents").join("CineForge").join("Szenen");
    let file = dir.join(DEMO_FILE_NAME);
    if !file.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        std::fs::write(&file, DEMO_SCENE).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(file)
}

pub struct SourcePlugin;

impl Plugin for SourcePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SceneLoaded>()
            .add_systems(Startup, |mut w: MessageWriter<SceneLoaded>| {
                w.write(SceneLoaded { new_file: true });
            })
            .add_systems(Update, (poll_file, manual_reload, drop_file));
    }
}

fn reload(src: &mut SceneSource, out: &mut MessageWriter<SceneLoaded>, new_file: bool) {
    match read_scene(&src.path) {
        Ok((scene, modified)) => {
            src.scene = scene;
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
