//! Zeit: Wiedergabe, Animation der Objekte, Kamerafahrt, Sonnenstand.
//!
//! Tasten: Leertaste = Abspielen/Pause · ←/→ = 1 s (mit Shift 0,1 s) ·
//! Pos1/Ende = Anfang/Ende · L = Schleife · V = durch die Szenen-Kamera schauen.

use bevy::{
    camera::Exposure,
    camera_controller::free_camera::FreeCameraState,
    prelude::*,
};
use cf_scene::{animation::camera_at, Sky};

use crate::{
    environment::{color, sun_direction, SceneView, SunLight},
    source::{SceneLoaded, SceneSource},
    spawn::Animated,
    MainCamera,
};

pub struct TimelinePlugin {
    pub interactive: bool,
}

#[derive(Resource, Debug)]
pub struct Playhead {
    pub time: f32,
    pub playing: bool,
    pub looping: bool,
    /// Fenster zeigt die (animierte) Szenen-Kamera statt der freien Flug-Kamera.
    pub camera_view: bool,
    /// Ein Render-Auftrag (Screenshot, Export) steuert gerade die Zeit.
    pub locked_by_job: bool,
}

impl Default for Playhead {
    fn default() -> Self {
        Self { time: 0.0, playing: false, looping: true, camera_view: false, locked_by_job: false }
    }
}

impl Plugin for TimelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Playhead>().add_systems(
            Update,
            (advance, animate_objects, draw_tubes, animate_environment, follow_scene_camera)
                .chain()
                .in_set(TimelineSet),
        );
        if self.interactive {
            app.add_systems(Update, playback_keys.before(TimelineSet));
        }
    }
}

/// Alles, was den Zustand zu einem Zeitpunkt herstellt. Render-Aufträge laufen davor.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimelineSet;

/// Kamera programmatisch versetzen, ohne dass die Flug-Kamera danach zurückspringt.
pub fn set_view(target: &mut Transform, new: Transform, state: Option<&mut FreeCameraState>) {
    *target = new;
    if let Some(s) = state {
        let (yaw, pitch, _) = new.rotation.to_euler(EulerRot::YXZ);
        s.yaw = yaw;
        s.pitch = pitch;
        s.velocity = Vec3::ZERO;
    }
}

fn playback_keys(keys: Res<ButtonInput<KeyCode>>, mut ph: ResMut<Playhead>, src: Res<SceneSource>) {
    if ph.locked_by_job {
        return;
    }
    let dur = src.scene.duration;
    let fine = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let step = if fine { 0.1 } else { 1.0 };
    if keys.just_pressed(KeyCode::Space) {
        if !ph.playing && ph.time >= dur - 1e-3 {
            ph.time = 0.0;
        }
        ph.playing = !ph.playing;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        ph.time = (ph.time + step).min(dur);
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        ph.time = (ph.time - step).max(0.0);
    }
    if keys.just_pressed(KeyCode::Home) {
        ph.time = 0.0;
    }
    if keys.just_pressed(KeyCode::End) {
        ph.time = dur;
    }
    if keys.just_pressed(KeyCode::KeyL) {
        ph.looping = !ph.looping;
    }
    if keys.just_pressed(KeyCode::KeyV) {
        ph.camera_view = !ph.camera_view;
    }
}

fn advance(time: Res<Time>, mut ph: ResMut<Playhead>, src: Res<SceneSource>, mut loaded: MessageReader<SceneLoaded>) {
    let dur = src.scene.duration;
    // Neue Datei: zurück auf Anfang – außer ein Aufnahme-Auftrag steuert die Zeit.
    if loaded.read().any(|m| m.new_file) && !ph.locked_by_job {
        ph.time = 0.0;
        ph.playing = false;
    }
    if ph.time > dur {
        ph.time = dur;
    }
    if !ph.playing || ph.locked_by_job {
        return;
    }
    ph.time += time.delta_secs();
    if ph.time >= dur {
        if ph.looping {
            ph.time = ph.time.rem_euclid(dur.max(1e-3));
        } else {
            ph.time = dur;
            ph.playing = false;
        }
    }
}

fn animate_objects(
    ph: Res<Playhead>,
    mut objects: Query<(&Animated, &mut Transform, &mut Visibility, Option<&mut PointLight>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let t = ph.time;
    for (a, mut tf, mut vis, light) in &mut objects {
        if let (Some(mut light), Some(i)) = (light, a.anim.light_intensity_at(a.base_intensity, t)) {
            light.intensity = i;
        }
        let s = a.anim.state_at(a.base_pos, a.base_rot, a.base_scale, t, a.hidden);
        let [rx, ry, rz] = s.rotation.map(f32::to_radians);
        let local = Transform {
            translation: Vec3::from_array(s.position),
            rotation: Quat::from_euler(EulerRot::YXZ, ry, rx, rz),
            scale: Vec3::from_array(s.scale),
        };
        let new = a.repeat_op.mul_transform(local);
        if *tf != new {
            *tf = new;
        }
        let v = if s.visible { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != v {
            *vis = v;
        }
        if let Some(h) = &a.material {
            if let Some(mut m) = materials.get_mut(h) {
                let c = s.color.unwrap_or(a.base_color);
                let opacity = s.opacity.unwrap_or(a.base_opacity);
                if a.has_pattern {
                    m.base_color = Color::srgba(1.0, 1.0, 1.0, opacity);
                } else if let Some(boost) = a.overlay {
                    // Einblendung (siehe overlay.rs): Farbe angehoben
                    m.base_color = crate::overlay::overlay_color(c, opacity, boost);
                } else {
                    m.base_color = Color::srgba(c.r, c.g, c.b, opacity);
                }
                let strength = s.emissive_strength.unwrap_or(a.base_emissive_strength).max(0.0);
                m.emissive = if a.base_emissive.is_black() {
                    LinearRgba::BLACK
                } else {
                    color(a.base_emissive).to_linear() * strength
                };
            }
        }
    }
}

/// „Linie zeichnet sich“: Mesh der Röhre passend zum gezeichneten Anteil neu erzeugen.
fn draw_tubes(
    ph: Res<Playhead>,
    mut tubes: Query<(&mut crate::spawn::DrawTube, &Animated)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut d, a) in &mut tubes {
        let f = cf_scene::animation::sample(&a.anim.draw, ph.time).unwrap_or(1.0).clamp(0.0, 1.0);
        if (f - d.last).abs() < 1e-4 {
            continue;
        }
        d.last = f;
        let data = cf_scene::modeling::tube_partial(&d.spec, f);
        let mesh = if data.indices.is_empty() {
            // Nichts gezeichnet: winziges unsichtbares Dreieck statt leerem Mesh
            crate::spawn::to_bevy_mesh(cf_scene::procedural::MeshData {
                positions: vec![[0.0; 3]; 3],
                normals: vec![[0.0, 1.0, 0.0]; 3],
                uvs: vec![[0.0; 2]; 3],
                indices: vec![0, 1, 2],
            })
        } else {
            crate::spawn::to_bevy_mesh(data)
        };
        let _ = meshes.insert(&d.mesh, mesh);
    }
}

fn animate_environment(
    ph: Res<Playhead>,
    src: Res<SceneSource>,
    mut sun: Query<(&mut Transform, &mut DirectionalLight), With<SunLight>>,
    mut exposure: Query<&mut Exposure, With<SceneView>>,
) {
    let env = &src.scene.environment;
    let Some(a) = &env.animate else { return };
    let t = ph.time;
    let sample = |k: &[cf_scene::animation::Key<f32>], base: f32| cf_scene::animation::sample(k, t).unwrap_or(base);
    let el = sample(&a.sun_elevation, env.sun.elevation);
    let az = sample(&a.sun_azimuth, env.sun.azimuth);
    let intensity = sample(&a.sun_intensity, env.sun.intensity);
    let ev = sample(&a.exposure, env.exposure);
    let atmosphere = matches!(env.sky, Sky::Atmosphere);
    let base_lux = if atmosphere {
        bevy::light::light_consts::lux::RAW_SUNLIGHT
    } else {
        bevy::light::light_consts::lux::DIRECT_SUNLIGHT
    };
    for (mut tf, mut light) in &mut sun {
        let dir = sun_direction(az, el);
        let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
        *tf = Transform::from_translation(dir).looking_at(Vec3::ZERO, up);
        light.illuminance = base_lux * intensity.max(0.0);
    }
    let base_ev = if atmosphere { 13.0 } else { Exposure::EV100_SUNLIGHT };
    for mut e in &mut exposure {
        e.ev100 = base_ev - ev;
    }
}

/// Transform der Szenen-Kamera zum Zeitpunkt t.
pub fn scene_camera_at(scene: &cf_scene::Scene, t: f32) -> (Transform, f32) {
    let (pos, look, fov) = camera_at(&scene.camera, t);
    let (p, l) = (Vec3::from_array(pos), Vec3::from_array(look));
    let tf = if (l - p).length_squared() < 1e-6 {
        Transform::from_translation(p)
    } else {
        Transform::from_translation(p).looking_at(l, Vec3::Y)
    };
    (tf, fov)
}

fn follow_scene_camera(
    ph: Res<Playhead>,
    src: Res<SceneSource>,
    mut cam: Query<(&mut Transform, &mut Projection, Option<&mut FreeCameraState>), With<MainCamera>>,
    mut was_following: Local<bool>,
) {
    let Ok((mut tf, mut proj, state)) = cam.single_mut() else { return };
    if ph.camera_view {
        let (new, fov) = scene_camera_at(&src.scene, ph.time);
        *tf = new;
        if let Projection::Perspective(p) = proj.as_mut() {
            p.fov = fov.clamp(5.0, 150.0).to_radians();
        }
        if let Some(mut s) = state {
            s.enabled = false;
        }
        *was_following = true;
    } else if *was_following {
        // Zurück zur Flug-Kamera: dort weiterfliegen, wo die Szenen-Kamera gerade ist.
        if let Some(mut s) = state {
            s.enabled = true;
            let current = *tf;
            set_view(&mut tf, current, Some(&mut s));
        }
        if let Projection::Perspective(p) = proj.as_mut() {
            p.fov = src.scene.camera.fov.clamp(5.0, 150.0).to_radians();
        }
        *was_following = false;
    }
}

// ---------------------------------------------------------------------------
// Zeitleiste (Anzeige)
// ---------------------------------------------------------------------------

pub fn format_time(t: f32) -> String {
    let t = t.max(0.0);
    let m = (t / 60.0).floor() as u32;
    let s = t - m as f32 * 60.0;
    format!("{m:02}:{s:04.1}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_format() {
        assert_eq!(format_time(0.0), "00:00.0");
        assert_eq!(format_time(83.45), "01:23.4");
    }
}
