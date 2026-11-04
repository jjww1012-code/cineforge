//! Baut aus den Objekten des Szenen-Dokuments Bevy-Entities.
//!
//! Jedes erzeugte Entity trägt [`SceneObjectId`] – darüber werden Objekte
//! später angeklickt, hervorgehoben und von Claude gezielt bearbeitet.

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    math::Affine2,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{Extent3d, Face, TextureDimension, TextureFormat},
};
use cf_scene::{
    modeling,
    procedural::{self, Ground, Heightfield, MeshData},
    textures, Kind, Material, Object, Pattern, Scene,
};

use crate::{
    environment::color,
    library::Library,
    project::ActiveProject,
    source::{SceneLoaded, SceneSource},
};

pub const FONT: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
pub const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");
pub const FONT_BLACK: &[u8] = include_bytes!("../assets/fonts/Inter-Black.ttf");

/// 3D-Modell, das noch lädt (Aufnahmen warten darauf).
#[derive(Component)]
pub struct ModelLoading(pub Handle<bevy::world_serialization::WorldAsset>);

/// Röhre, deren gezeichneter Anteil animiert wird.
#[derive(Component)]
pub struct DrawTube {
    pub spec: cf_scene::TubeSpec,
    pub mesh: Handle<Mesh>,
    pub last: f32,
}

pub struct SpawnPlugin;

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Library>()
            .init_resource::<SceneStats>()
            .init_resource::<BuiltScene>()
            .init_resource::<crate::media::ImageCache>()
            .add_systems(Update, (rebuild_scene, track_model_loading));
    }
}

/// Wurzel aller Szenen-Objekte; wird beim Neuladen komplett ersetzt.
#[derive(Component)]
pub struct SceneRoot;

/// Stabile id des Objekts aus dem Szenen-Dokument.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SceneObjectId(pub String);

/// Animiertes Objekt: Grundwerte + Animation; die Zeitleiste berechnet daraus
/// in jedem Frame Transform, Sichtbarkeit und ggf. Material.
#[derive(Component)]
pub struct Animated {
    pub anim: cf_scene::animation::Animation,
    pub base_pos: [f32; 3],
    pub base_rot: [f32; 3],
    pub base_scale: [f32; 3],
    pub hidden: bool,
    /// Versatz durch `repeat` (Kopie i).
    pub repeat_op: Transform,
    /// Eigenes Material, falls Farbe/Leuchten/Deckkraft animiert werden.
    pub material: Option<Handle<StandardMaterial>>,
    pub base_color: cf_scene::Rgb,
    pub base_emissive: cf_scene::Rgb,
    pub base_emissive_strength: f32,
    pub base_opacity: f32,
    pub has_pattern: bool,
    /// Grund-Lichtstärke (nur point_light).
    pub base_intensity: f32,
    /// Einblendung: Faktor, um den die Farbe gegen die Tonwertkurve angehoben wird.
    pub overlay: Option<f32>,
}

/// Statistik und Warnungen für die Anzeige.
#[derive(Resource, Default, Debug)]
pub struct SceneStats {
    pub objects: usize,
    pub instances: usize,
    pub triangles: usize,
    pub build_ms: f32,
    /// Wie viele oberste Objekte beim letzten Laden neu gebaut wurden.
    pub rebuilt: usize,
    pub warnings: Vec<String>,
}

/// Was beim letzten Aufbau entstanden ist: je oberstem Objekt ein Halter-Entity mit
/// Fingerabdruck. Beim nächsten Laden werden nur geänderte Objekte neu gebaut –
/// so bleibt beim Live-Bearbeiten alles andere unangetastet (kein Flackern, kein Nachladen).
#[derive(Resource, Default)]
pub struct BuiltScene {
    root: Option<Entity>,
    objects: HashMap<String, BuiltObject>,
    horizon: Option<(u64, Entity)>,
}

struct BuiltObject {
    hash: u64,
    slot: Entity,
    stats: SceneStats,
}

fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn file_stamp(path: &std::path::Path) -> String {
    let m = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    format!("{}@{m:?}", path.display())
}

/// Fingerabdruck eines obersten Objekts inkl. allem, wovon sein Aussehen abhängt:
/// Gelände unter einem Scatter, Bibliotheks-Dateien, Bilder, SVG-Dateien, 3D-Modelle.
fn object_fingerprint(o: &Object, scene: &Scene, project: &ActiveProject, library: &mut Library) -> u64 {
    let mut key = serde_json::to_string(o).unwrap_or_default();
    fn deps(o: &Object, scene: &Scene, project: &ActiveProject, library: &mut Library, depth: u32, key: &mut String) {
        if depth > MAX_INSTANCE_DEPTH {
            return;
        }
        let file = |folder: &str, f: &str, key: &mut String| {
            if let Ok(p) = crate::media::resolve(project, folder, f) {
                key.push_str(&file_stamp(&p));
            }
        };
        if let Some(t) = &o.material.texture {
            file(crate::media::IMAGES, t, key);
        }
        match &o.kind {
            Kind::Group { children } => {
                for c in children {
                    deps(c, scene, project, library, depth, key);
                }
            }
            Kind::Scatter(s) => {
                if let Some(id) = &s.on_terrain {
                    if let Some(t) = procedural::find_terrain(&scene.objects, id) {
                        key.push_str(&serde_json::to_string(&t).unwrap_or_default());
                    }
                }
            }
            Kind::Instance { asset, .. } => {
                if let Some(p) = Library::find_path(project, asset) {
                    key.push_str(&file_stamp(&p));
                }
                if let Ok(a) = library.load(project, asset) {
                    deps(&a.object, scene, project, library, depth + 1, key);
                }
            }
            Kind::Image { file: f, .. } => file(crate::media::IMAGES, f, key),
            Kind::Svg { file: Some(f), .. } => file(crate::media::IMAGES, f, key),
            Kind::Model { file: f } => file(crate::media::MODELS, f, key),
            _ => {}
        }
    }
    deps(o, scene, project, library, 0, &mut key);
    hash_str(&key)
}

#[allow(clippy::too_many_arguments)]
fn rebuild_scene(
    mut commands: Commands,
    fx: Option<Res<crate::fx::FxAssets>>,
    mut loaded: MessageReader<SceneLoaded>,
    src: Res<SceneSource>,
    project: Res<ActiveProject>,
    mut library: ResMut<Library>,
    mut built: ResMut<BuiltScene>,
    roots: Query<Entity, With<SceneRoot>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut image_cache: ResMut<crate::media::ImageCache>,
    asset_server: Res<AssetServer>,
) {
    if loaded.read().count() == 0 {
        return;
    }
    let started = std::time::Instant::now();
    let root = match built.root.filter(|r| roots.contains(*r)) {
        Some(r) => r,
        None => {
            for r in &roots {
                commands.entity(r).despawn();
            }
            *built = BuiltScene::default();
            let r = commands.spawn((SceneRoot, Name::new("Szene"), Transform::default(), Visibility::default())).id();
            built.root = Some(r);
            r
        }
    };
    let scene = &src.scene;
    let mut old = std::mem::take(&mut built.objects);
    let mut next: HashMap<String, BuiltObject> = HashMap::new();
    let mut b = Builder {
        fx: fx.as_deref().cloned(),
        scene,
        project: &project,
        library: &mut library,
        commands: &mut commands,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        image_cache: &mut image_cache,
        asset_server: asset_server.clone(),
        material_cache: HashMap::new(),
        texture_cache: HashMap::new(),
        stats: SceneStats::default(),
    };
    let mut rebuilt = 0;
    for (i, o) in scene.objects.iter().enumerate() {
        let mut key = o.id.clone();
        if next.contains_key(&key) {
            key = format!("{}#{i}", o.id);
        }
        let hash = object_fingerprint(o, scene, b.project, b.library);
        match old.remove(&key) {
            Some(prev) if prev.hash == hash => {
                next.insert(key, prev);
            }
            prev => {
                if let Some(prev) = prev {
                    b.commands.entity(prev.slot).despawn();
                }
                let slot = b
                    .commands
                    .spawn((ObjectSlot, Transform::default(), Visibility::default(), ChildOf(root)))
                    .id();
                b.stats = SceneStats::default();
                b.spawn_object(o, slot, None, 0, "");
                let stats = std::mem::take(&mut b.stats);
                next.insert(key, BuiltObject { hash, slot, stats });
                rebuilt += 1;
            }
        }
    }
    let removed = old.len();
    for (_, gone) in old {
        b.commands.entity(gone.slot).despawn();
    }
    // Horizont-Boden nur neu, wenn sich das größte Gelände geändert hat.
    let horizon_key = hash_str(&format!("{:?}", horizon_source(scene)));
    if built.horizon.map(|(h, _)| h) != Some(horizon_key) {
        if let Some((_, e)) = built.horizon.take() {
            b.commands.entity(e).despawn();
        }
        let slot = b.commands.spawn((ObjectSlot, Transform::default(), Visibility::default(), ChildOf(root))).id();
        b.spawn_horizon_ground(slot);
        built.horizon = Some((horizon_key, slot));
    }
    let mut stats = SceneStats::default();
    for o in next.values() {
        stats.objects += o.stats.objects;
        stats.instances += o.stats.instances;
        stats.triangles += o.stats.triangles;
        stats.warnings.extend(o.stats.warnings.iter().cloned());
    }
    stats.rebuilt = rebuilt;
    stats.build_ms = started.elapsed().as_secs_f32() * 1000.0;
    built.objects = next;
    info!(
        "Szene aktualisiert: {rebuilt} von {} Objekten neu gebaut, {removed} entfernt, {:.0} ms",
        scene.objects.len(),
        stats.build_ms
    );
    for w in &stats.warnings {
        warn!("{w}");
    }
    commands.insert_resource(stats);
}

/// Halter für ein oberstes Objekt (inkl. Wiederholungen); wird als Ganzes ersetzt.
#[derive(Component)]
pub struct ObjectSlot;

/// Woraus der Horizont-Boden entsteht (größtes Gelände: Position + Material).
fn horizon_source(scene: &Scene) -> Option<([f32; 3], Material)> {
    let mut found: Option<(f32, [f32; 3], Material)> = None;
    for o in &scene.objects {
        if let Kind::Terrain(t) = &o.kind {
            let area = t.size[0] * t.size[1];
            if found.as_ref().is_none_or(|(a, _, _)| area > *a) {
                found = Some((area, o.position, o.material.clone()));
            }
        }
    }
    found.map(|(_, p, m)| (p, m))
}

struct Builder<'a, 'w, 's> {
    fx: Option<crate::fx::FxAssets>,
    scene: &'a Scene,
    project: &'a ActiveProject,
    library: &'a mut Library,
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    images: &'a mut Assets<Image>,
    image_cache: &'a mut crate::media::ImageCache,
    asset_server: AssetServer,
    /// Gleiche Materialien teilen sich ein GPU-Material ⇒ weniger Draw-Calls.
    material_cache: HashMap<String, Handle<StandardMaterial>>,
    texture_cache: HashMap<String, (Handle<Image>, Option<Handle<Image>>)>,
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

/// Schritt einer Wiederholung: Drehung/Skalierung um den Drehpunkt, dann Verschiebung.
pub fn repeat_step(r: &cf_scene::Repeat, base: &Transform) -> Transform {
    let [rx, ry, rz] = r.rotation.map(f32::to_radians);
    let center = base.translation + Vec3::from_array(r.pivot);
    let rot_scale = Transform {
        translation: Vec3::ZERO,
        rotation: Quat::from_euler(EulerRot::YXZ, ry, rx, rz),
        scale: Vec3::splat(r.scale),
    };
    Transform::from_translation(Vec3::from_array(r.offset) + center)
        .mul_transform(rot_scale)
        .mul_transform(Transform::from_translation(-center))
}

const MAX_INSTANCE_DEPTH: u32 = 8;

impl Builder<'_, '_, '_> {
    /// Erzeugt ein Objekt (inkl. Wiederholungen) unter `parent`.
    fn spawn_object(&mut self, o: &Object, parent: Entity, mat_override: Option<&Material>, depth: u32, prefix: &str) {
        let base = object_transform(o);
        match &o.repeat {
            Some(r) => {
                let step = repeat_step(r, &base);
                let mut op = Transform::IDENTITY;
                for _ in 0..r.count {
                    self.spawn_single(o, parent, op, mat_override, depth, prefix);
                    op = step.mul_transform(op);
                }
            }
            None => self.spawn_single(o, parent, Transform::IDENTITY, mat_override, depth, prefix),
        }
    }

    /// `op` = Versatz durch Wiederholung (Kopie i), wird vor die Objekt-Transformation gesetzt.
    fn spawn_single(
        &mut self,
        o: &Object,
        parent: Entity,
        op: Transform,
        mat_override: Option<&Material>,
        depth: u32,
        prefix: &str,
    ) {
        self.stats.objects += 1;
        let mut transform = op.mul_transform(object_transform(o));
        let visibility = if o.hidden { Visibility::Hidden } else { Visibility::Inherited };
        // Scatter auf Gelände: Rotation/Skalierung würden die Bodenhaftung zerstören.
        if let Kind::Scatter(s) = &o.kind {
            if s.on_terrain.is_some() {
                transform.rotation = Quat::IDENTITY;
                transform.scale = Vec3::ONE;
            }
        }
        let full_id = format!("{prefix}{}", o.id);
        let entity = self
            .commands
            .spawn((
                SceneObjectId(full_id.clone()),
                Name::new(o.display_name().to_string()),
                transform,
                visibility,
                ChildOf(parent),
            ))
            .id();
        let material = mat_override.unwrap_or(&o.material);
        let animated_material = o.animate.as_ref().is_some_and(|a| a.affects_material());
        if let Some(anim) = &o.animate {
            self.commands.entity(entity).insert(Animated {
                anim: anim.clone(),
                base_pos: o.position,
                base_rot: o.rotation,
                base_scale: o.scale.to_array(),
                hidden: o.hidden,
                repeat_op: op,
                material: None,
                base_color: material.color,
                base_emissive: if material.emissive.is_black() && anim.emissive_strength.is_empty() {
                    material.emissive
                } else if material.emissive.is_black() {
                    material.color
                } else {
                    material.emissive
                },
                base_emissive_strength: material.emissive_strength,
                base_opacity: material.opacity,
                has_pattern: material.pattern.is_some(),
                base_intensity: match &o.kind {
                    Kind::PointLight(l) => l.intensity,
                    _ => 0.0,
                },
                overlay: None,
            });
        }

        match &o.kind {
            Kind::Group { children } => {
                for c in children {
                    self.spawn_object(c, entity, mat_override, depth, prefix);
                }
            }
            Kind::Instance { asset, material_override } => {
                if depth >= MAX_INSTANCE_DEPTH {
                    self.stats.warnings.push(format!("\"{full_id}\": Bibliotheks-Objekte zu tief verschachtelt"));
                    return;
                }
                match self.library.load(self.project, asset) {
                    Ok(a) => {
                        let ov = material_override.as_ref().or(mat_override);
                        let child_prefix = format!("{full_id}/");
                        self.spawn_object(&a.object, entity, ov, depth + 1, &child_prefix);
                    }
                    Err(e) => {
                        self.stats.warnings.push(format!("\"{full_id}\": {e}"));
                        // Platzhalter, damit man sieht, wo etwas fehlt.
                        let mesh = self.meshes.add(Cuboid::new(1.0, 1.0, 1.0));
                        let mat = self.materials.add(StandardMaterial {
                            base_color: Color::srgb(1.0, 0.0, 1.0),
                            unlit: true,
                            ..default()
                        });
                        self.commands.entity(entity).insert((Mesh3d(mesh), MeshMaterial3d(mat)));
                    }
                }
            }
            Kind::Svg { svg, file, width, depth } => {
                let text = match (svg, file) {
                    (Some(s), _) => Ok(s.clone()),
                    (None, Some(f)) => crate::media::resolve(self.project, crate::media::IMAGES, f)
                        .and_then(|p| std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))),
                    _ => Err("svg oder file fehlt".to_string()),
                };
                match text.and_then(|t| cf_scene::svg::svg_shapes(&t, *width, *depth)) {
                    Ok(shapes) => {
                        for sh in shapes {
                            let mut m = material.clone();
                            m.color = sh.color;
                            m.opacity = (material.opacity * sh.opacity).clamp(0.0, 1.0);
                            m.unlit = material.unlit || *depth == 0.0;
                            m.double_sided = true;
                            m.pattern = None;
                            let tris = sh.mesh.indices.len() / 3;
                            let mesh = self.meshes.add(to_bevy_mesh(sh.mesh));
                            let mat = self.material(&m);
                            self.commands.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Transform::default(), ChildOf(entity)));
                            self.stats.triangles += tris;
                        }
                    }
                    Err(e) => self.stats.warnings.push(format!("\"{full_id}\": {e}")),
                }
            }
            Kind::Model { file } => {
                match crate::media::resolve(self.project, crate::media::MODELS, file) {
                    Ok(path) => {
                        let rel = crate::project::root_dir()
                            .ok()
                            .and_then(|root| path.strip_prefix(&root).ok().map(|p| p.to_path_buf()))
                            .map(|p| p.to_string_lossy().replace('\\', "/"));
                        match rel {
                            Some(rel) => {
                                let handle: Handle<bevy::world_serialization::WorldAsset> = self
                                    .asset_server
                                    .load(GltfAssetLabel::Scene(0).from_asset(format!("cineforge://{rel}")));
                                self.commands.spawn((
                                    bevy::world_serialization::WorldAssetRoot(handle.clone()),
                                    ModelLoading(handle),
                                    Transform::default(),
                                    ChildOf(entity),
                                ));
                            }
                            None => self.stats.warnings.push(format!("\"{full_id}\": Modell muss im CineForge-Ordner liegen")),
                        }
                    }
                    Err(e) => self.stats.warnings.push(format!("\"{full_id}\": {e}")),
                }
            }
            Kind::Image { file, width } => {
                match crate::media::resolve(self.project, crate::media::IMAGES, file)
                    .and_then(|p| self.image_cache.load(&p, self.images))
                {
                    Ok(img) => {
                        let h = width * img.height as f32 / img.width.max(1) as f32;
                        let mesh = self.meshes.add(Rectangle::new(*width, h));
                        let mut m = material.clone();
                        m.texture = Some(file.clone());
                        let mat = self.material(&m);
                        self.commands.entity(entity).insert((Mesh3d(mesh), MeshMaterial3d(mat)));
                        self.stats.triangles += 2;
                    }
                    Err(e) => self.stats.warnings.push(format!("\"{full_id}\": {e}")),
                }
            }
            Kind::Particles(spec) => {
                if let Some(fx) = self.fx.clone() {
                    let n = crate::fx::spawn_emitter(self.commands, self.materials, &fx, entity, spec.resolve());
                    self.stats.instances += n;
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
                let ground_data =
                    spec.on_terrain.as_deref().and_then(|id| procedural::find_terrain(&self.scene.objects, id));
                let hf = ground_data.map(|(t, _, _)| Heightfield::new(t));
                let ground = ground_data
                    .zip(hf.as_ref())
                    .map(|((_, offset, scale), hf)| Ground { heightfield: hf, offset, scale });
                let world_pos = transform.translation.to_array();
                let instances = procedural::scatter_instances(spec, world_pos, ground.as_ref());
                let needs_tangents = material.pattern.as_ref().is_some_and(|p| p.bump > 0.0);
                if let Some((mesh, tris)) = self.shape_mesh(&spec.shape.kind, material.flat, needs_tangents) {
                    let mat = self.material(material);
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
                let needs_tangents = material.pattern.as_ref().is_some_and(|p| p.bump > 0.0);
                if let Some((mesh, tris)) = self.shape_mesh(kind, material.flat, needs_tangents) {
                    let mat = if animated_material {
                        // Eigenes Material, damit nur dieses Objekt seine Farbe ändert.
                        let fades = o.animate.as_ref().is_some_and(|a| !a.opacity.is_empty());
                        let h = self.material_unique(material, fades);
                        self.commands.queue({
                            let h = h.clone();
                            move |world: &mut World| {
                                if let Some(mut a) = world.get_mut::<Animated>(entity) {
                                    a.material = Some(h);
                                }
                            }
                        });
                        h
                    } else {
                        self.material(material)
                    };
                    if let (Kind::Tube(spec), Some(anim)) = (kind, &o.animate) {
                        if !anim.draw.is_empty() {
                            self.commands.entity(entity).insert(DrawTube { spec: spec.clone(), mesh: mesh.clone(), last: -1.0 });
                        }
                    }
                    self.commands.entity(entity).insert((Mesh3d(mesh), MeshMaterial3d(mat)));
                    self.stats.triangles += tris;
                }
            }
        }
    }

    /// Hinter jedem Gelände eine sehr große Bodenfläche in Geländefarbe, knapp darunter.
    /// So bricht die Welt am Horizont nicht ab; der Dunst lässt sie weich auslaufen.
    fn spawn_horizon_ground(&mut self, root: Entity) {
        let Some((pos, mut mat)) = horizon_source(self.scene) else { return };
        mat.pattern = None;
        let mesh = self.meshes.add(Plane3d::default().mesh().size(40_000.0, 40_000.0));
        let material = self.material(&mat);
        self.commands.spawn((
            Name::new("Horizont-Boden"),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(pos[0], pos[1] - 0.3, pos[2]),
            bevy::light::NotShadowCaster,
            ChildOf(root),
        ));
    }

    fn shape_mesh(&mut self, kind: &Kind, flat: bool, tangents: bool) -> Option<(Handle<Mesh>, usize)> {
        let generated = |d: MeshData| Some(to_bevy_mesh(d));
        let mut mesh: Mesh = match kind {
            // Eigener Generator statt Cuboid: UV in Metern (Muster gleich groß auf allen Größen).
            Kind::Box { size } => generated(modeling::rounded_box(*size, 0.0, 1))?,
            Kind::RoundedBox { size, radius, segments } => generated(modeling::rounded_box(*size, *radius, *segments))?,
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
            Kind::Pyramid { size } => generated(procedural::pyramid_mesh(*size))?,
            Kind::Terrain(t) => generated(procedural::terrain_mesh(t))?,
            Kind::Extrude(e) => generated(modeling::extrude(e))?,
            Kind::Lathe(l) => generated(modeling::lathe(l))?,
            Kind::Tube(t) => generated(modeling::tube(t))?,
            Kind::Text(t) => match modeling::text(
                match t.weight {
                    cf_scene::TextWeight::Regular => FONT,
                    cf_scene::TextWeight::Bold => FONT_BOLD,
                    cf_scene::TextWeight::Black => FONT_BLACK,
                },
                t,
            ) {
                Ok(d) => generated(d)?,
                Err(e) => {
                    self.stats.warnings.push(e);
                    return None;
                }
            },
            Kind::Group { .. }
            | Kind::Scatter(_)
            | Kind::PointLight(_)
            | Kind::Instance { .. }
            | Kind::Particles(_)
            | Kind::Image { .. }
            | Kind::Svg { .. }
            | Kind::Model { .. } => return None,
        };
        if mesh.count_vertices() == 0 {
            return None;
        }
        let already_flat = matches!(kind, Kind::Pyramid { .. } | Kind::Terrain(_) | Kind::Extrude(_) | Kind::Text(_) | Kind::Box { .. });
        if flat && !already_flat {
            mesh.duplicate_vertices();
            mesh.compute_flat_normals();
        }
        if tangents {
            if let Err(e) = mesh.generate_tangents() {
                self.stats.warnings.push(format!("Struktur (bump) nicht möglich: {e}"));
            }
        }
        let tris = mesh.indices().map(|i| i.len() / 3).unwrap_or_else(|| mesh.count_vertices() / 3);
        Some((self.meshes.add(mesh), tris))
    }

    fn pattern_textures(&mut self, p: &Pattern, base: cf_scene::Rgb) -> (Handle<Image>, Option<Handle<Image>>) {
        let key = format!("{p:?}{}", base.to_hex());
        if let Some(t) = self.texture_cache.get(&key) {
            return t.clone();
        }
        let imgs = textures::pattern_images(p, base, 512);
        let color = self.images.add(make_texture(imgs.size, imgs.color, true));
        let normal = imgs.normal.map(|n| self.images.add(make_texture(imgs.size, n, false)));
        self.texture_cache.insert(key, (color.clone(), normal.clone()));
        (color, normal)
    }

    /// Nicht geteiltes Material (für animierte Farben); `fades` erzwingt Transparenz.
    fn material_unique(&mut self, m: &Material, fades: bool) -> Handle<StandardMaterial> {
        let shared = self.material(m);
        let mut copy = self.materials.get(&shared).cloned().unwrap_or_default();
        if fades {
            copy.alpha_mode = AlphaMode::Blend;
        }
        self.materials.add(copy)
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
        // Bild als Oberfläche hat Vorrang vor einem Muster.
        let mut has_alpha = false;
        let texture = m.texture.as_ref().and_then(|f| {
            match crate::media::resolve(self.project, crate::media::IMAGES, f).and_then(|p| self.image_cache.load(&p, self.images)) {
                Ok(img) => {
                    has_alpha = img.has_alpha;
                    Some(img.handle)
                }
                Err(e) => {
                    self.stats.warnings.push(e);
                    None
                }
            }
        });
        let (base_color, base_color_texture, normal_map_texture, uv_transform) = match (&texture, &m.pattern) {
            (Some(t), _) => (Color::srgba(1.0, 1.0, 1.0, m.opacity), Some(t.clone()), None, Affine2::IDENTITY),
            (None, Some(p)) => {
                let (c, n) = self.pattern_textures(p, m.color);
                (Color::srgba(1.0, 1.0, 1.0, m.opacity), Some(c), n, Affine2::from_scale(Vec2::splat(p.scale)))
            }
            (None, None) => (Color::srgba(m.color.r, m.color.g, m.color.b, m.opacity), None, None, Affine2::IDENTITY),
        };
        let handle = self.materials.add(StandardMaterial {
            base_color,
            base_color_texture,
            normal_map_texture,
            uv_transform,
            perceptual_roughness: m.roughness.clamp(0.02, 1.0),
            metallic: m.metallic.clamp(0.0, 1.0),
            emissive,
            specular_transmission: m.transmission.clamp(0.0, 1.0),
            diffuse_transmission: 0.0,
            thickness: if m.transmission > 0.0 { 0.5 } else { 0.0 },
            ior: m.ior.clamp(1.0, 3.0),
            clearcoat: m.clearcoat.clamp(0.0, 1.0),
            clearcoat_perceptual_roughness: 0.1,
            unlit: m.unlit,
            double_sided: m.double_sided,
            cull_mode: if m.double_sided { None } else { Some(Face::Back) },
            alpha_mode: if m.opacity < 1.0 || has_alpha { AlphaMode::Blend } else { AlphaMode::Opaque },
            ..default()
        });
        self.material_cache.insert(key, handle.clone());
        handle
    }
}

/// Erzeugt eine kachelbare Textur mit Mipmaps (kein Flimmern in der Ferne).
fn make_texture(size: u32, data: Vec<u8>, srgb: bool) -> Image {
    let format = if srgb { TextureFormat::Rgba8UnormSrgb } else { TextureFormat::Rgba8Unorm };
    let mut levels = vec![data];
    let mut s = size as usize;
    while s > 1 {
        let prev = levels.last().expect("mindestens eine Stufe");
        let ns = s / 2;
        let mut next = vec![0u8; ns * ns * 4];
        for y in 0..ns {
            for x in 0..ns {
                for c in 0..4 {
                    let at = |xx: usize, yy: usize| prev[(yy * s + xx) * 4 + c] as u32;
                    let sum = at(2 * x, 2 * y) + at(2 * x + 1, 2 * y) + at(2 * x, 2 * y + 1) + at(2 * x + 1, 2 * y + 1);
                    next[(y * ns + x) * 4 + c] = (sum / 4) as u8;
                }
            }
        }
        levels.push(next);
        s = ns;
    }
    let mip_count = levels.len() as u32;
    let mut img = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        levels[0].clone(),
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.data = Some(levels.concat());
    img.texture_descriptor.mip_level_count = mip_count;
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rep(offset: [f32; 3], rotation: [f32; 3], pivot: [f32; 3]) -> cf_scene::Repeat {
        cf_scene::Repeat { count: 4, offset, rotation, scale: 1.0, pivot }
    }

    #[test]
    fn repeat_row_circle_and_spiral() {
        let base = Transform::from_xyz(10.0, 0.0, 0.0);
        // Reihe
        let step = repeat_step(&rep([2.0, 0.0, 0.0], [0.0; 3], [0.0; 3]), &base);
        assert!(step.mul_transform(base).translation.abs_diff_eq(Vec3::new(12.0, 0.0, 0.0), 1e-4));
        // Drehung ohne Pivot: bleibt am Ort, dreht sich nur
        let step = repeat_step(&rep([0.0, 1.0, 0.0], [0.0, 90.0, 0.0], [0.0; 3]), &base);
        let t1 = step.mul_transform(base);
        assert!(t1.translation.abs_diff_eq(Vec3::new(10.0, 1.0, 0.0), 1e-4), "{:?}", t1.translation);
        // Kreis um Mittelpunkt 5 m links des Objekts: nach 2 × 90° gegenüber
        let step = repeat_step(&rep([0.0; 3], [0.0, 90.0, 0.0], [-5.0, 0.0, 0.0]), &base);
        let t2 = step.mul_transform(step.mul_transform(base));
        assert!(t2.translation.abs_diff_eq(Vec3::new(0.0, 0.0, 0.0), 1e-3), "{:?}", t2.translation);
    }
}

/// Entfernt die Lade-Markierung, sobald ein Modell (samt Texturen) geladen ist – oder fehlgeschlagen.
fn track_model_loading(
    mut commands: Commands,
    loading: Query<(Entity, &ModelLoading)>,
    server: Res<AssetServer>,
    mut stats: ResMut<SceneStats>,
) {
    for (e, m) in &loading {
        if server.is_loaded_with_dependencies(&m.0) {
            commands.entity(e).remove::<ModelLoading>();
        } else if let Some(bevy::asset::LoadState::Failed(err)) = server.get_load_state(&m.0) {
            stats.warnings.push(format!("Modell konnte nicht geladen werden: {err}"));
            warn!("Modell konnte nicht geladen werden: {err}");
            commands.entity(e).remove::<ModelLoading>();
        }
    }
}

pub fn to_bevy_mesh(d: MeshData) -> Mesh {
    // CPU-Kopie behalten: nötig, damit man Objekte in der Vorschau anklicken kann.
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, d.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, d.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, d.uvs)
        .with_inserted_indices(Indices::U32(d.indices))
}
