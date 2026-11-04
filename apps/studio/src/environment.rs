//! Himmel, Sonne, Nebel, Belichtung und Kamera-Einstellungen aus dem Szenen-Dokument.

use bevy::{
    camera::Exposure,
    camera_controller::free_camera::FreeCameraState,
    core_pipeline::tonemapping::Tonemapping,
    light::{
        atmosphere::ScatteringMedium, light_consts::lux, Atmosphere, AtmosphereEnvironmentMapLight,
        CascadeShadowConfigBuilder, SunDisk,
    },
    pbr::{AtmosphereSettings, DistanceFog, FogFalloff},
    post_process::bloom::Bloom,
    prelude::*,
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
            .add_systems(Update, (apply_environment, reset_camera_key));
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
) {
    let Some(new_file) = loaded.read().fold(None, |acc: Option<bool>, m| Some(acc.unwrap_or(false) | m.new_file))
    else {
        return;
    };
    for e in &old {
        commands.entity(e).despawn();
    }
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
    if atmosphere {
        sun.insert(SunDisk::EARTH);
        commands.spawn((EnvEntity, Name::new("Atmosphäre"), Atmosphere::earth(medium.0.clone())));
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

    for view in &views {
        configure_view(&mut commands.entity(view), scene);
    }
}

/// Setzt alle kamerabezogenen Einstellungen (Belichtung, Himmel, Nebel, Bloom, Bildwinkel).
pub fn configure_view(cam: &mut EntityCommands, scene: &cf_scene::Scene) {
    let env = &scene.environment;
    cam.insert((
        Tonemapping::AgX,
        // Für Glas/Wasser (Material transmission); in Bevy 0.20 muss das pro Kamera aktiviert werden.
        bevy::pbr::ScreenSpaceTransmission::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: scene.camera.fov.clamp(5.0, 150.0).to_radians(),
            near: 0.1,
            far: 20_000.0,
            ..default()
        }),
    ));
    if matches!(env.sky, Sky::Atmosphere) {
        cam.insert((
            AtmosphereSettings::default(),
            AtmosphereEnvironmentMapLight { intensity: env.ambient.max(0.0), ..default() },
            Exposure { ev100: 13.0 - env.exposure },
        ));
    } else {
        cam.remove::<(AtmosphereSettings, AtmosphereEnvironmentMapLight)>()
            .insert(Exposure { ev100: Exposure::EV100_SUNLIGHT - env.exposure });
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
