//! Darstellung der Partikel-Effekte.
//!
//! Jeder Emitter hat einen festen Vorrat (Pool) an Flächen, die immer zur
//! Kamera zeigen. In jedem Frame wird der Zustand aus der Zeit berechnet
//! (`cf_scene::particles`). Farbe/Deckkraft über die Lebenszeit kommen aus
//! 8 vorbereiteten Materialien je Emitter (Alters-Stufen).

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, Face, TextureDimension, TextureFormat},
};
use cf_scene::particles::{Blend, Resolved};

use crate::{render::RenderQueue, timeline::Playhead, MainCamera};

const AGE_STEPS: usize = 8;

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_assets)
            .add_systems(Update, update_emitters.after(crate::timeline::TimelineSet));
    }
}

#[derive(Resource, Clone)]
pub struct FxAssets {
    quad: Handle<Mesh>,
    dot: Handle<Image>,
}

fn init_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut images: ResMut<Assets<Image>>) {
    // Weicher runder Fleck (Alpha fällt zum Rand ab)
    let size = 64u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let d = (dx * dx + dy * dy).sqrt();
            let a = (1.0 - d).clamp(0.0, 1.0).powf(1.6);
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    let dot = images.add(Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ));
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    commands.insert_resource(FxAssets { quad, dot });
}

/// Ein Partikel-Emitter (am Objekt-Entity).
#[derive(Component)]
pub struct Emitter {
    pub resolved: Resolved,
    pub slots: Vec<Entity>,
    pub materials: Vec<Handle<StandardMaterial>>,
}

/// Legt Pool und Materialien für einen Emitter an.
pub fn spawn_emitter(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    assets: &FxAssets,
    entity: Entity,
    resolved: Resolved,
) -> usize {
    let mats: Vec<Handle<StandardMaterial>> = (0..AGE_STEPS)
        .map(|k| {
            let age = (k as f32 + 0.5) / AGE_STEPS as f32;
            let (c, alpha) = resolved.look_at_age(age);
            let additive = resolved.blend == Blend::Add;
            materials.add(StandardMaterial {
                base_color: Color::srgba(c.r, c.g, c.b, alpha),
                base_color_texture: Some(assets.dot.clone()),
                emissive: if resolved.glow > 0.0 {
                    Color::srgb(c.r, c.g, c.b).to_linear() * resolved.glow * alpha
                } else {
                    LinearRgba::BLACK
                },
                // Unbeleuchtet: Partikel zeigen ihre eigene Farbe (auch nachts gut sichtbar).
                unlit: true,
                alpha_mode: if additive { AlphaMode::Add } else { AlphaMode::Blend },
                double_sided: true,
                cull_mode: None::<Face>,
                ..default()
            })
        })
        .collect();
    let n = resolved.pool_size() as usize;
    let slots: Vec<Entity> = (0..n)
        .map(|_| {
            commands
                .spawn((
                    Mesh3d(assets.quad.clone()),
                    MeshMaterial3d(mats[0].clone()),
                    Transform::from_scale(Vec3::ZERO),
                    Visibility::Hidden,
                    bevy::light::NotShadowCaster,
                    ChildOf(entity),
                ))
                .id()
        })
        .collect();
    commands.entity(entity).insert(Emitter { resolved, slots, materials: mats });
    n
}

#[allow(clippy::type_complexity)]
fn update_emitters(
    ph: Res<Playhead>,
    jobs: Res<RenderQueue>,
    emitters: Query<(&Emitter, &GlobalTransform)>,
    main_cam: Query<&Transform, (With<MainCamera>, With<Camera3d>)>,
    cams: Query<&Transform, (With<Camera3d>, Without<MainCamera>)>,
    mut slots: Query<(&mut Transform, &mut Visibility, &mut MeshMaterial3d<StandardMaterial>), (Without<Emitter>, Without<Camera3d>)>,
) {
    // Billboards zeigen zur Kamera, die gerade gerendert wird (Aufnahme oder Fenster).
    let cam_world = jobs
        .active_camera()
        .and_then(|e| cams.get(e).ok())
        .or_else(|| main_cam.single().ok())
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    for (em, gt) in &emitters {
        let to_local = gt.affine().inverse();
        let cam = to_local.transform_point3(cam_world);
        let parts = em.resolved.particles_at(ph.time);
        for (j, slot) in em.slots.iter().enumerate() {
            let Ok((mut tf, mut vis, mut mat)) = slots.get_mut(*slot) else { continue };
            match parts.get(j) {
                Some(p) => {
                    let pos = Vec3::from_array(p.position);
                    let to_cam = (cam - pos).normalize_or(Vec3::Z);
                    let vel = Vec3::from_array(p.velocity);
                    let (up, stretch) = if em.resolved.stretch > 0.0 && vel.length_squared() > 1e-6 {
                        (vel.normalize(), 1.0 + vel.length() * em.resolved.stretch / p.size.max(1e-4))
                    } else {
                        // Leichte Drehung je Partikel, damit es lebendiger wirkt.
                        let a = p.index as f32 * 2.399;
                        (Vec3::new(a.cos(), a.sin(), 0.0), 1.0)
                    };
                    let mut new = Transform::from_translation(pos);
                    new.look_to(-to_cam, up);
                    new.scale = Vec3::new(p.size, p.size * stretch, 1.0);
                    *tf = new;
                    if *vis != Visibility::Inherited {
                        *vis = Visibility::Inherited;
                    }
                    let k = ((p.age * AGE_STEPS as f32) as usize).min(AGE_STEPS - 1);
                    if mat.0 != em.materials[k] {
                        mat.0 = em.materials[k].clone();
                    }
                }
                None => {
                    if *vis != Visibility::Hidden {
                        *vis = Visibility::Hidden;
                    }
                }
            }
        }
    }
}
