//! Himmel, Sonne, Nebel, Belichtung und Kamera-Einstellungen aus dem Szenen-Dokument.

use bevy::{
    anti_alias::smaa::Smaa,
    camera::Exposure,
    camera_controller::free_camera::FreeCameraState,
    core_pipeline::tonemapping::Tonemapping,
    light::{
        atmosphere::ScatteringMedium, light_consts::lux, Atmosphere, AtmosphereEnvironmentMapLight,
        CascadeShadowConfigBuilder, SunDisk,
    },
    light::{FogVolume, VolumetricFog, VolumetricLight},
    pbr::{AtmosphereSettings, DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion},
    post_process::{
        bloom::Bloom,
        dof::{DepthOfField, DepthOfFieldMode},
        effect_stack::Vignette,
        motion_blur::MotionBlur,
    },
    prelude::*,
    render::view::{ColorGrading, ColorGradingGlobal},
};
use cf_scene::{Environment, Rgb, Sky};

use crate::{
    source::{SceneLoaded, SceneSource},
    MainCamera,
};

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_media)
            .add_systems(Update, (apply_environment, reset_camera_key))
            .add_systems(Update, (follow_focus, preview_motion_blur).after(crate::timeline::TimelineSet));
    }
}

/// Alles, was von der Umgebung erzeugt wurde und beim Neuladen ersetzt wird.
#[derive(Component)]
struct EnvEntity;

/// Die Sonne (für die Animation des Sonnenstands).
#[derive(Component)]
pub struct SunLight;

#[derive(Resource)]
struct EarthMedium(Handle<ScatteringMedium>);

fn init_media(mut commands: Commands, mut media: ResMut<Assets<ScatteringMedium>>) {
    commands.insert_resource(EarthMedium(media.add(ScatteringMedium::earth(256, 256))));
}

pub fn color(c: Rgb) -> Color {
    Color::srgb(c.r, c.g, c.b)
}

/// Richtung *zur* Sonne aus Azimut (0° = -Z/Norden, 90° = +X/Osten) und Höhe.
pub fn sun_direction(azimuth_deg: f32, elevation_deg: f32) -> Vec3 {
    let (az, el) = (azimuth_deg.to_radians(), elevation_deg.to_radians());
    Vec3::new(az.sin() * el.cos(), el.sin(), -az.cos() * el.cos()).normalize()
}

/// Markiert jede Kamera, die die Szene zeigt (Fenster-Kamera und unsichtbare
/// Aufnahme-Kameras). Alle bekommen dieselben Umgebungs-Einstellungen.
#[derive(Component)]
pub struct SceneView;

fn apply_environment(
    mut commands: Commands,
    mut loaded: MessageReader<SceneLoaded>,
    src: Res<SceneSource>,
    medium: Res<EarthMedium>,
    old: Query<Entity, With<EnvEntity>>,
    mut main_cam: Query<(&mut Transform, Option<&mut FreeCameraState>), With<MainCamera>>,
    views: Query<Entity, With<SceneView>>,
    mut last: Local<Option<u64>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(new_file) = loaded.read().fold(None, |acc: Option<bool>, m| Some(acc.unwrap_or(false) | m.new_file))
    else {
        return;
    };
    let scene = &src.scene;
    let env: &Environment = &scene.environment;
    let atmosphere = matches!(env.sky, Sky::Atmosphere);

    // Kamera-Position nur beim Öffnen einer Datei setzen, damit man beim
    // Live-Editieren nicht ständig zurückgeworfen wird.
    if new_file {
        if let Ok((mut tf, state)) = main_cam.single_mut() {
            crate::timeline::set_view(&mut tf, scene_camera_transform(&scene.camera), state.map(|s| s.into_inner()));
        }
    }

    // Umgebung nur neu aufbauen, wenn sich etwas daran geändert hat (Objekt-Änderungen
    // sollen Himmel und Licht nicht neu starten).
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        let text = format!(
            "{}|{:?}|{}|{:?}|{:?}|{:?}|{}",
            serde_json::to_string(env).unwrap_or_default(),
            scene.camera.orthographic,
            scene.camera.fov,
            scene.camera.dof,
            scene.camera.motion_blur,
            ground_color(scene),
            scene.exact_colors()
        );
        text.hash(&mut h);
        h.finish()
    };
    if *last == Some(key) && !old.is_empty() {
        return;
    }
    *last = Some(key);
    for e in &old {
        commands.entity(e).despawn();
    }

    // Sonne
    let dir = sun_direction(env.sun.azimuth, env.sun.elevation);
    let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let base_lux = if atmosphere { lux::RAW_SUNLIGHT } else { lux::DIRECT_SUNLIGHT };
    let mut sun = commands.spawn((
        EnvEntity,
        SunLight,
        Name::new("Sonne"),
        DirectionalLight {
            color: color(env.sun.color),
            illuminance: base_lux * env.sun.intensity.max(0.0),
            shadow_maps_enabled: env.sun.shadows,
            ..default()
        },
        Transform::from_translation(dir).looking_at(Vec3::ZERO, up),
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 25.0,
            maximum_distance: 800.0,
            num_cascades: 4,
            ..default()
        }
        .build(),
    ));
    if env.light_rays > 0.0 {
        sun.insert(VolumetricLight);
    }
    if atmosphere {
        sun.insert(SunDisk::EARTH);
        let mut atmo = Atmosphere::earth(medium.0.clone());
        // Bodenfarbe der Himmels-Simulation = Farbe des Geländes, damit es am Horizont
        // nahtlos weitergeht (sonst sieht man hinter dem Gelände einen grauen Streifen).
        if let Some(c) = ground_color(scene) {
            let lin = color(c).to_linear();
            atmo.ground_albedo = Vec3::new(lin.red, lin.green, lin.blue);
        }
        commands.spawn((EnvEntity, Name::new("Atmosphäre"), atmo));
        commands.insert_resource(GlobalAmbientLight::NONE);
    } else {
        let Sky::Color { color: sky } = &env.sky else { unreachable!() };
        commands.insert_resource(ClearColor(color(*sky)));
        commands.insert_resource(GlobalAmbientLight {
            color: color(*sky).mix(&Color::WHITE, 0.5),
            brightness: 2500.0 * env.ambient.max(0.0),
            ..default()
        });
    }

    // Sterne: nur nachts (Sonne unter dem Horizont oder dunkler Farb-Himmel)
    let night = match &env.sky {
        Sky::Atmosphere => ((3.0 - env.sun.elevation) / 10.0).clamp(0.0, 1.0),
        Sky::Color { color: c } => (1.0 - (c.r + c.g + c.b) / 1.2).clamp(0.0, 1.0),
    };
    if env.stars > 0.0 && night > 0.0 {
        commands.spawn((
            EnvEntity,
            Name::new("Sterne"),
            Mesh3d(star_mesh(&mut meshes, env.stars)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE.with_alpha(night),
                unlit: true,
                fog_enabled: false,
                alpha_mode: AlphaMode::Add,
                cull_mode: None,
                ..default()
            })),
            Transform::from_translation(Vec3::from_array(scene.camera.position)),
            bevy::light::NotShadowCaster,
            bevy::picking::Pickable::IGNORE,
        ));
    }
    if let Some(m) = &env.moon {
        let mdir = sun_direction(m.azimuth, m.elevation.max(1.0));
        let up = if mdir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
        commands.spawn((
            EnvEntity,
            Name::new("Mondlicht"),
            DirectionalLight {
                color: color(m.color),
                illuminance: 2500.0 * m.intensity.max(0.0),
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_translation(mdir).looking_at(Vec3::ZERO, up),
        ));
        let disk = 70.0 * m.size.clamp(0.1, 10.0);
        commands.spawn((
            EnvEntity,
            Name::new("Mond"),
            Mesh3d(meshes.add(Sphere::new(disk).mesh().uv(32, 16))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.97, 0.9),
                unlit: true,
                fog_enabled: false,
                ..default()
            })),
            Transform::from_translation(Vec3::from_array(scene.camera.position) + mdir * 2800.0),
            bevy::light::NotShadowCaster,
            bevy::picking::Pickable::IGNORE,
        ));
    }
    if env.light_rays > 0.0 {
        // Dunst-Volumen um die Szene, in dem das Sonnenlicht sichtbar wird.
        let c = Vec3::from_array(scene.camera.look_at);
        let fog_color = env.fog.as_ref().map(|f| color(f.color)).unwrap_or(Color::WHITE);
        commands.spawn((
            EnvEntity,
            Name::new("Lichtstrahlen"),
            // Sehr dünner Dunst (das Volumen ist hunderte Meter groß): nur das Licht wird sichtbar.
            FogVolume { fog_color, density_factor: 0.004 * env.light_rays, scattering: 0.5, absorption: 0.05, ..default() },
            Transform::from_translation(c).with_scale(Vec3::new(400.0, 120.0, 400.0)),
        ));
    }

    for view in &views {
        configure_view(&mut commands.entity(view), scene);
    }
}

/// Sternenhimmel: kleine leuchtende Vierecke auf einer großen Kugel (deterministisch).
fn star_mesh(meshes: &mut Assets<Mesh>, density: f32) -> Handle<Mesh> {
    let n = (2500.0 * density.clamp(0.0, 4.0)) as usize;
    let mut rng = cf_scene::noise::Rng::new(4242);
    let (mut pos, mut col, mut idx) = (Vec::new(), Vec::new(), Vec::new());
    let r = 3000.0;
    for i in 0..n {
        // gleichmäßig auf der oberen Halbkugel (etwas unter den Horizont)
        let y = rng.range(-0.08, 1.0);
        let a = rng.range(0.0, std::f32::consts::TAU);
        let s = (1.0 - y * y).sqrt();
        let dir = Vec3::new(s * a.cos(), y, s * a.sin());
        let size = rng.range(2.0, 6.5) * if rng.f32() > 0.97 { 2.2 } else { 1.0 };
        let b = rng.range(0.35, 1.0);
        let tint = rng.f32();
        let c = [b * (0.85 + 0.15 * tint), b * 0.92, b * (1.0 - 0.12 * tint), 1.0];
        let up = if dir.y.abs() > 0.95 { Vec3::X } else { Vec3::Y };
        let t1 = dir.cross(up).normalize() * size;
        let t2 = dir.cross(t1).normalize() * size;
        let center = dir * r;
        let base = (i * 4) as u32;
        for p in [center - t1 - t2, center + t1 - t2, center + t1 + t2, center - t1 + t2] {
            pos.push(p.to_array());
            col.push(c);
        }
        // Vorderseite zur Kugelmitte (Kamera) hin
        idx.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    let normals = vec![[0.0, 1.0, 0.0]; pos.len()];
    meshes.add(
        Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
            .with_inserted_indices(bevy::mesh::Indices::U32(idx)),
    )
}

/// Fokus auf ein Objekt: Schärfe-Entfernung jedes Bild nachführen.
#[derive(Component)]
pub struct FocusOn(pub String);

fn follow_focus(
    mut views: Query<(&Transform, &FocusOn, &mut DepthOfField)>,
    objects: Query<(&crate::spawn::SceneObjectId, &GlobalTransform)>,
) {
    for (cam, focus, mut dof) in &mut views {
        if let Some((_, gt)) = objects.iter().find(|(id, _)| id.0 == focus.0) {
            let d = (gt.translation() - cam.translation).length().max(0.05);
            if (dof.focal_distance - d).abs() > 1e-3 {
                dof.focal_distance = d;
            }
        }
    }
}

/// In der Vorschau nur beim Abspielen verwischen (sonst schmiert jedes Springen).
fn preview_motion_blur(
    ph: Option<Res<crate::timeline::Playhead>>,
    src: Res<SceneSource>,
    mut cams: Query<&mut MotionBlur, With<MainCamera>>,
) {
    let Some(ph) = ph else { return };
    let want = if ph.playing { src.scene.camera.motion_blur.unwrap_or(0.0) } else { 0.0 };
    for mut mb in &mut cams {
        if mb.shutter_angle != want {
            mb.shutter_angle = want;
        }
    }
}

/// Farbe des größten Bodens der Szene (Terrain, sonst große flache Ebene/Box).
fn ground_color(scene: &cf_scene::Scene) -> Option<Rgb> {
    let mut best: Option<(f32, Rgb)> = None;
    scene.walk(&mut |o| {
        let area = match &o.kind {
            cf_scene::Kind::Terrain(t) => t.size[0] * t.size[1] * 10.0, // Gelände bevorzugen
            cf_scene::Kind::Plane { size } => size[0] * size[1],
            cf_scene::Kind::Box { size } if size[1] < 1.0 => size[0] * size[2],
            _ => 0.0,
        };
        if area > 1000.0 && best.is_none_or(|(a, _)| area > a) {
            best = Some((area, o.material.color));
        }
    });
    best.map(|(_, c)| c)
}

/// Perspektive oder 2D-Kamera (orthografisch) laut Szene.
pub fn scene_projection(cam: &cf_scene::CameraSpec) -> Projection {
    match cam.orthographic {
        Some(h) => Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::FixedVertical { viewport_height: h.max(0.01) },
            // Kleiner Tiefenbereich = hohe Genauigkeit für dicht gestapelte 2D-Ebenen.
            near: -50.0,
            far: 2_000.0,
            ..OrthographicProjection::default_3d()
        }),
        None => Projection::Perspective(PerspectiveProjection {
            fov: cam.fov.clamp(5.0, 150.0).to_radians(),
            near: 0.1,
            far: 20_000.0,
            ..default()
        }),
    }
}

/// Setzt alle kamerabezogenen Einstellungen (Belichtung, Himmel, Nebel, Bloom, Bildwinkel).
pub fn configure_view(cam: &mut EntityCommands, scene: &cf_scene::Scene) {
    let env = &scene.environment;
    cam.insert((
        if scene.exact_colors() { Tonemapping::None } else { Tonemapping::AgX },
        // Für Glas/Wasser (Material transmission); in Bevy 0.20 muss das pro Kamera aktiviert werden.
        bevy::pbr::ScreenSpaceTransmission::default(),
        scene_projection(&scene.camera),
    ));
    // Im Cartoon-Stil gibt es keine Tonwertkurve: beleuchtete Flächen eine Stufe dunkler
    // belichten, damit sie nicht ausbrennen (unbeleuchtete 2D-Grafiken bleiben exakt).
    let cartoon_shift = if scene.exact_colors() { 1.0 } else { 0.0 };
    if matches!(env.sky, Sky::Atmosphere) {
        cam.insert((
            AtmosphereSettings::default(),
            AtmosphereEnvironmentMapLight { intensity: env.ambient.max(0.0), ..default() },
            Exposure { ev100: 13.0 - env.exposure + cartoon_shift },
        ));
    } else {
        cam.remove::<(AtmosphereSettings, AtmosphereEnvironmentMapLight)>()
            .insert(Exposure { ev100: Exposure::EV100_SUNLIGHT - env.exposure + cartoon_shift });
    }
    // Sicherer Modus (nach Absturz): keine aufwendigen Effekte
    let safe = crate::crash::safe_mode();
    // Kontaktschatten (SSAO) brauchen MSAA aus; dann glättet SMAA die Kanten.
    if scene.ao_enabled() && !safe {
        cam.insert((Msaa::Off, ScreenSpaceAmbientOcclusion::default(), Smaa::default()));
    } else {
        cam.remove::<(ScreenSpaceAmbientOcclusion, Smaa)>().insert(Msaa::Sample4);
    }
    let camspec = &scene.camera;
    match camspec.dof.as_ref().filter(|_| !safe) {
        Some(d) => {
            let distance = match &d.focus {
                cf_scene::Focus::Distance(x) => *x,
                cf_scene::Focus::Object(_) => (Vec3::from_array(camspec.look_at) - Vec3::from_array(camspec.position)).length(),
            };
            cam.insert(DepthOfField {
                mode: DepthOfFieldMode::Gaussian,
                focal_distance: distance.max(0.05),
                aperture_f_stops: d.aperture,
                ..default()
            });
            match &d.focus {
                cf_scene::Focus::Object(id) => {
                    cam.insert(FocusOn(id.clone()));
                }
                _ => {
                    cam.remove::<FocusOn>();
                }
            }
        }
        None => {
            cam.remove::<(DepthOfField, FocusOn)>();
        }
    }
    match camspec.motion_blur.filter(|_| !safe) {
        Some(s) if s > 0.0 => {
            cam.insert(MotionBlur { shutter_angle: s, ..default() });
        }
        _ => {
            cam.remove::<MotionBlur>();
        }
    }
    if env.light_rays > 0.0 && !safe {
        cam.insert(VolumetricFog { ambient_intensity: 0.0, ..default() });
    } else {
        cam.remove::<VolumetricFog>();
    }
    match &env.grade {
        Some(g) => {
            // Nur die globale Sättigung: Bevys Kontrast/Weißabgleich verfälschten im Test die Farben.
            cam.insert(ColorGrading {
                global: ColorGradingGlobal { post_saturation: g.saturation, ..default() },
                ..default()
            });
            if g.vignette > 0.0 {
                cam.insert(Vignette { intensity: g.vignette, ..default() });
            } else {
                cam.remove::<Vignette>();
            }
        }
        None => {
            cam.remove::<(ColorGrading, Vignette)>().insert(ColorGrading::default());
        }
    }
    match &env.fog {
        Some(f) => {
            let c = color(f.color);
            cam.insert(DistanceFog {
                color: c,
                directional_light_color: color(env.sun.color).with_alpha(0.4),
                directional_light_exponent: 20.0,
                falloff: FogFalloff::from_visibility_colors(f.visibility.max(1.0), c.mix(&Color::BLACK, 0.25), c),
            });
        }
        None => {
            cam.remove::<DistanceFog>();
        }
    }
    if env.bloom > 0.0 {
        cam.insert(Bloom { intensity: env.bloom, ..Bloom::NATURAL });
    } else {
        cam.remove::<Bloom>();
    }
}

pub fn scene_camera_transform(c: &cf_scene::CameraSpec) -> Transform {
    let pos = Vec3::from_array(c.position);
    let target = Vec3::from_array(c.look_at);
    if (target - pos).length_squared() < 1e-6 {
        return Transform::from_translation(pos);
    }
    Transform::from_translation(pos).looking_at(target, Vec3::Y)
}

fn reset_camera_key(
    keys: Res<ButtonInput<KeyCode>>,
    src: Res<SceneSource>,
    ph: Option<Res<crate::timeline::Playhead>>,
    mut cam: Query<(&mut Transform, Option<&mut FreeCameraState>), With<MainCamera>>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        if let Ok((mut tf, state)) = cam.single_mut() {
            let t = ph.map(|p| p.time).unwrap_or(0.0);
            let (new, _) = crate::timeline::scene_camera_at(&src.scene, t);
            crate::timeline::set_view(&mut tf, new, state.map(|s| s.into_inner()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_direction_matches_compass() {
        assert!(sun_direction(0.0, 0.0).abs_diff_eq(Vec3::NEG_Z, 1e-5));
        assert!(sun_direction(90.0, 0.0).abs_diff_eq(Vec3::X, 1e-5));
        assert!(sun_direction(123.0, 90.0).abs_diff_eq(Vec3::Y, 1e-5));
    }
}
