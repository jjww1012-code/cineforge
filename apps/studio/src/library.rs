//! Objekt-Bibliothek der App: findet, lädt (mit Cache) und speichert
//! Bibliotheks-Objekte. Suchreihenfolge: Projekt (`Objekte/`), dann global
//! (`Dokumente/CineForge/Bibliothek/`).

use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use bevy::prelude::*;
use cf_scene::{
    library::{referenced_assets, AssetFile},
    project::safe_name,
    Object,
};

use crate::project::{global_library_dir, ActiveProject};

/// Mitgelieferte Objekte (Cartoon-Stil), erzeugt mit `tools/make_starter_library.py`.
const STARTER: &str = include_str!("../assets/bibliothek.json");

/// Legt die mitgelieferten Objekte in der globalen Bibliothek an – nur fehlende Dateien,
/// eigene Änderungen bleiben erhalten.
pub fn install_starter_library() {
    let Ok(dir) = global_library_dir() else { return };
    let Ok(list) = serde_json::from_str::<Vec<AssetFile>>(STARTER) else { return };
    let _ = std::fs::create_dir_all(&dir);
    for a in list {
        let path = dir.join(format!("{}.json", a.name));
        if !path.exists() {
            if let Ok(text) = serde_json::to_string_pretty(&a) {
                let _ = std::fs::write(&path, text);
            }
        }
    }
}

/// Maximale Verschachtelung von Bibliotheks-Objekten in Bibliotheks-Objekten.
const MAX_DEPTH: u32 = 8;

#[derive(Resource, Default)]
pub struct Library {
    cache: HashMap<PathBuf, (Option<SystemTime>, AssetFile)>,
}

pub struct AssetInfo {
    pub name: String,
    pub description: String,
    pub scope: &'static str,
    pub extent: Option<([f32; 3], [f32; 3])>,
}

fn dirs(project: &ActiveProject) -> Vec<(PathBuf, &'static str)> {
    let mut v = Vec::new();
    if let Some(d) = project.objects_dir() {
        v.push((d, "Projekt"));
    }
    if let Ok(d) = global_library_dir() {
        v.push((d, "global"));
    }
    v
}

impl Library {
    pub fn find_path(project: &ActiveProject, name: &str) -> Option<PathBuf> {
        let name = safe_name(name).ok()?;
        dirs(project).into_iter().map(|(d, _)| d.join(format!("{name}.json"))).find(|p| p.is_file())
    }

    pub fn load(&mut self, project: &ActiveProject, name: &str) -> Result<AssetFile, String> {
        let path = Self::find_path(project, name).ok_or_else(|| {
            let known: Vec<String> = Self::list(project).into_iter().map(|a| a.name).collect();
            format!(
                "Bibliotheks-Objekt \"{name}\" nicht gefunden. Vorhanden: {}",
                if known.is_empty() { "(keine – mit save_asset anlegen)".into() } else { known.join(", ") }
            )
        })?;
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if let Some((m, a)) = self.cache.get(&path) {
            if *m == modified {
                return Ok(a.clone());
            }
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let asset = AssetFile::from_json(&text)?;
        self.cache.insert(path, (modified, asset.clone()));
        Ok(asset)
    }

    pub fn list(project: &ActiveProject) -> Vec<AssetInfo> {
        let mut out: Vec<AssetInfo> = Vec::new();
        for (dir, scope) in dirs(project) {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            let mut files: Vec<PathBuf> =
                rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
            files.sort();
            for p in files {
                let name = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                if out.iter().any(|a| a.name == name) {
                    continue; // Projekt-Objekt überdeckt globales gleichen Namens
                }
                let parsed = std::fs::read_to_string(&p).ok().and_then(|t| AssetFile::from_json(&t).ok());
                out.push(AssetInfo {
                    name,
                    description: parsed.as_ref().map(|a| a.description.clone()).unwrap_or_else(|| "(Datei fehlerhaft)".into()),
                    scope,
                    extent: parsed.as_ref().and_then(|a| cf_scene::library::approx_extent(&a.object)),
                });
            }
        }
        out
    }

    pub fn save(&mut self, project: &ActiveProject, asset: &AssetFile, global: bool) -> Result<PathBuf, String> {
        asset.validate()?;
        let name = safe_name(&asset.name)?;
        let dir = if global {
            global_library_dir()?
        } else {
            project.objects_dir().ok_or("Kein Projekt geöffnet – mit global: true in der globalen Bibliothek speichern")?
        };
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join(format!("{name}.json"));
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, asset.to_json_pretty())
            .and_then(|_| std::fs::rename(&tmp, &path))
            .map_err(|e| format!("Speichern fehlgeschlagen ({}): {e}", path.display()))?;
        self.cache.remove(&path);
        Ok(path)
    }

    /// Prüft, ob alle (auch verschachtelten) Verweise auflösbar sind.
    pub fn check(&mut self, project: &ActiveProject, objects: &[Object]) -> Result<(), String> {
        self.check_rec(project, objects, 0, &mut Vec::new())
    }

    fn check_rec(&mut self, project: &ActiveProject, objects: &[Object], depth: u32, chain: &mut Vec<String>) -> Result<(), String> {
        let mut refs = Vec::new();
        referenced_assets(objects, &mut refs);
        for name in refs {
            if chain.contains(&name) {
                return Err(format!("Bibliotheks-Objekt \"{name}\" enthält sich selbst ({} → {name})", chain.join(" → ")));
            }
            if depth >= MAX_DEPTH {
                return Err(format!("Bibliotheks-Objekte sind zu tief verschachtelt (max. {MAX_DEPTH})"));
            }
            let asset = self.load(project, &name)?;
            chain.push(name);
            self.check_rec(project, std::slice::from_ref(&asset.object), depth + 1, chain)?;
            chain.pop();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn starter_library_objects_are_valid() {
        let list: Vec<super::AssetFile> = serde_json::from_str(super::STARTER).unwrap();
        assert!(list.len() >= 10);
        for a in list {
            let scene = cf_scene::Scene::from_json(&serde_json::json!({"objects": [a.object]}).to_string());
            assert!(scene.is_ok(), "{}: {:?}", a.name, scene.err());
        }
    }
}
