//! Einblendungen: Szenenname, Klartext-Beschreibung, Steuerung, Fehler, FPS.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

use crate::{
    control::ControlStatus,
    project::ActiveProject,
    source::{SceneLoaded, SceneSource},
    spawn::SceneStats,
    MainCamera,
};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (install_font, setup).chain())
            .add_systems(Update, (update_info, update_status, update_error, toggle, window_title));
    }
}

#[derive(Component)]
struct HudRoot;
#[derive(Component)]
struct InfoText;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct ErrorText;

const CONTROLS: &str =
    "Rechte Maustaste + Maus: umschauen  ·  WASD: fliegen  ·  E/Q: hoch/runter  ·  Shift: schnell  ·  Mausrad: Tempo\nC: Szenen-Kamera  ·  R: neu laden  ·  Cmd/Strg+Z: rückgängig  ·  H: Infos aus  ·  F12: Screenshot  ·  JSON-Datei ins Fenster ziehen: öffnen";

fn panel() -> BackgroundColor {
    BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.72))
}

/// Ersetzt Bevys Mini-Standardschrift (ohne Umlaute) durch Inter (SIL Open Font License).
fn install_font(mut fonts: ResMut<Assets<Font>>) {
    const INTER: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
    let _ = fonts.insert(&Handle::<Font>::default(), Font::from_bytes(INTER.to_vec()));
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(px(14)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            root.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(8), ..default() })
                .with_children(|top| {
                    top.spawn((
                        Node { padding: UiRect::axes(px(12), px(8)), max_width: px(720), border_radius: BorderRadius::all(px(8)), ..default() },
                        panel(),
                    ))
                    .with_children(|p| {
                        p.spawn((InfoText, Text::new(""), TextFont::from_font_size(15.0)));
                    });
                    top.spawn((
                        Node {
                            padding: UiRect::axes(px(12), px(8)),
                            max_width: px(720),
                            display: Display::None,
                            border_radius: BorderRadius::all(px(8)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.45, 0.05, 0.05, 0.85)),
                        ErrorText,
                    ))
                    .with_children(|p| {
                        p.spawn((Text::new(""), TextFont::from_font_size(14.0)));
                    });
                });
            root.spawn((
                Node { padding: UiRect::axes(px(12), px(8)), border_radius: BorderRadius::all(px(8)), ..default() },
                panel(),
            ))
                .with_children(|p| {
                    p.spawn((
                        StatusText,
                        Text::new(""),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::srgb(0.8, 0.82, 0.88)),
                    ));
                });
        });
}

fn update_info(
    mut loaded: MessageReader<SceneLoaded>,
    src: Res<SceneSource>,
    project: Res<ActiveProject>,
    mut q: Query<&mut Text, With<InfoText>>,
) {
    let scene_changed = loaded.read().count() > 0;
    if !scene_changed && !project.is_changed() {
        return;
    }
    let s = &src.scene;
    let name = if s.name.is_empty() { "Unbenannte Szene" } else { &s.name };
    let mut text = String::new();
    if let Some(p) = project.name() {
        let open = project.data.tasks.iter().filter(|t| t.status != cf_scene::project::Status::Done).count();
        text.push_str(&format!("Projekt: {p}  ·  {open} offene Aufgaben\n"));
        if !project.data.next_step.is_empty() {
            text.push_str(&format!("Nächster Schritt: {}\n", project.data.next_step));
        }
        text.push('\n');
    }
    text.push_str(&format!("{name}\n"));
    if !s.description.is_empty() {
        text.push_str(&s.description);
        text.push('\n');
    }
    text.push_str(&format!("Datei: {}", src.path.display()));
    for mut t in &mut q {
        t.0 = text.clone();
    }
}

fn update_error(
    src: Res<SceneSource>,
    mut panel: Query<(&mut Node, &Children), With<ErrorText>>,
    mut texts: Query<&mut Text>,
) {
    if !src.is_changed() {
        return;
    }
    for (mut node, children) in &mut panel {
        match &src.error {
            Some(e) => {
                node.display = Display::Flex;
                for c in children.iter() {
                    if let Ok(mut t) = texts.get_mut(c) {
                        t.0 = format!("Änderung nicht übernommen – vorherige Fassung bleibt sichtbar:\n{e}");
                    }
                }
            }
            None => node.display = Display::None,
        }
    }
}

fn update_status(
    diagnostics: Res<DiagnosticsStore>,
    stats: Res<SceneStats>,
    src: Res<SceneSource>,
    control: Option<Res<ControlStatus>>,
    setup: Option<Res<crate::claude_setup::SetupState>>,
    cam: Query<&Transform, With<MainCamera>>,
    mut q: Query<&mut Text, With<StatusText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let pos = cam.single().map(|t| t.translation).unwrap_or_default();
    let line = format!(
        "{fps:.0} FPS  ·  {} Objekte  ·  {} Instanzen  ·  {:.1} Mio. Dreiecke  ·  Aufbau {:.0} ms ({} neu)  ·  Kamera [{:.0}, {:.0}, {:.0}]\n{CONTROLS}",
        stats.objects,
        stats.instances,
        stats.triangles as f32 / 1e6,
        stats.build_ms,
        stats.rebuilt,
        pos.x,
        pos.y,
        pos.z
    );
    let claude = match control.as_deref() {
        Some(ControlStatus { port: Some(p), calls, last_action, .. }) => {
            let last = last_action.as_deref().map(|a| format!(" · zuletzt: {a}")).unwrap_or_default();
            format!("Claude-Verbindung: bereit (Port {p}) · {calls} Aktionen{last}")
        }
        Some(ControlStatus { error: Some(e), .. }) => format!("Claude-Verbindung: {e}"),
        _ => "Claude-Verbindung: aus".to_string(),
    };
    let desktop = setup.map(|s| format!("  ·  {}", s.message)).unwrap_or_default();
    let undo = match src.history_len() {
        1 => "1 Schritt".to_string(),
        n => format!("{n} Schritte"),
    };
    let line = format!("{line}\n{claude}{desktop}  ·  Rückgängig: {undo}");
    for mut t in &mut q {
        t.0 = line.clone();
    }
}

fn toggle(keys: Res<ButtonInput<KeyCode>>, mut q: Query<&mut Visibility, With<HudRoot>>) {
    if keys.just_pressed(KeyCode::KeyH) {
        for mut v in &mut q {
            v.toggle_visible_hidden();
        }
    }
}

fn window_title(mut loaded: MessageReader<SceneLoaded>, src: Res<SceneSource>, mut w: Query<&mut Window>) {
    if loaded.read().count() == 0 {
        return;
    }
    for mut win in &mut w {
        win.title = format!("CineForge Studio – {}", src.scene.name);
    }
}
