//! Grundlagen der Anzeige: Schrift mit Umlauten und Fenstertitel.
//! Die eigentliche Oberfläche steckt in `studio_ui`.

use bevy::prelude::*;

use crate::source::{SceneLoaded, SceneSource};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, install_font).add_systems(Update, window_title);
    }
}

/// Ersetzt Bevys Mini-Standardschrift (ohne Umlaute) durch Inter (SIL Open Font License).
fn install_font(mut fonts: ResMut<Assets<Font>>) {
    let _ = fonts.insert(&Handle::<Font>::default(), Font::from_bytes(crate::spawn::FONT.to_vec()));
}

fn window_title(mut loaded: MessageReader<SceneLoaded>, src: Res<SceneSource>, mut w: Query<&mut Window>) {
    if loaded.read().count() == 0 {
        return;
    }
    for mut win in &mut w {
        win.title = format!("CineForge Studio – {}", src.scene.name);
    }
}
