//! Himmel, Sonne, Nebel, Belichtung und Kamera-Einstellungen aus dem Szenen-Dokument.

use bevy::{
    camera::Exposure,
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

fn apply_environment(
    mut commands: Commands,
    mut loaded: MessageReader<SceneLoaded>,
    src: Res<SceneSource>,
    medium: Res<EarthMedium>,
    old: Query<Entity, With<EnvEntity>>,
    mut camera: Query<(Entity, &mut Transform, &mut Projection), With<MainCamera>>,
) {
    let Some(new_file) = loaded.read().fold(None, |acc: Option<bool>, m| Some(acc.unwrap_or(false) | m.new_file))
    else {
        return;
    };
    let Ok((cam, mut cam_tf, mut projection)) = camera.single_mut() else { return };
    for e in &old {
        commands.entity(e).despawn();
    }

    let scene = &src.scene;
    let env: &Environment = &scene.environment;
    let atmosphere = matches!(env.sky, Sky::Atmosphere);

    // Kamera: Position nur beim ersten Laden setzen, damit man beim Live-Editieren
    // nicht ständig zurückgeworfen wird.
    if new_file {
        *cam_tf = scene_camera_transform(&scene.camera);
    }
    if let Projection::Perspective(p) = projection.as_mut() {
        p.fov = scene.camera.fov.clamp(5.0, 150.0).to_radians();
        p.far = 20_000.0;
        p.near = 0.1;
    }

    // Sonne
    let dir = sun_direction(env.sun.azimuth, env.sun.elevation);
    let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let base_lux = if atmosphere { lux::RAW_SUNLIGHT } else { lux::DIRECT_SUNLIGHT };
    let mut sun = commands.spawn((
        EnvEntity,
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
    }

    let mut cam_cmd = commands.entity(cam);
    cam_cmd.insert(Tonemapping::AgX);

    if atmosphere {
        commands.spawn((
            EnvEntity,
            Name::new("Atmosphäre"),
            Atmosphere::earth(medium.0.clone()),
        ));
        commands.insert_resource(GlobalAmbientLight::NONE);
        commands.entity(cam).insert((
            AtmosphereSettings::default(),
            AtmosphereEnvironmentMapLight { intensity: env.ambient.max(0.0), ..default() },
            Exposure { ev100: 13.0 - env.exposure },
        ));
    } else {
        let Sky::Color { color: sky } = &env.sky else { unreachable!() };
        commands.insert_resource(ClearColor(color(*sky)));
        commands.insert_resource(GlobalAmbientLight {
            color: color(*sky).mix(&Color::WHITE, 0.5),
            brightness: 2500.0 * env.ambient.max(0.0),
            ..default()
        });
        commands
            .entity(cam)
            .remove::<(AtmosphereSettings, AtmosphereEnvironmentMapLight)>()
            .insert(Exposure { ev100: Exposure::EV100_SUNLIGHT - env.exposure });
    }

    match &env.fog {
        Some(f) => {
            let c = color(f.color);
            commands.entity(cam).insert(DistanceFog {
                color: c,
                directional_light_color: color(env.sun.color).with_alpha(0.4),
                directional_light_exponent: 20.0,
                falloff: FogFalloff::from_visibility_colors(
                    f.visibility.max(1.0),
                    c.mix(&Color::BLACK, 0.25),
                    c,
                ),
            });
        }
        None => {
            commands.entity(cam).remove::<DistanceFog>();
        }
    }

    if env.bloom > 0.0 {
        commands.entity(cam).insert(Bloom { intensity: env.bloom, ..Bloom::NATURAL });
    } else {
        commands.entity(cam).remove::<Bloom>();
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
    mut cam: Query<&mut Transform, With<MainCamera>>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        if let Ok(mut tf) = cam.single_mut() {
            *tf = scene_camera_transform(&src.scene.camera);
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
