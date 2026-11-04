//! Baut aus den Objekten des Szenen-Dokuments Bevy-Entities.
//!
//! Jedes erzeugte Entity trägt [`SceneObjectId`] – darüber werden Objekte
//! später angeklickt, hervorgehoben und von Claude gezielt bearbeitet.

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use cf_scene::{
    procedural::{self, Ground, Heightfield, MeshData},
    Kind, Material, Object, Scene,
};

use crate::{
    environment::color,
    source::{SceneLoaded, SceneSource},
};

pub struct SpawnPlugin;

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, rebuild_scene);
    }
}

/// Wurzel aller Szenen-Objekte; wird beim Neuladen komplett ersetzt.
#[derive(Component)]
pub struct SceneRoot;

/// Stabile id des Objekts aus dem Szenen-Dokument.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SceneObjectId(pub String);

/// Statistik für die Anzeige.
#[derive(Resource, Default, Debug)]
pub struct SceneStats {
    pub objects: usize,
    pub instances: usize,
    pub triangles: usize,
    pub build_ms: f32,
}

fn rebuild_scene(
    mut commands: Commands,
    mut loaded: MessageReader<SceneLoaded>,
    src: Res<SceneSource>,
    roots: Query<Entity, With<SceneRoot>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if loaded.read().count() == 0 {
        return;
    }
    let started = std::time::Instant::now();
    for r in &roots {
        commands.entity(r).despawn();
    }
    let root = commands
        .spawn((SceneRoot, Name::new("Szene"), Transform::default(), Visibility::default()))
        .id();
    let mut b = Builder {
        scene: &src.scene,
        commands: &mut commands,
        meshes: &mut meshes,
        materials: &mut materials,
        material_cache: HashMap::new(),
        stats: SceneStats::default(),
    };
    for o in &src.scene.objects {
        b.spawn_object(o, root);
    }
    let mut stats = b.stats;
    stats.build_ms = started.elapsed().as_secs_f32() * 1000.0;
    info!(
        "Szene aufgebaut: {} Objekte, {} Instanzen, {} Dreiecke in {:.0} ms",
        stats.objects, stats.instances, stats.triangles, stats.build_ms
    );
    commands.insert_resource(stats);
}

struct Builder<'a, 'w, 's> {
    scene: &'a Scene,
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    /// Gleiche Materialien teilen sich ein GPU-Material ⇒ weniger Draw-Calls.
    material_cache: HashMap<String, Handle<StandardMaterial>>,
    stats: SceneStats,
}

pub fn object_transform(o: &Object) -> Transform {
    let [rx, ry, rz] = o.rotation.map(f32::to_radians);
    Transform {
        translation: Vec3::from_array(o.position),
        rotation: Quat::from_euler(EulerRot::YXZ, ry, rx, rz),
        scale: Vec3::from_array(o.scale.to_array()),
    }
}

impl Builder<'_, '_, '_> {
    fn spawn_object(&mut self, o: &Object, parent: Entity) {
        self.stats.objects += 1;
        let visibility = if o.hidden { Visibility::Hidden } else { Visibility::Inherited };
        let mut transform = object_transform(o);
        // Scatter auf Gelände: Rotation/Skalierung würden die Bodenhaftung zerstören.
        if let Kind::Scatter(s) = &o.kind {
            if s.on_terrain.is_some() {
                transform.rotation = Quat::IDENTITY;
                transform.scale = Vec3::ONE;
            }
        }
        let entity = self
            .commands
            .spawn((
                SceneObjectId(o.id.clone()),
                Name::new(o.display_name().to_string()),
                transform,
                visibility,
                ChildOf(parent),
            ))
            .id();

        match &o.kind {
            Kind::Group { children } => {
                for c in children {
                    self.spawn_object(c, entity);
                }
            }
            Kind::PointLight(l) => {
                self.commands.entity(entity).insert(PointLight {
                    color: color(l.color),
                    intensity: l.intensity.max(0.0),
                    range: l.range.max(0.1),
                    shadow_maps_enabled: l.shadows,
                    ..default()
                });
            }
            Kind::Scatter(spec) => {
                let ground_data = spec
                    .on_terrain
                    .as_deref()
                    .and_then(|id| procedural::find_terrain(&self.scene.objects, id));
                let hf = ground_data.map(|(t, _, _)| Heightfield::new(t));
                let ground = ground_data.zip(hf.as_ref()).map(|((_, offset, scale), hf)| Ground {
                    heightfield: hf,
                    offset,
                    scale,
                });
                let instances = procedural::scatter_instances(spec, o.position, ground.as_ref());
                if let Some((mesh, tris)) = self.shape_mesh(&spec.shape.kind, o.material.flat) {
                    let mat = self.material(&o.material);
                    for inst in &instances {
                        self.commands.spawn((
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(mat.clone()),
                            Transform {
                                translation: Vec3::from_array(inst.position),
                                rotation: Quat::from_rotation_y(inst.yaw),
                                scale: Vec3::splat(inst.scale),
                            },
                            ChildOf(entity),
                        ));
                    }
                    self.stats.instances += instances.len();
                    self.stats.triangles += tris * instances.len();
                }
            }
            kind => {
                if let Some((mesh, tris)) = self.shape_mesh(kind, o.material.flat) {
                    let mat = self.material(&o.material);
                    self.commands.entity(entity).insert((Mesh3d(mesh), MeshMaterial3d(mat)));
                    self.stats.triangles += tris;
                }
            }
        }
    }

    fn shape_mesh(&mut self, kind: &Kind, flat: bool) -> Option<(Handle<Mesh>, usize)> {
        let mut mesh: Mesh = match kind {
            Kind::Box { size } => Cuboid::new(size[0], size[1], size[2]).into(),
            Kind::Sphere { radius } => {
                if flat {
                    // Wenige Facetten ⇒ Low-Poly-Look (gut für Steine).
                    Sphere::new(*radius).mesh().ico(1).ok()?
                } else {
                    Sphere::new(*radius).mesh().uv(48, 32)
                }
            }
            Kind::Cylinder { radius, height } => Cylinder::new(*radius, *height).mesh().resolution(48).into(),
            Kind::Cone { radius, height } => Cone::new(*radius, *height).mesh().resolution(48).into(),
            Kind::Capsule { radius, length } => Capsule3d::new(*radius, *length).into(),
            Kind::Torus { radius, thickness } => Torus::new(radius - thickness, radius + thickness).into(),
            Kind::Plane { size } => Plane3d::default().mesh().size(size[0], size[1]).into(),
            Kind::Pyramid { size } => to_bevy_mesh(procedural::pyramid_mesh(*size)),
            Kind::Terrain(t) => to_bevy_mesh(procedural::terrain_mesh(t)),
            Kind::Group { .. } | Kind::Scatter(_) | Kind::PointLight(_) => return None,
        };
        if flat && !matches!(kind, Kind::Pyramid { .. } | Kind::Terrain(_)) {
            mesh.duplicate_vertices();
            mesh.compute_flat_normals();
        }
        let tris = mesh.indices().map(|i| i.len() / 3).unwrap_or_else(|| mesh.count_vertices() / 3);
        Some((self.meshes.add(mesh), tris))
    }

    fn material(&mut self, m: &Material) -> Handle<StandardMaterial> {
        let key = format!("{m:?}");
        if let Some(h) = self.material_cache.get(&key) {
            return h.clone();
        }
        let emissive = if m.emissive.is_black() {
            LinearRgba::BLACK
        } else {
            color(m.emissive).to_linear() * m.emissive_strength.max(0.0)
        };
        let handle = self.materials.add(StandardMaterial {
            base_color: Color::srgba(m.color.r, m.color.g, m.color.b, m.opacity),
            perceptual_roughness: m.roughness.clamp(0.02, 1.0),
            metallic: m.metallic.clamp(0.0, 1.0),
            emissive,
            alpha_mode: if m.opacity < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
            ..default()
        });
        self.material_cache.insert(key, handle.clone());
        handle
    }
}

fn to_bevy_mesh(d: MeshData) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, d.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, d.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, d.uvs)
        .with_inserted_indices(Indices::U32(d.indices))
}
