//! Formen verfeinern: Unterteilen (auf Wunsch geglättet, Loop-Verfahren) und
//! Verformen (verdrehen, biegen, verjüngen, Welle, aufblähen, Oberflächen-Rauschen).
//!
//! Damit werden aus einfachen Grundformen detaillierte Modelle: ein unterteilter,
//! geglätteter Würfel wird zum Kieselstein, eine Kugel mit Rauschen zum Felsen,
//! eine verdrehte Box zur gedrehten Säule, ein gebogener Zylinder zum Ast.
//!
//! Alles läuft auf „verschweißten“ Punkten (gleiche Position = ein Punkt), damit an
//! Nähten (UV-Kanten, Würfelkanten) keine Risse entstehen.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::noise::Noise3;
use crate::procedural::MeshData;

/// Höchste Stufe beim Unterteilen (jede Stufe vervierfacht die Dreiecke).
pub const MAX_SUBDIVIDE: u32 = 5;
/// Obergrenze für Dreiecke nach dem Unterteilen.
pub const MAX_TRIANGLES: usize = 1_500_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    X,
    #[default]
    Y,
    Z,
}

impl Axis {
    fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }
}

fn axis_x() -> Axis {
    Axis::X
}
fn d90() -> f32 {
    90.0
}
fn d45() -> f32 {
    45.0
}
fn half() -> f32 {
    0.5
}
fn tenth() -> f32 {
    0.1
}
fn one() -> f32 {
    1.0
}
fn three() -> u32 {
    3
}

/// Eine Verformung. Alle wirken im eigenen Koordinatensystem des Objekts
/// (vor Drehung/Skalierung) und beziehen sich auf seine Ausdehnung.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Deform {
    /// Verdrehen um die Achse: unten 0°, oben `angle`.
    Twist {
        #[serde(default = "d90")]
        angle: f32,
        #[serde(default)]
        axis: Axis,
    },
    /// Zu einem Bogen biegen (insgesamt `angle` Grad), das untere Ende bleibt stehen.
    /// Biegt Richtung +X (bei Achse x Richtung +Y). Andere Richtung: Objekt drehen oder negativer Winkel.
    Bend {
        #[serde(default = "d45")]
        angle: f32,
        #[serde(default)]
        axis: Axis,
    },
    /// Verjüngen: Querschnitt unten 1, oben `scale` (0 = Spitze, 2 = doppelt so breit).
    Taper {
        #[serde(default = "half")]
        scale: f32,
        #[serde(default)]
        axis: Axis,
    },
    /// Welle entlang `axis`, Auslenkung senkrecht dazu (bei x/z nach oben, bei y nach +X).
    Wave {
        #[serde(default = "tenth")]
        amplitude: f32,
        #[serde(default = "one")]
        wavelength: f32,
        #[serde(default = "axis_x")]
        axis: Axis,
        #[serde(default)]
        phase: f32,
    },
    /// Oberfläche nach außen (positiv) oder innen (negativ) drücken, in Metern.
    Inflate {
        #[serde(default = "tenth")]
        amount: f32,
    },
    /// Unregelmäßige Oberfläche (Felsen, Erde, Rinde, Haut): Ausschlag `amount` in Metern,
    /// `scale` = Beulen pro Meter, `octaves` = Feinheit (1–6).
    Noise {
        #[serde(default = "tenth")]
        amount: f32,
        #[serde(default = "one")]
        scale: f32,
        #[serde(default = "three")]
        octaves: u32,
        #[serde(default)]
        seed: u32,
    },
}

impl Deform {
    pub fn validate(&self) -> Option<String> {
        match self {
            Deform::Wave { wavelength, .. } if *wavelength <= 0.0 => Some("wave: wavelength muss größer als 0 sein".into()),
            Deform::Noise { scale, .. } if *scale <= 0.0 => Some("noise: scale muss größer als 0 sein".into()),
            Deform::Noise { octaves, .. } if *octaves > 8 => Some("noise: höchstens 8 octaves".into()),
            Deform::Taper { scale, .. } if *scale < 0.0 => Some("taper: scale darf nicht negativ sein".into()),
            _ => None,
        }
    }
}

/// Netz mit verschweißten Punkten; UVs je Dreiecks-Ecke.
struct Welded {
    pos: Vec<[f32; 3]>,
    tris: Vec<[u32; 3]>,
    uv: Vec<[[f32; 2]; 3]>,
}

fn weld(m: &MeshData) -> Welded {
    // Toleranz relativ zur Größe, damit kleine und große Objekte gleich behandelt werden.
    let (lo, hi) = bounds(&m.positions);
    let size = (0..3).map(|i| hi[i] - lo[i]).fold(0.0_f32, f32::max).max(1e-6);
    let q = 1e5 / size;
    let mut map: HashMap<[i64; 3], u32> = HashMap::new();
    let mut pos = Vec::new();
    let mut remap = Vec::with_capacity(m.positions.len());
    for p in &m.positions {
        let key = [(p[0] * q).round() as i64, (p[1] * q).round() as i64, (p[2] * q).round() as i64];
        let id = *map.entry(key).or_insert_with(|| {
            pos.push(*p);
            (pos.len() - 1) as u32
        });
        remap.push(id);
    }
    let uv_of = |i: u32| m.uvs.get(i as usize).copied().unwrap_or([0.0, 0.0]);
    let mut tris = Vec::with_capacity(m.indices.len() / 3);
    let mut uv = Vec::with_capacity(m.indices.len() / 3);
    for t in m.indices.chunks_exact(3) {
        let w = [remap[t[0] as usize], remap[t[1] as usize], remap[t[2] as usize]];
        if w[0] == w[1] || w[1] == w[2] || w[0] == w[2] {
            continue; // entartetes Dreieck
        }
        tris.push(w);
        uv.push([uv_of(t[0]), uv_of(t[1]), uv_of(t[2])]);
    }
    Welded { pos, tris, uv }
}

fn bounds(p: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in p {
        for i in 0..3 {
            lo[i] = lo[i].min(v[i]);
            hi[i] = hi[i].max(v[i]);
        }
    }
    if p.is_empty() {
        return ([0.0; 3], [0.0; 3]);
    }
    (lo, hi)
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt();
    if l > 1e-12 { mul(a, 1.0 / l) } else { [0.0, 1.0, 0.0] }
}
fn mid2(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]
}

/// Eine Stufe Unterteilung: jedes Dreieck → 4. Mit `smooth` nach dem Loop-Verfahren
/// (rundet die Form ab), sonst bleibt die Form exakt erhalten.
fn subdivide_once(w: &Welded, smooth: bool) -> Welded {
    let n = w.pos.len();
    // Kanten: (kleiner, größer) → gegenüberliegende Punkte
    let mut edges: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for t in &w.tris {
        for k in 0..3 {
            let (a, b, c) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3]);
            edges.entry((a.min(b), a.max(b))).or_default().push(c);
        }
    }
    // Feste Reihenfolge (HashMap-Reihenfolge ist zufällig) ⇒ bitgenau gleiche Ergebnisse.
    let mut edge_keys: Vec<(u32, u32)> = edges.keys().copied().collect();
    edge_keys.sort_unstable();
    let mut pos = w.pos.clone();
    if smooth {
        // Nachbarn und Randnachbarn je Punkt
        let mut nb: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut boundary_nb: Vec<Vec<u32>> = vec![Vec::new(); n];
        for &(a, b) in &edge_keys {
            let opp = &edges[&(a, b)];
            nb[a as usize].push(b);
            nb[b as usize].push(a);
            if opp.len() == 1 {
                boundary_nb[a as usize].push(b);
                boundary_nb[b as usize].push(a);
            }
        }
        for i in 0..n {
            let p = w.pos[i];
            if !boundary_nb[i].is_empty() {
                if boundary_nb[i].len() == 2 {
                    let (b1, b2) = (w.pos[boundary_nb[i][0] as usize], w.pos[boundary_nb[i][1] as usize]);
                    pos[i] = add(mul(p, 0.75), mul(add(b1, b2), 0.125));
                }
                continue; // Ecken/komplizierte Ränder bleiben stehen
            }
            let k = nb[i].len();
            if k < 3 {
                continue;
            }
            let beta = if k == 3 { 3.0 / 16.0 } else { 3.0 / (8.0 * k as f32) };
            let sum = nb[i].iter().fold([0.0; 3], |s, &j| add(s, w.pos[j as usize]));
            pos[i] = add(mul(p, 1.0 - k as f32 * beta), mul(sum, beta));
        }
    }
    // Neue Kantenpunkte
    let mut edge_pt: HashMap<(u32, u32), u32> = HashMap::with_capacity(edges.len());
    for &(a, b) in &edge_keys {
        let opp = &edges[&(a, b)];
        let (pa, pb) = (w.pos[a as usize], w.pos[b as usize]);
        let p = if smooth && opp.len() == 2 {
            let (c, d) = (w.pos[opp[0] as usize], w.pos[opp[1] as usize]);
            add(mul(add(pa, pb), 0.375), mul(add(c, d), 0.125))
        } else {
            mul(add(pa, pb), 0.5)
        };
        pos.push(p);
        edge_pt.insert((a, b), (pos.len() - 1) as u32);
    }
    let e = |a: u32, b: u32| edge_pt[&(a.min(b), a.max(b))];
    let mut tris = Vec::with_capacity(w.tris.len() * 4);
    let mut uv = Vec::with_capacity(w.tris.len() * 4);
    for (t, u) in w.tris.iter().zip(&w.uv) {
        let (a, b, c) = (t[0], t[1], t[2]);
        let (ab, bc, ca) = (e(a, b), e(b, c), e(c, a));
        let (uab, ubc, uca) = (mid2(u[0], u[1]), mid2(u[1], u[2]), mid2(u[2], u[0]));
        tris.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
        uv.extend([[u[0], uab, uca], [uab, u[1], ubc], [uca, ubc, u[2]], [uab, ubc, uca]]);
    }
    Welded { pos, tris, uv }
}

/// Glatte Normalen je Punkt (flächengewichtet).
fn point_normals(w: &Welded) -> Vec<[f32; 3]> {
    let mut nrm = vec![[0.0; 3]; w.pos.len()];
    for t in &w.tris {
        let (a, b, c) = (w.pos[t[0] as usize], w.pos[t[1] as usize], w.pos[t[2] as usize]);
        let f = cross(sub(b, a), sub(c, a));
        for &i in t {
            nrm[i as usize] = add(nrm[i as usize], f);
        }
    }
    nrm.into_iter().map(norm).collect()
}

fn apply_deform(w: &mut Welded, d: &Deform) {
    let (lo, hi) = bounds(&w.pos);
    let center = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
    let along = |p: &[f32; 3], ax: usize| {
        let len = (hi[ax] - lo[ax]).max(1e-6);
        ((p[ax] - lo[ax]) / len, len)
    };
    // Die beiden anderen Achsen (rechtshändig: y → (z, x), x → (y, z), z → (x, y))
    let others = |ax: usize| ((ax + 1) % 3, (ax + 2) % 3);
    match d {
        Deform::Twist { angle, axis } => {
            let ax = axis.index();
            let (u, v) = others(ax);
            for p in &mut w.pos {
                let (t, _) = along(p, ax);
                let a = angle.to_radians() * t;
                let (s, c) = a.sin_cos();
                let (pu, pv) = (p[u] - center[u], p[v] - center[v]);
                p[u] = center[u] + pu * c - pv * s;
                p[v] = center[v] + pu * s + pv * c;
            }
        }
        Deform::Taper { scale, axis } => {
            let ax = axis.index();
            let (u, v) = others(ax);
            for p in &mut w.pos {
                let (t, _) = along(p, ax);
                let s = 1.0 + (scale - 1.0) * t;
                p[u] = center[u] + (p[u] - center[u]) * s;
                p[v] = center[v] + (p[v] - center[v]) * s;
            }
        }
        Deform::Bend { angle, axis } => {
            let theta = angle.to_radians();
            if theta.abs() < 1e-5 {
                return;
            }
            let ax = axis.index();
            // Biegerichtung: bei y nach +x, bei x nach +y, bei z nach +y
            let dir = if ax == 1 { 0 } else { 1 };
            for p in &mut w.pos {
                let (t, len) = along(p, ax);
                let r = len / theta; // Radius des Bogens
                let phi = theta * t;
                let x = p[dir] - center[dir]; // Abstand von der Mittellinie
                let (s, c) = phi.sin_cos();
                p[dir] = center[dir] + r - (r - x) * c;
                p[ax] = lo[ax] + (r - x) * s;
            }
        }
        Deform::Wave { amplitude, wavelength, axis, phase } => {
            let ax = axis.index();
            let dir = if ax == 1 { 0 } else { 1 };
            let k = std::f32::consts::TAU / wavelength.max(1e-4);
            for p in &mut w.pos {
                p[dir] += amplitude * (p[ax] * k + phase.to_radians()).sin();
            }
        }
        Deform::Inflate { amount } => {
            let n = point_normals(w);
            for (p, n) in w.pos.iter_mut().zip(n) {
                *p = add(*p, mul(n, *amount));
            }
        }
        Deform::Noise { amount, scale, octaves, seed } => {
            let n = point_normals(w);
            let noise = Noise3::new(*seed);
            for (p, n) in w.pos.iter_mut().zip(n) {
                let h = noise.fbm(p[0] * scale, p[1] * scale, p[2] * scale, *octaves);
                *p = add(*p, mul(n, h * amount));
            }
        }
    }
}

/// Zurück in ein normales Netz. Normalen werden geglättet, außer an Kanten, die schärfer
/// als `crease_deg` sind (so bleiben Würfelkanten scharf, Rundungen weich).
fn unweld(w: &Welded, crease_deg: f32) -> MeshData {
    let face_n: Vec<[f32; 3]> = w
        .tris
        .iter()
        .map(|t| {
            let (a, b, c) = (w.pos[t[0] as usize], w.pos[t[1] as usize], w.pos[t[2] as usize]);
            cross(sub(b, a), sub(c, a))
        })
        .collect();
    let mut incident: Vec<Vec<u32>> = vec![Vec::new(); w.pos.len()];
    for (fi, t) in w.tris.iter().enumerate() {
        for &i in t {
            incident[i as usize].push(fi as u32);
        }
    }
    let cos_limit = crease_deg.to_radians().cos();
    let mut out = MeshData::default();
    let mut map: HashMap<(u32, [i32; 2], [i32; 3]), u32> = HashMap::new();
    for (fi, (t, uvs)) in w.tris.iter().zip(&w.uv).enumerate() {
        let fnorm = norm(face_n[fi]);
        for k in 0..3 {
            let pi = t[k];
            let mut n = [0.0; 3];
            for &other in &incident[pi as usize] {
                let on = face_n[other as usize];
                if dot(norm(on), fnorm) >= cos_limit {
                    n = add(n, on);
                }
            }
            let n = norm(n);
            let uv = uvs[k];
            let key = (
                pi,
                [(uv[0] * 1e4).round() as i32, (uv[1] * 1e4).round() as i32],
                [(n[0] * 1e3).round() as i32, (n[1] * 1e3).round() as i32, (n[2] * 1e3).round() as i32],
            );
            let idx = *map.entry(key).or_insert_with(|| {
                out.positions.push(w.pos[pi as usize]);
                out.normals.push(n);
                out.uvs.push(uv);
                (out.positions.len() - 1) as u32
            });
            out.indices.push(idx);
        }
    }
    out
}

/// Ergebnis von [`reshape`].
pub struct Reshaped {
    pub mesh: MeshData,
    /// Tatsächlich verwendete Unterteilungs-Stufen (wegen der Dreiecks-Obergrenze evtl. weniger).
    pub levels: u32,
}

/// Unterteilen und verformen. `rounded` (= Loop-Glättung) ohne `subdivide` bedeutet 2 Stufen.
pub fn reshape(m: &MeshData, subdivide: u32, smooth: bool, deform: &[Deform]) -> Reshaped {
    let mut w = weld(m);
    let wanted = if smooth && subdivide == 0 { 2 } else { subdivide.min(MAX_SUBDIVIDE) };
    let mut levels = 0;
    while levels < wanted && w.tris.len() * 4 <= MAX_TRIANGLES {
        w = subdivide_once(&w, smooth);
        levels += 1;
    }
    for d in deform {
        apply_deform(&mut w, d);
    }
    let crease = if smooth { 80.0 } else { 50.0 };
    Reshaped { mesh: unweld(&w, crease), levels }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modeling::rounded_box;

    fn cube() -> MeshData {
        rounded_box([1.0, 1.0, 1.0], 0.0, 1)
    }

    #[test]
    fn subdivide_multiplies_triangles_and_keeps_shape() {
        let c = cube();
        let r = reshape(&c, 2, false, &[]);
        assert_eq!(r.levels, 2);
        assert_eq!(r.mesh.indices.len() / 3, c.indices.len() / 3 * 16);
        let (lo, hi) = bounds(&r.mesh.positions);
        assert!((lo[0] + 0.5).abs() < 1e-4 && (hi[1] - 0.5).abs() < 1e-4, "Form bleibt ein Würfel");
        // Würfelkanten bleiben scharf: es gibt Normalen genau entlang der Achsen
        assert!(r.mesh.normals.iter().all(|n| n.iter().filter(|x| x.abs() > 0.99).count() == 1));
    }

    #[test]
    fn smooth_rounds_cube_without_cracks() {
        let r = reshape(&cube(), 3, true, &[]);
        let (lo, hi) = bounds(&r.mesh.positions);
        assert!(hi[0] < 0.5 && lo[0] > -0.5, "geglättet = kleiner und rund");
        // geschlossen: jede Kante gehört zu genau zwei Dreiecken (über verschweißte Punkte)
        let w = weld(&r.mesh);
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &w.tris {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                *edges.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        assert!(edges.values().all(|&n| n == 2), "keine Risse");
    }

    #[test]
    fn twist_bend_taper_wave() {
        let tall = rounded_box([0.4, 2.0, 0.4], 0.0, 1);
        let r = reshape(&tall, 3, false, &[Deform::Twist { angle: 90.0, axis: Axis::Y }]);
        // in halber Höhe um 45° gedreht: Ecke (0.2, 0, 0.2) liegt jetzt bei (0.283, 0, 0)
        assert!(
            r.mesh.positions.iter().any(|p| p[1].abs() < 1e-4 && (p[0] - 0.2828).abs() < 1e-3 && p[2].abs() < 1e-3),
            "Verdrehung in der Mitte"
        );
        let r = reshape(&tall, 3, false, &[Deform::Taper { scale: 0.0, axis: Axis::Y }]);
        let top: Vec<_> = r.mesh.positions.iter().filter(|p| (p[1] - 1.0).abs() < 1e-4).collect();
        assert!(top.iter().all(|p| p[0].abs() < 1e-4 && p[2].abs() < 1e-4), "Spitze");
        let r = reshape(&tall, 4, false, &[Deform::Bend { angle: 90.0, axis: Axis::Y }]);
        let (lo, hi) = bounds(&r.mesh.positions);
        assert!(hi[0] > 1.0 && hi[1] < 0.5, "zur Seite gebogen: {lo:?} {hi:?}");
        let r = reshape(&rounded_box([4.0, 0.2, 1.0], 0.0, 1), 4, false, &[Deform::Wave { amplitude: 0.3, wavelength: 2.0, axis: Axis::X, phase: 0.0 }]);
        let (_, hi) = bounds(&r.mesh.positions);
        assert!(hi[1] > 0.35, "Welle hebt an: {hi:?}");
    }

    #[test]
    fn noise_is_deterministic_and_seamless() {
        let a = reshape(&cube(), 3, true, &[Deform::Noise { amount: 0.1, scale: 3.0, octaves: 4, seed: 7 }]);
        let b = reshape(&cube(), 3, true, &[Deform::Noise { amount: 0.1, scale: 3.0, octaves: 4, seed: 7 }]);
        assert_eq!(a.mesh.positions, b.mesh.positions);
        let w = weld(&a.mesh);
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &w.tris {
            for k in 0..3 {
                let (x, y) = (t[k], t[(k + 1) % 3]);
                *edges.entry((x.min(y), x.max(y))).or_default() += 1;
            }
        }
        assert!(edges.values().all(|&n| n == 2), "Rauschen reißt keine Nähte auf");
    }

    #[test]
    fn triangle_budget_limits_levels() {
        let r = reshape(&cube(), 99, false, &[]);
        assert!(r.levels <= MAX_SUBDIVIDE);
        assert!(r.mesh.indices.len() / 3 <= MAX_TRIANGLES);
    }
}
