//! Ton in der Vorschau: Beim Abspielen hört man Stimmen, Geräusche und Musik des Shots
//! synchron zum Bild – auch nach einem Sprung in der Zeitleiste.
//!
//! Quellen: `audio` des Storyboard-Shots dieser Szene, der passende Ausschnitt des
//! `soundtrack` (Film-Zeit minus Beginn des Shots) und Lippen-Sync-Stimmen (`talk`).

use std::time::Duration;

use bevy::{audio::Volume, prelude::*};

use crate::{
    project::ActiveProject,
    source::{SceneLoaded, SceneSource},
    timeline::Playhead,
};

pub struct PreviewAudioPlugin;

impl Plugin for PreviewAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreviewAudio>().add_systems(Update, drive_audio.after(crate::timeline::TimelineSet));
    }
}

/// Ein Stück Ton in Szenen-Zeit.
#[derive(Clone, Debug, PartialEq)]
struct Clip {
    path: String,
    start: f32,
    offset: f32,
    duration: Option<f32>,
    volume: f32,
}

#[derive(Resource, Default)]
struct PreviewAudio {
    playing: bool,
    /// Szenen-Zeit und Echtzeit beim Start (zum Erkennen von Sprüngen).
    anchor: Option<(f32, f64)>,
    clips: Vec<Clip>,
    /// Bereits gestartete Clips (Index in `clips`).
    started: Vec<usize>,
    sinks: Vec<Entity>,
}

/// Alle Töne, die zur geöffneten Szene gehören.
fn collect_clips(src: &SceneSource, project: &ActiveProject) -> Vec<Clip> {
    let mut out = Vec::new();
    let Ok(root) = crate::project::root_dir() else { return out };
    let rel = |file: &str| -> Option<String> {
        let p = crate::media::resolve(project, crate::media::AUDIO, file).ok()?;
        let r = p.strip_prefix(&root).ok()?;
        Some(format!("cineforge://{}", r.to_string_lossy().replace('\\', "/")))
    };
    let current = src.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let shots = &project.data.storyboard;
    if let Some(idx) = shots.iter().position(|s| s.scene.as_deref() == Some(current.as_str())) {
        let shot_start: f32 = shots[..idx].iter().map(|s| s.duration).sum();
        for a in &shots[idx].audio {
            if let Some(path) = rel(&a.file) {
                out.push(Clip { path, start: a.start, offset: a.offset, duration: a.duration, volume: a.volume });
            }
        }
        for a in &project.data.soundtrack {
            let local = a.start - shot_start;
            let end = a.duration.map(|d| local + d);
            if end.is_some_and(|e| e <= 0.0) || local >= shots[idx].duration {
                continue;
            }
            if let Some(path) = rel(&a.file) {
                out.push(Clip { path, start: local, offset: a.offset, duration: a.duration, volume: a.volume });
            }
        }
    }
    // Lippen-Sync-Stimmen (falls nicht schon als Shot-Ton eingetragen)
    let mut talks = Vec::new();
    src.scene.walk(&mut |o| {
        if let Some(t) = o.animate.as_ref().and_then(|a| a.talk.as_ref()) {
            talks.push((t.audio.clone(), t.start));
        }
        if let cf_scene::Kind::Character(c) = &o.kind {
            if let Some(t) = &c.talk {
                talks.push((t.audio.clone(), t.start));
            }
        }
    });
    for (file, start) in talks {
        if let Some(path) = rel(&file) {
            if !out.iter().any(|c| c.path == path && (c.start - start).abs() < 0.05) {
                out.push(Clip { path, start, offset: 0.0, duration: None, volume: 1.0 });
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn drive_audio(
    mut commands: Commands,
    mut state: ResMut<PreviewAudio>,
    mut loaded: MessageReader<SceneLoaded>,
    ph: Res<Playhead>,
    src: Res<SceneSource>,
    project: Res<ActiveProject>,
    time: Res<Time<Real>>,
    asset_server: Res<AssetServer>,
) {
    let changed = loaded.read().count() > 0 || project.is_changed();
    let want = ph.playing && !ph.locked_by_job;
    let now = time.elapsed_secs_f64();
    // Sprung erkannt? (Zeit weicht vom erwarteten Verlauf ab)
    let jumped = match state.anchor {
        Some((t0, r0)) => want && (ph.time - (t0 + (now - r0) as f32)).abs() > 0.2,
        None => false,
    };
    if !want || changed || jumped || (want && !state.playing) {
        for e in state.sinks.drain(..) {
            commands.entity(e).despawn();
        }
        state.started.clear();
        state.playing = false;
        state.anchor = None;
        if !want {
            return;
        }
        state.clips = collect_clips(&src, &project);
        state.playing = true;
        state.anchor = Some((ph.time, now));
    }
    // Clips starten, sobald ihre Zeit erreicht ist
    let t = ph.time;
    for i in 0..state.clips.len() {
        if state.started.contains(&i) {
            continue;
        }
        let c = state.clips[i].clone();
        let end = c.duration.map(|d| c.start + d);
        if t + 0.02 < c.start || end.is_some_and(|e| t >= e) {
            continue;
        }
        let into = c.offset + (t - c.start).max(0.0);
        let mut settings = PlaybackSettings::DESPAWN.with_volume(Volume::Linear(c.volume.max(0.0)));
        settings.start_position = Some(Duration::from_secs_f32(into.max(0.0)));
        if let Some(d) = c.duration {
            settings.duration = Some(Duration::from_secs_f32((d - (t - c.start).max(0.0)).max(0.01)));
        }
        let e = commands.spawn((Name::new("Vorschau-Ton"), AudioPlayer::<AudioSource>(asset_server.load(c.path.clone())), settings)).id();
        state.sinks.push(e);
        state.started.push(i);
    }
}
