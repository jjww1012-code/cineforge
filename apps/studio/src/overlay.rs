//! Einblendungen: Texte fest im Bild (Untertitel, Bauchbinden, Daten).
//!
//! Jede Kamera, die die Szene zeigt (Fenster und Aufnahme), bekommt die Texte
//! als Kinder direkt vor die Linse gesetzt. Position und Größe werden in jedem
//! Frame aus Bildwinkel und Seitenverhältnis berechnet – so stehen sie in
//! 16:9 und 9:16 an derselben relativen Stelle.

use bevy::{prelude::*, window::PrimaryWindow};
use cf_scene::{
    animation::{Animation, Ease, Key},
    Overlay, TextSpec,
};

use crate::{
    environment::SceneView,
    source::{SceneLoaded, SceneSource},
    spawn::{to_bevy_mesh, Animated, FONT, FONT_BLACK, FONT_BOLD},
};

/// Abstand vor der Kamera (knapp hinter der Nahgrenze von 0,1 m).
const DIST: f32 = 0.12;

pub struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (sync_overlays, place_overlays).chain().before(crate::timeline::TimelineSet));
    }
}

/// Seitenverhältnis einer Aufnahme-Kamera (Breite / Höhe).
#[derive(Component, Clone, Copy)]
pub struct ViewAspect(pub f32);

#[derive(Component)]
pub struct OverlayItem {
    pub overlay: Overlay,
    pub camera: Entity,
}

fn opacity_keys(o: &Overlay) -> Vec<Key<f32>> {
    let f = o.fade.max(0.0);
    let lin = |t: f32, v: f32| Key { t: t.max(0.0), v, ease: Ease::Linear };
    let mut k = if o.start <= 0.0 || f == 0.0 {
        vec![lin(0.0, if o.start <= 0.0 { 1.0 } else { 0.0 }), lin(o.start, 1.0)]
    } else {
        vec![lin(0.0, 0.0), lin(o.start, 0.0), lin(o.start + f, 1.0)]
    };
    if let Some(end) = o.end {
        let last = k.last().map(|x| x.t).unwrap_or(0.0);
        k.push(lin(end.max(last), 1.0));
        k.push(lin(end.max(last) + f.max(0.01), 0.0));
    }
    k.sort_by(|a, b| a.t.total_cmp(&b.t));
    k
}

/// Einblendungen sollen genau ihre Farbe zeigen: Die filmische Tonwertkurve (AgX)
/// dunkelt reines Weiß ab, daher wird die Farbe vorab angehoben (im Cartoon-Stil nicht nötig).
pub fn overlay_boost(scene: &cf_scene::Scene) -> f32 {
    if scene.exact_colors() { 1.0 } else { 2.2 }
}

pub fn overlay_color(c: cf_scene::Rgb, alpha: f32, boost: f32) -> Color {
    let l = Color::srgb(c.r, c.g, c.b).to_linear();
    Color::LinearRgba(LinearRgba::new(l.red * boost, l.green * boost, l.blue * boost, alpha))
}

#[allow(clippy::too_many_arguments)]
fn sync_overlays(
    mut commands: Commands,
    mut loaded: MessageReader<SceneLoaded>,
    src: Res<SceneSource>,
    new_views: Query<Entity, Added<SceneView>>,
    views: Query<Entity, With<SceneView>>,
    items: Query<(Entity, &OverlayItem)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let reload = loaded.read().count() > 0;
    let targets: Vec<Entity> = if reload { views.iter().collect() } else { new_views.iter().collect() };
    if targets.is_empty() {
        return;
    }
    for (e, item) in &items {
        if reload || targets.contains(&item.camera) {
            commands.entity(e).despawn();
        }
    }
    let boost = overlay_boost(&src.scene);
    for cam in targets {
        for o in &src.scene.overlays {
            let spec = TextSpec {
                text: o.text.clone(),
                size: 1.0,
                depth: 0.0,
                align: o.align,
                letter_spacing: 0.0,
                weight: o.weight,
                italic: o.italic,
            };
            let font = match o.weight {
                cf_scene::TextWeight::Regular => FONT,
                cf_scene::TextWeight::Bold => FONT_BOLD,
                cf_scene::TextWeight::Black => FONT_BLACK,
            };
            let Ok(data) = cf_scene::modeling::text(font, &spec) else { continue };
            if data.indices.is_empty() {
                continue;
            }
            let mat = materials.add(StandardMaterial {
                base_color: overlay_color(o.color, 0.0, boost),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                double_sided: true,
                ..default()
            });
            let anim = Animation { opacity: opacity_keys(o), ..default() };
            commands.spawn((
                Name::new("Einblendung"),
                OverlayItem { overlay: o.clone(), camera: cam },
                Mesh3d(meshes.add(to_bevy_mesh(data))),
                MeshMaterial3d(mat.clone()),
                Transform::from_scale(Vec3::ZERO),
                bevy::light::NotShadowCaster,
                Animated {
                    anim,
                    base_pos: [0.0, 0.0, -DIST],
                    base_rot: [0.0; 3],
                    base_scale: [0.0; 3],
                    hidden: false,
                    repeat_op: Transform::IDENTITY,
                    material: Some(mat),
                    base_color: o.color,
                    base_emissive: cf_scene::Rgb::BLACK,
                    base_emissive_strength: 0.0,
                    base_opacity: 1.0,
                    has_pattern: false,
                    base_intensity: 0.0,
                    overlay: Some(boost),
                    talk: None,
                },
                ChildOf(cam),
            ));
        }
    }
}

fn place_overlays(
    mut items: Query<(&OverlayItem, &mut Animated)>,
    cams: Query<(&Projection, Option<&ViewAspect>)>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let win_aspect = window.single().map(|w| w.width() / w.height().max(1.0)).unwrap_or(16.0 / 9.0);
    for (item, mut a) in &mut items {
        let Ok((proj, aspect)) = cams.get(item.camera) else { continue };
        let aspect = aspect.map(|v| v.0).unwrap_or(win_aspect);
        let h = match proj {
            Projection::Perspective(p) => (p.fov / 2.0).tan() * DIST,
            Projection::Orthographic(o) => match o.scaling_mode {
                bevy::camera::ScalingMode::FixedVertical { viewport_height } => viewport_height / 2.0 * o.scale,
                _ => (o.area.height() / 2.0).max(0.01),
            },
            _ => (25f32.to_radians()).tan() * DIST,
        };
        let o = &item.overlay;
        // Größe bezogen auf die kürzere Bildseite: passt in 16:9 und 9:16 gleichermaßen.
        let size = o.size * 2.0 * h * aspect.min(1.0);
        let pos = [o.x * h * aspect, o.y * h, -DIST];
        let scale = [size, size, size];
        if a.base_pos != pos || a.base_scale != scale {
            a.base_pos = pos;
            a.base_scale = scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ov(start: f32, end: Option<f32>) -> Overlay {
        serde_json::from_value(serde_json::json!({"text": "x", "start": start, "end": end})).unwrap()
    }

    #[test]
    fn opacity_keys_fade_in_and_out() {
        let k = opacity_keys(&ov(2.0, Some(5.0)));
        let s = |t| cf_scene::animation::sample(&k, t).unwrap();
        assert_eq!(s(1.0), 0.0);
        assert!((s(2.15) - 0.5).abs() < 0.01);
        assert_eq!(s(4.0), 1.0);
        assert_eq!(s(9.0), 0.0);
        let from_start = opacity_keys(&ov(0.0, None));
        assert_eq!(cf_scene::animation::sample(&from_start, 0.0), Some(1.0));
    }
}
