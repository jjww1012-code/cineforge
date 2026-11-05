//! Bewegung, die Filme leicht macht:
//! - Pfade: ein Objekt läuft eine weiche Kurve entlang (gleichmäßiges Tempo) und dreht
//!   sich in Laufrichtung – Figuren, Autos, Vögel.
//! - Welt-Position eines Objekts zu jedem Zeitpunkt (auch in animierten Gruppen).
//! - Kamera-Automatik: auf ein Objekt schauen (`target`), ihm folgen (`follow`), es
//!   umkreisen (`orbit`). Alles deterministisch – gleiches Bild bei jedem Export.

use serde::{Deserialize, Serialize};

use crate::animation::{camera_at, Ease};
use crate::model::{Kind, Object, Scene};

fn yes() -> bool {
    true
}

/// Bewegung entlang eines Pfades.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PathMotion {
    /// Punkte, durch die der Weg führt (mindestens 2), in Koordinaten des Elternobjekts.
    pub points: Vec<[f32; 3]>,
    /// Losgehen (s).
    #[serde(default)]
    pub start: f32,
    /// Ankommen (s).
    pub end: f32,
    /// Tempo-Verlauf: in_out (Standard: anfahren, bremsen), linear (gleichmäßig) …
    #[serde(default)]
    pub ease: Ease,
    /// In Laufrichtung drehen (Vorderseite = +Z).
    #[serde(default = "yes")]
    pub face: bool,
    /// Geschlossene Runde (zurück zum ersten Punkt).
    #[serde(default)]
    pub closed: bool,
}

const STEPS: usize = 24;

impl PathMotion {
    pub fn validate(&self, id: &str, p: &mut Vec<String>) {
        if self.points.len() < 2 {
            p.push(format!("\"{id}\" animate.path: mindestens 2 Punkte"));
        }
        if !(self.end > self.start) {
            p.push(format!("\"{id}\" animate.path: end muss nach start liegen"));
        }
    }

    fn control(&self) -> Vec<[f32; 3]> {
        let mut c = self.points.clone();
        if self.closed && c.len() >= 2 {
            c.push(c[0]);
        }
        c
    }

    /// Dicht abgetastete Kurve (Catmull-Rom durch alle Punkte).
    fn polyline(&self) -> Vec<[f32; 3]> {
        let c = self.control();
        let n = c.len();
        if n < 2 {
            return c;
        }
        let at = |i: isize| -> [f32; 3] {
            if self.closed {
                let m = (n - 1) as isize; // letzter Punkt = erster
                c[(i.rem_euclid(m)) as usize]
            } else {
                c[i.clamp(0, n as isize - 1) as usize]
            }
        };
        let mut out = Vec::with_capacity((n - 1) * STEPS + 1);
        for i in 0..n - 1 {
            let (p0, p1, p2, p3) = (at(i as isize - 1), at(i as isize), at(i as isize + 1), at(i as isize + 2));
            for s in 0..STEPS {
                let u = s as f32 / STEPS as f32;
                let (u2, u3) = (u * u, u * u * u);
                let f = |k: usize| {
                    0.5 * ((2.0 * p1[k])
                        + (-p0[k] + p2[k]) * u
                        + (2.0 * p0[k] - 5.0 * p1[k] + 4.0 * p2[k] - p3[k]) * u2
                        + (-p0[k] + 3.0 * p1[k] - 3.0 * p2[k] + p3[k]) * u3)
                };
                out.push([f(0), f(1), f(2)]);
            }
        }
        out.push(c[n - 1]);
        out
    }

    /// Position und Blickrichtung (Gier-Winkel in Grad) zum Zeitpunkt t.
    pub fn sample(&self, t: f32) -> ([f32; 3], f32) {
        let line = self.polyline();
        if line.len() < 2 {
            return (line.first().copied().unwrap_or([0.0; 3]), 0.0);
        }
        let mut cum = vec![0.0f32];
        for w in line.windows(2) {
            let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2) + (w[1][2] - w[0][2]).powi(2)).sqrt();
            cum.push(cum.last().copied().unwrap_or(0.0) + d);
        }
        let total = *cum.last().unwrap_or(&0.0);
        let raw = ((t - self.start) / (self.end - self.start).max(1e-4)).clamp(0.0, 1.0);
        let dist = self.ease.apply(raw).clamp(0.0, 1.0) * total;
        let i = cum.partition_point(|c| *c < dist).clamp(1, line.len() - 1);
        let (a, b) = (line[i - 1], line[i]);
        let seg = (cum[i] - cum[i - 1]).max(1e-6);
        let f = ((dist - cum[i - 1]) / seg).clamp(0.0, 1.0);
        let pos = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f];
        let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
        let yaw = if dx.abs() + dz.abs() > 1e-6 { dx.atan2(dz).to_degrees() } else { 0.0 };
        (pos, yaw)
    }

    /// Weglänge in Metern (z. B. für das Schritt-Tempo).
    pub fn length(&self) -> f32 {
        self.polyline()
            .windows(2)
            .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2) + (w[1][2] - w[0][2]).powi(2)).sqrt())
            .sum()
    }
}

// ---------------------------------------------------------------------------
// Welt-Position
// ---------------------------------------------------------------------------

type Mat3 = [[f32; 3]; 3];

fn mat_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut r = [[0.0; 3]; 3];
    for (i, row) in r.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    r
}

fn mat_vec(m: &Mat3, v: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
}

/// Drehung (Grad, Reihenfolge wie in der App: Y, dann X, dann Z) mal Skalierung.
fn rot_scale(rot: [f32; 3], scale: [f32; 3]) -> Mat3 {
    let [rx, ry, rz] = rot.map(f32::to_radians);
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();
    let ry_m = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rx_m = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let rz_m = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];
    let r = mat_mul(&mat_mul(&ry_m, &rx_m), &rz_m);
    let s = [[scale[0], 0.0, 0.0], [0.0, scale[1], 0.0], [0.0, 0.0, scale[2]]];
    mat_mul(&r, &s)
}

/// Lage eines Objekts (ohne Eltern) zum Zeitpunkt t: (Position, Drehung, Skalierung).
pub fn local_state(o: &Object, t: f32) -> ([f32; 3], [f32; 3], [f32; 3]) {
    match &o.animate {
        Some(a) => {
            let s = a.state_at(o.position, o.rotation, o.scale.to_array(), t, o.hidden);
            (s.position, s.rotation, s.scale)
        }
        None => (o.position, o.rotation, o.scale.to_array()),
    }
}

fn chain<'a>(objs: &'a [Object], id: &str, out: &mut Vec<&'a Object>) -> bool {
    for o in objs {
        out.push(o);
        if o.id == id {
            return true;
        }
        if let Kind::Group { children } = &o.kind {
            if chain(children, id, out) {
                return true;
            }
        }
        out.pop();
    }
    false
}

/// Welt-Position eines Objekts (auch in animierten Gruppen) zum Zeitpunkt t.
pub fn world_position(scene: &Scene, id: &str, t: f32) -> Option<[f32; 3]> {
    let mut path = Vec::new();
    if !chain(&scene.objects, id, &mut path) {
        return None;
    }
    let mut m: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut p = [0.0; 3];
    for o in path {
        let (lp, lr, ls) = local_state(o, t);
        let wp = mat_vec(&m, lp);
        p = [p[0] + wp[0], p[1] + wp[1], p[2] + wp[2]];
        m = mat_mul(&m, &rot_scale(lr, ls));
    }
    Some(p)
}

// ---------------------------------------------------------------------------
// Kamera-Automatik
// ---------------------------------------------------------------------------

fn default_follow_offset() -> [f32; 3] {
    [0.0, 2.0, 6.0]
}
fn default_smooth() -> f32 {
    0.4
}
fn default_radius() -> f32 {
    8.0
}
fn default_height() -> f32 {
    2.5
}
fn default_orbit_speed() -> f32 {
    20.0
}

/// Kamera folgt einem Objekt (z. B. einer laufenden Figur).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Follow {
    /// id des Objekts.
    pub target: String,
    /// Abstand zum Objekt in Weltkoordinaten (Standard: 2 m höher, 6 m dahinter in +Z).
    #[serde(default = "default_follow_offset")]
    pub offset: [f32; 3],
    /// Nachziehen in Sekunden (0 = starr, 0.4 = weich wie ein Kameramann).
    #[serde(default = "default_smooth")]
    pub smooth: f32,
}

/// Kamera kreist um ein Objekt oder einen Punkt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Orbit {
    /// id des Objekts (oder `center` angeben).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center: Option<[f32; 3]>,
    #[serde(default = "default_radius")]
    pub radius: f32,
    /// Kamerahöhe über dem Mittelpunkt.
    #[serde(default = "default_height")]
    pub height: f32,
    /// Grad pro Sekunde (negativ = andersherum).
    #[serde(default = "default_orbit_speed")]
    pub speed: f32,
    /// Startwinkel in Grad (0 = von +Z aus gesehen, also von vorne).
    #[serde(default)]
    pub start_angle: f32,
}

/// Einstellungsgröße wie im Film.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShotSize {
    /// Totale: Figur klein in der Umgebung.
    Wide,
    /// Halbtotale: ganze Figur.
    Full,
    /// Halbnah: Hüfte aufwärts.
    #[default]
    Medium,
    /// Nah: Brust aufwärts.
    Close,
    /// Großaufnahme: Gesicht.
    ExtremeClose,
}

fn d_frame_height() -> f32 {
    6.0
}

/// Kamera automatisch auf ein Objekt einstellen (statt Position auszurechnen).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    /// id des Objekts (meist eine Figur).
    pub target: String,
    #[serde(default)]
    pub shot: ShotSize,
    /// Blickwinkel um das Objekt in Grad: 0 = von vorne (+Z), 90 = von rechts, −30 = schräg links.
    #[serde(default)]
    pub angle: f32,
    /// Kamerahöhe als Winkel: 0 = Augenhöhe, 20 = leicht von oben, −15 = von unten (heldenhaft).
    #[serde(default)]
    pub tilt: f32,
    /// Höhe des Objekts in Metern (bei Figuren automatisch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    /// Kamerabewegung: Einstellung am Ende (Heranfahren, Wegfahren, Bogen, Kran). Ohne `start`/`end`
    /// läuft sie über die ganze Einstellung (bis zum nächsten Schnitt bzw. Szenenende).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<FrameTo>,
}

/// Ziel einer Kamerabewegung bei `frame`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct FrameTo {
    /// Einstellungsgröße am Ende (wide → close = Heranfahren).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot: Option<ShotSize>,
    /// Blickwinkel am Ende (Grad) – Kamera fährt im Bogen um das Objekt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<f32>,
    /// Kamerahöhe am Ende (Grad) – z. B. 40 = Kran nach oben.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tilt: Option<f32>,
    /// Zusätzlicher Zoom am Ende: 1.15 = 15 % näher (sanftes „Push-in“), 0.85 = weiter weg.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<f32>,
    #[serde(default)]
    pub ease: crate::animation::Ease,
}

fn shot_view(shot: ShotSize, h: f32) -> (f32, f32) {
    // (sichtbare Bildhöhe, Blickpunkt-Höhe) relativ zur Objekthöhe
    match shot {
        ShotSize::Wide => (h * 4.0, h * 0.5),
        ShotSize::Full => (h * 1.35, h * 0.5),
        ShotSize::Medium => (h * 0.95, h * 0.66),
        ShotSize::Close => (h * 0.74, h * 0.76),
        ShotSize::ExtremeClose => (h * 0.42, h * 0.8),
    }
}

fn find_obj<'a>(objs: &'a [Object], id: &str) -> Option<&'a Object> {
    objs.iter().find_map(|o| if o.id == id { Some(o) } else if let Kind::Group { children } = &o.kind { find_obj(children, id) } else { None })
}

/// Kamera (Position, Blickpunkt) für eine Einstellungsgröße; `span` = Zeitraum der Einstellung
/// (für Kamerabewegungen ohne eigene start/end).
fn framed(scene: &Scene, f: &Frame, fov: f32, t: f32, span: (f32, f32)) -> Option<([f32; 3], [f32; 3])> {
    let p = world_position(scene, &f.target, t)?;
    let obj = find_obj(&scene.objects, &f.target)?;
    let h = f.height.unwrap_or(match &obj.kind {
        Kind::Character(c) => c.height * obj.scale.to_array()[1],
        _ => d_frame_height().min(2.0) * obj.scale.to_array()[1],
    });
    let (mut view, mut look_y) = shot_view(f.shot, h);
    let (mut angle, mut tilt) = (f.angle, f.tilt);
    if let Some(to) = &f.to {
        let (a, b) = (to.start.unwrap_or(span.0), to.end.unwrap_or(span.1));
        let u = to.ease.apply(if b > a + 1e-4 { (t - a) / (b - a) } else { 1.0 });
        let lerp = |x: f32, y: f32| x + (y - x) * u;
        let (v1, l1) = shot_view(to.shot.unwrap_or(f.shot), h);
        view = lerp(view, v1 / to.zoom.unwrap_or(1.0).clamp(0.2, 5.0));
        look_y = lerp(look_y, l1);
        angle = lerp(angle, to.angle.unwrap_or(f.angle));
        tilt = lerp(tilt, to.tilt.unwrap_or(f.tilt));
    }
    let dist = (view / 2.0) / (fov.to_radians() / 2.0).tan().max(0.05);
    // Blickrichtung des Objekts (Gier) berücksichtigen: „von vorne“ heißt vor seinem Gesicht.
    let yaw = (facing_yaw(scene, obj, t) + angle).to_radians();
    let tilt = tilt.to_radians();
    let look = [p[0], p[1] + look_y, p[2]];
    let pos = [
        look[0] + dist * tilt.cos() * yaw.sin(),
        look[1] + dist * tilt.sin(),
        look[2] + dist * tilt.cos() * yaw.cos(),
    ];
    Some((pos, look))
}

/// Wohin eine Figur schaut, relativ zu ihrer eigenen Drehung (Grad, −80…80):
/// bei `look_at` dreht sie Kopf und Oberkörper zum Ziel (Stand am Szenenanfang).
pub fn look_yaw(scene: &Scene, obj: &Object) -> f32 {
    let Kind::Character(c) = &obj.kind else { return 0.0 };
    c.look_at
        .as_ref()
        .and_then(|target| {
            let me = world_position(scene, &obj.id, 0.0)?;
            let other = world_position(scene, target, 0.0)?;
            let (dx, dz) = (other[0] - me[0], other[2] - me[2]);
            if dx.abs() + dz.abs() < 1e-5 {
                return None;
            }
            let world = dx.atan2(dz).to_degrees();
            let (_, rot, _) = local_state(obj, 0.0);
            Some((((world - rot[1] + 540.0) % 360.0) - 180.0).clamp(-80.0, 80.0))
        })
        .unwrap_or(0.0)
}

/// Blickrichtung in der Welt (Gier in Grad): eigene Drehung plus Blick zum `look_at`-Ziel.
pub fn facing_yaw(scene: &Scene, obj: &Object, t: f32) -> f32 {
    let (_, rot, _) = local_state(obj, t);
    rot[1] + look_yaw(scene, obj)
}

/// Mittelwert der Objekt-Position im Zeitfenster [t − smooth, t] (deterministisches Nachziehen).
fn smoothed_position(scene: &Scene, id: &str, t: f32, smooth: f32) -> Option<[f32; 3]> {
    if smooth <= 0.0 {
        return world_position(scene, id, t);
    }
    let n = 12;
    let mut sum = [0.0; 3];
    for i in 0..n {
        let ti = t - smooth * (i as f32 / (n - 1) as f32);
        let p = world_position(scene, id, ti)?;
        sum = [sum[0] + p[0], sum[1] + p[1], sum[2] + p[2]];
    }
    Some(sum.map(|v| v / n as f32))
}

/// Harter Kamera-Schnitt innerhalb einer Szene: ab Zeitpunkt `t` gelten diese Einstellungen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CameraCut {
    pub t: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look_at: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fov: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow: Option<Follow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbit: Option<Orbit>,
}

/// Kamera (Position, Blickpunkt, Bildwinkel) zum Zeitpunkt t inkl. Schnitte, follow/orbit/frame/target.
pub fn camera_shot(scene: &Scene, t: f32) -> ([f32; 3], [f32; 3], f32) {
    let cut = scene.camera.cuts.iter().filter(|c| c.t <= t + 1e-4).max_by(|a, b| a.t.total_cmp(&b.t));
    // Zeitraum der aktuellen Einstellung: vom Schnitt bis zum nächsten Schnitt (bzw. Szenenende)
    let start = cut.map(|c| c.t).unwrap_or(0.0);
    let end = scene.camera.cuts.iter().map(|c| c.t).filter(|&x| x > start + 1e-4).fold(scene.duration, f32::min);
    let span = (start, end.max(start));
    let Some(cut) = cut else { return camera_shot_base(scene, &scene.camera, t, span) };
    // Einstellung des Schnitts ersetzt die Grund-Kamera (Kamerafahrt der Grund-Kamera gilt nicht mehr).
    let mut cam = scene.camera.clone();
    cam.animate = None;
    cam.cuts.clear();
    if cut.position.is_some() || cut.look_at.is_some() || cut.frame.is_some() || cut.follow.is_some() || cut.orbit.is_some() {
        cam.frame = None;
        cam.follow = None;
        cam.orbit = None;
        cam.target = None;
    }
    if let Some(p) = cut.position {
        cam.position = p;
    }
    if let Some(l) = cut.look_at {
        cam.look_at = l;
    }
    if let Some(f) = cut.fov {
        cam.fov = f;
    }
    cam.target = cut.target.clone().or(cam.target);
    cam.frame = cut.frame.clone().or(cam.frame);
    cam.follow = cut.follow.clone().or(cam.follow);
    cam.orbit = cut.orbit.clone().or(cam.orbit);
    camera_shot_base(scene, &cam, t, span)
}

fn camera_shot_base(scene: &Scene, cam: &crate::model::CameraSpec, t: f32, span: (f32, f32)) -> ([f32; 3], [f32; 3], f32) {
    let (mut pos, mut look, fov) = camera_at(cam, t);
    let offset = cam.target_offset;
    let add = |a: [f32; 3], b: [f32; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    if let Some(f) = &cam.follow {
        // Position zieht weich nach, der Blick bleibt genau auf dem Objekt (Figur bleibt im Bild).
        if let (Some(p), Some(now)) = (smoothed_position(scene, &f.target, t, f.smooth), world_position(scene, &f.target, t)) {
            pos = add(p, f.offset);
            look = add(now, offset);
        }
    }
    if let Some(o) = &cam.orbit {
        let center = match (&o.target, o.center) {
            (Some(id), _) => world_position(scene, id, t),
            (None, Some(c)) => Some(c),
            _ => None,
        };
        if let Some(c) = center {
            let a = (o.start_angle + o.speed * t).to_radians();
            pos = [c[0] + o.radius * a.sin(), c[1] + o.height, c[2] + o.radius * a.cos()];
            look = add(c, offset);
        }
    }
    if let Some(id) = &cam.target {
        if let Some(p) = world_position(scene, id, t) {
            look = add(p, offset);
        }
    }
    if let Some(f) = &cam.frame {
        if let Some((p, l)) = framed(scene, f, fov, t, span) {
            pos = p;
            look = l;
        }
    }
    (pos, look, fov)
}

/// Rechnerische Bildkontrolle einer Szene (ohne Rendern): Figuren und Tiere, die nie im Bild sind,
/// Aktionen oder Sätze außerhalb des Bildes, Kamera in einer Figur. Liefert verständliche Hinweise.
pub fn framing_report(scene: &Scene) -> Vec<String> {
    let mut out = Vec::new();
    if scene.camera.orthographic.is_some() || scene.duration <= 0.0 {
        return out;
    }
    // Darsteller sammeln: (id, Höhe, Handlungen (Name, von, bis), spricht (von, bis))
    struct Actor {
        id: String,
        h: f32,
        acts: Vec<(String, f32, f32)>,
        talk: Vec<(f32, f32)>,
    }
    let mut actors: Vec<Actor> = Vec::new();
    scene.walk(&mut |o| match &o.kind {
        Kind::Character(c) => {
            let mut acts = Vec::new();
            let mut talk = Vec::new();
            for a in &c.actions {
                let end = a.end.unwrap_or(a.start + 2.0);
                let name = serde_json::to_value(a.kind).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                if name == "talk" {
                    talk.push((a.start, end));
                } else if name != "idle" {
                    acts.push((name, a.start, end));
                }
            }
            actors.push(Actor { id: o.id.clone(), h: c.height * o.scale.to_array()[1], acts, talk });
        }
        Kind::Animal(a) => {
            let acts = a
                .actions
                .iter()
                .map(|x| (serde_json::to_value(x.kind).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default(), x.start, x.end.unwrap_or(x.start + 2.0)))
                .filter(|(n, _, _)| n != "idle")
                .collect();
            actors.push(Actor { id: o.id.clone(), h: a.size.unwrap_or(0.6) * 0.7 * o.scale.to_array()[1], acts, talk: vec![] });
        }
        _ => {}
    });
    if actors.is_empty() {
        return out;
    }
    let n = ((scene.duration * 4.0).ceil() as usize).clamp(4, 400);
    let times: Vec<f32> = (0..=n).map(|i| scene.duration * i as f32 / n as f32).collect();
    let cams: Vec<([f32; 3], [f32; 3], f32)> = times.iter().map(|&t| camera_shot(scene, t)).collect();
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let norm = |a: [f32; 3]| {
        let l = dot(a, a).sqrt().max(1e-6);
        [a[0] / l, a[1] / l, a[2] / l]
    };
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    for actor in &actors {
        let mut visible = vec![false; times.len()];
        let mut inside_at: Option<f32> = None;
        for (i, &t) in times.iter().enumerate() {
            let Some(p) = world_position(scene, &actor.id, t) else { continue };
            let c = [p[0], p[1] + actor.h * 0.55, p[2]];
            let (pos, look, fov) = cams[i];
            let f = norm(sub(look, pos));
            let r = norm(cross(f, [0.0, 1.0, 0.0]));
            let u = cross(r, f);
            let d = sub(c, pos);
            let z = dot(d, f);
            if dot(d, d).sqrt() < actor.h * 0.3 && inside_at.is_none() {
                inside_at = Some(t);
            }
            if z < 0.05 {
                continue;
            }
            let tv = (fov.to_radians() / 2.0).tan();
            let (x, y) = (dot(d, r) / z, dot(d, u) / z);
            visible[i] = x.abs() <= tv * 16.0 / 9.0 && y.abs() <= tv * 1.05;
        }
        let share = |a: f32, b: f32| {
            let idx: Vec<usize> = (0..times.len()).filter(|&i| times[i] >= a - 1e-3 && times[i] <= b + 1e-3).collect();
            if idx.is_empty() { 1.0 } else { idx.iter().filter(|&&i| visible[i]).count() as f32 / idx.len() as f32 }
        };
        if let Some(t) = inside_at {
            out.push(format!("Kamera steckt bei {t:.1} s (fast) in „{}“ – weiter weg stellen", actor.id));
        }
        if !visible.iter().any(|v| *v) {
            out.push(format!("„{}“ ist in der ganzen Szene nicht im Bild", actor.id));
            continue;
        }
        for (name, a, b) in &actor.acts {
            if share(*a, *b) < 0.5 {
                out.push(format!("„{}“ macht „{name}“ ({a:.1}–{b:.1} s) meist außerhalb des Bildes", actor.id));
            }
        }
        let (talk_total, talk_vis) = actor.talk.iter().fold((0.0, 0.0), |(tt, tv), (a, b)| (tt + (b - a), tv + (b - a) * share(*a, *b)));
        if talk_total > 0.5 && talk_vis / talk_total < 0.4 {
            out.push(format!("„{}“ spricht meist, ohne im Bild zu sein (nur {:.0} % sichtbar) – gewollt? Sonst Schnitt auf den Sprecher", actor.id, 100.0 * talk_vis / talk_total));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_report_finds_offscreen_actors() {
        let s = Scene::from_json(r#"{"duration": 4, "objects": [
            {"id": "max", "type": "character", "actions": [{"type": "wave", "start": 0, "end": 2}]},
            {"id": "weg", "type": "character", "position": [0, 0, 40], "actions": [{"type": "jump", "start": 1}]}],
            "camera": {"position": [0, 1, 5], "look_at": [0, 0.7, 0]}}"#).unwrap();
        let r = framing_report(&s);
        assert!(r.iter().any(|x| x.contains("„weg“ ist in der ganzen Szene nicht im Bild")), "{r:?}");
        assert!(!r.iter().any(|x| x.contains("„max“")), "max ist sichtbar: {r:?}");
        let close = Scene::from_json(r#"{"duration": 2, "objects": [{"id": "max", "type": "character"}], "camera": {"position": [0, 0.7, 0.1], "look_at": [0, 0.7, -5]}}"#).unwrap();
        assert!(framing_report(&close).iter().any(|x| x.contains("Kamera steckt")), "{:?}", framing_report(&close));
    }

    #[test]
    fn path_moves_evenly_and_faces_direction() {
        let p = PathMotion { points: vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]], start: 1.0, end: 3.0, ease: Ease::Linear, face: true, closed: false };
        assert_eq!(p.sample(0.0).0, [0.0, 0.0, 0.0]);
        let (mid, yaw) = p.sample(2.0);
        assert!((mid[0] - 5.0).abs() < 1e-3 && (yaw - 90.0).abs() < 1e-3, "{mid:?} {yaw}");
        assert!((p.sample(9.0).0[0] - 10.0).abs() < 1e-4);
        assert!((p.length() - 10.0).abs() < 1e-3);
    }

    #[test]
    fn world_position_in_animated_group_and_camera_follow() {
        let s = Scene::from_json(
            r#"{"objects": [{"id": "g", "type": "group", "position": [10, 0, 0], "rotation": [0, 90, 0],
                "children": [{"id": "held", "type": "sphere", "position": [0, 0, 2],
                  "animate": {"path": {"points": [[0,0,2],[0,0,6]], "start": 0, "end": 4, "ease": "linear"}}}]}],
               "camera": {"follow": {"target": "held", "offset": [0, 2, 5], "smooth": 0}, "target_offset": [0, 1, 0]}}"#,
        )
        .unwrap();
        // Gruppe um 90° gedreht: lokales +Z zeigt in Welt +X
        let p0 = world_position(&s, "held", 0.0).unwrap();
        assert!((p0[0] - 12.0).abs() < 1e-3 && p0[2].abs() < 1e-3, "{p0:?}");
        let p2 = world_position(&s, "held", 2.0).unwrap();
        assert!((p2[0] - 14.0).abs() < 1e-3, "{p2:?}");
        let (pos, look, _) = camera_shot(&s, 2.0);
        assert!((pos[0] - 14.0).abs() < 1e-3 && (pos[2] - 5.0).abs() < 1e-3 && (look[1] - 1.0).abs() < 1e-3, "{pos:?} {look:?}");
    }

    #[test]
    fn frame_close_up_is_near_the_face() {
        let s = Scene::from_json(r#"{"objects": [{"id": "max", "type": "character", "height": 1.2, "position": [2, 0, 0]}],
            "camera": {"fov": 40, "frame": {"target": "max", "shot": "close"}}}"#).unwrap();
        let (pos, look, _) = camera_shot(&s, 0.0);
        assert!((look[1] - 0.912).abs() < 0.01 && (look[0] - 2.0).abs() < 1e-4, "{look:?}");
        assert!(pos[2] > 0.5 && pos[2] < 1.5 && (pos[0] - 2.0).abs() < 1e-3, "nah vor dem Gesicht: {pos:?}");
        let wide = Scene::from_json(r#"{"objects": [{"id": "max", "type": "character"}], "camera": {"frame": {"target": "max", "shot": "wide", "angle": 90}}}"#).unwrap();
        let (p2, _, _) = camera_shot(&wide, 0.0);
        assert!(p2[0] > 3.0, "von rechts und weit weg: {p2:?}");
    }

    #[test]
    fn close_up_faces_the_look_at_direction() {
        // Max schaut nach rechts zu Lena → „von vorne“ heißt rechts von Max
        let s = Scene::from_json(r#"{"objects": [{"id": "max", "type": "character", "look_at": "lena"}, {"id": "lena", "type": "character", "position": [3, 0, 0]}],
            "camera": {"frame": {"target": "max", "shot": "close"}}}"#).unwrap();
        let (pos, _, _) = camera_shot(&s, 0.0);
        assert!(pos[0] > 0.5 && pos[2].abs() < 0.2, "Kamera vor Max' Gesicht (rechts): {pos:?}");
    }

    #[test]
    fn frame_to_pushes_in_and_arcs() {
        let s = Scene::from_json(r#"{"duration": 4, "objects": [{"id": "max", "type": "character"}],
            "camera": {"frame": {"target": "max", "shot": "wide", "to": {"shot": "close", "angle": 90}}}}"#).unwrap();
        let (p0, _, _) = camera_shot(&s, 0.0);
        let (p1, _, _) = camera_shot(&s, 4.0);
        let d = |p: [f32; 3]| (p[0] * p[0] + p[2] * p[2]).sqrt();
        assert!(d(p0) > 3.0 * d(p1), "fährt heran: {p0:?} → {p1:?}");
        assert!(p0[2] > 1.0 && p0[0].abs() < 1e-3, "Start von vorne: {p0:?}");
        assert!(p1[0] > 0.5 && p1[2].abs() < 0.05, "Ende von rechts (90°): {p1:?}");
        // in einem Schnitt: Bewegung bis zum nächsten Schnitt
        let c = Scene::from_json(r#"{"duration": 6, "objects": [{"id": "max", "type": "character"}],
            "camera": {"cuts": [{"t": 1, "frame": {"target": "max", "shot": "medium", "to": {"zoom": 1.5}}}, {"t": 3, "position": [0, 5, 20]}]}}"#).unwrap();
        let near = d(camera_shot(&c, 2.99).0);
        let far = d(camera_shot(&c, 1.0).0);
        assert!(near < far * 0.75, "Push-in bis zum Schnitt: {far} → {near}");
    }

    #[test]
    fn camera_cuts_switch_setups() {
        let s = Scene::from_json(r#"{"objects": [{"id": "a", "type": "character", "position": [-2, 0, 0]}, {"id": "b", "type": "character", "position": [2, 0, 0]}],
            "camera": {"position": [0, 2, 10], "look_at": [0, 1, 0],
              "cuts": [{"t": 2, "frame": {"target": "a", "shot": "close"}}, {"t": 4, "frame": {"target": "b", "shot": "close"}}, {"t": 6, "position": [0, 5, 20], "look_at": [0, 0, 0]}]}}"#).unwrap();
        assert_eq!(camera_shot(&s, 1.0).0, [0.0, 2.0, 10.0]);
        assert!((camera_shot(&s, 2.5).1[0] + 2.0).abs() < 1e-3, "Nah auf a");
        assert!((camera_shot(&s, 4.0).1[0] - 2.0).abs() < 1e-3, "Nah auf b");
        assert_eq!(camera_shot(&s, 7.0).0, [0.0, 5.0, 20.0]);
    }

    #[test]
    fn orbit_circles_target() {
        let s = Scene::from_json(r#"{"objects": [{"id": "a", "type": "box", "position": [1, 0, 1]}], "camera": {"orbit": {"target": "a", "radius": 5, "height": 2, "speed": 90}}}"#).unwrap();
        let (p0, l0, _) = camera_shot(&s, 0.0);
        assert!((p0[2] - 6.0).abs() < 1e-3 && (p0[1] - 2.0).abs() < 1e-3 && l0 == [1.0, 0.0, 1.0], "{p0:?}");
        let (p1, _, _) = camera_shot(&s, 1.0);
        assert!((p1[0] - 6.0).abs() < 1e-3, "nach 1 s um 90° weiter: {p1:?}");
    }
}
