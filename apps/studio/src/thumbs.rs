//! Vorschaubilder der Shots für die Film-Leiste. Ist eine Storyboard-Szene geöffnet und
//! nichts zu tun, wird ein kleines Bild aus der Mitte des Shots gerendert und unter
//! `<Projekt>/Export/.vorschau/<szene>.png` gespeichert (bleibt nach Neustart erhalten).

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::SystemTime,
};

use base64::Engine;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    project::ActiveProject,
    render::{JobRequest, RenderQueue, ViewSpec},
    source::SceneSource,
    timeline::Playhead,
    tools::{Content, ToolOutput},
};

#[derive(Resource, Default)]
pub struct ShotThumbs {
    pub images: HashMap<String, Handle<Image>>,
    /// Stand der Szenen-Datei, für den das Bild gilt.
    stamps: HashMap<String, Option<SystemTime>>,
    pending: Option<(String, Option<SystemTime>, std::sync::Mutex<Receiver<ToolOutput>>)>,
    /// Bilder geändert ⇒ Film-Leiste neu aufbauen.
    pub changed: bool,
    loaded_dir: Option<PathBuf>,
    /// Fehlversuche je Szene (schwarzes Bild), höchstens 3.
    attempts: HashMap<String, u32>,
}

fn thumb_dir(project: &ActiveProject) -> Option<PathBuf> {
    project.dir.as_ref().map(|d| d.join("Export").join(".vorschau"))
}

fn to_image(rgba: image::RgbaImage) -> Image {
    let (w, h) = rgba.dimensions();
    Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn update_thumbs(
    mut thumbs: ResMut<ShotThumbs>,
    mut jobs: ResMut<RenderQueue>,
    mut images: ResMut<Assets<Image>>,
    src: Res<SceneSource>,
    project: Res<ActiveProject>,
    ph: Res<Playhead>,
    mut loaded: MessageReader<crate::source::SceneLoaded>,
    time: Res<Time>,
    mut since_load: Local<f32>,
) {
    if loaded.read().count() > 0 {
        *since_load = 0.0;
    }
    *since_load += time.delta_secs();
    let Some(dir) = thumb_dir(&project) else { return };
    // Gespeicherte Bilder einmal je Projekt laden
    if thumbs.loaded_dir.as_ref() != Some(&dir) {
        thumbs.images.clear();
        thumbs.stamps.clear();
        thumbs.pending = None;
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                let Some(name) = p.file_stem().map(|s| s.to_string_lossy().to_string()) else { continue };
                if let Ok(img) = image::open(&p) {
                    let h = images.add(to_image(img.to_rgba8()));
                    thumbs.images.insert(name.clone(), h);
                    thumbs.stamps.insert(name, std::fs::metadata(&p).and_then(|m| m.modified()).ok());
                }
            }
        }
        thumbs.loaded_dir = Some(dir.clone());
        thumbs.changed = true;
    }
    // Fertiges Bild abholen
    if let Some((name, stamp, rx)) = thumbs.pending.take() {
        let got = rx.lock().map(|r| r.try_recv()).unwrap_or(Err(mpsc::TryRecvError::Disconnected));
        match got {
            Ok(out) => {
                if let Some(Content::Image { base64, .. }) = out.content.iter().find(|c| matches!(c, Content::Image { .. })) {
                    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(base64) {
                        if let Ok(img) = image::load_from_memory(&bytes) {
                            // Fast schwarz = Szene war noch nicht fertig aufgebaut ⇒ später erneut
                            let bright = img.to_luma8().pixels().map(|p| p.0[0] as u32).max().unwrap_or(0);
                            if bright < 12 {
                                *thumbs.attempts.entry(name).or_insert(0) += 1;
                                return;
                            }
                            let _ = std::fs::create_dir_all(&dir);
                            let _ = img.save(dir.join(format!("{name}.png")));
                            let h = images.add(to_image(img.to_rgba8()));
                            thumbs.images.insert(name.clone(), h);
                            thumbs.stamps.insert(name, stamp);
                            thumbs.changed = true;
                        }
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => thumbs.pending = Some((name, stamp, rx)),
            Err(_) => {}
        }
        return;
    }
    // Neues Bild nötig? Nur wenn nichts anderes läuft.
    // Erst wenn die Szene ein paar Sekunden steht (Shader/Modelle fertig) und nichts anderes läuft
    if *since_load < 4.0 || !jobs.idle() || jobs.export_running() || ph.playing || ph.locked_by_job {
        return;
    }
    let current = src.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if !project.data.storyboard.iter().any(|s| s.scene.as_deref() == Some(current.as_str())) {
        return;
    }
    let fresh = match (thumbs.stamps.get(&current), src.modified) {
        (Some(Some(t)), Some(m)) => *t >= m,
        _ => false,
    };
    if fresh || thumbs.attempts.get(&current).copied().unwrap_or(0) >= 3 {
        return;
    }
    let (tx, rx) = mpsc::channel();
    jobs.push(JobRequest::Still {
        reply: tx,
        time: Some(src.scene.duration * 0.5),
        view: ViewSpec { user_view: None, position: None, look_at: None, fov: None },
        width: 192,
        height: 108,
    });
    thumbs.pending = Some((current, src.modified.or(Some(SystemTime::now())), std::sync::Mutex::new(rx)));
}
