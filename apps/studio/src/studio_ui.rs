//! Studio-Oberfläche im Stil eines Video-Editors (ähnlich CapCut):
//! oben Projekt und Claude-Status, links Szenen / Objekte / Medien, in der Mitte die
//! Vorschau mit Wiedergabe-Leiste, rechts Eigenschaften und „Was passiert gerade“,
//! unten die Zeitleiste mit Spuren (Film, Kamera, Texte, Ton, animierte Objekte).
//!
//! Die 3D-Ansicht rendert in ein Bild, das als Fläche (`ViewportNode`) in der
//! Oberfläche sitzt. Ein Klick auf ein Objekt – in der Vorschau oder in der Liste –
//! wählt es aus und hebt es hervor; Claude sieht die Auswahl (`get_selection`).

use bevy::{
    camera::{primitives::Aabb, visibility::RenderLayers, ClearColorConfig, RenderTarget},
    camera_controller::free_camera::FreeCameraState,
    gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    input::mouse::AccumulatedMouseScroll,
    picking::{
        events::PointerClick,
        hover::{HoverMap, Hovered},
        mesh_picking::MeshPickingPlugin,
        pointer::PointerButton,
    },
    prelude::*,
    render::render_resource::TextureFormat,
    ui::{widget::ViewportNode, RelativeCursorPosition},
};
use cf_scene::{project::Status, Kind, Object, Scene};

use crate::{
    control::ControlStatus,
    overlay::ViewAspect,
    project::ActiveProject,
    source::{SceneLoaded, SceneSource},
    spawn::{SceneObjectId, SceneStats},
    timeline::{format_time, Playhead},
    MainCamera,
};

pub struct StudioUiPlugin;

impl Plugin for StudioUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<MeshPickingPlugin>() {
            app.add_plugins(MeshPickingPlugin);
        }
        app.init_resource::<Selection>()
            .init_resource::<UiState>()
            .add_systems(PostStartup, setup)
            .add_systems(
                Update,
                (
                    fit_preview,
                    clicks,
                    hover_colors,
                    scroll_lists,
                    scrub_timeline,
                    camera_control_gate,
                    (rebuild_left, rebuild_props, rebuild_activity, rebuild_tracks),
                    (update_topbar, update_transport, update_playhead, update_error),
                    draw_selection,
                )
                    .chain()
                    .after(crate::timeline::TimelineSet),
            );
    }
}

/// Was der Mensch gerade ausgewählt hat (id aus dem Szenen-Dokument).
#[derive(Resource, Default, Debug, Clone)]
pub struct Selection {
    pub id: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LeftTab {
    Scenes,
    Objects,
    Media,
}

#[derive(Resource)]
struct UiState {
    tab: LeftTab,
    /// Vorschau im Hochformat (9:16, TikTok/Reels) statt 16:9.
    portrait: bool,
    /// Listen neu aufbauen (Tab gewechselt, Auswahl geändert …).
    dirty: bool,
    media_scan: f32,
}

impl Default for UiState {
    fn default() -> Self {
        Self { tab: LeftTab::Objects, portrait: false, dirty: true, media_scan: 0.0 }
    }
}

/// Bild, in das die Vorschau-Kamera rendert (F12 speichert es).
#[derive(Resource)]
pub struct PreviewTarget(pub Handle<Image>);

#[derive(Resource)]
struct UiFonts {
    regular: Handle<Font>,
    bold: Handle<Font>,
}

// ---------------------------------------------------------------------------
// Farben
// ---------------------------------------------------------------------------

const BG: Color = Color::srgb(0.055, 0.059, 0.071);
const PANEL: Color = Color::srgb(0.086, 0.090, 0.110);
const PANEL_2: Color = Color::srgb(0.118, 0.125, 0.149);
const HOVER: Color = Color::srgb(0.160, 0.169, 0.200);
const SELECTED: Color = Color::srgb(0.105, 0.290, 0.290);
const LINE: Color = Color::srgb(0.150, 0.158, 0.188);
const TEXT: Color = Color::srgb(0.90, 0.91, 0.93);
const DIM: Color = Color::srgb(0.56, 0.58, 0.64);
const ACCENT: Color = Color::srgb(0.18, 0.84, 0.76);
const VIOLET: Color = Color::srgb(0.58, 0.50, 1.0);
const AMBER: Color = Color::srgb(1.0, 0.72, 0.30);
const RED: Color = Color::srgb(0.95, 0.35, 0.35);
const PREVIEW_BG: Color = Color::srgb(0.030, 0.032, 0.040);

/// Ebene, auf der nur die Vorschau-Kamera Hilfslinien (Auswahl-Rahmen) sieht –
/// Aufnahmen und Exporte bleiben sauber.
const GIZMO_LAYER: usize = 7;
const LABEL_W: f32 = 132.0;
const ROW_H: f32 = 24.0;
const MAX_OBJECT_TRACKS: usize = 6;

// ---------------------------------------------------------------------------
// Marker
// ---------------------------------------------------------------------------

#[derive(Component)]
struct PreviewArea;
#[derive(Component)]
struct PreviewNode;
#[derive(Component)]
struct ErrorBanner;
#[derive(Component)]
struct CamBadge;
#[derive(Component)]
struct TopTitle;
#[derive(Component)]
struct ClaudeDot;
#[derive(Component)]
struct ClaudeText;
#[derive(Component)]
struct JobText;
#[derive(Component)]
struct LeftList;
#[derive(Component)]
struct PropsBody;
#[derive(Component)]
struct ActivityBody;
#[derive(Component)]
struct FilmLane;
#[derive(Component)]
struct TrackLabels;
#[derive(Component)]
struct TrackLanes;
#[derive(Component)]
struct LanesArea;
#[derive(Component)]
struct PlayheadLine;
#[derive(Component)]
struct TimeLabel;
#[derive(Component)]
struct StatsLabel;
#[derive(Component)]
struct Scrollable;

/// Klickbare Elemente und was sie tun.
#[derive(Component, Clone, Debug)]
enum Action {
    Tab(LeftTab),
    Select(Option<String>),
    OpenScene(String),
    PlayPause,
    ToStart,
    Loop,
    CameraView,
    Portrait,
    ExportScene,
    ExportFilm,
    CancelExport,
    OpenExportFolder,
}

/// Hintergrundfarbe je nach Zustand (normal / Maus darüber).
#[derive(Component, Clone, Copy)]
struct HoverBg {
    normal: Color,
    hover: Color,
}

// ---------------------------------------------------------------------------
// Aufbau
// ---------------------------------------------------------------------------

fn txt(fonts: &UiFonts, s: impl Into<String>, size: f32, color: Color, bold: bool) -> impl Bundle {
    (
        Text::new(s),
        TextFont::from_font_size(size).with_font(if bold { fonts.bold.clone() } else { fonts.regular.clone() }),
        TextColor(color),
        Pickable::IGNORE,
    )
}

fn button(action: Action, normal: Color, node: Node) -> impl Bundle {
    (action, node, BackgroundColor(normal), HoverBg { normal, hover: HOVER }, Hovered::default())
}

fn pill_node() -> Node {
    Node {
        padding: UiRect::axes(px(10), px(5)),
        border_radius: BorderRadius::all(px(6)),
        align_items: AlignItems::Center,
        ..default()
    }
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut images: ResMut<Assets<Image>>,
    mut gizmo_store: ResMut<GizmoConfigStore>,
    main_cam: Query<Entity, With<MainCamera>>,
) {
    let ui_fonts = UiFonts {
        regular: Handle::<Font>::default(),
        bold: fonts.add(Font::from_bytes(crate::spawn::FONT_BOLD.to_vec())),
    };

    // Vorschau-Kamera rendert in ein Bild; die Größe passt `ViewportNode` automatisch an.
    let image = images.add(Image::new_target_texture(1280, 720, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb)));
    let Ok(cam) = main_cam.single() else { return };
    commands.insert_resource(PreviewTarget(image.clone()));
    commands.entity(cam).insert((
        RenderTarget::Image(image.clone().into()),
        RenderLayers::from_layers(&[0, GIZMO_LAYER]),
        ViewAspect(16.0 / 9.0),
    ));
    let (config, _) = gizmo_store.config_mut::<DefaultGizmoConfigGroup>();
    config.render_layers = RenderLayers::layer(GIZMO_LAYER);
    config.line.width = 2.5;

    // Kamera nur für die Oberfläche.
    commands.spawn((
        Camera2d,
        Camera { order: 1, clear_color: ClearColorConfig::Custom(BG), ..default() },
        Msaa::Off,
        IsDefaultUiCamera,
    ));

    let f = &ui_fonts;
    commands
        .spawn((
            Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, ..default() },
            BackgroundColor(BG),
        ))
        .with_children(|root| {
            // ---------------- Kopfzeile ----------------
            root.spawn((
                Node {
                    height: px(44),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(px(14), px(0)),
                    column_gap: px(14),
                    border: UiRect::bottom(px(1)),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(LINE),
            ))
            .with_children(|top| {
                top.spawn(txt(f, "CineForge", 16.0, ACCENT, true));
                top.spawn((TopTitle, txt(f, "", 14.0, TEXT, false)));
                top.spawn(Node { flex_grow: 1.0, ..default() });
                top.spawn((JobText, txt(f, "", 13.0, AMBER, false)));
                for (action, label, bg) in [
                    (Action::ExportScene, "Szene exportieren", PANEL_2),
                    (Action::ExportFilm, "Film exportieren", ACCENT.with_alpha(0.25)),
                    (Action::CancelExport, "Abbrechen", PANEL_2),
                    (Action::OpenExportFolder, "Export-Ordner", PANEL_2),
                ] {
                    top.spawn(button(action, bg, pill_node())).with_children(|b| {
                        b.spawn(txt(f, label, 12.0, TEXT, true));
                    });
                }
                top.spawn((pill_node(), BackgroundColor(PANEL_2))).with_children(|p| {
                    p.spawn((
                        ClaudeDot,
                        Node { width: px(8), height: px(8), margin: UiRect::right(px(8)), border_radius: BorderRadius::all(px(4)), ..default() },
                        BackgroundColor(DIM),
                    ));
                    p.spawn((ClaudeText, txt(f, "Claude", 13.0, TEXT, false)));
                });
            });

            // ---------------- Mitte ----------------
            root.spawn(Node { flex_grow: 1.0, min_height: px(0), flex_direction: FlexDirection::Row, ..default() })
                .with_children(|mid| {
                    // Links: Szenen / Objekte / Medien
                    mid.spawn((
                        Node {
                            width: px(270),
                            flex_shrink: 0.0,
                            flex_direction: FlexDirection::Column,
                            border: UiRect::right(px(1)),
                            ..default()
                        },
                        BackgroundColor(PANEL),
                        BorderColor::all(LINE),
                    ))
                    .with_children(|left| {
                        left.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: px(4),
                            padding: UiRect::all(px(8)),
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|tabs| {
                            for (tab, label) in [(LeftTab::Scenes, "Szenen"), (LeftTab::Objects, "Objekte"), (LeftTab::Media, "Medien")] {
                                tabs.spawn(button(
                                    Action::Tab(tab),
                                    PANEL,
                                    Node { flex_grow: 1.0, justify_content: JustifyContent::Center, ..pill_node() },
                                ))
                                .with_children(|b| {
                                    b.spawn(txt(f, label, 13.0, TEXT, true));
                                });
                            }
                        });
                        left.spawn((
                            LeftList,
                            Scrollable,
                            ScrollPosition::default(),
                            Node {
                                flex_grow: 1.0,
                                min_height: px(0),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::axes(px(8), px(4)),
                                row_gap: px(1),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                        ));
                    });

                    // Mitte: Vorschau + Wiedergabe
                    mid.spawn(Node { flex_grow: 1.0, min_width: px(0), flex_direction: FlexDirection::Column, ..default() })
                        .with_children(|center| {
                            center
                                .spawn((
                                    PreviewArea,
                                    RelativeCursorPosition::default(),
                                    Node {
                                        flex_grow: 1.0,
                                        min_height: px(0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                    BackgroundColor(PREVIEW_BG),
                                ))
                                .with_children(|area| {
                                    area.spawn((PreviewNode, ViewportNode::new(cam), Node { width: px(640), height: px(360), ..default() }));
                                    area.spawn((
                                        CamBadge,
                                        Node { position_type: PositionType::Absolute, left: px(10), top: px(10), ..pill_node() },
                                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                                        Pickable::IGNORE,
                                    ))
                                    .with_children(|b| {
                                        b.spawn(txt(f, "", 12.0, TEXT, false));
                                    });
                                    area.spawn((
                                        ErrorBanner,
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: px(12),
                                            right: px(12),
                                            bottom: px(12),
                                            display: Display::None,
                                            ..pill_node()
                                        },
                                        BackgroundColor(Color::srgba(0.45, 0.06, 0.06, 0.92)),
                                        Pickable::IGNORE,
                                    ))
                                    .with_children(|b| {
                                        b.spawn(txt(f, "", 13.0, TEXT, false));
                                    });
                                });
                            center
                                .spawn((
                                    Node {
                                        height: px(46),
                                        flex_shrink: 0.0,
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        column_gap: px(8),
                                        border: UiRect::top(px(1)),
                                        ..default()
                                    },
                                    BackgroundColor(PANEL),
                                    BorderColor::all(LINE),
                                ))
                                .with_children(|bar| {
                                    let tb = |a: Action| button(a, PANEL_2, pill_node());
                                    bar.spawn(tb(Action::ToStart)).with_children(|b| {
                                        b.spawn(txt(f, "◀◀", 12.0, TEXT, true));
                                    });
                                    bar.spawn(button(Action::PlayPause, ACCENT.with_alpha(0.22), Node { width: px(44), justify_content: JustifyContent::Center, ..pill_node() }))
                                        .with_children(|b| {
                                            b.spawn(txt(f, "▶", 15.0, ACCENT, true));
                                        });
                                    bar.spawn((TimeLabel, txt(f, "00:00.0 / 00:05.0", 15.0, TEXT, true)));
                                    bar.spawn(tb(Action::Loop)).with_children(|b| {
                                        b.spawn(txt(f, "Schleife", 12.0, TEXT, false));
                                    });
                                    bar.spawn(tb(Action::CameraView)).with_children(|b| {
                                        b.spawn(txt(f, "Szenen-Kamera", 12.0, TEXT, false));
                                    });
                                    bar.spawn(tb(Action::Portrait)).with_children(|b| {
                                        b.spawn(txt(f, "16:9", 12.0, TEXT, false));
                                    });
                                });
                        });

                    // Rechts: Eigenschaften + Was passiert
                    mid.spawn((
                        Node {
                            width: px(320),
                            flex_shrink: 0.0,
                            flex_direction: FlexDirection::Column,
                            border: UiRect::left(px(1)),
                            ..default()
                        },
                        BackgroundColor(PANEL),
                        BorderColor::all(LINE),
                    ))
                    .with_children(|right| {
                        let header = |p: &mut ChildSpawnerCommands, title: &str| {
                            p.spawn(Node { padding: UiRect::new(px(14), px(14), px(12), px(6)), flex_shrink: 0.0, ..default() })
                                .with_children(|h| {
                                    h.spawn(txt(f, title, 12.0, DIM, true));
                                });
                        };
                        header(right, "EIGENSCHAFTEN");
                        right.spawn((
                            PropsBody,
                            Scrollable,
                            ScrollPosition::default(),
                            Node {
                                flex_grow: 1.5,
                                flex_basis: px(0),
                                min_height: px(0),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::axes(px(14), px(2)),
                                row_gap: px(5),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                        ));
                        right.spawn((Node { height: px(1), flex_shrink: 0.0, ..default() }, BackgroundColor(LINE)));
                        header(right, "WAS PASSIERT");
                        right.spawn((
                            ActivityBody,
                            Scrollable,
                            ScrollPosition::default(),
                            Node {
                                flex_grow: 1.0,
                                flex_basis: px(0),
                                min_height: px(0),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::new(px(14), px(14), px(2), px(10)),
                                row_gap: px(6),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                        ));
                    });
                });

            // ---------------- Zeitleiste ----------------
            root.spawn((
                Node {
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    border: UiRect::top(px(1)),
                    padding: UiRect::new(px(0), px(10), px(6), px(8)),
                    row_gap: px(4),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(LINE),
            ))
            .with_children(|tl| {
                // Film-Leiste (alle Shots, eigener Maßstab)
                tl.spawn(Node { flex_direction: FlexDirection::Row, height: px(30), ..default() }).with_children(|row| {
                    row.spawn(Node { width: px(LABEL_W), padding: UiRect::left(px(14)), align_items: AlignItems::Center, flex_shrink: 0.0, ..default() })
                        .with_children(|l| {
                            l.spawn(txt(f, "Film", 12.0, DIM, true));
                        });
                    row.spawn((FilmLane, Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, column_gap: px(2), overflow: Overflow::clip(), ..default() }));
                });
                // Szene: Lineal + Spuren
                tl.spawn(Node { flex_direction: FlexDirection::Row, ..default() }).with_children(|row| {
                    row.spawn((TrackLabels, Node { width: px(LABEL_W), flex_shrink: 0.0, flex_direction: FlexDirection::Column, ..default() }));
                    row.spawn((
                        LanesArea,
                        RelativeCursorPosition::default(),
                        Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, ..default() },
                    ))
                    .with_children(|lanes| {
                        lanes.spawn((TrackLanes, Node { flex_direction: FlexDirection::Column, ..default() }, Pickable::IGNORE));
                        lanes.spawn((
                            PlayheadLine,
                            Node { position_type: PositionType::Absolute, top: px(0), bottom: px(0), width: px(2), left: percent(0), ..default() },
                            BackgroundColor(Color::WHITE),
                            Pickable::IGNORE,
                        ));
                    });
                });
                tl.spawn(Node { padding: UiRect::left(px(14)), ..default() }).with_children(|l| {
                    l.spawn((StatsLabel, txt(f, "", 11.0, DIM, false)));
                });
            });
        });
    commands.insert_resource(ui_fonts);
}

// ---------------------------------------------------------------------------
// Vorschau
// ---------------------------------------------------------------------------

/// Vorschau-Fläche im gewählten Seitenverhältnis mittig in den freien Platz einpassen.
fn fit_preview(
    state: Res<UiState>,
    area: Query<&ComputedNode, With<PreviewArea>>,
    mut node: Query<&mut Node, With<PreviewNode>>,
    mut cams: Query<&mut ViewAspect, With<MainCamera>>,
) {
    let Ok(area) = area.single() else { return };
    let avail = area.size() * area.inverse_scale_factor();
    if avail.x < 10.0 || avail.y < 10.0 {
        return;
    }
    let aspect = if state.portrait { 9.0 / 16.0 } else { 16.0 / 9.0 };
    let margin = 12.0;
    let (aw, ah) = (avail.x - 2.0 * margin, avail.y - 2.0 * margin);
    let (w, h) = if aw / ah > aspect { (ah * aspect, ah) } else { (aw, aw / aspect) };
    let (w, h) = (w.floor().max(16.0), h.floor().max(9.0));
    for mut n in &mut node {
        if n.width != px(w) || n.height != px(h) {
            n.width = px(w);
            n.height = px(h);
        }
    }
    for mut a in &mut cams {
        if (a.0 - aspect).abs() > 1e-4 {
            a.0 = aspect;
        }
    }
}

/// Flug-Kamera nur steuern, wenn die Maus über der Vorschau ist (sonst würde z. B.
/// das Mausrad in Listen die Fluggeschwindigkeit ändern).
fn camera_control_gate(
    area: Query<&RelativeCursorPosition, With<PreviewArea>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cams: Query<&mut FreeCameraState, With<MainCamera>>,
) {
    let over = area.single().map(|r| r.cursor_over()).unwrap_or(true);
    for mut s in &mut cams {
        let enabled = over || mouse.pressed(MouseButton::Right);
        if s.enabled != enabled {
            s.enabled = enabled;
        }
    }
}

/// Rahmen um das ausgewählte Objekt (nur in der Vorschau sichtbar).
fn draw_selection(
    sel: Res<Selection>,
    ids: Query<(Entity, &SceneObjectId)>,
    children: Query<&Children>,
    bounds: Query<(&Aabb, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    let Some(want) = &sel.id else { return };
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for (e, id) in &ids {
        if &id.0 != want {
            continue;
        }
        let mut stack = vec![e];
        while let Some(x) = stack.pop() {
            if let Ok((aabb, tf)) = bounds.get(x) {
                let (c, h) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
                for i in 0..8 {
                    let corner = c + h * Vec3::new(
                        if i & 1 == 0 { -1.0 } else { 1.0 },
                        if i & 2 == 0 { -1.0 } else { 1.0 },
                        if i & 4 == 0 { -1.0 } else { 1.0 },
                    );
                    let w = tf.transform_point(corner);
                    lo = lo.min(w);
                    hi = hi.max(w);
                }
            }
            if let Ok(ch) = children.get(x) {
                stack.extend(ch.iter());
            }
        }
    }
    if lo.x <= hi.x {
        let pad = (hi - lo).max_element() * 0.02 + 0.02;
        let size = hi - lo + Vec3::splat(2.0 * pad);
        gizmos.cube(Transform::from_translation((lo + hi) / 2.0).with_scale(size), ACCENT);
    }
}

// ---------------------------------------------------------------------------
// Bedienung
// ---------------------------------------------------------------------------

/// Alle Klicks: Knöpfe der Oberfläche und Objekte in der Vorschau.
#[allow(clippy::too_many_arguments)]
fn clicks(
    mut clicks: MessageReader<PointerClick>,
    parents: Query<&ChildOf>,
    actions: Query<&Action>,
    ids: Query<&SceneObjectId>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<UiState>,
    mut sel: ResMut<Selection>,
    mut ph: ResMut<Playhead>,
    mut src: ResMut<SceneSource>,
    project: Res<ActiveProject>,
    mut out: MessageWriter<SceneLoaded>,
    ui_cmd: Option<Res<crate::control::UiCommands>>,
) {
    let mut todo: Vec<Action> = Vec::new();
    for c in clicks.read() {
        if c.button != PointerButton::Primary {
            continue;
        }
        let mut e = c.entity;
        loop {
            if let Ok(a) = actions.get(e) {
                todo.push(a.clone());
                break;
            }
            if let Ok(id) = ids.get(e) {
                // Teile eines Bibliotheks-Objekts („haus/tuer“) gehören zum Objekt „haus“.
                let id = id.0.split('/').next().unwrap_or(&id.0).to_string();
                todo.push(Action::Select(Some(id)));
                break;
            }
            match parents.get(e) {
                Ok(p) => e = p.parent(),
                Err(_) => break,
            }
        }
    }
    if keys.just_pressed(KeyCode::Escape) && sel.id.is_some() {
        todo.push(Action::Select(None));
    }
    for action in todo {
        match action {
            Action::Tab(t) => {
                state.tab = t;
                state.dirty = true;
                state.media_scan = 0.0;
            }
            Action::Select(id) => {
                if sel.id != id {
                    sel.id = id;
                }
            }
            Action::OpenScene(name) => {
                if let Ok(dir) = project.scenes_dir() {
                    let path = dir.join(format!("{name}.json"));
                    if path.is_file() && path != src.path {
                        if let Err(e) = src.open_file(path, &mut out) {
                            warn!("Szene öffnen: {e}");
                        }
                        sel.id = None;
                    }
                }
            }
            Action::PlayPause if !ph.locked_by_job => {
                if !ph.playing && ph.time >= src.scene.duration - 1e-3 {
                    ph.time = 0.0;
                }
                ph.playing = !ph.playing;
            }
            Action::ToStart if !ph.locked_by_job => ph.time = 0.0,
            Action::Loop => ph.looping = !ph.looping,
            Action::CameraView => ph.camera_view = !ph.camera_view,
            Action::Portrait => state.portrait = !state.portrait,
            Action::ExportScene => {
                if let Some(c) = &ui_cmd {
                    let (w, h) = if state.portrait { (1080, 1920) } else { (1920, 1080) };
                    c.send("export_video", serde_json::json!({"width": w, "height": h}));
                }
            }
            Action::ExportFilm => {
                if let Some(c) = &ui_cmd {
                    let (w, h) = if state.portrait { (1080, 1920) } else { (1920, 1080) };
                    c.send("export_film", serde_json::json!({"width": w, "height": h}));
                }
            }
            Action::CancelExport => {
                if let Some(c) = &ui_cmd {
                    c.send("cancel_export", serde_json::json!({}));
                }
            }
            Action::OpenExportFolder => {
                if let Ok(dir) = project.export_dir() {
                    let _ = std::fs::create_dir_all(&dir);
                    let opener = if cfg!(target_os = "macos") {
                        "open"
                    } else if cfg!(target_os = "windows") {
                        "explorer"
                    } else {
                        "xdg-open"
                    };
                    let _ = std::process::Command::new(opener).arg(&dir).spawn();
                }
            }
            _ => {}
        }
    }
}

fn hover_colors(mut q: Query<(&Hovered, &HoverBg, &mut BackgroundColor), Changed<Hovered>>) {
    for (h, c, mut bg) in &mut q {
        bg.0 = if h.get() { c.hover } else { c.normal };
    }
}

/// Mausrad scrollt die Liste unter dem Mauszeiger.
fn scroll_lists(
    wheel: Res<AccumulatedMouseScroll>,
    hover: Res<HoverMap>,
    parents: Query<&ChildOf>,
    mut lists: Query<&mut ScrollPosition, With<Scrollable>>,
) {
    if wheel.delta.y == 0.0 {
        return;
    }
    let dy = match wheel.unit {
        bevy::input::mouse::MouseScrollUnit::Line => wheel.delta.y * 24.0,
        bevy::input::mouse::MouseScrollUnit::Pixel => wheel.delta.y,
    };
    for hits in hover.values() {
        for e in hits.keys() {
            let mut x = *e;
            loop {
                if let Ok(mut pos) = lists.get_mut(x) {
                    pos.0.y = (pos.0.y - dy).max(0.0);
                    return;
                }
                match parents.get(x) {
                    Ok(p) => x = p.parent(),
                    Err(_) => break,
                }
            }
        }
    }
}

/// Klicken/Ziehen in der Zeitleiste setzt den Zeitpunkt.
fn scrub_timeline(
    q: Query<&RelativeCursorPosition, With<LanesArea>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut ph: ResMut<Playhead>,
    src: Res<SceneSource>,
    mut dragging: Local<bool>,
) {
    let Ok(rel) = q.single() else { return };
    if mouse.just_pressed(MouseButton::Left) && rel.cursor_over() {
        *dragging = true;
    }
    if !mouse.pressed(MouseButton::Left) {
        *dragging = false;
    }
    if !*dragging || ph.locked_by_job {
        return;
    }
    if let Some(n) = rel.normalized {
        ph.time = (n.x + 0.5).clamp(0.0, 1.0) * src.scene.duration;
        ph.playing = false;
    }
}

// ---------------------------------------------------------------------------
// Inhalte
// ---------------------------------------------------------------------------

fn row_node(indent: usize) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(8),
        padding: UiRect::new(px(8.0 + indent as f32 * 14.0), px(8), px(5), px(5)),
        border_radius: BorderRadius::all(px(6)),
        flex_shrink: 0.0,
        ..default()
    }
}

fn kind_name(k: &Kind) -> &'static str {
    match k {
        Kind::Box { .. } => "Box",
        Kind::Sphere { .. } => "Kugel",
        Kind::Cylinder { .. } => "Zylinder",
        Kind::Cone { .. } => "Kegel",
        Kind::Capsule { .. } => "Kapsel",
        Kind::Torus { .. } => "Ring",
        Kind::Plane { .. } => "Fläche",
        Kind::Pyramid { .. } => "Pyramide",
        Kind::RoundedBox { .. } => "Box (rund)",
        Kind::Extrude(_) => "Extrusion",
        Kind::Lathe(_) => "Drehkörper",
        Kind::Tube(_) => "Röhre",
        Kind::Text(_) => "3D-Text",
        Kind::Image { .. } => "Bild",
        Kind::Svg { .. } => "2D-Grafik",
        Kind::Model { .. } => "3D-Modell",
        Kind::Particles(_) => "Effekt",
        Kind::Instance { .. } => "Bibliothek",
        Kind::Terrain(_) => "Gelände",
        Kind::Scatter(_) => "Verteilung",
        Kind::Group { .. } => "Gruppe",
        Kind::PointLight(_) => "Licht",
        Kind::Character(_) => "Figur",
        Kind::Tree(_) => "Baum",
        Kind::Water(_) => "Wasser",
    }
}

fn action_name(k: cf_scene::character::ActionKind) -> &'static str {
    use cf_scene::character::ActionKind as A;
    match k {
        A::Idle => "steht",
        A::Walk => "geht",
        A::Run => "rennt",
        A::Wave => "winkt",
        A::Jump => "springt",
        A::Cheer => "jubelt",
        A::Dance => "tanzt",
        A::Point => "zeigt",
        A::Nod => "nickt",
        A::ShakeHead => "schüttelt den Kopf",
        A::Talk => "spricht",
        A::Sit => "sitzt",
        A::Think => "denkt nach",
    }
}

fn kind_color(k: &Kind) -> Color {
    match k {
        Kind::Group { .. } | Kind::Instance { .. } | Kind::Character(_) => VIOLET,
        Kind::PointLight(_) | Kind::Particles(_) => AMBER,
        Kind::Svg { .. } | Kind::Image { .. } | Kind::Text(_) => ACCENT,
        _ => DIM,
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild_left(
    mut commands: Commands,
    mut loaded: MessageReader<SceneLoaded>,
    mut state: ResMut<UiState>,
    sel: Res<Selection>,
    src: Res<SceneSource>,
    project: Res<ActiveProject>,
    time: Res<Time>,
    fonts: Option<Res<UiFonts>>,
    list: Query<Entity, With<LeftList>>,
    tabs: Query<(&Action, &Children)>,
    mut tab_text: Query<&mut TextColor>,
    mut tab_bg: Query<&mut HoverBg>,
) {
    let Some(f) = fonts else { return };
    let scene_changed = loaded.read().count() > 0;
    state.media_scan -= time.delta_secs();
    let media_due = state.tab == LeftTab::Media && state.media_scan <= 0.0;
    let changed = state.dirty
        || sel.is_changed()
        || media_due
        || (scene_changed && state.tab != LeftTab::Media)
        || (project.is_changed() && state.tab == LeftTab::Scenes);
    if !changed {
        return;
    }
    if media_due {
        state.media_scan = 3.0;
    }
    state.dirty = false;
    // Tab-Knöpfe hervorheben
    for (a, children) in &tabs {
        if let Action::Tab(t) = a {
            let active = *t == state.tab;
            for c in children.iter() {
                if let Ok(mut col) = tab_text.get_mut(c) {
                    col.0 = if active { ACCENT } else { DIM };
                }
            }
        }
    }
    let _ = &mut tab_bg;
    let Ok(list) = list.single() else { return };
    commands.entity(list).despawn_children();
    let f = &*f;
    commands.entity(list).with_children(|l| match state.tab {
        LeftTab::Objects => {
            let scene = &src.scene;
            let all_sel = sel.id.is_none();
            l.spawn(button(Action::Select(None), if all_sel { SELECTED } else { Color::NONE }, row_node(0))).with_children(|r| {
                r.spawn(txt(f, "Szene", 13.0, TEXT, true));
                r.spawn(txt(f, format!("{} Objekte", count_objects(&scene.objects)), 12.0, DIM, false));
            });
            fn rec(l: &mut ChildSpawnerCommands, f: &UiFonts, objs: &[Object], depth: usize, sel: &Option<String>) {
                for o in objs {
                    let is_sel = sel.as_deref() == Some(o.id.as_str());
                    l.spawn(button(Action::Select(Some(o.id.clone())), if is_sel { SELECTED } else { Color::NONE }, row_node(depth)))
                        .with_children(|r| {
                            r.spawn((Node { width: px(6), height: px(6), border_radius: BorderRadius::all(px(3)), ..default() }, BackgroundColor(kind_color(&o.kind)), Pickable::IGNORE));
                            r.spawn(txt(f, o.id.clone(), 13.0, if o.hidden { DIM } else { TEXT }, is_sel));
                            r.spawn(Node { flex_grow: 1.0, ..default() });
                            let mut tag = kind_name(&o.kind).to_string();
                            if o.animate.is_some() {
                                tag.push_str(" ◆");
                            }
                            r.spawn(txt(f, tag, 11.0, DIM, false));
                        });
                    if let Kind::Group { children } = &o.kind {
                        rec(l, f, children, depth + 1, sel);
                    }
                }
            }
            rec(l, f, &scene.objects, 0, &sel.id);
            if !scene.overlays.is_empty() {
                l.spawn(Node { padding: UiRect::new(px(8), px(8), px(12), px(4)), ..default() }).with_children(|h| {
                    h.spawn(txt(f, "EINBLENDUNGEN", 11.0, DIM, true));
                });
                for o in &scene.overlays {
                    l.spawn(row_node(0)).with_children(|r| {
                        r.spawn(txt(f, "T", 12.0, ACCENT, true));
                        r.spawn(txt(f, o.text.replace('\n', " / ").chars().take(34).collect::<String>(), 13.0, TEXT, false));
                    });
                }
            }
        }
        LeftTab::Scenes => {
            let current = src.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let shots = &project.data.storyboard;
            if project.dir.is_none() {
                l.spawn(row_node(0)).with_children(|r| {
                    r.spawn(txt(f, "Kein Projekt geöffnet – Claude legt eins mit new_project an.", 12.0, DIM, false));
                });
            }
            if !shots.is_empty() {
                l.spawn(Node { padding: UiRect::new(px(8), px(8), px(6), px(4)), ..default() }).with_children(|h| {
                    let total: f32 = shots.iter().map(|s| s.duration).sum();
                    h.spawn(txt(f, format!("STORYBOARD · {} SHOTS · {}", shots.len(), format_time(total)), 11.0, DIM, true));
                });
            }
            let mut listed = std::collections::HashSet::new();
            for (i, s) in shots.iter().enumerate() {
                let is_cur = s.scene.as_deref() == Some(current.as_str());
                if let Some(sc) = &s.scene {
                    listed.insert(sc.clone());
                }
                let action = match &s.scene {
                    Some(sc) => Action::OpenScene(sc.clone()),
                    None => Action::Select(None),
                };
                l.spawn(button(action, if is_cur { SELECTED } else { Color::NONE }, row_node(0))).with_children(|r| {
                    r.spawn(txt(f, format!("{:02}", i + 1), 12.0, DIM, true));
                    let title = if s.title.is_empty() { s.id.clone() } else { s.title.clone() };
                    r.spawn(txt(f, title.chars().take(24).collect::<String>(), 13.0, TEXT, is_cur));
                    r.spawn(Node { flex_grow: 1.0, ..default() });
                    let (mark, col) = match s.status {
                        Status::Done => ("✓", ACCENT),
                        Status::InProgress => ("●", AMBER),
                        Status::Blocked => ("⚠", RED),
                        Status::Open => ("○", DIM),
                    };
                    let kind = if s.clip.is_some() { "Clip · " } else { "" };
                    r.spawn(txt(f, format!("{kind}{:.0} s", s.duration), 11.0, DIM, false));
                    r.spawn(txt(f, mark, 12.0, col, true));
                });
            }
            // Weitere Szenen-Dateien
            let mut others: Vec<String> = project
                .scenes_dir()
                .ok()
                .and_then(|d| std::fs::read_dir(d).ok())
                .map(|rd| {
                    rd.filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.extension().is_some_and(|x| x == "json"))
                        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                        .filter(|n| !listed.contains(n))
                        .collect()
                })
                .unwrap_or_default();
            others.sort();
            if !others.is_empty() {
                l.spawn(Node { padding: UiRect::new(px(8), px(8), px(12), px(4)), ..default() }).with_children(|h| {
                    h.spawn(txt(f, "WEITERE SZENEN", 11.0, DIM, true));
                });
                for n in others {
                    let is_cur = n == current;
                    l.spawn(button(Action::OpenScene(n.clone()), if is_cur { SELECTED } else { Color::NONE }, row_node(0))).with_children(|r| {
                        r.spawn(txt(f, n, 13.0, TEXT, is_cur));
                    });
                }
            }
        }
        LeftTab::Media => {
            for (folder, mark, col) in [
                (crate::media::AUDIO, "♪", ACCENT),
                (crate::media::CLIPS, "▶", AMBER),
                (crate::media::IMAGES, "□", VIOLET),
                (crate::media::MODELS, "◆", TEXT),
            ] {
                let (_, files) = crate::media::list(&project, folder);
                l.spawn(Node { padding: UiRect::new(px(8), px(8), px(10), px(4)), ..default() }).with_children(|h| {
                    h.spawn(txt(f, format!("{} · {}", folder.to_uppercase(), files.len()), 11.0, DIM, true));
                });
                if files.is_empty() {
                    l.spawn(row_node(0)).with_children(|r| {
                        r.spawn(txt(f, "leer", 12.0, DIM, false));
                    });
                }
                for file in files {
                    l.spawn(row_node(0)).with_children(|r| {
                        r.spawn(txt(f, mark, 12.0, col, true));
                        r.spawn(txt(f, file, 13.0, TEXT, false));
                    });
                }
            }
        }
    });
}

fn count_objects(objs: &[Object]) -> usize {
    objs.iter()
        .map(|o| 1 + if let Kind::Group { children } = &o.kind { count_objects(children) } else { 0 })
        .sum()
}

fn find_object<'a>(objs: &'a [Object], id: &str) -> Option<&'a Object> {
    for o in objs {
        if o.id == id {
            return Some(o);
        }
        if let Kind::Group { children } = &o.kind {
            if let Some(f) = find_object(children, id) {
                return Some(f);
            }
        }
    }
    None
}

/// Zahl kurz und ohne Rundungsrauschen (1.600000023 → 1.6).
fn num(x: f64) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s == "-0" { "0".into() } else { s }
}

fn v3(v: [f32; 3]) -> String {
    let f = |x: f32| {
        let s = format!("{x:.2}");
        let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
        if s == "-0" { "0".into() } else { s }
    };
    format!("{}, {}, {}", f(v[0]), f(v[1]), f(v[2]))
}

fn prop_row(p: &mut ChildSpawnerCommands, f: &UiFonts, key: &str, value: impl Into<String>, swatch: Option<Color>) {
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(8), align_items: AlignItems::Center, flex_shrink: 0.0, ..default() })
        .with_children(|r| {
            r.spawn(Node { width: px(92), flex_shrink: 0.0, ..default() }).with_children(|k| {
                k.spawn(txt(f, key, 12.0, DIM, false));
            });
            if let Some(c) = swatch {
                r.spawn((Node { width: px(14), height: px(14), border_radius: BorderRadius::all(px(3)), flex_shrink: 0.0, ..default() }, BackgroundColor(c), Pickable::IGNORE));
            }
            r.spawn(Node { flex_grow: 1.0, flex_shrink: 1.0, min_width: px(0), ..default() }).with_children(|v| {
                v.spawn(txt(f, value, 12.0, TEXT, false));
            });
        });
}

fn rebuild_props(
    mut commands: Commands,
    mut loaded: MessageReader<SceneLoaded>,
    sel: Res<Selection>,
    src: Res<SceneSource>,
    fonts: Option<Res<UiFonts>>,
    body: Query<Entity, With<PropsBody>>,
) {
    let Some(f) = fonts else { return };
    if loaded.read().count() == 0 && !sel.is_changed() {
        return;
    }
    let Ok(body) = body.single() else { return };
    commands.entity(body).despawn_children();
    let f = &*f;
    let scene = &src.scene;
    commands.entity(body).with_children(|p| match sel.id.as_deref().and_then(|id| find_object(&scene.objects, id)) {
        Some(o) => object_props(p, f, o),
        None => scene_props(p, f, scene, sel.id.as_deref()),
    });
}

fn object_props(p: &mut ChildSpawnerCommands, f: &UiFonts, o: &Object) {
    p.spawn(Node { flex_direction: FlexDirection::Column, padding: UiRect::bottom(px(4)), flex_shrink: 0.0, ..default() }).with_children(|h| {
        h.spawn(txt(f, o.id.clone(), 16.0, TEXT, true));
        h.spawn(txt(f, format!("{} · {}", kind_name(&o.kind), cf_scene::ops::kind_label(&o.kind)), 12.0, kind_color(&o.kind), false));
    });
    if let Some(n) = &o.name {
        prop_row(p, f, "Name", n.clone(), None);
    }
    prop_row(p, f, "Position", v3(o.position), None);
    if o.rotation != [0.0; 3] {
        prop_row(p, f, "Drehung", format!("{}°", v3(o.rotation)), None);
    }
    let s = o.scale.to_array();
    if s != [1.0; 3] {
        prop_row(p, f, "Größe", v3(s), None);
    }
    if o.hidden {
        prop_row(p, f, "Sichtbar", "nein (versteckt)", None);
    }
    let m = &o.material;
    if !matches!(
        o.kind,
        Kind::Group { .. } | Kind::Model { .. } | Kind::Svg { .. } | Kind::Particles(_) | Kind::Instance { .. } | Kind::Character(_) | Kind::Tree(_) | Kind::Water(_)
    ) {
        prop_row(p, f, "Farbe", m.color.to_hex(), Some(crate::environment::color(m.color)));
        let mut look = vec![format!("rau {:.2}", m.roughness)];
        if m.metallic > 0.0 {
            look.push(format!("Metall {:.2}", m.metallic));
        }
        if m.transmission > 0.0 {
            look.push(format!("Glas {:.2}", m.transmission));
        }
        if m.opacity < 1.0 {
            look.push(format!("Deckkraft {:.2}", m.opacity));
        }
        if m.unlit {
            look.push("ohne Licht".into());
        }
        prop_row(p, f, "Material", look.join(" · "), None);
        if !m.emissive.is_black() {
            prop_row(p, f, "Leuchten", format!("{} ×{}", m.emissive.to_hex(), m.emissive_strength), Some(crate::environment::color(m.emissive)));
        }
        if let Some(pat) = &m.pattern {
            prop_row(p, f, "Muster", format!("{:?}", pat.kind).to_lowercase(), None);
        }
        if let Some(t) = &m.texture {
            prop_row(p, f, "Bild", t.clone(), None);
        }
    }
    // Typ-spezifische Felder (kompakt)
    if let Ok(serde_json::Value::Object(map)) = serde_json::to_value(&o.kind) {
        for (k, v) in map {
            if matches!(k.as_str(), "type" | "children" | "shape") {
                continue;
            }
            if k == "actions" {
                if let Kind::Character(c) = &o.kind {
                    for a in &c.actions {
                        let end = a.end.unwrap_or(a.start + 2.0);
                        prop_row(p, f, "Aktion", format!("{} · {:.1}–{:.1} s", action_name(a.kind), a.start, end), None);
                    }
                }
                continue;
            }
            if k == "talk" {
                if let Kind::Character(c) = &o.kind {
                    if let Some(t) = &c.talk {
                        prop_row(p, f, "Spricht", format!("{} ab {:.1} s", t.audio, t.start), None);
                    }
                }
                continue;
            }
            let val = match &v {
                serde_json::Value::Number(n) => n.as_f64().map(num).unwrap_or_else(|| n.to_string()),
                serde_json::Value::Array(a) if a.len() <= 4 && a.iter().all(|x| x.is_number()) => {
                    a.iter().filter_map(|x| x.as_f64()).map(num).collect::<Vec<_>>().join(", ")
                }
                serde_json::Value::String(s) if s.len() > 60 => format!("({} Zeichen)", s.len()),
                serde_json::Value::Array(a) if a.len() > 4 => format!("({} Werte)", a.len()),
                other => {
                    let s = other.to_string();
                    if s.chars().count() > 60 { format!("{}…", s.chars().take(57).collect::<String>()) } else { s }
                }
            };
            prop_row(p, f, &k, val.trim_matches('"').to_string(), None);
        }
    }
    if let Some(r) = &o.repeat {
        prop_row(p, f, "Wiederholung", format!("×{} · Schritt [{}]", r.count, v3(r.offset)), None);
    }
    if let Some(a) = &o.animate {
        let mut parts = Vec::new();
        let mut add = |name: &str, n: usize| {
            if n > 0 {
                parts.push(format!("{name} ({n})"));
            }
        };
        add("Position", a.position.len());
        add("Drehung", a.rotation.len());
        add("Größe", a.scale.len());
        add("Farbe", a.color.len());
        add("Deckkraft", a.opacity.len());
        add("Leuchten", a.emissive_strength.len());
        add("Sichtbar", a.visible.len());
        add("Zeichnen", a.draw.len());
        add("Licht", a.intensity.len());
        if a.spin != [0.0; 3] {
            parts.push(format!("dreht sich [{}]°/s", v3(a.spin)));
        }
        if a.flicker > 0.0 {
            parts.push("flackert".into());
        }
        prop_row(p, f, "Animation", parts.join(" · "), None);
    }
    hint(p, f, &format!("Sag Claude z. B.: „Mach {} größer“ oder „Lass {} hüpfen“.", o.id, o.id));
}

fn scene_props(p: &mut ChildSpawnerCommands, f: &UiFonts, scene: &Scene, missing: Option<&str>) {
    p.spawn(Node { flex_direction: FlexDirection::Column, padding: UiRect::bottom(px(4)), flex_shrink: 0.0, ..default() }).with_children(|h| {
        h.spawn(txt(f, if scene.name.is_empty() { "Unbenannte Szene".to_string() } else { scene.name.clone() }, 16.0, TEXT, true));
        if !scene.description.is_empty() {
            h.spawn(txt(f, scene.description.clone(), 12.0, DIM, false));
        }
    });
    if let Some(id) = missing {
        prop_row(p, f, "Auswahl", format!("„{id}“ (Teil eines Bibliotheks-Objekts)"), None);
    }
    prop_row(p, f, "Länge", format!("{} · {} Bilder/s", format_time(scene.duration), scene.fps), None);
    let env = &scene.environment;
    match &env.sky {
        cf_scene::Sky::Atmosphere => prop_row(p, f, "Himmel", "Atmosphäre (echter Himmel)", None),
        cf_scene::Sky::Color { color } => prop_row(p, f, "Himmel", color.to_hex(), Some(crate::environment::color(*color))),
    }
    prop_row(p, f, "Sonne", format!("Richtung {:.0}° · Höhe {:.0}°", env.sun.azimuth, env.sun.elevation), Some(crate::environment::color(env.sun.color)));
    if let Some(fog) = &env.fog {
        prop_row(p, f, "Nebel", format!("Sicht {:.0} m", fog.visibility), Some(crate::environment::color(fog.color)));
    }
    prop_row(p, f, "Stil", if scene.exact_colors() { "Cartoon (exakte Farben)" } else { "Film" }, None);
    let cam = &scene.camera;
    let lens = match cam.orthographic {
        Some(h) => format!("2D-Kamera · Bildhöhe {h} m"),
        None => format!("Bildwinkel {:.0}°", cam.fov),
    };
    prop_row(p, f, "Kamera", format!("[{}] → [{}]", v3(cam.position), v3(cam.look_at)), None);
    prop_row(p, f, "Objektiv", lens, None);
    if cam.animate.is_some() {
        prop_row(p, f, "Kamerafahrt", "ja", None);
    }
    prop_row(p, f, "Objekte", count_objects(&scene.objects).to_string(), None);
    if !scene.overlays.is_empty() {
        prop_row(p, f, "Einblendungen", scene.overlays.len().to_string(), None);
    }
    hint(p, f, "Klick auf ein Objekt (links oder in der Vorschau) wählt es aus – Claude weiß dann, was du meinst.");
}

fn hint(p: &mut ChildSpawnerCommands, f: &UiFonts, text: &str) {
    p.spawn((
        Node { margin: UiRect::top(px(10)), padding: UiRect::all(px(10)), border_radius: BorderRadius::all(px(8)), flex_shrink: 0.0, ..default() },
        BackgroundColor(PANEL_2),
    ))
    .with_children(|h| {
        h.spawn(txt(f, text, 12.0, DIM, false));
    });
}

fn rebuild_activity(
    mut commands: Commands,
    project: Res<ActiveProject>,
    fonts: Option<Res<UiFonts>>,
    body: Query<Entity, With<ActivityBody>>,
    mut first: Local<bool>,
) {
    let Some(f) = fonts else { return };
    if !project.is_changed() && *first {
        return;
    }
    *first = true;
    let Ok(body) = body.single() else { return };
    commands.entity(body).despawn_children();
    let f = &*f;
    let d = &project.data;
    commands.entity(body).with_children(|p| {
        if project.dir.is_none() {
            p.spawn(txt(f, "Noch kein Projekt. Sag Claude z. B.: „Erstelle in CineForge einen Kurzfilm über …“", 12.0, DIM, false));
            return;
        }
        if !d.next_step.is_empty() {
            p.spawn((
                Node { padding: UiRect::all(px(10)), border_radius: BorderRadius::all(px(8)), flex_direction: FlexDirection::Column, row_gap: px(3), flex_shrink: 0.0, ..default() },
                BackgroundColor(SELECTED.with_alpha(0.6)),
            ))
            .with_children(|b| {
                b.spawn(txt(f, "NÄCHSTER SCHRITT", 10.0, ACCENT, true));
                b.spawn(txt(f, d.next_step.clone(), 12.0, TEXT, false));
            });
        }
        let open = d.tasks.iter().filter(|t| t.status != Status::Done).count();
        if !d.tasks.is_empty() {
            p.spawn(txt(f, format!("Aufgaben: {} erledigt, {open} offen", d.tasks.len() - open), 12.0, DIM, false));
        }
        for e in d.log.iter().rev().take(40) {
            let time = e.time.get(11..16).unwrap_or(&e.time).to_string();
            let who = match e.actor.as_str() {
                "claude" => "Claude".to_string(),
                "mensch" => "Du".to_string(),
                other => other.to_string(),
            };
            p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(8), flex_shrink: 0.0, ..default() }).with_children(|r| {
                r.spawn(Node { width: px(40), flex_shrink: 0.0, ..default() }).with_children(|t| {
                    t.spawn(txt(f, time, 11.0, DIM, false));
                });
                r.spawn(Node { flex_direction: FlexDirection::Column, flex_shrink: 1.0, min_width: px(0), ..default() }).with_children(|c| {
                    c.spawn(txt(f, who, 11.0, if e.actor == "mensch" { AMBER } else { ACCENT }, true));
                    c.spawn(txt(f, e.action.clone(), 12.0, TEXT, false));
                });
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Zeitleiste
// ---------------------------------------------------------------------------

/// Eine Spur: Beschriftung + Inhalte (Balken von–bis, Rauten für Keyframes).
struct Track {
    label: String,
    color: Color,
    bars: Vec<(f32, f32, String)>,
    keys: Vec<f32>,
}

fn key_times<T>(k: &[cf_scene::animation::Key<T>]) -> impl Iterator<Item = f32> + '_ {
    k.iter().map(|k| k.t)
}

fn object_tracks(objs: &[Object], prefix: &str, out: &mut Vec<Track>, duration: f32) {
    for o in objs {
        let id = format!("{prefix}{}", o.id);
        if let Kind::Character(c) = &o.kind {
            let mut bars: Vec<(f32, f32, String)> =
                c.actions.iter().map(|a| (a.start, a.end.unwrap_or(a.start + 2.0), action_name(a.kind).to_string())).collect();
            if let Some(p) = o.animate.as_ref().and_then(|a| a.path.as_ref()) {
                if c.auto_walk && !c.actions.iter().any(|a| matches!(a.kind, cf_scene::character::ActionKind::Walk | cf_scene::character::ActionKind::Run)) {
                    bars.push((p.start, p.end, "geht den Weg".into()));
                }
            }
            if let Some(t) = &c.talk {
                bars.push((t.start, duration, format!("♪ {}", t.audio)));
            }
            out.push(Track { label: id.clone(), color: Color::srgb(1.0, 0.62, 0.3), bars, keys: vec![] });
            continue;
        }
        if let Some(a) = &o.animate {
            let mut keys: Vec<f32> = key_times(&a.position)
                .chain(key_times(&a.rotation))
                .chain(key_times(&a.scale))
                .chain(key_times(&a.color))
                .chain(key_times(&a.opacity))
                .chain(key_times(&a.emissive_strength))
                .chain(key_times(&a.visible))
                .chain(key_times(&a.draw))
                .chain(key_times(&a.intensity))
                .collect();
            keys.sort_by(f32::total_cmp);
            keys.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
            let mut bars = Vec::new();
            if let Some(p) = &a.path {
                bars.push((p.start, p.end, "Weg".to_string()));
                keys.extend([p.start, p.end]);
            }
            if a.talk.is_some() || a.blink.is_some() || a.breathe != 0.0 {
                let what = if a.talk.is_some() { "spricht" } else if a.blink.is_some() { "blinzelt" } else { "atmet" };
                bars.push((a.talk.as_ref().map(|t| t.start).unwrap_or(0.0), duration, what.to_string()));
            }
            if a.spin != [0.0; 3] || a.flicker > 0.0 {
                bars.push((0.0, duration, if a.spin != [0.0; 3] { "dreht sich" } else { "flackert" }.to_string()));
            } else if keys.len() >= 2 && bars.is_empty() {
                bars.push((keys[0], *keys.last().unwrap_or(&0.0), String::new()));
            }
            out.push(Track { label: id.clone(), color: VIOLET, bars, keys });
        } else if let Kind::Particles(p) = &o.kind {
            let end = p.end.unwrap_or(duration);
            out.push(Track { label: id.clone(), color: AMBER, bars: vec![(p.start, end, format!("{:?}", p.preset).to_lowercase())], keys: vec![] });
        }
        if let Kind::Group { children } = &o.kind {
            object_tracks(children, &format!("{id}/"), out, duration);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild_tracks(
    mut commands: Commands,
    mut loaded: MessageReader<SceneLoaded>,
    src: Res<SceneSource>,
    project: Res<ActiveProject>,
    fonts: Option<Res<UiFonts>>,
    labels: Query<Entity, With<TrackLabels>>,
    lanes: Query<Entity, With<TrackLanes>>,
    film: Query<Entity, With<FilmLane>>,
    mut first: Local<bool>,
) {
    let Some(f) = fonts else { return };
    if loaded.read().count() == 0 && !project.is_changed() && *first {
        return;
    }
    *first = true;
    let (Ok(labels), Ok(lanes), Ok(film)) = (labels.single(), lanes.single(), film.single()) else { return };
    let f = &*f;
    let scene = &src.scene;
    let dur = scene.duration.max(0.01);
    let current = src.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

    // Film-Leiste
    commands.entity(film).despawn_children();
    commands.entity(film).with_children(|row| {
        let shots = &project.data.storyboard;
        if shots.is_empty() {
            row.spawn(Node { align_items: AlignItems::Center, ..default() }).with_children(|n| {
                n.spawn(txt(f, "Noch kein Storyboard – Claude plant die Shots mit update_plan.", 12.0, DIM, false));
            });
            return;
        }
        for s in shots {
            let is_cur = s.scene.as_deref() == Some(current.as_str());
            let base = if s.clip.is_some() { AMBER.with_alpha(0.35) } else if is_cur { ACCENT.with_alpha(0.55) } else { PANEL_2 };
            let action = match &s.scene {
                Some(sc) => Action::OpenScene(sc.clone()),
                None => Action::Select(None),
            };
            row.spawn(button(
                action,
                base,
                Node {
                    flex_grow: s.duration.max(0.1),
                    flex_basis: px(0),
                    min_width: px(4),
                    padding: UiRect::axes(px(6), px(0)),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    border_radius: BorderRadius::all(px(4)),
                    border: UiRect::all(px(if is_cur { 1.0 } else { 0.0 })),
                    ..default()
                },
            ))
            .insert(BorderColor::all(ACCENT))
            .with_children(|b| {
                let title = if s.title.is_empty() { s.id.clone() } else { s.title.clone() };
                b.spawn((txt(f, title, 11.0, TEXT, is_cur), TextLayout::no_wrap()));
            });
        }
    });

    // Spuren der geöffneten Szene
    let mut tracks: Vec<Track> = Vec::new();
    let cam_keys: Vec<f32> = scene
        .camera
        .animate
        .as_ref()
        .map(|a| {
            let mut k: Vec<f32> = key_times(&a.position).chain(key_times(&a.look_at)).chain(key_times(&a.fov)).collect();
            k.sort_by(f32::total_cmp);
            k.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
            k
        })
        .unwrap_or_default();
    let cam_bar = match (cam_keys.first(), cam_keys.last()) {
        (Some(a), Some(b)) if b > a => vec![(*a, *b, "Kamerafahrt".to_string())],
        (Some(_), _) => vec![],
        _ => vec![(0.0, dur, "feste Kamera".to_string())],
    };
    tracks.push(Track { label: "Kamera".into(), color: ACCENT, bars: cam_bar, keys: cam_keys });
    if !scene.overlays.is_empty() {
        let bars = scene.overlays.iter().map(|o| (o.start, o.end.unwrap_or(dur), o.text.replace('\n', " / "))).collect();
        tracks.push(Track { label: "Text".into(), color: Color::srgb(0.95, 0.55, 0.75), bars, keys: vec![] });
    }
    if let Some(shot) = project.data.storyboard.iter().find(|s| s.scene.as_deref() == Some(current.as_str())) {
        if !shot.audio.is_empty() {
            let bars = shot
                .audio
                .iter()
                .map(|a| (a.start, a.duration.map(|d| a.start + d).unwrap_or(dur).min(dur), a.file.clone()))
                .collect();
            tracks.push(Track { label: "Ton".into(), color: Color::srgb(0.35, 0.65, 1.0), bars, keys: vec![] });
        }
    }
    let mut objs = Vec::new();
    object_tracks(&scene.objects, "", &mut objs, dur);
    let extra = objs.len().saturating_sub(MAX_OBJECT_TRACKS);
    tracks.extend(objs.into_iter().take(MAX_OBJECT_TRACKS));

    commands.entity(labels).despawn_children();
    commands.entity(lanes).despawn_children();
    commands.entity(labels).with_children(|l| {
        // Lineal-Beschriftung
        l.spawn(Node { height: px(20), padding: UiRect::left(px(14)), align_items: AlignItems::Center, ..default() }).with_children(|n| {
            n.spawn(txt(f, "Szene", 11.0, DIM, true));
        });
        for t in &tracks {
            l.spawn(Node { height: px(ROW_H), padding: UiRect::left(px(14)), align_items: AlignItems::Center, overflow: Overflow::clip(), ..default() })
                .with_children(|n| {
                    n.spawn((txt(f, t.label.clone(), 12.0, TEXT, false), TextLayout::no_wrap()));
                });
        }
        if extra > 0 {
            l.spawn(Node { height: px(16), padding: UiRect::left(px(14)), ..default() }).with_children(|n| {
                n.spawn(txt(f, format!("+{extra} weitere"), 11.0, DIM, false));
            });
        }
    });
    commands.entity(lanes).with_children(|l| {
        // Lineal mit Sekunden-Marken
        l.spawn((Node { height: px(20), ..default() }, Pickable::IGNORE)).with_children(|r| {
            let step = [0.5_f32, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0].into_iter().find(|s| dur / s <= 14.0).unwrap_or(120.0);
            let mut t = 0.0;
            while t <= dur + 1e-3 {
                let x = t / dur * 100.0;
                r.spawn((
                    Node { position_type: PositionType::Absolute, left: percent(x), bottom: px(0), width: px(1), height: px(6), ..default() },
                    BackgroundColor(DIM),
                    Pickable::IGNORE,
                ));
                if t + step <= dur + 1e-3 || t == 0.0 {
                    r.spawn((Node { position_type: PositionType::Absolute, left: percent(x), top: px(0), margin: UiRect::left(px(3)), ..default() }, Pickable::IGNORE))
                        .with_children(|n| {
                            n.spawn(txt(f, ruler_label(t, dur), 10.0, DIM, false));
                        });
                }
                t += step;
            }
        });
        for t in &tracks {
            l.spawn((
                Node { height: px(ROW_H), margin: UiRect::vertical(px(1)), border_radius: BorderRadius::all(px(4)), ..default() },
                BackgroundColor(PANEL_2),
                Pickable::IGNORE,
            ))
            .with_children(|r| {
                for (a, b, label) in &t.bars {
                    let (a, b) = (a.clamp(0.0, dur), b.clamp(0.0, dur));
                    r.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(a / dur * 100.0),
                            width: percent(((b - a) / dur * 100.0).max(0.4)),
                            top: px(3),
                            bottom: px(3),
                            padding: UiRect::left(px(6)),
                            align_items: AlignItems::Center,
                            overflow: Overflow::clip(),
                            border_radius: BorderRadius::all(px(3)),
                            ..default()
                        },
                        BackgroundColor(t.color.with_alpha(0.35)),
                        Pickable::IGNORE,
                    ))
                    .with_children(|bar| {
                        if !label.is_empty() {
                            bar.spawn((txt(f, label.clone(), 11.0, TEXT, false), TextLayout::no_wrap()));
                        }
                    });
                }
                for k in &t.keys {
                    r.spawn((
                        Node { position_type: PositionType::Absolute, left: percent(k.clamp(0.0, dur) / dur * 100.0), top: px(3), margin: UiRect::left(px(-5)), ..default() },
                        Pickable::IGNORE,
                    ))
                    .with_children(|n| {
                        n.spawn(txt(f, "◆", 12.0, t.color, false));
                    });
                }
            });
        }
    });
}

/// Kurze Zeitmarke fürs Lineal: „1,5 s“ bzw. „2:30“ bei langen Szenen.
fn ruler_label(t: f32, dur: f32) -> String {
    if dur > 90.0 {
        format!("{}:{:02}", (t / 60.0).floor() as u32, (t % 60.0).round() as u32)
    } else if (t - t.round()).abs() < 1e-3 {
        format!("{} s", t.round() as i32)
    } else {
        format!("{:.1} s", t).replace('.', ",")
    }
}

fn update_playhead(ph: Res<Playhead>, src: Res<SceneSource>, mut line: Query<&mut Node, With<PlayheadLine>>) {
    let x = (ph.time / src.scene.duration.max(1e-3) * 100.0).clamp(0.0, 100.0);
    for mut n in &mut line {
        if n.left != percent(x) {
            n.left = percent(x);
        }
    }
}

// ---------------------------------------------------------------------------
// Laufende Anzeigen
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn update_topbar(
    src: Res<SceneSource>,
    project: Res<ActiveProject>,
    control: Option<Res<ControlStatus>>,
    jobs: Option<Res<crate::render::RenderQueue>>,
    mut title: Query<&mut Text, (With<TopTitle>, Without<ClaudeText>, Without<JobText>)>,
    mut claude: Query<&mut Text, (With<ClaudeText>, Without<TopTitle>, Without<JobText>)>,
    mut job: Query<&mut Text, (With<JobText>, Without<TopTitle>, Without<ClaudeText>)>,
    mut dot: Query<&mut BackgroundColor, With<ClaudeDot>>,
) {
    let scene_name = if src.scene.name.is_empty() {
        src.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
    } else {
        src.scene.name.clone()
    };
    let t = match project.name() {
        Some(p) => format!("{p}  ›  {scene_name}"),
        None => scene_name,
    };
    for mut x in &mut title {
        if x.0 != t {
            x.0 = t.clone();
        }
    }
    let (msg, col) = match control.as_deref() {
        Some(ControlStatus { port: Some(_), calls, last_action, .. }) => {
            let last = last_action.as_deref().map(|a| format!(" · {a}")).unwrap_or_default();
            (format!("Claude verbunden · {calls} Aktionen{last}"), ACCENT)
        }
        Some(ControlStatus { error: Some(e), .. }) => (format!("Claude: {e}"), RED),
        _ => ("Claude-Verbindung aus".to_string(), DIM),
    };
    for mut x in &mut claude {
        if x.0 != msg {
            x.0 = msg.clone();
        }
    }
    for mut d in &mut dot {
        d.0 = col;
    }
    let j = jobs.as_ref().filter(|j| j.export_running()).and_then(|j| j.export_summary()).unwrap_or_default();
    let j = j.lines().next().unwrap_or("").to_string();
    for mut x in &mut job {
        if x.0 != j {
            x.0 = j.clone();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_transport(
    ph: Res<Playhead>,
    src: Res<SceneSource>,
    state: Res<UiState>,
    stats: Res<SceneStats>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    buttons: Query<(&Action, &Children)>,
    mut texts: Query<&mut Text, (Without<TimeLabel>, Without<StatsLabel>)>,
    mut colors: Query<&mut TextColor>,
    mut time_label: Query<&mut Text, (With<TimeLabel>, Without<StatsLabel>)>,
    mut stats_label: Query<&mut Text, (With<StatsLabel>, Without<TimeLabel>)>,
    badge: Query<&Children, With<CamBadge>>,
) {
    let t = format!("{} / {}", format_time(ph.time), format_time(src.scene.duration));
    for mut x in &mut time_label {
        if x.0 != t {
            x.0 = t.clone();
        }
    }
    for (a, children) in &buttons {
        let (label, on) = match a {
            Action::PlayPause => (if ph.playing { "II" } else { "▶" }, true),
            Action::Loop => ("Schleife", ph.looping),
            Action::CameraView => (if ph.camera_view { "Szenen-Kamera" } else { "Freie Kamera" }, ph.camera_view),
            Action::Portrait => (if state.portrait { "9:16" } else { "16:9" }, true),
            _ => continue,
        };
        for c in children.iter() {
            if let Ok(mut x) = texts.get_mut(c) {
                if x.0 != label {
                    x.0 = label.to_string();
                }
            }
            if let Ok(mut col) = colors.get_mut(c) {
                let want = if matches!(a, Action::PlayPause) { ACCENT } else if on { TEXT } else { DIM };
                if col.0 != want {
                    col.0 = want;
                }
            }
        }
    }
    let status = if ph.locked_by_job { "● Aufnahme läuft".to_string() } else if ph.camera_view { "Szenen-Kamera (wie im Film)".to_string() } else { "Freie Kamera · rechte Maustaste + WASD".to_string() };
    for children in &badge {
        for c in children.iter() {
            if let Ok(mut x) = texts.get_mut(c) {
                if x.0 != status {
                    x.0 = status.clone();
                }
            }
        }
    }
    let fps = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let line = format!(
        "{fps:.0} FPS · {} Objekte · {:.2} Mio. Dreiecke · Aufbau {:.0} ms ({} neu) · Rückgängig: {} · Leertaste: Abspielen · ←/→: springen · C: Szenen-Kamera · Esc: Auswahl aufheben",
        stats.objects,
        stats.triangles as f32 / 1e6,
        stats.build_ms,
        stats.rebuilt,
        src.history_len()
    );
    for mut x in &mut stats_label {
        if x.0 != line {
            x.0 = line.clone();
        }
    }
}

fn update_error(
    src: Res<SceneSource>,
    mut banner: Query<(&mut Node, &Children), With<ErrorBanner>>,
    mut texts: Query<&mut Text>,
    stats: Res<SceneStats>,
) {
    if !src.is_changed() && !stats.is_changed() {
        return;
    }
    let msg = match (&src.error, stats.warnings.first()) {
        (Some(e), _) => Some(format!("Änderung nicht übernommen – vorherige Fassung bleibt sichtbar: {e}")),
        (None, _) if src.notice.is_some() => src.notice.clone(),
        (None, Some(w)) => Some(format!("⚠ {w}{}", if stats.warnings.len() > 1 { format!(" (+{} weitere)", stats.warnings.len() - 1) } else { String::new() })),
        _ => None,
    };
    for (mut node, children) in &mut banner {
        node.display = if msg.is_some() { Display::Flex } else { Display::None };
        for c in children.iter() {
            if let Ok(mut t) = texts.get_mut(c) {
                t.0 = msg.clone().unwrap_or_default();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ui_symbols_exist_in_font() {
        for ch in "◀▶◆●○✓⚠♪□·›…„“–".chars() {
            assert!(cf_scene::modeling::font_has_char(crate::spawn::FONT, ch), "Zeichen fehlt in Inter: {ch}");
        }
    }
}
