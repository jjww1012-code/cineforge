//! Film-Export für lange Filme (bis über 1 Stunde).
//!
//! - **Shot-Cache:** Jeder Shot wird einzeln als Videodatei gerendert und unter
//!   einem Fingerabdruck seines Inhalts gespeichert (`Export/.cache/`). Ändert
//!   sich ein Shot nicht, wird er nicht neu gerendert. Bricht ein Export ab,
//!   geht er beim nächsten Mal mit dem ersten fehlenden Shot weiter.
//! - **Video-Clips** (KI-Videos, Aufnahmen) werden auf Format und Bildrate des
//!   Films gebracht und wie Shots behandelt.
//! - **Zusammenfügen ohne Qualitätsverlust** (ffmpeg concat) und **Ton-Mischung**
//!   (Soundtrack, Ton je Shot, Originalton der Clips).

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Instant,
};

use bevy::prelude::*;
use cf_scene::project::{format_duration, AudioClip, Project};

use crate::{
    library::Library,
    media,
    project::ActiveProject,
    render::{JobRequest, RenderQueue, Segment},
    video::{self, Quality},
};

/// Wird erhöht, wenn sich die Darstellung ändert (macht alte Cache-Dateien ungültig).
const CACHE_VERSION: u32 = 3; // 3: Kontaktschatten (SSAO) im Film-Stil, neue Darstellung
const FADE: f32 = 0.4;

#[derive(Debug, Clone)]
pub enum TaskKind {
    Scene { path: PathBuf },
    Clip { file: PathBuf, offset: f32 },
}

#[derive(Debug, Clone)]
pub struct FilmTask {
    pub shot_id: String,
    pub kind: TaskKind,
    pub duration: f32,
    pub fade_in: bool,
    pub fade_out: bool,
    pub cache: PathBuf,
    /// Überblendung zum nächsten Shot (ffmpeg-xfade-Art, Dauer).
    pub xfade: Option<(&'static str, f32)>,
}

/// Dauer einer Überblendung (Sekunden).
pub const XFADE_SECONDS: f32 = 0.6;

/// Übergangs-Name → ffmpeg-xfade-Art (None = harter Schnitt bzw. Abblende über Schwarz).
pub fn xfade_kind(name: &str) -> Option<&'static str> {
    match name {
        "dissolve" | "ueberblenden" | "überblenden" => Some("fade"),
        "wipe" | "wischen" => Some("wipeleft"),
        "slide" | "schieben" => Some("slideleft"),
        "circle" | "kreis" | "iris" => Some("circleopen"),
        _ => None,
    }
}

/// Alle erlaubten Übergänge (für Prüfung und Hilfe).
pub const TRANSITIONS: &str = "cut, fade, dissolve, wipe, slide, circle";

/// Ton mit absoluter Startzeit im Film.
#[derive(Debug, Clone)]
pub struct MixClip {
    pub path: PathBuf,
    pub start: f32,
    pub offset: f32,
    pub duration: Option<f32>,
    pub volume: f32,
    pub fade_in: f32,
    pub fade_out: f32,
}

type Shared = Arc<Mutex<Option<Result<(), String>>>>;

enum State {
    Idle,
    Rendering,
    Transcoding(Shared),
    Finalizing(Shared),
    Done,
    Failed(String),
}

pub struct FilmPlan {
    pub tasks: Vec<FilmTask>,
    pub audio: Vec<MixClip>,
    pub output: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub quality: Quality,
    pub total: f32,
    idx: usize,
    state: State,
    cached: usize,
    rendered: usize,
    started: Instant,
    finished: Option<Instant>,
}

impl FilmPlan {
    pub fn running(&self) -> bool {
        !matches!(self.state, State::Done | State::Failed(_))
    }

    pub fn cancel(&mut self) {
        if self.running() {
            self.state = State::Failed("vom Benutzer abgebrochen".into());
            self.finished = Some(Instant::now());
        }
    }

    pub fn summary(&self, current_export: Option<String>) -> String {
        let n = self.tasks.len();
        let elapsed = self.finished.unwrap_or_else(Instant::now).duration_since(self.started).as_secs_f32();
        match &self.state {
            State::Done => format!(
                "Film fertig: {} ({}, {n} Shots – {} neu gerendert, {} aus dem Cache) in {}",
                self.output.display(),
                format_duration(self.total),
                self.rendered,
                self.cached,
                format_duration(elapsed)
            ),
            State::Failed(e) => format!(
                "Film-Export fehlgeschlagen bei Shot {}/{n}: {e}\nBereits gerenderte Shots bleiben im Cache – ein neuer export_film macht dort weiter.",
                (self.idx + 1).min(n)
            ),
            State::Finalizing(_) => format!("Film wird zusammengefügt und der Ton gemischt … ({n} Shots, {})", format_duration(self.total)),
            _ => {
                let done_secs: f32 = self.tasks[..self.idx.min(n)].iter().map(|t| t.duration).sum();
                let shot = self.tasks.get(self.idx).map(|t| t.shot_id.as_str()).unwrap_or("–");
                let detail = match &self.state {
                    State::Transcoding(_) => "Clip wird umgewandelt".to_string(),
                    _ => current_export.unwrap_or_default(),
                };
                format!(
                    "Film-Export läuft: Shot {}/{n} („{shot}“) · {} von {} gerendert · {} aus dem Cache · seit {}\n{detail}",
                    (self.idx + 1).min(n),
                    format_duration(done_secs),
                    format_duration(self.total),
                    self.cached,
                    format_duration(elapsed)
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Plan erstellen
// ---------------------------------------------------------------------------

pub struct PlanOptions {
    pub width: u32,
    pub height: u32,
    pub fps: Option<u32>,
    pub quality: Quality,
    pub fade_all: bool,
    /// Übergang für alle Shots (dissolve, wipe, slide, circle), sonst je Shot.
    pub transition_all: Option<String>,
    pub output: PathBuf,
}

fn hash_file(h: &mut DefaultHasher, p: &Path) {
    if let Ok(meta) = std::fs::metadata(p) {
        meta.len().hash(h);
        if let Ok(m) = meta.modified() {
            m.hash(h);
        }
    }
    p.hash(h);
}

fn resolve_audio(project: &ActiveProject, a: &AudioClip, base: f32) -> Result<MixClip, String> {
    Ok(MixClip {
        path: media::resolve(project, media::AUDIO, &a.file)?,
        start: base + a.start,
        offset: a.offset,
        duration: a.duration,
        volume: a.volume.clamp(0.0, 4.0),
        fade_in: a.fade_in.max(0.0),
        fade_out: a.fade_out.max(0.0),
    })
}

/// Prüft das Storyboard und baut den Plan. Liefert alle Probleme auf einmal.
pub fn build_plan(project: &ActiveProject, library: &mut Library, opt: PlanOptions) -> Result<FilmPlan, Vec<String>> {
    let data: &Project = &project.data;
    let mut problems = Vec::new();
    if data.storyboard.is_empty() {
        return Err(vec!["Das Storyboard ist leer – erst mit update_plan Shots anlegen.".into()]);
    }
    let scenes_dir = project.scenes_dir().map_err(|e| vec![e])?;
    let export_dir = project.export_dir().map_err(|e| vec![e])?;
    // Je Format/Qualität ein eigener Cache-Ordner (16:9 und 9:16 löschen sich nicht gegenseitig).
    let cache_dir = export_dir.join(".cache").join(format!(
        "{}x{}_{}fps_{:?}",
        opt.width,
        opt.height,
        opt.fps.map(|f| f.to_string()).unwrap_or_else(|| "auto".into()),
        opt.quality
    ).to_lowercase());
    let mut fps = opt.fps;
    let mut tasks = Vec::new();
    let mut audio = Vec::new();
    let mut film_t = 0.0f32;
    let n = data.storyboard.len();
    // Vorab laden, damit die Bildrate feststeht, bevor Fingerabdrücke berechnet werden.
    let mut loaded = Vec::new();
    for shot in &data.storyboard {
        let scene = shot.scene.as_ref().map(|name| {
            let path = scenes_dir.join(format!("{name}.json"));
            let parsed = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| cf_scene::Scene::from_json(&t).map(|s| (s, t)).map_err(|e| e.to_string()));
            (path, parsed)
        });
        if let Some((_, Ok((s, _)))) = &scene {
            fps.get_or_insert(s.fps);
        }
        loaded.push(scene);
    }
    let fps = fps.unwrap_or(30).clamp(1, 120);
    for (i, (shot, scene)) in data.storyboard.iter().zip(loaded).enumerate() {
        let fade = |at: usize| data.storyboard.get(at).and_then(|s| s.transition.as_deref()) == Some("fade");
        let trans_name = opt.transition_all.clone().or_else(|| shot.transition.clone()).unwrap_or_else(|| "cut".into());
        if !matches!(trans_name.as_str(), "cut" | "fade") && xfade_kind(&trans_name).is_none() {
            problems.push(format!("Shot {}: Übergang „{trans_name}“ unbekannt ({TRANSITIONS})", shot.id));
        }
        let xfade = if i + 1 < n { xfade_kind(&trans_name).map(|k| (k, XFADE_SECONDS)) } else { None };
        let fade_in = i == 0 || opt.fade_all || (i > 0 && fade(i - 1));
        let fade_out = i + 1 == n || opt.fade_all || fade(i);
        let mut h = DefaultHasher::new();
        (CACHE_VERSION, opt.width, opt.height, fps, format!("{:?}", opt.quality), fade_in, fade_out).hash(&mut h);
        let mut auto_audio: Vec<(String, f32)> = Vec::new();
        let (kind, duration) = match (scene, &shot.clip) {
            (Some((path, Ok((s, text)))), _) => {
                // Stimmen sprechender Figuren und die Geräusch-Spur der Szene gehören automatisch dazu
                s.walk(&mut |o| {
                    if let Some(t) = o.animate.as_ref().and_then(|a| a.talk.as_ref()) {
                        auto_audio.push((t.audio.clone(), t.start));
                    }
                    if let cf_scene::Kind::Character(c) = &o.kind {
                        if let Some(t) = &c.talk {
                            auto_audio.push((t.audio.clone(), t.start));
                        }
                    }
                });
                if let Some(stem) = path.file_stem() {
                    let sfx = format!("sfx_{}.wav", stem.to_string_lossy());
                    if media::resolve(project, media::AUDIO, &sfx).is_ok() {
                        auto_audio.push((sfx, 0.0));
                    }
                }
                text.hash(&mut h);
                // Bibliotheks-Objekte gehören zum Inhalt
                let mut refs = Vec::new();
                cf_scene::library::referenced_assets(&s.objects, &mut refs);
                for r in refs {
                    match library.load(project, &r) {
                        Ok(a) => a.to_json_pretty().hash(&mut h),
                        Err(e) => problems.push(format!("Shot {}: {e}", shot.id)),
                    }
                }
                if let Err(e) = library.check(project, &s.objects) {
                    problems.push(format!("Shot {}: {e}", shot.id));
                }
                // Bilder/Texturen der Szene (Änderung der Datei → neu rendern)
                s.walk(&mut |o| {
                    let files = [
                        match &o.kind {
                            cf_scene::Kind::Image { file, .. } => Some(file.clone()),
                            _ => None,
                        },
                        o.material.texture.clone(),
                    ];
                    for f in files.into_iter().flatten() {
                        if let Ok(p) = media::resolve(project, media::IMAGES, &f) {
                            hash_file(&mut h, &p);
                        }
                    }
                });
                let d = if shot.duration > 0.0 { shot.duration } else { s.duration };
                d.to_bits().hash(&mut h);
                (TaskKind::Scene { path }, d)
            }
            (Some((path, Err(e))), _) => {
                problems.push(format!("Shot {}: Szene {} nicht ladbar ({e})", shot.id, path.display()));
                continue;
            }
            (None, Some(c)) => match media::resolve(project, media::CLIPS, &c.file) {
                Ok(file) => {
                    hash_file(&mut h, &file);
                    c.offset.to_bits().hash(&mut h);
                    let info = media::probe(&file).unwrap_or_default();
                    let d = if shot.duration > 0.0 {
                        shot.duration
                    } else {
                        info.duration.map(|d| (d - c.offset).max(0.1)).unwrap_or(5.0)
                    };
                    d.to_bits().hash(&mut h);
                    if !info.has_video {
                        problems.push(format!("Shot {}: „{}“ enthält kein Video", shot.id, c.file));
                    }
                    if c.use_audio && info.has_audio {
                        audio.push(MixClip {
                            path: file.clone(),
                            start: film_t,
                            offset: c.offset,
                            duration: Some(d),
                            volume: c.volume,
                            fade_in: 0.05,
                            fade_out: 0.05,
                        });
                    }
                    (TaskKind::Clip { file, offset: c.offset }, d)
                }
                Err(e) => {
                    problems.push(format!("Shot {}: {e}", shot.id));
                    continue;
                }
            },
            (None, None) => {
                problems.push(format!("Shot {} „{}“ hat weder Szene noch Clip", shot.id, shot.title));
                continue;
            }
        };
        for a in &shot.audio {
            match resolve_audio(project, a, film_t) {
                Ok(m) => audio.push(m),
                Err(e) => problems.push(format!("Shot {} Ton: {e}", shot.id)),
            }
        }
        for (file, start) in auto_audio {
            if shot.audio.iter().any(|a| a.file == file) {
                continue; // schon eingetragen (z. B. mit eigener Lautstärke)
            }
            match media::resolve(project, media::AUDIO, &file) {
                Ok(path) => audio.push(MixClip {
                    path,
                    start: film_t + start,
                    offset: 0.0,
                    duration: Some((duration - start).max(0.05)),
                    volume: if file.starts_with("sfx_") { 0.7 } else { 1.0 },
                    fade_in: 0.0,
                    fade_out: 0.05,
                }),
                Err(e) => problems.push(format!("Shot {} Ton: {e}", shot.id)),
            }
        }
        let safe_id: String = shot.id.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
        tasks.push(FilmTask {
            shot_id: shot.id.clone(),
            kind,
            duration,
            fade_in,
            fade_out,
            cache: cache_dir.join(format!("{safe_id}_{:016x}.mp4", h.finish())),
            xfade,
        });
        // Bei Überblendung beginnt der nächste Shot schon während dieser ausklingt.
        film_t += duration - xfade.map(|(_, d)| d.min(duration * 0.5)).unwrap_or(0.0);
    }
    for a in &data.soundtrack {
        match resolve_audio(project, a, 0.0) {
            Ok(m) => audio.push(m),
            Err(e) => problems.push(format!("Soundtrack: {e}")),
        }
    }
    if video::find_ffmpeg().is_none() {
        problems.push("ffmpeg nicht gefunden – für Filme mit Cache, Clips und Ton nötig (siehe README).".into());
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let _ = std::fs::create_dir_all(&cache_dir);
    Ok(FilmPlan {
        tasks,
        audio,
        output: opt.output,
        width: opt.width,
        height: opt.height,
        fps,
        quality: opt.quality,
        total: film_t,
        idx: 0,
        state: State::Idle,
        cached: 0,
        rendered: 0,
        started: Instant::now(),
        finished: None,
    })
}

/// Übersicht für check_film.
pub fn describe_plan(plan: &FilmPlan) -> String {
    let cached = plan.tasks.iter().filter(|t| t.cache.is_file()).count();
    let todo: f32 = plan.tasks.iter().filter(|t| !t.cache.is_file()).map(|t| t.duration).sum();
    let clips = plan.tasks.iter().filter(|t| matches!(t.kind, TaskKind::Clip { .. })).count();
    format!(
        "Film: {} Shots ({} Clips), Länge {}, {}×{} bei {} fps\n\
         Ton: {} Spuren\n\
         Cache: {cached} Shots fertig, noch zu rendern: {} Shots ({} Filmzeit = {} Bilder)\n\
         Ziel: {}",
        plan.tasks.len(),
        clips,
        format_duration(plan.total),
        plan.width,
        plan.height,
        plan.fps,
        plan.audio.len(),
        plan.tasks.len() - cached,
        format_duration(todo),
        (todo * plan.fps as f32).round() as u64,
        plan.output.display()
    )
}

// ---------------------------------------------------------------------------
// Ablauf
// ---------------------------------------------------------------------------

pub struct FilmPlugin;

impl Plugin for FilmPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, drive_film.before(crate::timeline::TimelineSet));
    }
}

fn part_path(cache: &Path) -> PathBuf {
    cache.with_extension("part.mp4")
}

fn drive_film(mut queue: ResMut<RenderQueue>, mut project: ResMut<ActiveProject>) {
    let Some(mut plan) = queue.film.take() else { return };
    if plan.running() {
        step(&mut plan, &mut queue);
        if !plan.running() {
            plan.finished = Some(Instant::now());
            let msg = plan.summary(None);
            info!("{msg}");
            project.log("system", msg.lines().next().unwrap_or("").to_string());
        }
    }
    queue.film = Some(plan);
}

fn step(plan: &mut FilmPlan, queue: &mut RenderQueue) {
    match &plan.state {
        State::Idle => {
            if plan.idx >= plan.tasks.len() {
                let shared: Shared = Arc::new(Mutex::new(None));
                let (s2, tasks, audio, out, total) =
                    (shared.clone(), plan.tasks.clone(), plan.audio.clone(), plan.output.clone(), plan.total);
                let enc = (plan.width, plan.height, plan.fps, plan.quality);
                std::thread::spawn(move || {
                    let r = finalize(&tasks, &audio, &out, total, enc);
                    *s2.lock().expect("Status") = Some(r);
                });
                plan.state = State::Finalizing(shared);
                return;
            }
            let task = plan.tasks[plan.idx].clone();
            if task.cache.is_file() {
                plan.cached += 1;
                plan.idx += 1;
                return;
            }
            match &task.kind {
                TaskKind::Scene { path } => {
                    queue.push(JobRequest::Export {
                        output: part_path(&task.cache),
                        width: plan.width,
                        height: plan.height,
                        fps: plan.fps,
                        segments: vec![Segment {
                            path: Some(path.clone()),
                            start: 0.0,
                            duration: task.duration,
                            fade_in: task.fade_in,
                            fade_out: task.fade_out,
                        }],
                        quality: plan.quality,
                    });
                    plan.state = State::Rendering;
                }
                TaskKind::Clip { file, offset } => {
                    let shared: Shared = Arc::new(Mutex::new(None));
                    let s2 = shared.clone();
                    let (file, offset, w, h, fps, q) = (file.clone(), *offset, plan.width, plan.height, plan.fps, plan.quality);
                    let out = part_path(&task.cache);
                    std::thread::spawn(move || {
                        let r = transcode_clip(&file, offset, task.duration, task.fade_in, task.fade_out, w, h, fps, q, &out);
                        *s2.lock().expect("Status") = Some(r);
                    });
                    plan.state = State::Transcoding(shared);
                }
            }
        }
        State::Rendering => {
            if !queue.idle() {
                return;
            }
            let err = queue.export.as_ref().and_then(|p| p.lock().ok().and_then(|p| p.error.clone()));
            let task = &plan.tasks[plan.idx];
            match err {
                Some(e) => plan.state = State::Failed(e),
                None => match std::fs::rename(part_path(&task.cache), &task.cache) {
                    Ok(()) => {
                        plan.rendered += 1;
                        plan.idx += 1;
                        plan.state = State::Idle;
                    }
                    Err(e) => plan.state = State::Failed(format!("Shot-Datei fehlt: {e}")),
                },
            }
        }
        State::Transcoding(shared) => {
            let r = shared.lock().expect("Status").clone();
            if let Some(r) = r {
                let task = &plan.tasks[plan.idx];
                plan.state = match r.and_then(|_| std::fs::rename(part_path(&task.cache), &task.cache).map_err(|e| e.to_string())) {
                    Ok(()) => {
                        plan.rendered += 1;
                        plan.idx += 1;
                        State::Idle
                    }
                    Err(e) => State::Failed(e),
                };
            }
        }
        State::Finalizing(shared) => {
            let r = shared.lock().expect("Status").clone();
            if let Some(r) = r {
                plan.state = match r {
                    Ok(()) => {
                        cleanup_cache(&plan.tasks);
                        State::Done
                    }
                    Err(e) => State::Failed(e),
                };
            }
        }
        State::Done | State::Failed(_) => {}
    }
}

/// Entfernt veraltete Shot-Dateien (gleiches Format, aber nicht mehr im Film).
fn cleanup_cache(tasks: &[FilmTask]) {
    let Some(dir) = tasks.first().and_then(|t| t.cache.parent()) else { return };
    let keep: std::collections::HashSet<&Path> = tasks.iter().map(|t| t.cache.as_path()).collect();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "mp4") && !keep.contains(p.as_path()) {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

fn run_ffmpeg(args: Vec<String>) -> Result<(), String> {
    let ff = video::find_ffmpeg().ok_or("ffmpeg nicht gefunden")?;
    let out = Command::new(ff).args(&args).output().map_err(|e| format!("ffmpeg: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(format!("ffmpeg-Fehler: {}", err.lines().rev().take(4).collect::<Vec<_>>().join(" | ")))
    }
}

/// Bringt einen Clip auf Größe, Bildrate und Länge des Films (mit Rand statt Verzerrung).
#[allow(clippy::too_many_arguments)]
fn transcode_clip(
    file: &Path,
    offset: f32,
    duration: f32,
    fade_in: bool,
    fade_out: bool,
    w: u32,
    h: u32,
    fps: u32,
    q: Quality,
    out: &Path,
) -> Result<(), String> {
    let ff = video::find_ffmpeg().ok_or("ffmpeg nicht gefunden")?;
    let enc = video::best_encoder(&ff);
    let mut vf = format!(
        "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,fps={fps},tpad=stop_mode=clone:stop_duration={duration}"
    );
    if fade_in {
        vf += &format!(",fade=t=in:st=0:d={FADE}");
    }
    if fade_out {
        vf += &format!(",fade=t=out:st={:.3}:d={FADE}", (duration - FADE).max(0.0));
    }
    vf += ",format=yuv420p";
    let mut args: Vec<String> = ["-y", "-hide_banner", "-loglevel", "error", "-ss"].map(String::from).to_vec();
    args.push(format!("{offset:.3}"));
    args.push("-i".into());
    args.push(file.to_string_lossy().into());
    args.extend(["-an", "-vf"].map(String::from));
    args.push(vf);
    args.push("-t".into());
    args.push(format!("{duration:.3}"));
    args.extend(video::encoder_args(&enc, w, h, fps, q));
    args.push(out.to_string_lossy().into());
    run_ffmpeg(args)
}

/// Fügt alle Shots zusammen und mischt den Ton dazu.
fn finalize(tasks: &[FilmTask], audio: &[MixClip], out: &Path, total: f32, enc: (u32, u32, u32, Quality)) -> Result<(), String> {
    if tasks.iter().any(|t| t.xfade.is_some()) {
        return finalize_xfade(tasks, audio, out, total, enc);
    }
    let list = out.with_extension("liste.txt");
    let body: String = tasks
        .iter()
        .map(|t| format!("file '{}'\n", t.cache.to_string_lossy().replace('\'', "'\\''")))
        .collect();
    std::fs::write(&list, body).map_err(|e| e.to_string())?;
    let tmp = out.with_extension("tmp.mp4");
    let mut args: Vec<String> =
        ["-y", "-hide_banner", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i"].map(String::from).to_vec();
    args.push(list.to_string_lossy().into());
    if audio.is_empty() {
        args.extend(["-c", "copy"].map(String::from));
    } else {
        for a in audio {
            args.push("-i".into());
            args.push(a.path.to_string_lossy().into());
        }
        args.push("-filter_complex".into());
        args.push(audio_filter(audio, total));
        args.extend(["-map", "0:v", "-map", "[aout]", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k"].map(String::from));
    }
    args.extend(["-movflags", "+faststart"].map(String::from));
    args.push(tmp.to_string_lossy().into());
    let r = run_ffmpeg(args);
    let _ = std::fs::remove_file(&list);
    r?;
    std::fs::rename(&tmp, out).map_err(|e| e.to_string())
}

/// Mit Überblendungen: Shots über xfade/concat-Filter verbinden (Video wird neu kodiert).
fn finalize_xfade(tasks: &[FilmTask], audio: &[MixClip], out: &Path, total: f32, enc: (u32, u32, u32, Quality)) -> Result<(), String> {
    let (w, h, fps, quality) = enc;
    let ff = video::find_ffmpeg().ok_or("ffmpeg nicht gefunden")?;
    let encoder = video::best_encoder(&ff);
    let mut args: Vec<String> = ["-y", "-hide_banner", "-loglevel", "error"].map(String::from).to_vec();
    for t in tasks {
        args.push("-i".into());
        args.push(t.cache.to_string_lossy().into());
    }
    for a in audio {
        args.push("-i".into());
        args.push(a.path.to_string_lossy().into());
    }
    let filter = xfade_filter(tasks);
    let mut full = filter;
    if !audio.is_empty() {
        full.push(';');
        full.push_str(&audio_filter_from(audio, total, tasks.len()));
    }
    args.push("-filter_complex".into());
    args.push(full);
    args.extend(["-map", "[vout]"].map(String::from));
    if !audio.is_empty() {
        args.extend(["-map", "[aout]", "-c:a", "aac", "-b:a", "192k"].map(String::from));
    }
    args.extend(video::encoder_args(&encoder, w, h, fps, quality));
    let tmp = out.with_extension("tmp.mp4");
    args.push(tmp.to_string_lossy().into());
    run_ffmpeg(args)?;
    std::fs::rename(&tmp, out).map_err(|e| e.to_string())
}

/// Filter-Kette: Shot für Shot anhängen – mit xfade (Überblendung) oder concat (Schnitt).
pub fn xfade_filter(tasks: &[FilmTask]) -> String {
    let mut parts = Vec::new();
    let mut acc = "[0:v]".to_string();
    let mut t_end = tasks.first().map(|t| t.duration).unwrap_or(0.0);
    for (k, next) in tasks.iter().enumerate().skip(1) {
        let prev = &tasks[k - 1];
        let label = if k + 1 == tasks.len() { "[vout]".to_string() } else { format!("[v{k}]") };
        match prev.xfade {
            Some((kind, d)) => {
                let d = d.min(prev.duration * 0.5).min(next.duration * 0.5);
                let offset = (t_end - d).max(0.0);
                parts.push(format!("{acc}[{k}:v]xfade=transition={kind}:duration={d:.3}:offset={offset:.3}{label}"));
                t_end = offset + next.duration;
            }
            None => {
                parts.push(format!("{acc}[{k}:v]concat=n=2:v=1:a=0{label}"));
                t_end += next.duration;
            }
        }
        acc = label;
    }
    if tasks.len() == 1 {
        parts.push("[0:v]null[vout]".into());
    }
    parts.join(";")
}

/// ffmpeg-Filter: jeden Ton zuschneiden, ein-/ausblenden, Lautstärke, an die richtige Stelle schieben, mischen.
pub fn audio_filter(audio: &[MixClip], total: f32) -> String {
    audio_filter_from(audio, total, 1)
}

/// Wie [`audio_filter`], die Ton-Eingänge beginnen bei Eingang `first`.
pub fn audio_filter_from(audio: &[MixClip], total: f32, first: usize) -> String {
    let mut parts = Vec::new();
    for (i, a) in audio.iter().enumerate() {
        let mut chain = format!("[{}:a]atrim=start={:.3}", i + first, a.offset.max(0.0));
        if let Some(d) = a.duration {
            chain += &format!(":duration={:.3}", d.max(0.01));
        }
        chain += ",asetpts=PTS-STARTPTS,aformat=sample_rates=48000:channel_layouts=stereo";
        if a.fade_in > 0.0 {
            chain += &format!(",afade=t=in:st=0:d={:.3}", a.fade_in);
        }
        if let (Some(d), true) = (a.duration, a.fade_out > 0.0) {
            chain += &format!(",afade=t=out:st={:.3}:d={:.3}", (d - a.fade_out).max(0.0), a.fade_out);
        }
        let ms = (a.start.max(0.0) * 1000.0).round() as u64;
        chain += &format!(",volume={:.3},adelay={ms}:all=1[a{i}]", a.volume);
        parts.push(chain);
    }
    let inputs: String = (0..audio.len()).map(|i| format!("[a{i}]")).collect();
    parts.push(format!(
        "{inputs}amix=inputs={}:normalize=0:dropout_transition=0,atrim=0:{:.3}[aout]",
        audio.len(),
        total
    ));
    parts.join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfade_chain_offsets() {
        let t = |d: f32, x: Option<(&'static str, f32)>| FilmTask {
            shot_id: "s".into(),
            kind: TaskKind::Clip { file: PathBuf::new(), offset: 0.0 },
            duration: d,
            fade_in: false,
            fade_out: false,
            cache: PathBuf::new(),
            xfade: x,
        };
        let f = xfade_filter(&[t(3.0, Some(("fade", 0.6))), t(4.0, None), t(2.0, None)]);
        assert!(f.contains("[0:v][1:v]xfade=transition=fade:duration=0.600:offset=2.400[v1]"), "{f}");
        assert!(f.contains("[v1][2:v]concat=n=2:v=1:a=0[vout]"), "{f}");
        assert_eq!(xfade_kind("circle"), Some("circleopen"));
        assert_eq!(xfade_kind("cut"), None);
    }

    #[test]
    fn audio_filter_places_and_mixes() {
        let a = vec![
            MixClip { path: "m.mp3".into(), start: 0.0, offset: 0.0, duration: None, volume: 0.6, fade_in: 1.0, fade_out: 2.0 },
            MixClip { path: "v.wav".into(), start: 12.5, offset: 0.2, duration: Some(3.0), volume: 1.0, fade_in: 0.0, fade_out: 0.5 },
        ];
        let f = audio_filter(&a, 60.0);
        assert!(f.contains("[1:a]atrim=start=0.000,"), "{f}");
        assert!(f.contains("adelay=12500:all=1[a1]"), "{f}");
        assert!(f.contains("afade=t=out:st=2.500:d=0.500"), "{f}");
        assert!(!f.contains("[a0]afade=t=out"), "ohne Länge kein Ausblenden");
        assert!(f.ends_with("amix=inputs=2:normalize=0:dropout_transition=0,atrim=0:60.000[aout]"), "{f}");
    }
}
