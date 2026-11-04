//! Projekte und Fortsetzungs-Gedächtnis.
//!
//! Ordnerstruktur:
//! ```text
//! Dokumente/CineForge/
//!   zuletzt.json                 ← zuletzt geöffnete Szene (wird beim Start wieder geöffnet)
//!   Bibliothek/                  ← projektübergreifende Objekte
//!   Projekte/<Name>/
//!     projekt.json               ← Logbuch: Ziel, Stil, Storyboard, Aufgaben, Notizen, Protokoll
//!     Szenen/*.json
//!     Objekte/*.json             ← Objekte dieses Projekts
//!     Export/                    ← fertige Videos
//! ```

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use cf_scene::project::{format_utc, safe_name, Project, PROJECT_FILE};

const DEMO_SCENE: &str = include_str!("../../../scenes/wueste.json");

pub fn now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs)
}

pub fn root_dir() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("CINEFORGE_HOME") {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or("Benutzerordner nicht gefunden")?;
    Ok(home.join("Documents").join("CineForge"))
}

pub fn projects_dir() -> Result<PathBuf, String> {
    Ok(root_dir()?.join("Projekte"))
}

pub fn global_library_dir() -> Result<PathBuf, String> {
    Ok(root_dir()?.join("Bibliothek"))
}

fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)
        .and_then(|_| std::fs::rename(&tmp, path))
        .map_err(|e| format!("Speichern fehlgeschlagen ({}): {e}", path.display()))
}

#[derive(Resource, Debug)]
pub struct ActiveProject {
    /// Projektordner (None = Szene liegt außerhalb eines Projekts).
    pub dir: Option<PathBuf>,
    pub data: Project,
}

impl ActiveProject {
    /// Ermittelt das Projekt, zu dem eine Szenen-Datei gehört (…/<Projekt>/Szenen/x.json).
    pub fn for_scene(scene_path: &Path) -> Self {
        let dir = scene_path
            .parent()
            .filter(|p| p.file_name().is_some_and(|n| n == "Szenen"))
            .and_then(Path::parent)
            .filter(|p| p.join(PROJECT_FILE).is_file())
            .map(Path::to_path_buf);
        let data = dir
            .as_ref()
            .and_then(|d| std::fs::read_to_string(d.join(PROJECT_FILE)).ok())
            .and_then(|t| Project::from_json(&t).ok())
            .unwrap_or_default();
        Self { dir, data }
    }

    pub fn name(&self) -> Option<&str> {
        self.dir.as_ref().map(|_| self.data.name.as_str())
    }

    pub fn scenes_dir(&self) -> Result<PathBuf, String> {
        match &self.dir {
            Some(d) => Ok(d.join("Szenen")),
            None => Ok(root_dir()?.join("Szenen")),
        }
    }

    pub fn objects_dir(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join("Objekte"))
    }

    pub fn export_dir(&self) -> Result<PathBuf, String> {
        match &self.dir {
            Some(d) => Ok(d.join("Export")),
            None => Ok(root_dir()?.join("Export")),
        }
    }

    pub fn require(&self) -> Result<(), String> {
        if self.dir.is_none() {
            Err("Die geöffnete Szene gehört zu keinem Projekt. Mit new_project ein Projekt anlegen oder mit open_project eines öffnen.".into())
        } else {
            Ok(())
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let Some(dir) = &self.dir else { return Ok(()) };
        write_atomic(&dir.join(PROJECT_FILE), &self.data.to_json_pretty())
    }

    /// Protokolliert eine Aktion und speichert sofort.
    pub fn log(&mut self, actor: &str, action: impl Into<String>) {
        if self.dir.is_none() {
            return;
        }
        self.data.log(&now(), actor, action);
        if let Err(e) = self.save() {
            warn!("{e}");
        }
    }

    /// Legt ein neues Projekt an (mit leerer Start-Szene) und liefert den Pfad dieser Szene.
    pub fn create(name: &str, goal: &str, style: &str) -> Result<(Self, PathBuf), String> {
        let name = safe_name(name)?;
        let dir = projects_dir()?.join(&name);
        if dir.join(PROJECT_FILE).exists() {
            return Err(format!("Projekt \"{name}\" gibt es schon – open_project benutzen"));
        }
        let mut data = Project::new(&name, goal);
        data.style = style.to_string();
        data.log(&now(), "system", "Projekt angelegt");
        let p = Self { dir: Some(dir.clone()), data };
        p.save()?;
        let scene = dir.join("Szenen").join("start.json");
        write_atomic(
            &scene,
            &cf_scene::Scene::from_json(&format!(r#"{{"name":"{name} – Start"}}"#))
                .map_err(|e| e.to_string())?
                .to_json_pretty(),
        )?;
        std::fs::create_dir_all(dir.join("Objekte")).ok();
        Ok((p, scene))
    }

    /// Öffnet ein Projekt und liefert die Szene, mit der weitergearbeitet wird
    /// (zuletzt geänderte Szene des Projekts).
    pub fn open(name: &str) -> Result<(Self, PathBuf), String> {
        let name = safe_name(name)?;
        let dir = projects_dir()?.join(&name);
        let text = std::fs::read_to_string(dir.join(PROJECT_FILE))
            .map_err(|_| format!("Projekt \"{name}\" nicht gefunden. Vorhanden: {}", list_projects().join(", ")))?;
        let data = Project::from_json(&text)?;
        let scenes = dir.join("Szenen");
        let latest = std::fs::read_dir(&scenes)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        let scene = match latest {
            Some(s) => s,
            None => {
                let s = scenes.join("start.json");
                write_atomic(&s, &cf_scene::Scene::from_json("{}").map_err(|e| e.to_string())?.to_json_pretty())?;
                s
            }
        };
        Ok((Self { dir: Some(dir), data }, scene))
    }
}

pub fn list_projects() -> Vec<String> {
    let Ok(dir) = projects_dir() else { return vec![] };
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join(PROJECT_FILE).is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    v.sort();
    v
}

// ---------------------------------------------------------------------------
// Zuletzt geöffnete Szene
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct LastState {
    scene: Option<PathBuf>,
}

pub fn remember_scene(path: &Path) {
    let Ok(root) = root_dir() else { return };
    let state = LastState { scene: Some(path.to_path_buf()) };
    let _ = write_atomic(&root.join("zuletzt.json"), &serde_json::to_string_pretty(&state).unwrap_or_default());
}

/// Szene für den Start ohne Argumente: zuletzt geöffnete Szene, sonst das Demo-Projekt.
pub fn startup_scene() -> Result<PathBuf, String> {
    let root = root_dir()?;
    let last: LastState = std::fs::read_to_string(root.join("zuletzt.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    if let Some(p) = last.scene.filter(|p| p.is_file()) {
        return Ok(p);
    }
    ensure_demo_project()
}

fn ensure_demo_project() -> Result<PathBuf, String> {
    let dir = projects_dir()?.join("Demo");
    let scene = dir.join("Szenen").join("wueste.json");
    if !dir.join(PROJECT_FILE).exists() {
        let mut p = Project::new("Demo", "Ausprobieren: eine Wüste mit Pyramiden bei Sonnenuntergang.");
        p.style = "Warme Abendstimmung, klare Formen, leichter Dunst.".into();
        p.next_step = "Szene erkunden (rechte Maustaste + WASD) oder Claude bitten, etwas zu verändern.".into();
        p.log(&now(), "system", "Demo-Projekt angelegt");
        write_atomic(&dir.join(PROJECT_FILE), &p.to_json_pretty())?;
    }
    if !scene.exists() {
        write_atomic(&scene, DEMO_SCENE)?;
    }
    Ok(scene)
}

pub struct ProjectPlugin {
    /// Nur im normalen Fenster-Modus merken, welche Szene zuletzt offen war
    /// (nicht bei einmaligen Screenshots über die Kommandozeile).
    pub remember: bool,
}

#[derive(Resource)]
struct RememberScenes(bool);

impl Plugin for ProjectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RememberScenes(self.remember)).add_systems(Update, follow_scene);
    }
}

/// Hält das aktive Projekt passend zur geöffneten Szene und merkt sich die Szene.
fn follow_scene(
    mut loaded: MessageReader<crate::source::SceneLoaded>,
    src: Res<crate::source::SceneSource>,
    mut project: ResMut<ActiveProject>,
    remember: Res<RememberScenes>,
) {
    if !loaded.read().any(|m| m.new_file) {
        return;
    }
    if remember.0 {
        remember_scene(&src.path);
    }
    let detected = ActiveProject::for_scene(&src.path);
    if detected.dir != project.dir {
        *project = detected;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_open_and_detect_project() {
        let tmp = std::env::temp_dir().join(format!("cf-test-{}", std::process::id()));
        // SAFETY: Tests in diesem Modul sind die einzigen, die CINEFORGE_HOME setzen.
        unsafe { std::env::set_var("CINEFORGE_HOME", &tmp) };
        let (mut p, scene) = ActiveProject::create("Mein Film", "Ziel", "Stil").unwrap();
        assert!(scene.ends_with("Projekte/Mein Film/Szenen/start.json"));
        p.data.next_step = "weiter mit Shot 2".into();
        p.save().unwrap();
        assert!(ActiveProject::create("Mein Film", "", "").is_err(), "doppelt");
        let detected = ActiveProject::for_scene(&scene);
        assert_eq!(detected.name(), Some("Mein Film"));
        assert_eq!(detected.data.next_step, "weiter mit Shot 2");
        let (opened, s2) = ActiveProject::open("Mein Film").unwrap();
        assert_eq!(s2, scene);
        assert_eq!(opened.data.style, "Stil");
        assert!(ActiveProject::open("Gibts nicht").is_err());
        assert_eq!(list_projects(), vec!["Mein Film".to_string()]);
        remember_scene(&scene);
        assert_eq!(startup_scene().unwrap(), scene);
        assert!(ActiveProject::for_scene(Path::new("/irgendwo/x.json")).dir.is_none());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
