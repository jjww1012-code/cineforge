//! Prozedurales Modellieren: abgerundete Quader, Extrusion, Drehkörper,
//! Röhren und 3D-Schrift. Alles engine-unabhängig als [`MeshData`].
//!
//! Dreiecke werden mit [`push_tri`] immer so angelegt, dass ihre Vorderseite
//! zur gewünschten Normalen zeigt – damit gibt es keine „umgedrehten“ Flächen,
//! egal in welcher Reihenfolge Punkte angegeben werden.

use crate::model::{ExtrudeSpec, LatheSpec, TextAlign, TextSpec, TubeSpec};
use crate::procedural::MeshData;

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: V3) -> f32 {
    dot(a, a).sqrt()
}
fn norm(a: V3) -> V3 {
    let l = len(a);
    if l < 1e-12 { [0.0, 1.0, 0.0] } else { mul(a, 1.0 / l) }
}

/// Fügt ein Dreieck mit flacher Normale hinzu; Reihenfolge wird so gewählt,
/// dass die Vorderseite (gegen den Uhrzeigersinn) zu `facing` zeigt.
fn push_tri(m: &mut MeshData, p: [V3; 3], uv: [[f32; 2]; 3], facing: V3) {
    let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
    if len(n) < 1e-12 {
        return; // entartet
    }
    let flip = dot(n, facing) < 0.0;
    let n = norm(if flip { mul(n, -1.0) } else { n });
    let order = if flip { [0, 2, 1] } else { [0, 1, 2] };
    let start = m.positions.len() as u32;
    for &i in &order {
        m.positions.push(p[i]);
        m.normals.push(n);
        m.uvs.push(uv[i]);
    }
    m.indices.extend_from_slice(&[start, start + 1, start + 2]);
}

/// Fügt ein Dreieck aus bereits vorhandenen Vertices hinzu (glatte Normalen),
/// ausgerichtet an der mittleren Vertex-Normale.
fn tri_indexed(m: &mut MeshData, a: u32, b: u32, c: u32) {
    let (pa, pb, pc) = (m.positions[a as usize], m.positions[b as usize], m.positions[c as usize]);
    let n = cross(sub(pb, pa), sub(pc, pa));
    let avg = add(add(m.normals[a as usize], m.normals[b as usize]), m.normals[c as usize]);
    if len(n) < 1e-12 {
        return;
    }
    if dot(n, avg) >= 0.0 {
        m.indices.extend_from_slice(&[a, b, c]);
    } else {
        m.indices.extend_from_slice(&[a, c, b]);
    }
}

fn push_vertex(m: &mut MeshData, p: V3, n: V3, uv: [f32; 2]) -> u32 {
    m.positions.push(p);
    m.normals.push(n);
    m.uvs.push(uv);
    (m.positions.len() - 1) as u32
}

// ---------------------------------------------------------------------------
// Abgerundeter Quader
// ---------------------------------------------------------------------------

pub fn rounded_box(size: [f32; 3], radius: f32, segments: u32) -> MeshData {
    let h = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
    let r = radius.clamp(0.0, h[0].min(h[1]).min(h[2]));
    let inner = [h[0] - r, h[1] - r, h[2] - r];
    let seg = segments.max(1) as usize;
    // Stützstellen je Achse: dicht an den Kanten, ein großes Stück in der Mitte.
    let samples = |axis: usize| -> Vec<f32> {
        let mut v: Vec<f32> = (0..=seg).map(|k| -h[axis] + r * k as f32 / seg as f32).collect();
        v.extend((0..=seg).map(|k| inner[axis] + r * k as f32 / seg as f32));
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        v
    };
    let mut m = MeshData::default();
    // Sechs Seiten: (feste Achse, Vorzeichen, Achse u, Achse v)
    for (fixed, sign) in [(0, 1.0), (0, -1.0), (1, 1.0), (1, -1.0), (2, 1.0), (2, -1.0)] {
        let (ua, va) = match fixed {
            0 => (2, 1),
            1 => (0, 2),
            _ => (0, 1),
        };
        let us = samples(ua);
        let vs = samples(va);
        let start = m.positions.len() as u32;
        for (j, &v) in vs.iter().enumerate() {
            for (i, &u) in us.iter().enumerate() {
                let mut p = [0.0; 3];
                p[fixed] = sign * h[fixed];
                p[ua] = u;
                p[va] = v;
                let c = [
                    p[0].clamp(-inner[0], inner[0]),
                    p[1].clamp(-inner[1], inner[1]),
                    p[2].clamp(-inner[2], inner[2]),
                ];
                let n = norm(sub(p, c));
                let pos = if r > 0.0 { add(c, mul(n, r)) } else { p };
                // UV in Metern, damit Muster auf allen Größen gleich groß wirken.
                let _ = (i, j);
                let uv = [u + h[ua], v + h[va]];
                push_vertex(&mut m, pos, n, uv);
            }
        }
        let w = us.len() as u32;
        for j in 0..vs.len() as u32 - 1 {
            for i in 0..w - 1 {
                let a = start + j * w + i;
                tri_indexed(&mut m, a, a + 1, a + w);
                tri_indexed(&mut m, a + 1, a + w + 1, a + w);
            }
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Prisma: Grundlage für Extrusion und Schrift
// ---------------------------------------------------------------------------

fn signed_area(ring: &[[f32; 2]]) -> f32 {
    let mut a = 0.0;
    for i in 0..ring.len() {
        let p = ring[i];
        let q = ring[(i + 1) % ring.len()];
        a += p[0] * q[1] - q[0] * p[1];
    }
    a / 2.0
}

/// Entfernt doppelte aufeinanderfolgende Punkte und einen wiederholten Endpunkt.
fn clean_ring(ring: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut out: Vec<[f32; 2]> = Vec::with_capacity(ring.len());
    for p in ring {
        if out.last().is_none_or(|q| (q[0] - p[0]).abs() > 1e-7 || (q[1] - p[1]).abs() > 1e-7) {
            out.push(*p);
        }
    }
    while out.len() > 1 {
        let (f, l) = (out[0], out[out.len() - 1]);
        if (f[0] - l[0]).abs() < 1e-7 && (f[1] - l[1]).abs() < 1e-7 {
            out.pop();
        } else {
            break;
        }
    }
    out
}

/// Zieht eine Fläche (Umriss + Löcher in der u/v-Ebene) entlang w von `w0`
/// bis `w1`. `map` bildet (u, v, w) auf 3D ab; `top_scale` skaliert die
/// Oberseite um den Ursprung. Seitennormalen sind flach (scharfe Kanten).
#[allow(clippy::too_many_arguments)]
fn prism(
    m: &mut MeshData,
    outer: &[[f32; 2]],
    holes: &[Vec<[f32; 2]>],
    w0: f32,
    w1: f32,
    top_scale: f32,
    map: &dyn Fn(f32, f32, f32) -> V3,
    up: V3,
) {
    let outer = clean_ring(outer);
    let holes: Vec<Vec<[f32; 2]>> = holes.iter().map(|h| clean_ring(h)).filter(|h| h.len() >= 3).collect();
    if outer.len() < 3 {
        return;
    }
    // Deckel triangulieren (earcut kann Löcher)
    let mut flat: Vec<f64> = outer.iter().flat_map(|p| [p[0] as f64, p[1] as f64]).collect();
    let mut hole_idx = Vec::new();
    let mut all: Vec<[f32; 2]> = outer.clone();
    for h in &holes {
        hole_idx.push(all.len());
        flat.extend(h.iter().flat_map(|p| [p[0] as f64, p[1] as f64]));
        all.extend(h.iter().copied());
    }
    let tris = earcutr::earcut(&flat, &hole_idx, 2).unwrap_or_default();
    let (min, max) = all.iter().fold(([f32::MAX; 2], [f32::MIN; 2]), |(lo, hi), p| {
        ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
    });
    let _ = max;
    let cap_uv = |p: [f32; 2]| [p[0] - min[0], p[1] - min[1]];
    let down = mul(up, -1.0);
    let has_top = top_scale > 1e-4;
    for t in tris.chunks(3) {
        let pts = [all[t[0]], all[t[1]], all[t[2]]];
        let uvs = [cap_uv(pts[0]), cap_uv(pts[1]), cap_uv(pts[2])];
        push_tri(m, pts.map(|p| map(p[0], p[1], w0)), uvs, down);
        if has_top {
            push_tri(m, pts.map(|p| map(p[0] * top_scale, p[1] * top_scale, w1)), uvs, up);
        }
    }
    // Flache Form (z. B. flache Schrift): keine Seitenwände.
    if (w1 - w0).abs() < 1e-7 {
        return;
    }
    // Seitenwände
    let mut rings: Vec<(&[[f32; 2]], bool)> = vec![(&outer, false)];
    rings.extend(holes.iter().map(|h| (h.as_slice(), true)));
    for (ring, is_hole) in rings {
        let ccw = signed_area(ring) > 0.0;
        // Normale zeigt vom Material weg: außen beim Umriss, ins Loch hinein bei Löchern.
        let sign = if ccw != is_hole { 1.0 } else { -1.0 };
        let mut along = 0.0;
        let wall = (w1 - w0).abs();
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let (du, dv) = (b[0] - a[0], b[1] - a[1]);
            let seg_len = (du * du + dv * dv).sqrt();
            let out2 = [sign * dv, -sign * du];
            let outward = sub(map(out2[0], out2[1], 0.0), map(0.0, 0.0, 0.0));
            let (u0, u1) = (along, along + seg_len);
            along += seg_len;
            let a0 = map(a[0], a[1], w0);
            let b0 = map(b[0], b[1], w0);
            let a1 = map(a[0] * top_scale, a[1] * top_scale, w1);
            let b1 = map(b[0] * top_scale, b[1] * top_scale, w1);
            push_tri(m, [a0, b0, b1], [[u0, 0.0], [u1, 0.0], [u1, wall]], outward);
            if has_top {
                push_tri(m, [a0, b1, a1], [[u0, 0.0], [u1, wall], [u0, wall]], outward);
            }
        }
    }
}

pub fn extrude(spec: &ExtrudeSpec) -> MeshData {
    let mut m = MeshData::default();
    prism(
        &mut m,
        &spec.profile,
        &spec.holes,
        0.0,
        spec.height,
        spec.taper,
        &|u, v, w| [u, w, v],
        [0.0, 1.0, 0.0],
    );
    m
}

// ---------------------------------------------------------------------------
// Drehkörper
// ---------------------------------------------------------------------------

pub fn lathe(spec: &LatheSpec) -> MeshData {
    let prof = &spec.profile;
    let n = prof.len();
    let mut m = MeshData::default();
    if n < 2 {
        return m;
    }
    let segs = spec.segments.max(3) as usize;
    // Immer segs+1 Spalten (bei 360° liegt die letzte auf der ersten – UV-Naht).
    let angle = spec.angle.min(360.0).to_radians();

    // 2D-Normalen je Profil-Abschnitt (zeigen nach außen, wenn von unten nach oben)
    let seg_normal: Vec<[f32; 2]> = (0..n - 1)
        .map(|i| {
            let (dr, dy) = (prof[i + 1][0] - prof[i][0], prof[i + 1][1] - prof[i][1]);
            let l = (dr * dr + dy * dy).sqrt().max(1e-9);
            [dy / l, -dr / l]
        })
        .collect();
    // Falls das Profil von oben nach unten angegeben wurde: Normalen umdrehen.
    let outward_sum: f32 = seg_normal.iter().zip(prof.windows(2)).map(|(nn, w)| nn[0] * (w[0][0] + w[1][0])).sum();
    let flip = if outward_sum < 0.0 { -1.0 } else { 1.0 };
    let seg_normal: Vec<[f32; 2]> = seg_normal.iter().map(|q| [q[0] * flip, q[1] * flip]).collect();

    let total_len: f32 = prof
        .windows(2)
        .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
        .sum::<f32>()
        .max(1e-6);
    let rmax = prof.iter().map(|q| q[0]).fold(0.0f32, f32::max).max(1e-3);
    let mut along = 0.0;
    for i in 0..n - 1 {
        // Weiche Übergänge nur bei flachen Knicken (< 35°), sonst scharfe Kante.
        let smooth_with = |j: Option<usize>| -> [f32; 2] {
            let own = seg_normal[i];
            match j {
                Some(j) => {
                    let o = seg_normal[j];
                    if own[0] * o[0] + own[1] * o[1] > 0.82 {
                        let s = [own[0] + o[0], own[1] + o[1]];
                        let l = (s[0] * s[0] + s[1] * s[1]).sqrt().max(1e-9);
                        [s[0] / l, s[1] / l]
                    } else {
                        own
                    }
                }
                None => own,
            }
        };
        let n_start = smooth_with(i.checked_sub(1));
        let n_end = smooth_with(if i + 1 < n - 1 { Some(i + 1) } else { None });
        let seg_len = ((prof[i + 1][0] - prof[i][0]).powi(2) + (prof[i + 1][1] - prof[i][1]).powi(2)).sqrt();
        let _ = total_len;
        let (v0, v1) = (along, along + seg_len);
        along += seg_len;
        let start = m.positions.len() as u32;
        for (p, nn, v) in [(prof[i], n_start, v0), (prof[i + 1], n_end, v1)] {
            for c in 0..=segs {
                let t = c as f32 / segs as f32;
                let a = t * angle;
                let (s, co) = a.sin_cos();
                push_vertex(&mut m, [p[0] * co, p[1], -p[0] * s], norm([nn[0] * co, nn[1], -nn[0] * s]), [t * angle * rmax, v]);
            }
        }
        let row = segs as u32 + 1;
        for c in 0..segs as u32 {
            let a = start + c;
            let b = start + c + 1;
            tri_indexed(&mut m, a, b, a + row);
            tri_indexed(&mut m, b, b + row, a + row);
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Röhre
// ---------------------------------------------------------------------------

fn catmull_rom(points: &[V3], closed: bool, per_segment: usize) -> Vec<V3> {
    let n = points.len();
    let get = |i: isize| -> V3 {
        if closed {
            points[i.rem_euclid(n as isize) as usize]
        } else {
            points[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let segs = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(segs * per_segment + 1);
    for s in 0..segs as isize {
        let (p0, p1, p2, p3) = (get(s - 1), get(s), get(s + 1), get(s + 2));
        for k in 0..per_segment {
            let t = k as f32 / per_segment as f32;
            let (t2, t3) = (t * t, t * t * t);
            let f = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * ((2.0 * b) + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3)
            };
            out.push([f(p0[0], p1[0], p2[0], p3[0]), f(p0[1], p1[1], p2[1], p3[1]), f(p0[2], p1[2], p2[2], p3[2])]);
        }
    }
    if !closed {
        out.push(points[n - 1]);
    }
    out
}

pub fn tube(spec: &TubeSpec) -> MeshData {
    tube_partial(spec, 1.0)
}

/// Schneidet einen Pfad nach `fraction` (0..1) seiner Länge ab.
fn cut_path(path: &[V3], closed: bool, fraction: f32) -> Vec<V3> {
    let mut pts: Vec<V3> = path.to_vec();
    if closed {
        pts.push(path[0]);
    }
    let total: f32 = pts.windows(2).map(|w| len(sub(w[1], w[0]))).sum();
    let target = total * fraction.clamp(0.0, 1.0);
    let mut out = vec![pts[0]];
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let l = len(sub(w[1], w[0]));
        if acc + l >= target {
            let t = if l > 1e-9 { (target - acc) / l } else { 0.0 };
            out.push(add(w[0], mul(sub(w[1], w[0]), t)));
            return out;
        }
        out.push(w[1]);
        acc += l;
    }
    out
}

/// Röhre, die nur bis `fraction` ihrer Länge gezeichnet ist (für „Linie zeichnet sich“).
pub fn tube_partial(spec: &TubeSpec, fraction: f32) -> MeshData {
    let mut m = MeshData::default();
    let mut pts: Vec<V3> = spec.path.clone();
    pts.dedup_by(|a, b| len(sub(*a, *b)) < 1e-6);
    if pts.len() < 2 {
        return m;
    }
    let closed = spec.closed && pts.len() >= 3;
    let path = if spec.smooth && pts.len() >= 3 { catmull_rom(&pts, closed, 10) } else { pts.clone() };
    let (path, closed) = if fraction < 0.999 { (cut_path(&path, closed, fraction), false) } else { (path, closed) };
    let mut path = path;
    path.dedup_by(|a, b| len(sub(*a, *b)) < 1e-6);
    if path.len() < 2 || fraction <= 0.0 {
        return m;
    }
    let count = path.len();
    let tangent = |i: usize| -> V3 {
        let (a, b) = if closed {
            (path[(i + count - 1) % count], path[(i + 1) % count])
        } else {
            (path[i.saturating_sub(1)], path[(i + 1).min(count - 1)])
        };
        norm(sub(b, a))
    };
    // Rotationsminimierende Rahmen (Parallel-Transport) gegen Verdrehen.
    let t0 = tangent(0);
    let helper = if t0[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let mut normal = norm(cross(cross(t0, helper), t0));
    let mut frames = Vec::with_capacity(count);
    let mut prev_t = t0;
    for i in 0..count {
        let t = tangent(i);
        let axis = cross(prev_t, t);
        if len(axis) > 1e-6 {
            let axis = norm(axis);
            let ang = dot(prev_t, t).clamp(-1.0, 1.0).acos();
            // Rodrigues-Rotation
            let (s, c) = ang.sin_cos();
            normal = add(
                add(mul(normal, c), mul(cross(axis, normal), s)),
                mul(axis, dot(axis, normal) * (1.0 - c)),
            );
        }
        normal = norm(sub(normal, mul(t, dot(normal, t))));
        let binormal = cross(t, normal);
        frames.push((normal, binormal));
        prev_t = t;
    }
    let r0 = spec.radius;
    let r1 = spec.radius_end.unwrap_or(spec.radius);
    let segs = spec.segments.max(3) as usize;
    let rows = if closed { count + 1 } else { count };
    // Länge für UV
    let mut acc = vec![0.0f32; rows];
    for i in 1..rows {
        acc[i] = acc[i - 1] + len(sub(path[i % count], path[i - 1]));
    }
    let total = acc[rows - 1].max(1e-6);
    let start = m.positions.len() as u32;
    for (i, &d) in acc.iter().enumerate() {
        let pi = i % count;
        let (nrm, bin) = frames[pi];
        let r = r0 + (r1 - r0) * (d / total);
        for k in 0..=segs {
            let a = k as f32 / segs as f32 * std::f32::consts::TAU;
            let dir = add(mul(nrm, a.cos()), mul(bin, a.sin()));
            push_vertex(&mut m, add(path[pi], mul(dir, r)), dir, [k as f32 / segs as f32 * std::f32::consts::TAU * r0, d]);
        }
    }
    let w = segs as u32 + 1;
    for i in 0..rows as u32 - 1 {
        for k in 0..segs as u32 {
            let a = start + i * w + k;
            tri_indexed(&mut m, a, a + 1, a + w);
            tri_indexed(&mut m, a + 1, a + w + 1, a + w);
        }
    }
    if !closed {
        // Deckel an beiden Enden
        for (end, rad) in [(0usize, r0), (count - 1, r1)] {
            if rad <= 1e-5 {
                continue;
            }
            let t = tangent(end);
            let facing = if end == 0 { mul(t, -1.0) } else { t };
            let (nrm, bin) = frames[end];
            let c = path[end];
            for k in 0..segs {
                let a0 = k as f32 / segs as f32 * std::f32::consts::TAU;
                let a1 = (k + 1) as f32 / segs as f32 * std::f32::consts::TAU;
                let p0 = add(c, mul(add(mul(nrm, a0.cos()), mul(bin, a0.sin())), rad));
                let p1 = add(c, mul(add(mul(nrm, a1.cos()), mul(bin, a1.sin())), rad));
                push_tri(&mut m, [c, p0, p1], [[0.5, 0.5], [0.5 + a0.cos() / 2.0, 0.5 + a0.sin() / 2.0], [0.5 + a1.cos() / 2.0, 0.5 + a1.sin() / 2.0]], facing);
            }
        }
    }
    m
}

// ---------------------------------------------------------------------------
// 3D-Schrift
// ---------------------------------------------------------------------------

struct Outline {
    contours: Vec<Vec<[f32; 2]>>,
    current: Vec<[f32; 2]>,
    scale: f32,
    offset: [f32; 2],
    /// Schrägstellung für Kursivschrift (x += y · shear).
    shear: f32,
}

impl Outline {
    fn pt(&self, x: f32, y: f32) -> [f32; 2] {
        let (lx, ly) = (x * self.scale, y * self.scale);
        [lx + ly * self.shear + self.offset[0], ly + self.offset[1]]
    }
}

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.current.len() >= 3 {
            self.contours.push(std::mem::take(&mut self.current));
        }
        self.current.clear();
        let p = self.pt(x, y);
        self.current.push(p);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.pt(x, y);
        self.current.push(p);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p0 = *self.current.last().unwrap_or(&[0.0, 0.0]);
        let (c, e) = (self.pt(x1, y1), self.pt(x, y));
        for k in 1..=6 {
            let t = k as f32 / 6.0;
            let u = 1.0 - t;
            self.current.push([
                u * u * p0[0] + 2.0 * u * t * c[0] + t * t * e[0],
                u * u * p0[1] + 2.0 * u * t * c[1] + t * t * e[1],
            ]);
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p0 = *self.current.last().unwrap_or(&[0.0, 0.0]);
        let (c1, c2, e) = (self.pt(x1, y1), self.pt(x2, y2), self.pt(x, y));
        for k in 1..=8 {
            let t = k as f32 / 8.0;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            self.current.push([
                a * p0[0] + b * c1[0] + c * c2[0] + d * e[0],
                a * p0[1] + b * c1[1] + c * c2[1] + d * e[1],
            ]);
        }
    }
    fn close(&mut self) {
        if self.current.len() >= 3 {
            self.contours.push(std::mem::take(&mut self.current));
        }
        self.current.clear();
    }
}

fn point_in_ring(p: [f32; 2], ring: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[j]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Erzeugt 3D-Schrift aus einer TrueType/OpenType-Schrift.
pub fn text(font_data: &[u8], spec: &TextSpec) -> Result<MeshData, String> {
    let face = ttf_parser::Face::parse(font_data, 0).map_err(|e| format!("Schrift nicht lesbar: {e}"))?;
    let scale = spec.size / face.units_per_em() as f32;
    let line_height = spec.size * 1.2;
    let spacing = spec.letter_spacing * spec.size;
    let mut m = MeshData::default();
    let lines: Vec<&str> = spec.text.lines().collect();
    for (li, line) in lines.iter().enumerate() {
        // Zeilenbreite für die Ausrichtung
        let glyphs: Vec<_> = line.chars().map(|c| face.glyph_index(c).or_else(|| face.glyph_index('?'))).collect();
        let advances: Vec<f32> = glyphs
            .iter()
            .map(|g| g.and_then(|g| face.glyph_hor_advance(g)).unwrap_or(0) as f32 * scale + spacing)
            .collect();
        let width: f32 = advances.iter().sum::<f32>() - if advances.is_empty() { 0.0 } else { spacing };
        let mut x = match spec.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => -width / 2.0,
            TextAlign::Right => -width,
        };
        let y = -(li as f32) * line_height;
        for (g, adv) in glyphs.iter().zip(&advances) {
            if let Some(g) = g {
                let shear = if spec.italic { 0.21 } else { 0.0 };
                let mut o = Outline { contours: vec![], current: vec![], scale, offset: [x, y], shear };
                if face.outline_glyph(*g, &mut o).is_some() {
                    use ttf_parser::OutlineBuilder;
                    o.close();
                    emit_glyph(&mut m, &o.contours, spec.depth);
                }
            }
            x += adv;
        }
    }
    Ok(m)
}

fn emit_glyph(m: &mut MeshData, contours: &[Vec<[f32; 2]>], depth: f32) {
    let contours: Vec<Vec<[f32; 2]>> = contours.iter().map(|c| clean_ring(c)).filter(|c| c.len() >= 3).collect();
    // Verschachtelungstiefe bestimmt: gerade = Umriss, ungerade = Loch.
    let depth_of = |i: usize| contours.iter().enumerate().filter(|(j, c)| *j != i && point_in_ring(contours[i][0], c)).count();
    let levels: Vec<usize> = (0..contours.len()).map(depth_of).collect();
    let (z0, z1) = (depth / 2.0, -depth / 2.0);
    for (i, outer) in contours.iter().enumerate() {
        if levels[i] % 2 == 1 {
            continue;
        }
        let holes: Vec<Vec<[f32; 2]>> = contours
            .iter()
            .enumerate()
            .filter(|(j, h)| levels[*j] == levels[i] + 1 && point_in_ring(h[0], outer))
            .map(|(_, h)| h.clone())
            .collect();
        if depth > 1e-6 {
            prism(m, outer, &holes, z0, z1, 1.0, &|u, v, w| [u, v, w], [0.0, 0.0, -1.0]);
        } else {
            // Flache Schrift: nur Vorderseite
            let mut tmp = MeshData::default();
            prism(&mut tmp, outer, &holes, 0.0, 0.0, 0.0, &|u, v, w| [u, v, w], [0.0, 0.0, -1.0]);
            let base = m.positions.len() as u32;
            m.positions.extend(tmp.positions);
            m.normals.extend(tmp.normals.iter().map(|n| mul(*n, 1.0)));
            m.uvs.extend(tmp.uvs);
            m.indices.extend(tmp.indices.iter().map(|i| i + base));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(m: &MeshData) {
        assert!(!m.indices.is_empty(), "leeres Mesh");
        assert_eq!(m.positions.len(), m.normals.len());
        assert_eq!(m.positions.len(), m.uvs.len());
        assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
        for n in &m.normals {
            assert!((len(*n) - 1.0).abs() < 1e-3, "Normale nicht normiert: {n:?}");
        }
        for p in &m.positions {
            assert!(p.iter().all(|v| v.is_finite()));
        }
    }

    /// Vorderseiten zeigen nach außen: Normale · (Flächenmitte − Schwerpunkt) > 0
    /// für (fast) alle Dreiecke eines konvexen Körpers.
    fn outward_ratio(m: &MeshData, center: V3) -> f32 {
        let mut good = 0;
        let tris = m.indices.chunks(3).count();
        for t in m.indices.chunks(3) {
            let (a, b, c) = (m.positions[t[0] as usize], m.positions[t[1] as usize], m.positions[t[2] as usize]);
            let n = cross(sub(b, a), sub(c, a));
            let mid = mul(add(add(a, b), c), 1.0 / 3.0);
            if dot(n, sub(mid, center)) > 0.0 {
                good += 1;
            }
        }
        good as f32 / tris as f32
    }

    #[test]
    fn rounded_box_has_size_and_faces_out() {
        let m = rounded_box([2.0, 1.0, 4.0], 0.2, 4);
        check(&m);
        let max = m.positions.iter().fold([0.0f32; 3], |acc, p| [acc[0].max(p[0].abs()), acc[1].max(p[1].abs()), acc[2].max(p[2].abs())]);
        assert!((max[0] - 1.0).abs() < 1e-4 && (max[1] - 0.5).abs() < 1e-4 && (max[2] - 2.0).abs() < 1e-4, "{max:?}");
        assert!(outward_ratio(&m, [0.0; 3]) > 0.999);
        // Radius 0 ergibt einen normalen Quader
        check(&rounded_box([1.0, 1.0, 1.0], 0.0, 2));
    }

    #[test]
    fn extrude_concave_with_hole_and_taper() {
        // L-förmiger Grundriss im Uhrzeigersinn, mit quadratischem Loch
        let spec = ExtrudeSpec {
            profile: vec![[0.0, 0.0], [0.0, 4.0], [1.0, 4.0], [1.0, 1.0], [3.0, 1.0], [3.0, 0.0]],
            holes: vec![],
            height: 2.0,
            taper: 1.0,
        };
        let m = extrude(&spec);
        check(&m);
        // Deckelfläche = 1*4 + 2*1 = 6 → Summe der Oberseiten-Dreiecke
        let top_area: f32 = m
            .indices
            .chunks(3)
            .filter(|t| m.normals[t[0] as usize][1] > 0.99)
            .map(|t| {
                let (a, b, c) = (m.positions[t[0] as usize], m.positions[t[1] as usize], m.positions[t[2] as usize]);
                len(cross(sub(b, a), sub(c, a))) / 2.0
            })
            .sum();
        assert!((top_area - 6.0).abs() < 1e-3, "Fläche {top_area}");
        let ring = ExtrudeSpec {
            profile: vec![[-2.0, -2.0], [2.0, -2.0], [2.0, 2.0], [-2.0, 2.0]],
            holes: vec![vec![[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]],
            height: 1.0,
            taper: 1.0,
        };
        let m = extrude(&ring);
        check(&m);
        let top_area: f32 = m
            .indices
            .chunks(3)
            .filter(|t| m.normals[t[0] as usize][1] > 0.99)
            .map(|t| {
                let (a, b, c) = (m.positions[t[0] as usize], m.positions[t[1] as usize], m.positions[t[2] as usize]);
                len(cross(sub(b, a), sub(c, a))) / 2.0
            })
            .sum();
        assert!((top_area - 12.0).abs() < 1e-3, "Ring-Fläche {top_area}");
        // Innenwände des Lochs zeigen zur Mitte (ins Loch)
        let inner_walls = m.indices.chunks(3).filter(|t| {
            let p = m.positions[t[0] as usize];
            let n = m.normals[t[0] as usize];
            n[1].abs() < 0.1 && p[0].abs() <= 1.0 + 1e-4 && p[2].abs() <= 1.0 + 1e-4
        });
        for t in inner_walls {
            let p = m.positions[t[0] as usize];
            let n = m.normals[t[0] as usize];
            assert!(n[0] * p[0] + n[2] * p[2] <= 1e-4, "Lochwand zeigt nach außen");
        }
        // Spitze (taper 0) → keine Oberseite, aber geschlossene Seiten
        let cone = extrude(&ExtrudeSpec { taper: 0.0, ..ring.clone() });
        check(&cone);
    }

    #[test]
    fn lathe_vase_and_partial() {
        let spec = LatheSpec {
            profile: vec![[0.0, 0.0], [1.0, 0.0], [1.2, 1.0], [0.6, 2.0], [0.8, 2.5]],
            segments: 32,
            angle: 360.0,
        };
        let m = lathe(&spec);
        check(&m);
        assert!(outward_ratio(&m, [0.0, 1.25, 0.0]) > 0.9);
        // Umgekehrte Reihenfolge ergibt dieselbe Ausrichtung
        let rev = LatheSpec { profile: spec.profile.iter().rev().cloned().collect(), ..spec.clone() };
        assert!(outward_ratio(&lathe(&rev), [0.0, 1.25, 0.0]) > 0.9);
        check(&lathe(&LatheSpec { angle: 180.0, ..spec }));
    }

    #[test]
    fn tube_partial_grows_with_fraction() {
        let spec = TubeSpec {
            path: vec![[0.0, 0.0, 0.0], [4.0, 0.0, 0.0]],
            radius: 0.1,
            radius_end: None,
            segments: 8,
            smooth: false,
            closed: false,
        };
        assert!(tube_partial(&spec, 0.0).indices.is_empty());
        let half = tube_partial(&spec, 0.5);
        check(&half);
        let maxx = half.positions.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!((maxx - 2.0).abs() < 1e-3, "halbe Länge: {maxx}");
    }

    #[test]
    fn tube_follows_path_and_tapers() {
        let spec = TubeSpec {
            path: vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0], [2.0, 3.0, 0.0], [4.0, 3.0, 1.0]],
            radius: 0.3,
            radius_end: Some(0.05),
            segments: 12,
            smooth: true,
            closed: false,
        };
        let m = tube(&spec);
        check(&m);
        // Start-Ring liegt um den ersten Punkt mit Radius 0.3
        let d = len(m.positions[0]);
        assert!((d - 0.3).abs() < 1e-3, "{d}");
        let ring = tube(&TubeSpec { closed: true, path: vec![[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [-1.0, 0.0, 0.0], [0.0, 0.0, -1.0]], ..spec });
        check(&ring);
    }

    #[test]
    fn text_with_holes_and_alignment() {
        let font = include_bytes!("../../../apps/studio/assets/fonts/Inter-Regular.ttf");
        let spec = TextSpec {
            text: "Hallo O".into(),
            size: 1.0,
            depth: 0.2,
            align: TextAlign::Center,
            letter_spacing: 0.0,
            weight: Default::default(),
            italic: false,
        };
        let m = text(font, &spec).unwrap();
        check(&m);
        let (minx, maxx) = m.positions.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
        assert!((minx + maxx).abs() < 0.15, "zentriert: {minx}..{maxx}");
        assert!(maxx - minx > 2.5 && maxx - minx < 5.0, "Breite {}", maxx - minx);
        // Das „O“ hat ein Loch: Punkt in der Mitte des O darf von keinem Vorderseiten-Dreieck bedeckt sein.
        let o_center_x = maxx - 0.37;
        let covered = m.indices.chunks(3).any(|t| {
            let ps: Vec<[f32; 2]> = t.iter().map(|&i| { let p = m.positions[i as usize]; [p[0], p[1]] }).collect();
            m.normals[t[0] as usize][2] > 0.99 && point_in_ring([o_center_x, 0.36], &ps)
        });
        assert!(!covered, "Loch im O fehlt");
        // Mehrzeilig + flach + kursiv
        check(&text(font, &TextSpec { text: "A\nB".into(), depth: 0.0, italic: true, ..spec.clone() }).unwrap());
        // Kursiv: Oberkanten sind nach rechts verschoben
        let upright = text(font, &TextSpec { text: "I".into(), ..spec.clone() }).unwrap();
        let slanted = text(font, &TextSpec { text: "I".into(), italic: true, ..spec }).unwrap();
        let top_x = |m: &MeshData| m.positions.iter().filter(|p| p[1] > 0.6).map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!(top_x(&slanted) > top_x(&upright) + 0.1, "kursiv");
        // Flache Schrift: nur Vorderseiten (Normalen ±Z), keine Dreiecke zum Ursprung
        let flat = text(font, &TextSpec { text: "1246".into(), depth: 0.0, size: 1.0, align: TextAlign::Left, letter_spacing: 0.0, weight: Default::default(), italic: false }).unwrap();
        check(&flat);
        assert!(flat.normals.iter().all(|n| n[2].abs() > 0.99), "flache Schrift hat Seitenwände");
        let minx = flat.positions.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
        assert!(minx > 0.0, "nichts links vom Textanfang (keine Fächer zum Ursprung): {minx}");
    }
}

#[cfg(test)]
mod glyph_check {
    /// Alle Sonderzeichen, die die App-Oberfläche benutzt, müssen in der Schrift vorhanden sein
    /// (sonst erscheinen leere Kästchen).
    #[test]
    fn ui_symbols_exist_in_font() {
        let font = include_bytes!("../../../apps/studio/assets/fonts/Inter-Regular.ttf");
        let face = ttf_parser::Face::parse(font, 0).unwrap();
        let missing: Vec<char> = "●▶✓·–←→„“…ÄÖÜäöüß×".chars().filter(|c| face.glyph_index(*c).is_none()).collect();
        assert!(missing.is_empty(), "Diese Zeichen fehlen in der Schrift: {missing:?}");
    }
}
