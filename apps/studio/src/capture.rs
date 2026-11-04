//! Screenshots: automatisch (für Claudes Bildkontrolle) und per Taste F12.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
};

/// Rendert die Szene, speichert nach `frames` Frames ein Bild und beendet die App.
pub struct AutoCapturePlugin {
    pub path: PathBuf,
    pub frames: u32,
    /// Zeitpunkt der Szene (Kamera folgt der Szenen-Kamera).
    pub time: f32,
}

#[derive(Resource)]
struct AutoCapture {
    path: PathBuf,
    frames_left: u32,
    requested: bool,
}

impl Plugin for AutoCapturePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AutoCapture {
            path: self.path.clone(),
            frames_left: self.frames.max(2),
            requested: false,
        })
        .add_systems(Update, auto_capture);
        let t = self.time;
        app.add_systems(Startup, move |mut ph: ResMut<crate::timeline::Playhead>| {
            ph.time = t;
            ph.camera_view = true;
            ph.locked_by_job = true;
        });
    }
}

fn auto_capture(mut commands: Commands, mut state: ResMut<AutoCapture>) {
    if state.requested {
        return;
    }
    if state.frames_left > 0 {
        state.frames_left -= 1;
        return;
    }
    state.requested = true;
    let path = state.path.clone();
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()))
        .observe(move |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            println!("Screenshot gespeichert: {}", path.display());
            exit.write(AppExit::Success);
        });
}

pub struct ManualCapturePlugin;

impl Plugin for ManualCapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, capture_on_key);
    }
}

fn capture_on_key(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>, preview: Option<Res<crate::studio_ui::PreviewTarget>>) {
    if keys.just_pressed(KeyCode::F12) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dir = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(|h| PathBuf::from(h).join("Documents").join("CineForge").join("Screenshots"))
            .unwrap_or_else(|| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("cineforge-{stamp}.png"));
        info!("Screenshot: {}", path.display());
        // Nur das Vorschau-Bild (ohne Oberfläche), falls vorhanden.
        let shot = match preview {
            Some(p) => Screenshot::image(p.0.clone()),
            None => Screenshot::primary_window(),
        };
        commands.spawn(shot).observe(save_to_disk(path));
    }
}
