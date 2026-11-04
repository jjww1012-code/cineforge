//! Render-Aufträge: Kontrollbild, Kontaktabzug, Video-Export.
//!
//! Ein Auftrag rendert über eine unsichtbare Kamera in ein Bild beliebiger
//! Größe (bis 4K) und steuert dabei die Zeit. Das Fenster des Menschen zeigt
//! währenddessen dieselbe Zeit an.
//!
//! Der Export arbeitet als Fließband: In jedem Frame wird der nächste
//! Zeitpunkt eingestellt und ein Bild angefordert; fertige Bilder gehen an
//! den Schreib-Thread (ffmpeg). So ist der Export so schnell, wie die GPU
//! rendern kann.

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        mpsc::{Sender, SyncSender},
        Arc, Mutex,
    },
    time::Instant,
};

use base64::Engine as _;
use bevy::{
    camera::RenderTarget,
    prelude::*,
    render::{
        render_resource::TextureFormat,
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    window::{PresentMode, PrimaryWindow},
};

use crate::{
    environment::{configure_view, SceneView},
    source::SceneSource,
    timeline::{scene_camera_at, Playhead, TimelineSet},
    tools::{Content, ToolOutput},
    video::{self, ExportProgress, Quality},
};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderQueue>().add_systems(Update, drive_jobs.before(TimelineSet));
    }
}

/// Blickpunkt für Kontrollbilder.
#[derive(Debug, Clone, Default)]
pub struct ViewSpec {
    pub user_view: Option<Transform>,
    pub position: Option<Vec3>,
    pub look_at: Option<Vec3>,
    pub fov: Option<f32>,
}

pub enum JobRequest {
    Still { reply: Sender<ToolOutput>, time: Option<f32>, view: ViewSpec, width: u32, height: u32 },
    Sheet { reply: Sender<ToolOutput>, times: Vec<f32>, width: u32, height: u32, columns: u32 },
    Export { output: PathBuf, width: u32, height: u32, fps: u32, start: f32, end: f32, quality: Quality },
}

/// Warteschlange + Status des laufenden/letzten Exports.
#[derive(Resource, Default)]
pub struct RenderQueue {
    pending: VecDeque<JobRequest>,
    active: Option<ActiveJob>,
    /// Fortschritt des aktuellen bzw. letzten Exports (für export_status / Anzeige).
    pub export: Option<Arc<Mutex<ExportProgress>>>,
    pub export_started: Option<Instant>,
    cancel: bool,
}

impl RenderQueue {
    pub fn push(&mut self, job: JobRequest) {
        self.pending.push_back(job);
    }

    pub fn export_running(&self) -> bool {
        self.export.as_ref().is_some_and(|p| !p.lock().map(|p| p.done).unwrap_or(true))
    }

    /// Kamera des laufenden Auftrags (für Billboards).
    pub fn active_camera(&self) -> Option<Entity> {
        self.active.as_ref().map(|j| j.camera)
    }

    pub fn cancel_export(&mut self) {
        self.cancel = true;
    }

    /// Kurztext für Anzeige/Werkzeuge.
    pub fn export_summary(&self) -> Option<String> {
        let p = self.export.as_ref()?.lock().ok()?.clone();
        let elapsed = self.export_started.map(|s| s.elapsed().as_secs_f32()).unwrap_or(p.seconds).max(0.001);
        Some(if let Some(e) = &p.error {
            format!("Export fehlgeschlagen: {e}")
        } else if p.done {
            format!(
                "Export fertig: {} ({} Bilder in {:.1} s = {:.1} Bilder/s, Encoder {})",
                p.output.display(),
                p.written,
                p.seconds,
                p.written as f32 / p.seconds.max(0.001),
                p.encoder
            )
        } else {
            let pct = p.written as f32 / p.total.max(1) as f32 * 100.0;
            let rate = p.written as f32 / elapsed;
            let eta = if rate > 0.0 { (p.total - p.written) as f32 / rate } else { 0.0 };
            format!("Export läuft: {pct:.0} % ({}/{} Bilder, {rate:.1} Bilder/s, noch ca. {eta:.0} s)", p.written, p.total)
        })
    }
}

enum Stage {
    Still { reply: Sender<ToolOutput>, time: Option<f32>, fixed: Option<(Transform, Option<f32>)>, done: Arc<Mutex<bool>> },
    Sheet { reply: Sender<ToolOutput>, times: Vec<f32>, next: usize, columns: u32, results: Arc<Mutex<Vec<Option<Image>>>> },
    Export { frames: u32, next: u32, fps: u32, start: f32, tx: Option<SyncSender<(u32, Vec<u8>)>>, captured: Arc<Mutex<u32>>, last_progress: Instant, last_count: u32 },
}

struct ActiveJob {
    camera: Entity,
    image: Handle<Image>,
    warmup: u32,
    waiting: bool,
    stage: Stage,
    saved_time: f32,
    saved_present: Option<PresentMode>,
}

fn warmup_frames() -> u32 {
    std::env::var("CINEFORGE_CAPTURE_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(8)
}

fn spawn_camera(commands: &mut Commands, images: &mut Assets<Image>, scene: &cf_scene::Scene, w: u32, h: u32) -> (Entity, Handle<Image>) {
    let image = images.add(Image::new_target_texture(w, h, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb)));
    let mut cam = commands.spawn((
        Name::new("Aufnahme-Kamera"),
        SceneView,
        Camera3d::default(),
        Camera { order: -1, ..default() },
        RenderTarget::Image(image.clone().into()),
        Msaa::Sample4,
        Transform::default(),
    ));
    configure_view(&mut cam, scene);
    let id = cam.id();
    (id, image)
}

#[allow(clippy::too_many_arguments)]
fn drive_jobs(
    mut commands: Commands,
    mut queue: ResMut<RenderQueue>,
    mut ph: ResMut<Playhead>,
    src: Res<SceneSource>,
    mut images: ResMut<Assets<Image>>,
    mut cams: Query<(&mut Transform, &mut Projection), Without<crate::MainCamera>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    // Neuen Auftrag starten
    if queue.active.is_none() {
        let Some(req) = queue.pending.pop_front() else { return };
        let saved_time = ph.time;
        ph.locked_by_job = true;
        ph.playing = false;
        let (w, h) = match &req {
            JobRequest::Still { width, height, .. } | JobRequest::Sheet { width, height, .. } => (*width, *height),
            JobRequest::Export { width, height, .. } => (*width, *height),
        };
        let (camera, image) = spawn_camera(&mut commands, &mut images, &src.scene, w, h);
        let mut saved_present = None;
        let stage = match req {
            JobRequest::Still { reply, time, view, .. } => {
                let fixed = if view.user_view.is_some() || view.position.is_some() {
                    let base = view.user_view.unwrap_or_else(|| scene_camera_at(&src.scene, time.unwrap_or(saved_time)).0);
                    let mut tf = base;
                    if let Some(p) = view.position {
                        tf.translation = p;
                    }
                    if let Some(l) = view.look_at {
                        if (l - tf.translation).length_squared() > 1e-6 {
                            tf.look_at(l, Vec3::Y);
                        }
                    }
                    Some((tf, view.fov))
                } else if let Some(l) = view.look_at {
                    let mut tf = scene_camera_at(&src.scene, time.unwrap_or(saved_time)).0;
                    tf.look_at(l, Vec3::Y);
                    Some((tf, view.fov))
                } else {
                    view.fov.map(|f| (scene_camera_at(&src.scene, time.unwrap_or(saved_time)).0, Some(f)))
                };
                Stage::Still { reply, time, fixed, done: Arc::new(Mutex::new(false)) }
            }
            JobRequest::Sheet { reply, times, columns, .. } => {
                let n = times.len();
                Stage::Sheet { reply, times, next: 0, columns, results: Arc::new(Mutex::new(vec![None; n])) }
            }
            JobRequest::Export { output, width, height, fps, start, end, quality } => {
                let frames = (((end - start) * fps as f32).round() as u32).max(1);
                let (tx, rx) = std::sync::mpsc::sync_channel(8);
                match video::start_writer(output, width, height, fps, frames, quality, rx) {
                    Ok(progress) => {
                        queue.export = Some(progress);
                        queue.export_started = Some(Instant::now());
                    }
                    Err(e) => {
                        queue.export = Some(Arc::new(Mutex::new(ExportProgress { done: true, error: Some(e), ..default() })));
                        commands.entity(camera).despawn();
                        images.remove(&image);
                        ph.locked_by_job = false;
                        return;
                    }
                }
                // Während des Exports nicht auf die Bildwiederholrate des Monitors warten.
                if let Ok(mut win) = window.single_mut() {
                    saved_present = Some(win.present_mode);
                    win.present_mode = PresentMode::AutoNoVsync;
                }
                queue.cancel = false;
                Stage::Export {
                    frames,
                    next: 0,
                    fps,
                    start,
                    tx: Some(tx),
                    captured: Arc::new(Mutex::new(0)),
                    last_progress: Instant::now(),
                    last_count: 0,
                }
            }
        };
        queue.active = Some(ActiveJob { camera, image, warmup: warmup_frames(), waiting: false, stage, saved_time, saved_present });
        return;
    }

    let cancel = std::mem::take(&mut queue.cancel);
    let mut job = queue.active.take().expect("aktiv");
    let scene = &src.scene;
    let set_camera = |cams: &mut Query<(&mut Transform, &mut Projection), Without<crate::MainCamera>>, tf: Transform, fov: f32| {
        if let Ok((mut t, mut p)) = cams.get_mut(job.camera) {
            *t = tf;
            if let Projection::Perspective(pp) = p.as_mut() {
                pp.fov = fov.clamp(5.0, 150.0).to_radians();
            }
        }
    };

    let mut finished = false;
    match &mut job.stage {
        Stage::Still { reply, time, fixed, done } => {
            let t = time.unwrap_or(job.saved_time);
            ph.time = t;
            let (tf, fov) = match fixed {
                Some((tf, fov)) => (*tf, fov.unwrap_or_else(|| scene_camera_at(scene, t).1)),
                None => scene_camera_at(scene, t),
            };
            set_camera(&mut cams, tf, fov);
            if job.warmup > 0 {
                job.warmup -= 1;
            } else if !job.waiting {
                job.waiting = true;
                let reply = reply.clone();
                let done = done.clone();
                let (w, h) = images.get(&job.image).map(|i| (i.width(), i.height())).unwrap_or((0, 0));
                commands.spawn(Screenshot::image(job.image.clone())).observe(move |cap: On<ScreenshotCaptured>| {
                    let out = match encode_jpeg(&cap.image) {
                        Ok((b64, _, _)) => ToolOutput {
                            content: vec![
                                Content::Image { base64: b64, mime: "image/jpeg" },
                                Content::Text(format!("Aufnahme {w}×{h} bei {}", crate::timeline::format_time(t))),
                            ],
                            is_error: false,
                        },
                        Err(e) => ToolOutput::error(format!("Bild konnte nicht erstellt werden: {e}")),
                    };
                    let _ = reply.send(out);
                    if let Ok(mut d) = done.lock() {
                        *d = true;
                    }
                });
            } else if done.lock().map(|d| *d).unwrap_or(true) {
                // Bild ist zurückgelesen und beantwortet → aufräumen.
                finished = true;
            }
        }
        Stage::Sheet { reply, times, next, columns, results } => {
            if *next < times.len() {
                let t = times[*next];
                ph.time = t;
                let (tf, fov) = scene_camera_at(scene, t);
                set_camera(&mut cams, tf, fov);
                if job.warmup > 0 {
                    job.warmup -= 1;
                } else {
                    let idx = *next;
                    let res = results.clone();
                    commands.spawn(Screenshot::image(job.image.clone())).observe(move |cap: On<ScreenshotCaptured>| {
                        if let Ok(mut r) = res.lock() {
                            r[idx] = Some(cap.image.clone());
                        }
                    });
                    *next += 1;
                    job.warmup = 1;
                }
            } else {
                let all = results.lock().map(|r| r.iter().all(Option::is_some)).unwrap_or(false);
                if all {
                    let imgs: Vec<Image> = results.lock().expect("Ergebnisse").iter().flatten().cloned().collect();
                    let _ = reply.send(compose_sheet(&imgs, times, *columns));
                    finished = true;
                }
            }
        }
        Stage::Export { frames, next, fps, start, tx, captured, last_progress, last_count } => {
            let got = *captured.lock().expect("Zähler");
            if got != *last_count {
                *last_count = got;
                *last_progress = Instant::now();
            }
            if cancel {
                tx.take(); // Schreib-Thread bricht ab
                if let Some(p) = &queue.export {
                    if let Ok(mut p) = p.lock() {
                        p.error = Some("vom Benutzer abgebrochen".into());
                    }
                }
                finished = true;
            } else if *next < *frames {
                let t = *start + *next as f32 / *fps as f32;
                ph.time = t;
                let (tf, fov) = scene_camera_at(scene, t);
                set_camera(&mut cams, tf, fov);
                if job.warmup > 0 {
                    job.warmup -= 1;
                } else if let Some(sender) = tx.clone() {
                    let idx = *next;
                    let count = captured.clone();
                    commands.spawn(Screenshot::image(job.image.clone())).observe(move |cap: On<ScreenshotCaptured>| {
                        if let Some(data) = cap.image.data.clone() {
                            let _ = sender.send((idx, data));
                        }
                        if let Ok(mut c) = count.lock() {
                            *c += 1;
                        }
                    });
                    *next += 1;
                }
            } else if got >= *frames {
                tx.take(); // alle Bilder unterwegs → Kanal schließen
                finished = true;
            } else if last_progress.elapsed().as_secs() > 60 {
                tx.take();
                if let Some(p) = &queue.export {
                    if let Ok(mut p) = p.lock() {
                        p.error = Some(format!("Zeitüberschreitung: nur {got} von {frames} Bildern kamen an"));
                    }
                }
                finished = true;
            }
        }
    }

    if finished {
        commands.entity(job.camera).despawn();
        images.remove(&job.image);
        ph.time = job.saved_time;
        ph.locked_by_job = false;
        if let (Some(pm), Ok(mut win)) = (job.saved_present, window.single_mut()) {
            win.present_mode = pm;
        }
    } else {
        queue.active = Some(job);
    }
}

pub fn encode_jpeg(img: &Image) -> Result<(String, u32, u32), String> {
    let dynamic = img.clone().try_into_dynamic().map_err(|e| e.to_string())?;
    let rgb = dynamic.to_rgb8();
    let (w, h) = rgb.dimensions();
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 88).encode_image(&rgb).map_err(|e| e.to_string())?;
    Ok((base64::engine::general_purpose::STANDARD.encode(bytes), w, h))
}

/// Setzt mehrere Bilder zu einem Raster zusammen (Lesereihenfolge: Zeile für Zeile).
fn compose_sheet(imgs: &[Image], times: &[f32], columns: u32) -> ToolOutput {
    let frames: Vec<image::RgbImage> =
        imgs.iter().filter_map(|i| i.clone().try_into_dynamic().ok()).map(|d| d.to_rgb8()).collect();
    let Some(first) = frames.first() else { return ToolOutput::error("Keine Bilder erhalten") };
    let (cw, ch) = first.dimensions();
    let cols = columns.clamp(1, frames.len() as u32);
    let rows = (frames.len() as u32).div_ceil(cols);
    let gap = 4;
    let mut sheet = image::RgbImage::from_pixel(cols * cw + (cols - 1) * gap, rows * ch + (rows - 1) * gap, image::Rgb([20, 20, 24]));
    for (i, f) in frames.iter().enumerate() {
        let (c, r) = (i as u32 % cols, i as u32 / cols);
        image::imageops::replace(&mut sheet, f, (c * (cw + gap)) as i64, (r * (ch + gap)) as i64);
    }
    let mut bytes = Vec::new();
    if let Err(e) = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85).encode_image(&sheet) {
        return ToolOutput::error(e.to_string());
    }
    let labels: Vec<String> = times.iter().enumerate().map(|(i, t)| format!("{}: {}", i + 1, crate::timeline::format_time(*t))).collect();
    ToolOutput {
        content: vec![
            Content::Image { base64: base64::engine::general_purpose::STANDARD.encode(bytes), mime: "image/jpeg" },
            Content::Text(format!(
                "Kontaktabzug {cols}×{rows}, Lesereihenfolge zeilenweise von links oben:\n{}",
                labels.join(" · ")
            )),
        ],
        is_error: false,
    }
}
