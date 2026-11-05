//! Natur-Generatoren: Bäume in mehreren Arten (mit Wind) – damit Welten schnell
//! lebendig werden. Ergebnis ist eine normale Gruppe aus Grundformen.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::color::Rgb;
use crate::model::Object;
use crate::noise::Rng;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TreeKind {
    /// Laubbaum mit runder, wolkiger Krone.
    #[default]
    Oak,
    /// Nadelbaum aus Kegel-Schichten.
    Pine,
    /// Palme mit gebogenem Stamm und Wedeln.
    Palm,
    /// Birke: schlank, heller Stamm, lockere Krone.
    Birch,
    /// Cartoon-Baum: Kugelkrone auf geradem Stamm.
    Lollipop,
}

fn d_height() -> f32 {
    5.0
}
fn c_leaves() -> Rgb {
    Rgb::new(0.25, 0.58, 0.25)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TreeSpec {
    #[serde(default)]
    pub kind: TreeKind,
    /// Höhe in Metern.
    #[serde(default = "d_height")]
    pub height: f32,
    /// Blattfarbe (Herbst: "#d9822b", Kirschblüte: "#f4b6c8").
    #[serde(default = "c_leaves")]
    pub leaves: Rgb,
    /// Stammfarbe (Standard je Art).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trunk: Option<Rgb>,
    /// Wiegen im Wind (0 = still, 0.3 = leicht, 1 = Sturm).
    #[serde(default)]
    pub wind: f32,
    /// Variante.
    #[serde(default)]
    pub seed: u32,
    /// Facetten-Look (Low-Poly).
    #[serde(default)]
    pub low_poly: bool,
}

impl TreeSpec {
    pub fn validate(&self, id: &str, p: &mut Vec<String>) {
        if !(0.3..=100.0).contains(&self.height) {
            p.push(format!("\"{id}\": height zwischen 0.3 und 100 Metern"));
        }
        if !(0.0..=2.0).contains(&self.wind) {
            p.push(format!("\"{id}\": wind zwischen 0 und 2"));
        }
    }
}

/// Wind: sanftes Wiegen als Keyframes (Drehung um x und z), deterministisch je seed.
fn sway(strength: f32, seed: u32, duration: f32) -> Value {
    if strength <= 0.0 {
        return Value::Null;
    }
    let mut rng = Rng::new(seed ^ 0x77aa);
    let (ph1, ph2) = (rng.f32() * 6.28, rng.f32() * 6.28);
    let n = (duration * 6.0).ceil() as usize + 1;
    let keys: Vec<Value> = (0..n)
        .map(|i| {
            let t = i as f32 / 6.0;
            let a = strength * 3.0;
            let rz = a * ((t * 1.3 + ph1).sin() + 0.4 * (t * 2.9 + ph2).sin());
            let rx = a * 0.5 * (t * 1.1 + ph2).sin();
            json!({"t": t, "v": [rx, 0.0, rz], "ease": "linear"})
        })
        .collect();
    json!({"rotation": keys})
}

/// Baut den Baum als Gruppe (Kinder-ids `<id>_…`). `duration` = Szenenlänge (für den Wind).
pub fn build_tree(obj: &Object, spec: &TreeSpec, duration: f32) -> Object {
    let id = &obj.id;
    let h = spec.height;
    let mut rng = Rng::new(spec.seed.wrapping_mul(7919) ^ 0x51);
    let leaf = |c: Rgb| json!({"color": c.to_hex(), "roughness": 0.85, "flat": spec.low_poly});
    let shade = |c: Rgb, f: f32| Rgb::new(c.r * f, c.g * f, c.b * f);
    let mut parts: Vec<Value> = Vec::new();
    let trunk_default = match spec.kind {
        TreeKind::Birch => Rgb::new(0.92, 0.9, 0.85),
        TreeKind::Palm => Rgb::new(0.62, 0.48, 0.32),
        _ => Rgb::new(0.42, 0.27, 0.15),
    };
    let bark = json!({"color": spec.trunk.unwrap_or(trunk_default).to_hex(), "roughness": 0.9,
        "pattern": {"type": if spec.kind == TreeKind::Birch { "stripes" } else { "noise" }, "color2": shade(spec.trunk.unwrap_or(trunk_default), 0.7).to_hex(), "scale": 3.0, "bump": 0.4}});
    let crown_sub = if spec.low_poly { 1 } else { 2 };
    match spec.kind {
        TreeKind::Oak | TreeKind::Birch | TreeKind::Lollipop => {
            let slim = spec.kind == TreeKind::Birch;
            let trunk_h = h * if spec.kind == TreeKind::Lollipop { 0.5 } else { 0.55 };
            let r = h * if slim { 0.025 } else { 0.05 };
            parts.push(json!({"id": format!("{id}_stamm"), "type": "cylinder", "radius": r, "height": trunk_h,
                "position": [0.0, trunk_h / 2.0, 0.0], "subdivide": 2, "deform": [{"type": "taper", "scale": 0.65}],
                "material": bark}));
            if spec.kind == TreeKind::Lollipop {
                parts.push(json!({"id": format!("{id}_krone"), "type": "sphere", "radius": h * 0.3,
                    "position": [0.0, trunk_h + h * 0.22, 0.0], "material": leaf(spec.leaves)}));
            } else {
                // Äste
                for i in 0..if slim { 3 } else { 4 } {
                    let a = i as f32 * 2.399 + rng.f32();
                    let len = h * rng.range(0.18, 0.28);
                    let y0 = trunk_h * rng.range(0.6, 0.95);
                    let (dx, dz) = (a.cos(), a.sin());
                    parts.push(json!({"id": format!("{id}_ast_{i}"), "type": "tube", "radius": r * 0.45, "radius_end": r * 0.15,
                        "path": [[0.0, y0, 0.0], [dx * len * 0.5, y0 + len * 0.45, dz * len * 0.5], [dx * len, y0 + len * 0.7, dz * len]],
                        "material": bark}));
                }
                // Wolkige Krone aus mehreren verbeulten Kugeln
                let blobs = if slim { 5 } else { 7 };
                let crown_y = trunk_h + h * 0.18;
                let spread = h * if slim { 0.16 } else { 0.24 };
                for i in 0..blobs {
                    let a = i as f32 * 2.399 + rng.f32() * 0.5;
                    let ring = if i == 0 { 0.0 } else { spread * rng.range(0.55, 1.0) };
                    let rr = h * if slim { 0.13 } else { 0.19 } * rng.range(0.75, 1.1);
                    let c = shade(spec.leaves, rng.range(0.82, 1.12));
                    parts.push(json!({"id": format!("{id}_laub_{i}"), "type": "sphere", "radius": rr,
                        "position": [a.cos() * ring, crown_y + rng.range(-0.08, 0.12) * h + if i == 0 { h * 0.08 } else { 0.0 }, a.sin() * ring],
                        "subdivide": crown_sub, "deform": [{"type": "noise", "amount": rr * 0.22, "scale": 2.2 / rr.max(0.05), "octaves": 3, "seed": spec.seed + i}],
                        "material": leaf(c)}));
                }
            }
        }
        TreeKind::Pine => {
            let trunk_h = h * 0.25;
            parts.push(json!({"id": format!("{id}_stamm"), "type": "cylinder", "radius": h * 0.035, "height": trunk_h + h * 0.2,
                "position": [0.0, (trunk_h + h * 0.2) / 2.0, 0.0], "material": bark}));
            let layers = 4;
            for i in 0..layers {
                let f = i as f32 / layers as f32;
                let rr = h * (0.28 - 0.17 * f);
                let ch = h * (0.36 - 0.1 * f);
                let y = trunk_h + f * h * 0.55;
                parts.push(json!({"id": format!("{id}_nadeln_{i}"), "type": "cone", "radius": rr, "height": ch,
                    "position": [0.0, y + ch / 2.0, 0.0], "rotation": [0.0, i as f32 * 37.0, 0.0],
                    "subdivide": 1, "deform": [{"type": "noise", "amount": rr * 0.08, "scale": 3.0 / rr.max(0.05), "octaves": 2, "seed": spec.seed + i}],
                    "material": leaf(shade(spec.leaves, 0.75 + 0.08 * i as f32))}));
            }
        }
        TreeKind::Palm => {
            let lean = rng.range(-1.0, 1.0) * h * 0.12;
            let top = [lean, h * 0.92, h * 0.04];
            parts.push(json!({"id": format!("{id}_stamm"), "type": "tube", "radius": h * 0.035, "radius_end": h * 0.022, "segments": 10,
                "path": [[0.0, 0.0, 0.0], [lean * 0.3, h * 0.35, 0.0], [lean * 0.8, h * 0.7, h * 0.02], top],
                "material": bark}));
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU + rng.f32() * 0.3;
                let len = h * rng.range(0.36, 0.46);
                let (dx, dz) = (a.cos(), a.sin());
                let p0 = top;
                let p1 = [top[0] + dx * len * 0.5, top[1] + h * 0.06, top[2] + dz * len * 0.5];
                let p2 = [top[0] + dx * len, top[1] - h * 0.12, top[2] + dz * len];
                let p3 = [top[0] + dx * len * 1.25, top[1] - h * 0.3, top[2] + dz * len * 1.25];
                parts.push(json!({"id": format!("{id}_wedel_{i}"), "type": "tube", "radius": h * 0.07, "radius_end": h * 0.006,
                    "path": [p0, p1, p2, p3], "segments": 6,
                    "material": leaf(shade(spec.leaves, rng.range(0.85, 1.1)))}));
            }
            for i in 0..3 {
                let a = i as f32 * 2.1;
                parts.push(json!({"id": format!("{id}_nuss_{i}"), "type": "sphere", "radius": h * 0.03,
                    "position": [top[0] + a.cos() * h * 0.04, top[1] - h * 0.04, top[2] + a.sin() * h * 0.04],
                    "material": {"color": "#5a3d1e", "roughness": 0.6}}));
            }
        }
    }
    let mut root = json!({"id": format!("{id}_baum"), "type": "group", "children": parts});
    let wind = sway(spec.wind, spec.seed, duration.max(1.0));
    if !wind.is_null() {
        root["animate"] = wind;
    }
    serde_json::from_value(root).expect("Baum ist ein gültiges Objekt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Kind, Scene};

    #[test]
    fn all_tree_kinds_are_valid() {
        for kind in ["oak", "pine", "palm", "birch", "lollipop"] {
            let s = Scene::from_json(&format!(r#"{{"objects": [{{"id": "b", "type": "tree", "kind": "{kind}", "wind": 0.4, "seed": 3}}]}}"#)).unwrap();
            let Kind::Tree(spec) = &s.objects[0].kind else { panic!() };
            let t = build_tree(&s.objects[0], spec, 5.0);
            let probe = Scene { objects: vec![t], ..Scene::from_json("{}").unwrap() };
            assert!(probe.validate().is_empty(), "{kind}: {:?}", probe.validate());
        }
    }
}

// ---------------------------------------------------------------------------
// Wasser
// ---------------------------------------------------------------------------

fn d_water_size() -> [f32; 2] {
    [40.0, 40.0]
}
fn d_res() -> u32 {
    96
}
fn d_wave() -> f32 {
    0.15
}
fn d_wavelength() -> f32 {
    4.0
}
fn c_water() -> Rgb {
    Rgb::new(0.08, 0.36, 0.55)
}

/// Wasserfläche mit Wellen (See, Meer, Pool).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WaterSpec {
    #[serde(default = "d_water_size")]
    pub size: [f32; 2],
    /// Gitterpunkte pro Seite (mehr = feinere Wellen, 32–256).
    #[serde(default = "d_res")]
    pub resolution: u32,
    /// Wellenhöhe in Metern (0 = spiegelglatt).
    #[serde(default = "d_wave")]
    pub wave_height: f32,
    /// Wellenlänge in Metern.
    #[serde(default = "d_wavelength")]
    pub wavelength: f32,
    /// Tempo-Faktor.
    #[serde(default = "crate::nature::one")]
    pub speed: f32,
    #[serde(default = "c_water")]
    pub color: Rgb,
    /// Windrichtung in Grad (0 = Wellen laufen Richtung -Z).
    #[serde(default)]
    pub direction: f32,
}

pub(crate) fn one() -> f32 {
    1.0
}

impl WaterSpec {
    pub fn validate(&self, id: &str, p: &mut Vec<String>) {
        if !(4..=512).contains(&self.resolution) {
            p.push(format!("\"{id}\": resolution zwischen 4 und 512"));
        }
        if !(self.size[0] > 0.0 && self.size[1] > 0.0) || !(self.wavelength > 0.0) {
            p.push(format!("\"{id}\": size und wavelength müssen größer als 0 sein"));
        }
    }

    /// Höhe und Steigung (dh/dx, dh/dz) an (x, z) zur Zeit t – Summe aus drei Wellenzügen.
    pub fn height(&self, x: f32, z: f32, t: f32) -> (f32, f32, f32) {
        let base = self.direction.to_radians();
        let waves = [(0.0_f32, 1.0_f32, 1.0_f32), (0.6, 0.55, 0.45), (-0.9, 0.33, 0.3)];
        let (mut h, mut dx, mut dz) = (0.0, 0.0, 0.0);
        for (k, (ang, len_f, amp_f)) in waves.iter().enumerate() {
            let a = base + ang;
            let (dirx, dirz) = (a.sin(), -a.cos());
            let l = self.wavelength * len_f;
            let kk = std::f32::consts::TAU / l;
            let c = (9.81 / kk).sqrt() * self.speed; // Tiefwasser-Geschwindigkeit
            let amp = self.wave_height * 0.5 * amp_f;
            let ph = kk * (dirx * x + dirz * z) - kk * c * t + k as f32 * 1.7;
            h += amp * ph.sin();
            dx += amp * kk * dirx * ph.cos();
            dz += amp * kk * dirz * ph.cos();
        }
        (h, dx, dz)
    }
}

#[cfg(test)]
mod water_tests {
    use super::*;

    #[test]
    fn waves_move_and_stay_bounded() {
        let w = WaterSpec { size: [10.0, 10.0], resolution: 32, wave_height: 0.4, wavelength: 3.0, speed: 1.0, color: c_water(), direction: 0.0 };
        let (a, _, _) = w.height(1.0, 2.0, 0.0);
        let (b, _, _) = w.height(1.0, 2.0, 0.7);
        assert!((a - b).abs() > 1e-3, "Wellen bewegen sich");
        for i in 0..200 {
            let (h, _, _) = w.height(i as f32 * 0.37, i as f32 * -0.21, i as f32 * 0.1);
            assert!(h.abs() <= 0.4 * 0.5 * (1.0 + 0.45 + 0.3) + 1e-4);
        }
    }
}
