//! Animation über die Zeit: Keyframes mit Bewegungskurven.
//!
//! Ein Keyframe `{"t": 2, "v": [0, 5, 0], "ease": "in_out"}` sagt: Zum
//! Zeitpunkt 2 s hat die Eigenschaft den Wert v; `ease` bestimmt, wie der Weg
//! vom vorherigen Keyframe hierher verläuft. Vor dem ersten Keyframe gilt
//! dessen Wert, nach dem letzten der Wert des letzten.

use serde::{Deserialize, Serialize};

use crate::color::Rgb;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Ease {
    Linear,
    /// Langsam anfahren
    In,
    /// Langsam abbremsen
    Out,
    /// Sanft anfahren und abbremsen (Standard, filmisch)
    #[default]
    InOut,
    /// Leicht über das Ziel hinaus und zurück (verspielt)
    OutBack,
    /// Abprallen am Ziel (wie ein Ball)
    OutBounce,
    /// Sprung ohne Übergang
    Step,
}

impl Ease {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,
            Ease::In => t * t * t,
            Ease::Out => 1.0 - (1.0 - t).powi(3),
            Ease::InOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Ease::OutBack => {
                let (c1, c3) = (1.70158, 2.70158);
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            Ease::OutBounce => {
                let (n1, d1) = (7.5625, 2.75);
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let t = t - 1.5 / d1;
                    n1 * t * t + 0.75
                } else if t < 2.5 / d1 {
                    let t = t - 2.25 / d1;
                    n1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / d1;
                    n1 * t * t + 0.984375
                }
            }
            Ease::Step => {
                if t >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Key<T> {
    pub t: f32,
    pub v: T,
    #[serde(default)]
    pub ease: Ease,
}

pub trait Lerp: Clone {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, o: &Self, t: f32) -> Self {
        self + (o - self) * t
    }
}

impl Lerp for [f32; 3] {
    fn lerp(&self, o: &Self, t: f32) -> Self {
        [self[0].lerp(&o[0], t), self[1].lerp(&o[1], t), self[2].lerp(&o[2], t)]
    }
}

impl Lerp for Rgb {
    fn lerp(&self, o: &Self, t: f32) -> Self {
        Rgb::new(self.r.lerp(&o.r, t), self.g.lerp(&o.g, t), self.b.lerp(&o.b, t))
    }
}

impl Lerp for bool {
    fn lerp(&self, o: &Self, t: f32) -> Self {
        if t >= 1.0 { *o } else { *self }
    }
}

/// Größe als Zahl oder [x, y, z].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ScaleValue {
    Uniform(f32),
    PerAxis([f32; 3]),
}

impl ScaleValue {
    pub fn to_array(self) -> [f32; 3] {
        match self {
            ScaleValue::Uniform(s) => [s; 3],
            ScaleValue::PerAxis(a) => a,
        }
    }
}

impl Lerp for ScaleValue {
    fn lerp(&self, o: &Self, t: f32) -> Self {
        ScaleValue::PerAxis(self.to_array().lerp(&o.to_array(), t))
    }
}

/// Wert einer Keyframe-Spur zum Zeitpunkt `time` (None bei leerer Spur).
pub fn sample<T: Lerp>(keys: &[Key<T>], time: f32) -> Option<T> {
    let first = keys.first()?;
    if time <= first.t {
        return Some(first.v.clone());
    }
    for w in keys.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if time < b.t {
            let span = (b.t - a.t).max(1e-6);
            let local = b.ease.apply((time - a.t) / span);
            return Some(a.v.lerp(&b.v, local));
        }
    }
    Some(keys.last().expect("nicht leer").v.clone())
}

/// Weicher Pfad (Catmull-Rom) durch alle Keyframe-Positionen; die Bewegungskurve
/// eines Keyframes bestimmt das Tempo auf dem Abschnitt dorthin.
pub fn sample_smooth(keys: &[Key<[f32; 3]>], time: f32) -> Option<[f32; 3]> {
    let n = keys.len();
    if n < 3 {
        return sample(keys, time);
    }
    if time <= keys[0].t {
        return Some(keys[0].v);
    }
    if time >= keys[n - 1].t {
        return Some(keys[n - 1].v);
    }
    let i = keys.windows(2).position(|w| time < w[1].t).unwrap_or(n - 2);
    let (a, b) = (&keys[i], &keys[i + 1]);
    let u = b.ease.apply((time - a.t) / (b.t - a.t).max(1e-6));
    let p0 = keys[i.saturating_sub(1)].v;
    let (p1, p2) = (a.v, b.v);
    let p3 = keys[(i + 2).min(n - 1)].v;
    let (u2, u3) = (u * u, u * u * u);
    let f = |k: usize| {
        0.5 * ((2.0 * p1[k])
            + (-p0[k] + p2[k]) * u
            + (2.0 * p0[k] - 5.0 * p1[k] + 4.0 * p2[k] - p3[k]) * u2
            + (-p0[k] + 3.0 * p1[k] - 3.0 * p2[k] + p3[k]) * u3)
    };
    Some([f(0), f(1), f(2)])
}

fn check_keys<T>(keys: &[Key<T>], name: &str, p: &mut Vec<String>) {
    for w in keys.windows(2) {
        if w[1].t < w[0].t {
            p.push(format!("{name}: Keyframes müssen nach Zeit sortiert sein ({} kommt nach {})", w[1].t, w[0].t));
            return;
        }
    }
    if keys.iter().any(|k| !k.t.is_finite() || k.t < 0.0) {
        p.push(format!("{name}: Zeiten müssen ≥ 0 sein"));
    }
}

/// Animation eines Objekts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Animation {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub position: Vec<Key<[f32; 3]>>,
    /// Drehung in Grad (pro Achse interpoliert, 0 → 360 = eine volle Umdrehung).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rotation: Vec<Key<[f32; 3]>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scale: Vec<Key<ScaleValue>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub color: Vec<Key<Rgb>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub emissive_strength: Vec<Key<f32>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub opacity: Vec<Key<f32>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub visible: Vec<Key<bool>>,
    /// Nur bei tube: gezeichneter Anteil 0..1 („Linie zeichnet sich“).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub draw: Vec<Key<f32>>,
    /// Lichtstärke (nur bei point_light), z. B. Blitz bei einer Explosion.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub intensity: Vec<Key<f32>>,
    /// Flackern der Lichtstärke (nur bei point_light): 0 = ruhig, 0.3 = Kerze/Feuer, 1 = stark.
    #[serde(skip_serializing_if = "is_zero")]
    pub flicker: f32,
    /// Dauerdrehung in Grad pro Sekunde (zusätzlich zu rotation).
    #[serde(skip_serializing_if = "is_zero3")]
    pub spin: [f32; 3],
    /// Position weich als Kurve durch alle Keyframes führen.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub smooth_path: bool,
    /// Blinzeln (für Augen): schließt sich von selbst in natürlichen Abständen (Höhe → 10 %).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blink: Option<Blink>,
    /// Atmen: sanftes Heben/Senken (Höhe ±amount, Breite gegengleich), z. B. 0.02.
    #[serde(skip_serializing_if = "is_zero")]
    pub breathe: f32,
    /// Lippen-Sync (für den Mund): öffnet sich mit der Lautstärke einer Audiodatei.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub talk: Option<Talk>,
}

/// Blinzeln.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Blink {
    /// Mittlerer Abstand in Sekunden.
    pub every: f32,
    /// Variante (verschiedene Figuren blinzeln zu verschiedenen Zeiten; beide Augen gleich lassen).
    pub seed: u32,
}

impl Default for Blink {
    fn default() -> Self {
        Self { every: 4.0, seed: 0 }
    }
}

/// Lippen-Sync aus einer Audiodatei.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Talk {
    /// Datei im Ordner Audio/ (z. B. von generate_voice).
    pub audio: String,
    /// Ab wann die Datei in dieser Szene läuft (s).
    #[serde(default)]
    pub start: f32,
    /// Höhe des geschlossenen Mundes (Anteil, Standard 0.15).
    #[serde(default = "talk_closed")]
    pub closed: f32,
    /// Verstärkung (leise Stimmen: 1.5–2).
    #[serde(default = "talk_gain")]
    pub gain: f32,
}

fn talk_closed() -> f32 {
    0.15
}
fn talk_gain() -> f32 {
    1.0
}

/// Wie weit der Mund offen ist (0..1) – aus der Lautstärke-Hüllkurve (`rate` Werte pro Sekunde).
pub fn mouth_open(envelope: &[f32], rate: f32, talk: &Talk, t: f32) -> f32 {
    let local = t - talk.start;
    if local < 0.0 || envelope.is_empty() {
        return 0.0;
    }
    let i = local * rate;
    let i0 = i.floor() as usize;
    if i0 + 1 >= envelope.len() {
        return 0.0;
    }
    let f = i - i0 as f32;
    let v = envelope[i0] * (1.0 - f) + envelope[i0 + 1] * f;
    (v * talk.gain).clamp(0.0, 1.0)
}

/// Lid-Faktor fürs Blinzeln (1 = offen, 0.1 = zu).
pub fn blink_factor(b: &Blink, t: f32) -> f32 {
    let every = b.every.max(0.5);
    let hash = |k: i64| {
        let mut x = (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (b.seed as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 31;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        ((x >> 40) as f32) / ((1u64 << 24) as f32)
    };
    let k = (t / every).floor() as i64;
    let mut f: f32 = 1.0;
    for j in k - 1..=k + 1 {
        let at = j as f32 * every + (hash(j) - 0.5) * 0.7 * every + every * 0.5;
        let d = t - at;
        // 0,06 s zu, 0,09 s auf
        let closed = if (0.0..0.06).contains(&d) {
            d / 0.06
        } else if (0.06..0.15).contains(&d) {
            1.0 - (d - 0.06) / 0.09
        } else {
            0.0
        };
        f = f.min(1.0 - 0.9 * closed);
    }
    f
}

fn is_zero3(v: &[f32; 3]) -> bool {
    *v == [0.0; 3]
}

/// Zustand eines Objekts zu einem Zeitpunkt.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectState {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub color: Option<Rgb>,
    pub emissive_strength: Option<f32>,
    pub opacity: Option<f32>,
    pub visible: bool,
    /// Faktor bzw. Wert für die Lichtstärke (None = unverändert).
    pub intensity: Option<f32>,
}

impl Animation {
    pub fn validate(&self, id: &str, p: &mut Vec<String>) {
        check_keys(&self.position, &format!("\"{id}\" animate.position"), p);
        check_keys(&self.rotation, &format!("\"{id}\" animate.rotation"), p);
        check_keys(&self.scale, &format!("\"{id}\" animate.scale"), p);
        check_keys(&self.color, &format!("\"{id}\" animate.color"), p);
        check_keys(&self.emissive_strength, &format!("\"{id}\" animate.emissive_strength"), p);
        check_keys(&self.opacity, &format!("\"{id}\" animate.opacity"), p);
        check_keys(&self.visible, &format!("\"{id}\" animate.visible"), p);
        check_keys(&self.intensity, &format!("\"{id}\" animate.intensity"), p);
        check_keys(&self.draw, &format!("\"{id}\" animate.draw"), p);
        if let Some(b) = &self.blink {
            if b.every < 0.5 {
                p.push(format!("\"{id}\" animate.blink.every: mindestens 0.5 Sekunden"));
            }
        }
        if !(-0.5..=0.5).contains(&self.breathe) {
            p.push(format!("\"{id}\" animate.breathe: zwischen -0.5 und 0.5 (0.02 = natürlich)"));
        }
        if let Some(tk) = &self.talk {
            if tk.audio.trim().is_empty() {
                p.push(format!("\"{id}\" animate.talk.audio fehlt"));
            }
            if !(0.0..=1.0).contains(&tk.closed) {
                p.push(format!("\"{id}\" animate.talk.closed: zwischen 0 und 1"));
            }
        }
        if self.opacity.iter().any(|k| !(0.0..=1.0).contains(&k.v)) {
            p.push(format!("\"{id}\" animate.opacity: Werte zwischen 0 und 1"));
        }
    }

    pub fn affects_material(&self) -> bool {
        !self.color.is_empty() || !self.emissive_strength.is_empty() || !self.opacity.is_empty()
    }

    pub fn last_key_time(&self) -> f32 {
        fn last<T>(k: &[Key<T>]) -> f32 {
            k.last().map(|k| k.t).unwrap_or(0.0)
        }
        last(&self.position)
            .max(last(&self.rotation))
            .max(last(&self.scale))
            .max(last(&self.color))
            .max(last(&self.emissive_strength))
            .max(last(&self.opacity))
            .max(last(&self.visible))
            .max(last(&self.intensity))
            .max(last(&self.draw))
    }

    /// Lichtstärke zum Zeitpunkt t (inkl. Flackern) ausgehend von `base`.
    pub fn light_intensity_at(&self, base: f32, t: f32) -> Option<f32> {
        if self.intensity.is_empty() && self.flicker == 0.0 {
            return None;
        }
        let v = sample(&self.intensity, t).unwrap_or(base);
        let f = self.flicker.clamp(0.0, 2.0);
        let n = 0.5 * (t * 13.1).sin() + 0.3 * (t * 7.3 + 1.7).sin() + 0.2 * (t * 23.7 + 0.4).sin();
        Some((v * (1.0 + f * 0.6 * n)).max(0.0))
    }

    /// Wendet die Animation auf die Grundwerte des Objekts an.
    pub fn state_at(&self, base_pos: [f32; 3], base_rot: [f32; 3], base_scale: [f32; 3], t: f32, hidden: bool) -> ObjectState {
        let position = if self.smooth_path { sample_smooth(&self.position, t) } else { sample(&self.position, t) }
            .unwrap_or(base_pos);
        let mut rotation = sample(&self.rotation, t).unwrap_or(base_rot);
        for (r, s) in rotation.iter_mut().zip(self.spin) {
            *r += s * t;
        }
        let mut scale = sample(&self.scale, t).map(ScaleValue::to_array).unwrap_or(base_scale);
        if self.breathe != 0.0 {
            let s = (t * std::f32::consts::TAU / 3.6).sin() * self.breathe;
            scale[1] *= 1.0 + s;
            scale[0] *= 1.0 - s * 0.5;
            scale[2] *= 1.0 - s * 0.5;
        }
        if let Some(b) = &self.blink {
            scale[1] *= blink_factor(b, t);
        }
        ObjectState {
            position,
            rotation,
            scale,
            color: sample(&self.color, t),
            emissive_strength: sample(&self.emissive_strength, t),
            opacity: sample(&self.opacity, t),
            visible: sample(&self.visible, t).unwrap_or(!hidden),
            intensity: None,
        }
    }
}

/// Kamerafahrt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct CameraAnimation {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub position: Vec<Key<[f32; 3]>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub look_at: Vec<Key<[f32; 3]>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fov: Vec<Key<f32>>,
    /// Weiche Kurve durch alle Kamera-Positionen (und Blickziele).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub smooth_path: bool,
    /// Leichtes Handkamera-Wackeln (0 = aus, 0.3 = dezent, 1 = stark).
    #[serde(skip_serializing_if = "is_zero")]
    pub shake: f32,
}

fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

impl CameraAnimation {
    pub fn validate(&self, p: &mut Vec<String>) {
        check_keys(&self.position, "camera.animate.position", p);
        check_keys(&self.look_at, "camera.animate.look_at", p);
        check_keys(&self.fov, "camera.animate.fov", p);
        if self.fov.iter().any(|k| !(5.0..=150.0).contains(&k.v)) {
            p.push("camera.animate.fov: Werte zwischen 5 und 150".into());
        }
    }

    pub fn last_key_time(&self) -> f32 {
        [self.position.last().map(|k| k.t), self.look_at.last().map(|k| k.t), self.fov.last().map(|k| k.t)]
            .into_iter()
            .flatten()
            .fold(0.0, f32::max)
    }
}

/// Kamera zu einem Zeitpunkt: (Position, Blickziel, Bildwinkel).
pub fn camera_at(cam: &crate::model::CameraSpec, t: f32) -> ([f32; 3], [f32; 3], f32) {
    let Some(a) = &cam.animate else { return (cam.position, cam.look_at, cam.fov) };
    let pick = |keys: &[Key<[f32; 3]>], base: [f32; 3]| {
        if a.smooth_path { sample_smooth(keys, t) } else { sample(keys, t) }.unwrap_or(base)
    };
    let mut pos = pick(&a.position, cam.position);
    let mut look = pick(&a.look_at, cam.look_at);
    if a.shake > 0.0 {
        // Deterministisches Wackeln aus überlagerten Sinuswellen.
        let s = a.shake * 0.08;
        let w = |f: f32, ph: f32| (t * f + ph).sin() + 0.5 * (t * f * 2.13 + ph * 1.7).sin();
        pos = [pos[0] + s * w(1.7, 0.3), pos[1] + s * w(2.3, 1.1), pos[2] + s * 0.5 * w(1.3, 2.0)];
        look = [look[0] + s * 2.0 * w(1.1, 0.7), look[1] + s * 2.0 * w(1.9, 2.4), look[2]];
    }
    (pos, look, sample(&a.fov, t).unwrap_or(cam.fov))
}

/// Animation der Umgebung (z. B. Sonnenuntergang im Zeitraffer).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct EnvironmentAnimation {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sun_elevation: Vec<Key<f32>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sun_azimuth: Vec<Key<f32>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sun_intensity: Vec<Key<f32>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exposure: Vec<Key<f32>>,
}

impl EnvironmentAnimation {
    pub fn validate(&self, p: &mut Vec<String>) {
        check_keys(&self.sun_elevation, "environment.animate.sun_elevation", p);
        check_keys(&self.sun_azimuth, "environment.animate.sun_azimuth", p);
        check_keys(&self.sun_intensity, "environment.animate.sun_intensity", p);
        check_keys(&self.exposure, "environment.animate.exposure", p);
    }

    pub fn is_empty(&self) -> bool {
        self.sun_elevation.is_empty() && self.sun_azimuth.is_empty() && self.sun_intensity.is_empty() && self.exposure.is_empty()
    }
}

#[cfg(test)]
mod life_tests {
    use super::*;

    #[test]
    fn blink_closes_briefly_and_deterministically() {
        let b = Blink { every: 3.0, seed: 4 };
        let samples: Vec<f32> = (0..3000).map(|i| blink_factor(&b, i as f32 * 0.01)).collect();
        let closed = samples.iter().filter(|f| **f < 0.5).count();
        assert!(closed > 5 && closed < 200, "selten und kurz zu: {closed}");
        assert!(samples.iter().all(|f| (0.1 - 1e-4..=1.0).contains(f)));
        assert_eq!(blink_factor(&b, 7.31), blink_factor(&b, 7.31));
        let other = Blink { every: 3.0, seed: 5 };
        assert!((0..3000).any(|i| (blink_factor(&b, i as f32 * 0.01) - blink_factor(&other, i as f32 * 0.01)).abs() > 0.3));
    }

    #[test]
    fn mouth_follows_envelope() {
        let talk = Talk { audio: "x.wav".into(), start: 1.0, closed: 0.15, gain: 2.0 };
        let env = vec![0.0, 0.5, 0.25, 0.0];
        assert_eq!(mouth_open(&env, 10.0, &talk, 0.5), 0.0);
        assert!((mouth_open(&env, 10.0, &talk, 1.1) - 1.0).abs() < 1e-5, "verstärkt und begrenzt");
        assert!((mouth_open(&env, 10.0, &talk, 1.15) - 0.75).abs() < 1e-5);
        assert_eq!(mouth_open(&env, 10.0, &talk, 9.0), 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k<T>(t: f32, v: T, ease: Ease) -> Key<T> {
        Key { t, v, ease }
    }

    #[test]
    fn eases_hit_endpoints() {
        for e in [Ease::Linear, Ease::In, Ease::Out, Ease::InOut, Ease::OutBack, Ease::OutBounce] {
            assert!(e.apply(0.0).abs() < 1e-5, "{e:?}");
            assert!((e.apply(1.0) - 1.0).abs() < 1e-4, "{e:?}");
        }
        assert!(Ease::OutBack.apply(0.7) > 1.0, "überschwingt");
        assert_eq!(Ease::Step.apply(0.99), 0.0);
        assert!((Ease::InOut.apply(0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn sampling_before_between_after() {
        let keys = vec![k(1.0, 0.0f32, Ease::Linear), k(3.0, 10.0, Ease::Linear), k(4.0, 0.0, Ease::Step)];
        assert_eq!(sample(&keys, 0.0), Some(0.0));
        assert_eq!(sample(&keys, 2.0), Some(5.0));
        assert_eq!(sample(&keys, 3.5), Some(10.0), "Step hält den alten Wert");
        assert_eq!(sample(&keys, 9.0), Some(0.0));
        assert_eq!(sample::<f32>(&[], 1.0), None);
    }

    #[test]
    fn smooth_path_passes_through_keys() {
        let keys = vec![
            k(0.0, [0.0, 0.0, 0.0], Ease::Linear),
            k(1.0, [10.0, 0.0, 0.0], Ease::Linear),
            k(2.0, [10.0, 0.0, 10.0], Ease::Linear),
            k(3.0, [0.0, 0.0, 10.0], Ease::Linear),
        ];
        for key in &keys {
            let p = sample_smooth(&keys, key.t).unwrap();
            assert!((p[0] - key.v[0]).abs() < 1e-4 && (p[2] - key.v[2]).abs() < 1e-4);
        }
        // Zwischen den Punkten weicht die Kurve von der Geraden ab (rundet die Ecke)
        let mid = sample_smooth(&keys, 1.5).unwrap();
        assert!(mid[0] > 10.0, "Kurve schwingt nach außen: {mid:?}");
    }

    #[test]
    fn object_state_with_spin_and_visibility() {
        let a: Animation = serde_json::from_str(
            r##"{"position":[{"t":0,"v":[0,0,0]},{"t":2,"v":[0,4,0],"ease":"linear"}],
                "spin":[0,90,0], "visible":[{"t":0,"v":false},{"t":1,"v":true}],
                "scale":[{"t":0,"v":0},{"t":1,"v":[1,2,1]}],
                "color":[{"t":0,"v":"#000000"},{"t":2,"v":"#ffffff","ease":"linear"}]}"##,
        )
        .unwrap();
        let s = a.state_at([9.0; 3], [0.0; 3], [1.0; 3], 1.0, false);
        assert_eq!(s.position, [0.0, 2.0, 0.0]);
        assert_eq!(s.rotation, [0.0, 90.0, 0.0]);
        assert_eq!(s.scale, [1.0, 2.0, 1.0]);
        assert!(s.visible);
        assert!((s.color.unwrap().r - 0.5).abs() < 1e-5);
        assert!(!a.state_at([0.0; 3], [0.0; 3], [1.0; 3], 0.5, false).visible);
        assert!(a.affects_material());
        assert_eq!(a.last_key_time(), 2.0);
        // Licht: Blitz + Flackern bleibt positiv und schwankt
        let l: Animation = serde_json::from_str(r#"{"intensity":[{"t":0,"v":0},{"t":0.1,"v":1000,"ease":"linear"}],"flicker":0.5}"#).unwrap();
        let vals: Vec<f32> = (0..20).map(|i| l.light_intensity_at(10.0, 0.2 + i as f32 * 0.05).unwrap()).collect();
        assert!(vals.iter().all(|v| *v >= 0.0));
        assert!(vals.iter().cloned().fold(f32::MIN, f32::max) - vals.iter().cloned().fold(f32::MAX, f32::min) > 50.0);
        assert!(Animation::default().light_intensity_at(5.0, 1.0).is_none());
        // Ohne Animation bleiben Grundwerte
        let none = Animation::default().state_at([1.0, 2.0, 3.0], [0.0; 3], [1.0; 3], 5.0, true);
        assert_eq!(none.position, [1.0, 2.0, 3.0]);
        assert!(!none.visible);
    }

    #[test]
    fn unsorted_keys_are_reported() {
        let a: Animation = serde_json::from_str(r#"{"position":[{"t":2,"v":[0,0,0]},{"t":1,"v":[1,1,1]}]}"#).unwrap();
        let mut p = vec![];
        a.validate("x", &mut p);
        assert!(p[0].contains("sortiert"), "{p:?}");
        assert!(serde_json::from_str::<Animation>(r#"{"position":[{"t":0,"v":[0,0,0],"ease":"wild"}]}"#).is_err());
    }
}
