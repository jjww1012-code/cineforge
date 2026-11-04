//! Prozedurale Geometrie, unabhängig von der Render-Engine.
//! Die Engine wandelt [`MeshData`] nur noch in GPU-Puffer um.

use crate::model::{Kind, Object, ScatterSpec, TerrainSpec, TerrainStyle};
use crate::noise::{Noise2, Rng};

#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

// ---------------------------------------------------------------------------
// Gelände
// ---------------------------------------------------------------------------

/// Höhenfunktion eines Geländes in dessen lokalen Koordinaten.
pub struct Heightfield {
    spec: TerrainSpec,
    noise: Noise2,
    warp: Noise2,
}

impl Heightfield {
    pub fn new(spec: &TerrainSpec) -> Self {
        Self {
            spec: spec.clone(),
            noise: Noise2::new(spec.seed),
            warp: Noise2::new(spec.seed.wrapping_add(7919)),
        }
    }

    /// Höhe in Metern an lokaler Position (x, z).
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let s = &self.spec;
        let f = 1.0 / s.feature_size;
        let (u, v) = (x * f, z * f);
        let h01 = match s.style {
            TerrainStyle::Flat => 0.0,
            TerrainStyle::Hills => (self.noise.fbm(u, v, 5) * 0.5 + 0.5).clamp(0.0, 1.0),
            TerrainStyle::Mountains => {
                let r = self.noise.ridged(u * 0.7, v * 0.7, 6);
                let base = self.noise.fbm(u * 0.3, v * 0.3, 3) * 0.5 + 0.5;
                (r.powf(1.6) * 0.8 + base * 0.35).clamp(0.0, 1.0)
            }
            TerrainStyle::Dunes => {
                // Lange Kämme quer zur Windrichtung, mit scharfem Grat und weichem Tal.
                let w = self.warp.fbm(u * 0.35, v * 0.35, 3);
                let along = u * 0.82 + v * 0.57 + w * 1.4;
                let ridge = 1.0 - (along * std::f32::consts::PI).sin().abs();
                let ridge = ridge.powf(1.5);
                let amp = self.noise.fbm(u * 0.25 + 3.1, v * 0.25 - 1.7, 2) * 0.5 + 0.6;
                let detail = self.noise.fbm(u * 3.0, v * 3.0, 3) * 0.04;
                (ridge * amp.clamp(0.15, 1.0) + detail).clamp(0.0, 1.0)
            }
        };
        let mut h = h01 * s.height;
        if s.flat_center_radius > 0.0 {
            let d = (x * x + z * z).sqrt();
            let t = smoothstep(s.flat_center_radius, s.flat_center_radius * 1.6, d);
            h *= t;
        }
        h
    }

    fn normal(&self, x: f32, z: f32, eps: f32) -> [f32; 3] {
        let hx = self.height(x + eps, z) - self.height(x - eps, z);
        let hz = self.height(x, z + eps) - self.height(x, z - eps);
        normalize([-hx, 2.0 * eps, -hz])
    }
}

pub fn terrain_mesh(spec: &TerrainSpec) -> MeshData {
    let hf = Heightfield::new(spec);
    let n = spec.resolution.max(2);
    let [w, d] = spec.size;
    let step_x = w / (n - 1) as f32;
    let eps = step_x * 0.5;
    let mut m = MeshData::default();
    let cap = (n * n) as usize;
    m.positions.reserve(cap);
    m.normals.reserve(cap);
    m.uvs.reserve(cap);
    for j in 0..n {
        for i in 0..n {
            let fx = i as f32 / (n - 1) as f32;
            let fz = j as f32 / (n - 1) as f32;
            let x = (fx - 0.5) * w;
            let z = (fz - 0.5) * d;
            m.positions.push([x, hf.height(x, z), z]);
            m.normals.push(hf.normal(x, z, eps));
            // UV in Metern / 4 ⇒ Texturen wiederholen sich alle 4 m.
            m.uvs.push([x / 4.0, z / 4.0]);
        }
    }
    m.indices.reserve(((n - 1) * (n - 1) * 6) as usize);
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let a = j * n + i;
            let b = a + 1;
            let c = a + n;
            let e = c + 1;
            // Gegen den Uhrzeigersinn von oben gesehen ⇒ Normalen zeigen nach oben.
            m.indices.extend_from_slice(&[a, c, b, b, c, e]);
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Pyramide
// ---------------------------------------------------------------------------

pub fn pyramid_mesh(size: [f32; 3]) -> MeshData {
    let [w, h, d] = size;
    let (hw, hd) = (w / 2.0, d / 2.0);
    let apex = [0.0, h, 0.0];
    let base = [[-hw, 0.0, -hd], [hw, 0.0, -hd], [hw, 0.0, hd], [-hw, 0.0, hd]];
    let mut m = MeshData::default();
    // Vier Seiten, jede mit eigener Flächennormale (scharfe Kanten).
    for k in 0..4 {
        let p0 = base[k];
        let p1 = base[(k + 1) % 4];
        // Reihenfolge so, dass die Fläche nach außen zeigt.
        let tri = [p1, p0, apex];
        let nrm = face_normal(tri[0], tri[1], tri[2]);
        let start = m.positions.len() as u32;
        for (p, uv) in tri.iter().zip([[1.0, 1.0], [0.0, 1.0], [0.5, 0.0]]) {
            m.positions.push(*p);
            m.normals.push(nrm);
            m.uvs.push(uv);
        }
        m.indices.extend_from_slice(&[start, start + 1, start + 2]);
    }
    // Boden
    let start = m.positions.len() as u32;
    for (p, uv) in base.iter().zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]) {
        m.positions.push(*p);
        m.normals.push([0.0, -1.0, 0.0]);
        m.uvs.push(uv);
    }
    m.indices
        .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    m
}

// ---------------------------------------------------------------------------
// Scatter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instance {
    /// Lokale Position relativ zum Scatter-Objekt.
    pub position: [f32; 3],
    /// Drehung um die Y-Achse in Radiant.
    pub yaw: f32,
    pub scale: f32,
}

/// Bodenhöhe in Weltkoordinaten (für `on_terrain`).
pub struct Ground<'a> {
    pub heightfield: &'a Heightfield,
    pub offset: [f32; 3],
    pub scale: [f32; 3],
}

impl Ground<'_> {
    pub fn world_height(&self, wx: f32, wz: f32) -> f32 {
        let lx = (wx - self.offset[0]) / self.scale[0];
        let lz = (wz - self.offset[2]) / self.scale[2];
        self.offset[1] + self.heightfield.height(lx, lz) * self.scale[1]
    }
}

pub fn scatter_instances(
    spec: &ScatterSpec,
    origin: [f32; 3],
    ground: Option<&Ground>,
) -> Vec<Instance> {
    let mut rng = Rng::new(spec.seed);
    let [aw, ad] = spec.area;
    let base_offset = base_offset(&spec.shape.kind);
    let shape_h = shape_height(&spec.shape.kind);
    let mut out = Vec::with_capacity(spec.count as usize);
    let mut attempts = 0u32;
    while out.len() < spec.count as usize && attempts < spec.count.saturating_mul(20).max(100) {
        attempts += 1;
        let x = rng.range(-aw / 2.0, aw / 2.0);
        let z = rng.range(-ad / 2.0, ad / 2.0);
        let s = rng.range(spec.scale_range[0], spec.scale_range[1]);
        let yaw = if spec.random_rotation {
            rng.range(0.0, std::f32::consts::TAU)
        } else {
            0.0
        };
        if spec.keep_clear_radius > 0.0 && (x * x + z * z).sqrt() < spec.keep_clear_radius {
            continue;
        }
        let ground_y = match ground {
            Some(g) => g.world_height(origin[0] + x, origin[2] + z) - origin[1],
            None => 0.0,
        };
        let y = ground_y + (base_offset - spec.sink * shape_h) * s;
        out.push(Instance { position: [x, y, z], yaw, scale: s });
    }
    out
}

/// Abstand vom Objekt-Ursprung zur Unterseite (damit Formen auf dem Boden stehen).
pub fn base_offset(kind: &Kind) -> f32 {
    match kind {
        Kind::Box { size } => size[1] / 2.0,
        Kind::Sphere { radius } => *radius,
        Kind::Cylinder { height, .. } | Kind::Cone { height, .. } => height / 2.0,
        Kind::Capsule { radius, length } => length / 2.0 + radius,
        Kind::Torus { thickness, .. } => *thickness,
        _ => 0.0,
    }
}

pub fn shape_height(kind: &Kind) -> f32 {
    match kind {
        Kind::Box { size } | Kind::Pyramid { size } => size[1],
        Kind::Sphere { radius } => radius * 2.0,
        Kind::Cylinder { height, .. } | Kind::Cone { height, .. } => *height,
        Kind::Capsule { radius, length } => length + radius * 2.0,
        Kind::Torus { thickness, .. } => thickness * 2.0,
        _ => 0.0,
    }
}

/// Sucht ein Terrain per id (inkl. Gruppen) und liefert es mit seiner Position/Skalierung.
/// Hinweis: Rotation von Terrain und übergeordneten Gruppen wird hier noch nicht berücksichtigt.
pub fn find_terrain<'a>(objects: &'a [Object], id: &str) -> Option<(&'a TerrainSpec, [f32; 3], [f32; 3])> {
    for o in objects {
        match &o.kind {
            Kind::Terrain(t) if o.id == id => return Some((t, o.position, o.scale.to_array())),
            Kind::Group { children } => {
                if let Some((t, p, s)) = find_terrain(children, id) {
                    let gs = o.scale.to_array();
                    let pos = [
                        o.position[0] + p[0] * gs[0],
                        o.position[1] + p[1] * gs[1],
                        o.position[2] + p[2] * gs[2],
                    ];
                    return Some((t, pos, [s[0] * gs[0], s[1] * gs[1], s[2] * gs[2]]));
                }
            }
            _ => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    normalize([
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ])
}
