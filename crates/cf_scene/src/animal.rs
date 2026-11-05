//! Tier-Generator (Cartoon-Stil): Hund, Katze, Hase, Vogel, Fisch – mit Gelenken und
//! Bewegungen (laufen, wedeln, springen, sitzen, fliegen, schwimmen). Auf einem `path`
//! bewegen sich Tiere automatisch passend (laufen / fliegen / schwimmen).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::color::Rgb;
use crate::model::Object;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnimalKind {
    #[default]
    Dog,
    Cat,
    Bunny,
    Bird,
    Fish,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnimalAction {
    Idle,
    Walk,
    Run,
    Wag,
    Jump,
    Sit,
    Fly,
    Swim,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnimalActionSpec {
    #[serde(rename = "type")]
    pub kind: AnimalAction,
    #[serde(default)]
    pub start: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<f32>,
    #[serde(default = "one")]
    pub speed: f32,
}

fn one() -> f32 {
    1.0
}
fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnimalSpec {
    #[serde(default)]
    pub kind: AnimalKind,
    /// Größe in Metern (Körperlänge; Standard je Art).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    /// Fell-/Grundfarbe (Standard je Art).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
    /// Zweitfarbe (Bauch, Schnauze, Flecken).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color2: Option<Rgb>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<AnimalActionSpec>,
    #[serde(default)]
    pub outline: bool,
    /// Auf einem `path` automatisch laufen/fliegen/schwimmen.
    #[serde(default = "yes")]
    pub auto_move: bool,
}

impl AnimalSpec {
    pub fn validate(&self, id: &str, p: &mut Vec<String>) {
        if self.size.is_some_and(|s| !(0.05..=20.0).contains(&s)) {
            p.push(format!("\"{id}\": size zwischen 0.05 und 20 Metern"));
        }
        for a in &self.actions {
            if a.end.is_some_and(|e| e <= a.start) {
                p.push(format!("\"{id}\": Aktion {:?}: end muss nach start liegen", a.kind));
            }
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Pose {
    body_y: f32,
    body_pitch: f32,
    legs: [f32; 4], // vorne l, vorne r, hinten l, hinten r (Drehung um x)
    tail: [f32; 2], // [y (wedeln), x (heben)]
    head: [f32; 2], // [x, y]
    wings: f32,     // Flügelschlag (Drehung um z)
    sway: f32,      // Fisch: Körper-Schlängeln (y)
}

impl Pose {
    fn add(&mut self, o: &Pose, w: f32) {
        self.body_y += o.body_y * w;
        self.body_pitch += o.body_pitch * w;
        for k in 0..4 {
            self.legs[k] += o.legs[k] * w;
        }
        for k in 0..2 {
            self.tail[k] += o.tail[k] * w;
            self.head[k] += o.head[k] * w;
        }
        self.wings += o.wings * w;
        self.sway += o.sway * w;
    }
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn action_pose(kind: AnimalAction, lt: f32, speed: f32) -> Pose {
    use std::f32::consts::TAU;
    let mut p = Pose::default();
    match kind {
        AnimalAction::Idle => {
            p.tail = [8.0 * (lt * TAU * 0.5).sin(), 0.0];
            p.head = [3.0 * (lt * TAU * 0.3).sin(), 10.0 * (lt * TAU * 0.17).sin()];
        }
        AnimalAction::Walk | AnimalAction::Run => {
            let run = kind == AnimalAction::Run;
            let f = if run { 3.0 } else { 2.0 } * speed;
            let ph = lt * TAU * f / 2.0;
            let a = if run { 45.0 } else { 25.0 };
            // diagonales Gangbild
            p.legs = [a * ph.sin(), -a * ph.sin(), -a * ph.sin(), a * ph.sin()];
            p.body_y = if run { 0.06 } else { 0.02 } * (2.0 * ph).sin().abs();
            p.tail = [12.0 * (2.0 * ph).sin(), 15.0];
            p.head[0] = 3.0 * (2.0 * ph).sin();
        }
        AnimalAction::Wag => {
            p.tail = [35.0 * (lt * TAU * 4.0 * speed).sin(), 25.0];
            p.head = [-5.0, 6.0 * (lt * TAU * 1.0).sin()];
        }
        AnimalAction::Jump => {
            let u = (lt / (0.8 / speed)).clamp(0.0, 1.0);
            let air = if (0.2..0.85).contains(&u) { ((u - 0.2) / 0.65 * std::f32::consts::PI).sin() } else { 0.0 };
            p.body_y = 0.5 * air - if u < 0.2 { 0.05 * smoothstep(u / 0.2) } else { 0.0 };
            p.body_pitch = if u < 0.5 { -20.0 * air } else { 15.0 * air };
            p.legs = [-50.0 * air, -50.0 * air, 40.0 * air, 40.0 * air];
            p.tail[1] = 30.0 * air;
        }
        AnimalAction::Sit => {
            p.body_pitch = -28.0;
            p.body_y = -0.08;
            p.legs = [28.0, 28.0, -70.0, -70.0];
            p.tail = [10.0 * (lt * TAU * 0.8).sin(), -20.0];
        }
        AnimalAction::Fly => {
            p.wings = 55.0 * (lt * TAU * 3.0 * speed).sin();
            p.body_y = 0.06 * (lt * TAU * 3.0 * speed).sin();
            p.legs = [-60.0, -60.0, -60.0, -60.0];
        }
        AnimalAction::Swim => {
            let ph = lt * TAU * 1.5 * speed;
            p.tail = [30.0 * ph.sin(), 0.0];
            p.sway = 8.0 * (ph - 0.6).sin();
        }
    }
    p
}

fn total_pose(actions: &[(AnimalAction, f32, f32, f32)], t: f32) -> Pose {
    let mut pose = Pose::default();
    for &(kind, start, end, speed) in actions {
        if t < start - 0.3 || t > end + 0.3 {
            continue;
        }
        let ramp = 0.2_f32.min((end - start) / 3.0);
        let w = smoothstep((t - start) / ramp + 1.0).min(smoothstep((end - t) / ramp + 1.0));
        if w > 0.0 {
            pose.add(&action_pose(kind, (t - start).max(0.0), speed), w);
        }
    }
    pose
}

/// Baut das Tier als Gruppe (Kinder-ids `<id>_…`), Vorderseite +Z, unten y = 0.
pub fn build_animal(obj: &Object, spec: &AnimalSpec) -> Object {
    let id = &obj.id;
    let kind = spec.kind;
    let size = spec.size.unwrap_or(match kind {
        AnimalKind::Dog => 0.8,
        AnimalKind::Cat => 0.6,
        AnimalKind::Bunny => 0.4,
        AnimalKind::Bird => 0.3,
        AnimalKind::Fish => 0.4,
    });
    let u = size;
    let (c1, c2) = match kind {
        AnimalKind::Dog => (Rgb::new(0.78, 0.55, 0.32), Rgb::new(0.97, 0.92, 0.84)),
        AnimalKind::Cat => (Rgb::new(0.55, 0.55, 0.58), Rgb::new(0.95, 0.95, 0.95)),
        AnimalKind::Bunny => (Rgb::new(0.92, 0.9, 0.88), Rgb::new(1.0, 0.8, 0.85)),
        AnimalKind::Bird => (Rgb::new(0.25, 0.55, 0.95), Rgb::new(1.0, 0.85, 0.3)),
        AnimalKind::Fish => (Rgb::new(1.0, 0.55, 0.15), Rgb::new(1.0, 1.0, 1.0)),
    };
    let (c1, c2) = (spec.color.unwrap_or(c1), spec.color2.unwrap_or(c2));
    let ol = spec.outline.then(|| json!({"width": 0.012 * u}));
    let mat = |c: Rgb, r: f32| {
        let mut m = json!({"color": c.to_hex(), "roughness": r});
        if let Some(o) = &ol {
            m["outline"] = o.clone();
        }
        m
    };
    let fur = mat(c1, 0.8);
    let fur2 = mat(c2, 0.8);

    // Aktionen (+ automatisch passend zum Pfad)
    let mut acts: Vec<(AnimalAction, f32, f32, f32)> = spec.actions.iter().map(|a| (a.kind, a.start, a.end.unwrap_or(a.start + 2.0), a.speed)).collect();
    let moving = |k: &AnimalAction| matches!(k, AnimalAction::Walk | AnimalAction::Run | AnimalAction::Fly | AnimalAction::Swim);
    if spec.auto_move {
        if let Some(path) = obj.animate.as_ref().and_then(|a| a.path.as_ref()) {
            if !acts.iter().any(|(k, s, e, _)| moving(k) && *s < path.end && *e > path.start) {
                let v = path.length() / (path.end - path.start).max(0.1);
                let (k, sp) = match kind {
                    AnimalKind::Bird => (AnimalAction::Fly, 1.0),
                    AnimalKind::Fish => (AnimalAction::Swim, (v / (1.2 * u)).clamp(0.4, 3.0)),
                    _ if v > 2.2 * u => (AnimalAction::Run, (v / (2.5 * u)).clamp(0.4, 3.0)),
                    _ => (AnimalAction::Walk, (v / (1.0 * u)).clamp(0.3, 3.0)),
                };
                acts.push((k, path.start, path.end, sp));
            }
        }
    }
    // Grundbewegung: Vögel flattern, Fische schwimmen immer ein wenig, andere atmen/wedeln leicht
    let t_end = acts.iter().map(|a| a.2).fold(0.0_f32, f32::max).max(6.0) + 0.5;
    if !acts.iter().any(|(k, ..)| *k == AnimalAction::Idle) {
        acts.insert(0, (AnimalAction::Idle, 0.0, t_end, 1.0));
    }
    if kind == AnimalKind::Fish && !acts.iter().any(|(k, ..)| *k == AnimalAction::Swim) {
        acts.push((AnimalAction::Swim, 0.0, t_end, 0.6));
    }
    let fps = 15.0;
    let n = (t_end * fps).ceil() as usize + 1;
    let times: Vec<f32> = (0..n).map(|i| i as f32 / fps).collect();
    let poses: Vec<Pose> = times.iter().map(|&t| total_pose(&acts, t)).collect();
    let rot = |f: &dyn Fn(&Pose) -> [f32; 3]| -> Value {
        json!({"rotation": Value::Array(times.iter().zip(&poses).map(|(t, p)| json!({"t": t, "v": f(p), "ease": "linear"})).collect())})
    };
    let seed = id.bytes().fold(11u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32)) % 1000;
    let eye = |side: f32, pos: [f32; 3], r: f32| {
        json!({"id": format!("{id}_auge_{side}"), "type": "group", "position": pos, "animate": {"blink": {"every": 3.0, "seed": seed}},
          "children": [
            {"id": format!("{id}_augweiss_{side}"), "type": "sphere", "radius": r, "scale": [1.0, 1.1, 0.7], "material": {"color": "#ffffff", "roughness": 0.2}},
            {"id": format!("{id}_pupil_{side}"), "type": "sphere", "radius": r * 0.6, "position": [0.0, 0.0, r * 0.55], "scale": [1.0, 1.1, 0.5], "material": {"color": "#111111", "unlit": true}},
            {"id": format!("{id}_glanz_{side}"), "type": "sphere", "radius": r * 0.22, "position": [r * 0.25, r * 0.3, r * 0.8], "material": {"color": "#ffffff", "unlit": true}}]})
    };

    let mut parts: Vec<Value> = Vec::new();
    match kind {
        AnimalKind::Dog | AnimalKind::Cat | AnimalKind::Bunny => {
            let leg_h = u * if kind == AnimalKind::Bunny { 0.18 } else { 0.32 };
            let body_r = u * if kind == AnimalKind::Cat { 0.15 } else { 0.18 };
            let body_len = u * 0.45;
            let hip_y = leg_h + body_r * 0.6;
            let leg = |k: usize, x: f32, z: f32| {
                let f: Box<dyn Fn(&Pose) -> [f32; 3]> = Box::new(move |p: &Pose| [p.legs[k], 0.0, 0.0]);
                json!({"id": format!("{id}_bein_{k}"), "type": "group", "position": [x, 0.0, z], "animate": rot(&*f),
                  "children": [
                    {"id": format!("{id}_beinform_{k}"), "type": "capsule", "radius": u * 0.055, "length": (leg_h - u * 0.11).max(0.01), "position": [0.0, -leg_h / 2.0, 0.0], "material": fur},
                    {"id": format!("{id}_pfote_{k}"), "type": "sphere", "radius": u * 0.065, "position": [0.0, -leg_h + u * 0.03, u * 0.02], "scale": [1.0, 0.7, 1.2], "material": fur2}]})
            };
            let hx = body_r * 0.6;
            let hz = body_len * 0.5;
            let head_r = u * if kind == AnimalKind::Bunny { 0.2 } else { 0.17 };
            let mut head = vec![
                json!({"id": format!("{id}_kopfform"), "type": "sphere", "radius": head_r, "material": fur}),
                json!({"id": format!("{id}_schnauze"), "type": "sphere", "radius": head_r * 0.55, "position": [0.0, -head_r * 0.3, head_r * 0.75], "scale": [1.1, 0.8, 1.0], "material": fur2}),
                json!({"id": format!("{id}_nase"), "type": "sphere", "radius": head_r * 0.16, "position": [0.0, -head_r * 0.15, head_r * 1.25],
                       "material": {"color": if kind == AnimalKind::Dog { "#1a1a1a" } else { "#f08aa0" }, "roughness": 0.2}}),
                eye(-1.0, [-head_r * 0.42, head_r * 0.22, head_r * 0.78], head_r * 0.24),
                eye(1.0, [head_r * 0.42, head_r * 0.22, head_r * 0.78], head_r * 0.24),
            ];
            match kind {
                AnimalKind::Dog => {
                    for s in [-1.0f32, 1.0] {
                        head.push(json!({"id": format!("{id}_ohr_{s}"), "type": "capsule", "radius": head_r * 0.22, "length": head_r * 0.7,
                            "position": [s * head_r * 0.85, -head_r * 0.05, -head_r * 0.1], "rotation": [0.0, 0.0, s * 15.0], "scale": [0.6, 1.0, 1.0], "material": mat(Rgb::new(c1.r * 0.7, c1.g * 0.6, c1.b * 0.55), 0.8)}));
                    }
                }
                AnimalKind::Cat => {
                    for s in [-1.0f32, 1.0] {
                        head.push(json!({"id": format!("{id}_ohr_{s}"), "type": "cone", "radius": head_r * 0.32, "height": head_r * 0.6,
                            "position": [s * head_r * 0.55, head_r * 0.85, -head_r * 0.05], "rotation": [0.0, 0.0, -s * 15.0], "material": fur}));
                        for k in 0..2 {
                            head.push(json!({"id": format!("{id}_schnurrhaar_{s}_{k}"), "type": "tube", "radius": head_r * 0.012,
                                "path": [[s * head_r * 0.4, -head_r * (0.25 + 0.1 * k as f32), head_r * 1.05], [s * head_r * 1.2, -head_r * (0.2 + 0.15 * k as f32), head_r * 0.9]],
                                "material": {"color": "#222222"}}));
                        }
                    }
                }
                _ => {
                    for s in [-1.0f32, 1.0] {
                        head.push(json!({"id": format!("{id}_ohr_{s}"), "type": "capsule", "radius": head_r * 0.2, "length": head_r * 1.3,
                            "position": [s * head_r * 0.35, head_r * 1.45, -head_r * 0.1], "rotation": [-10.0, 0.0, -s * 10.0], "scale": [1.0, 1.0, 0.5], "material": fur}));
                    }
                }
            }
            let tail = match kind {
                AnimalKind::Bunny => json!({"id": format!("{id}_schwanzform"), "type": "sphere", "radius": u * 0.08, "position": [0.0, 0.0, -u * 0.04], "material": fur2}),
                AnimalKind::Cat => json!({"id": format!("{id}_schwanzform"), "type": "tube", "radius": u * 0.035, "radius_end": u * 0.03,
                    "path": [[0.0, 0.0, 0.0], [0.0, u * 0.15, -u * 0.15], [0.0, u * 0.4, -u * 0.12]], "material": fur}),
                _ => json!({"id": format!("{id}_schwanzform"), "type": "tube", "radius": u * 0.04, "radius_end": u * 0.015,
                    "path": [[0.0, 0.0, 0.0], [0.0, u * 0.1, -u * 0.12], [0.0, u * 0.22, -u * 0.18]], "material": fur}),
            };
            let body = json!({"id": format!("{id}_koerper"), "type": "group", "position": [0.0, hip_y, 0.0],
              "animate": {"position": Value::Array(times.iter().zip(&poses).map(|(t, p)| json!({"t": t, "v": [0.0, hip_y + p.body_y * u, 0.0], "ease": "linear"})).collect()),
                          "rotation": Value::Array(times.iter().zip(&poses).map(|(t, p)| json!({"t": t, "v": [p.body_pitch, 0.0, 0.0], "ease": "linear"})).collect())},
              "children": [
                {"id": format!("{id}_rumpf"), "type": "capsule", "radius": body_r, "length": body_len, "rotation": [90.0, 0.0, 0.0], "material": fur, "animate": {"breathe": 0.02}},
                {"id": format!("{id}_bauch"), "type": "sphere", "radius": body_r * 0.85, "position": [0.0, -body_r * 0.3, body_len * 0.2], "scale": [0.9, 0.8, 1.3], "material": fur2},
                {"id": format!("{id}_kopf"), "type": "group", "position": [0.0, body_r * 0.9 + head_r * 0.4, hz + head_r * 0.5], "animate": rot(&|p: &Pose| [p.head[0], p.head[1], 0.0]), "children": head},
                {"id": format!("{id}_schwanz"), "type": "group", "position": [0.0, body_r * 0.3, -hz - body_r * 0.7], "animate": rot(&|p: &Pose| [p.tail[1], p.tail[0], 0.0]), "children": [tail]},
                leg(0, -hx, hz * 0.8), leg(1, hx, hz * 0.8), leg(2, -hx, -hz * 0.8), leg(3, hx, -hz * 0.8)
              ]});
            let mut b = body;
            // Beine hängen am Körper: Beinlänge vom Körper aus nach unten
            if let Some(ch) = b["children"].as_array_mut() {
                for c in ch.iter_mut().filter(|c| c["id"].as_str().is_some_and(|s| s.contains("_bein_"))) {
                    let pos = c["position"].clone();
                    c["position"] = json!([pos[0], -body_r * 0.6 + 0.0, pos[2]]);
                }
            }
            parts.push(b);
        }
        AnimalKind::Bird => {
            let r = u * 0.3;
            let wing = |s: f32| {
                let f: Box<dyn Fn(&Pose) -> [f32; 3]> = Box::new(move |p: &Pose| [0.0, 0.0, -s * p.wings]);
                json!({"id": format!("{id}_fluegel_{s}"), "type": "group", "position": [s * r * 0.8, r * 0.1, 0.0], "animate": rot(&*f),
                  "children": [{"id": format!("{id}_fluegelform_{s}"), "type": "sphere", "radius": r * 0.7, "position": [s * r * 0.6, 0.0, -r * 0.1], "scale": [1.0, 0.18, 0.6], "material": fur}]})
            };
            let body = json!({"id": format!("{id}_koerper"), "type": "group", "position": [0.0, r * 1.4, 0.0],
              "animate": {"position": Value::Array(times.iter().zip(&poses).map(|(t, p)| json!({"t": t, "v": [0.0, r * 1.4 + p.body_y * u, 0.0], "ease": "linear"})).collect())},
              "children": [
                {"id": format!("{id}_rumpf"), "type": "sphere", "radius": r, "scale": [0.9, 0.9, 1.15], "material": fur},
                {"id": format!("{id}_brust"), "type": "sphere", "radius": r * 0.75, "position": [0.0, -r * 0.2, r * 0.35], "material": fur2},
                {"id": format!("{id}_kopf"), "type": "group", "position": [0.0, r * 0.75, r * 0.55], "animate": rot(&|p: &Pose| [p.head[0], p.head[1], 0.0]),
                  "children": [
                    {"id": format!("{id}_kopfform"), "type": "sphere", "radius": r * 0.6, "material": fur},
                    {"id": format!("{id}_schnabel"), "type": "cone", "radius": r * 0.18, "height": r * 0.45, "position": [0.0, -r * 0.05, r * 0.7], "rotation": [90.0, 0.0, 0.0], "material": {"color": "#ffb020", "roughness": 0.4}},
                    eye(-1.0, [-r * 0.28, r * 0.15, r * 0.45], r * 0.15), eye(1.0, [r * 0.28, r * 0.15, r * 0.45], r * 0.15)]},
                {"id": format!("{id}_schwanz"), "type": "cone", "radius": r * 0.35, "height": r * 0.8, "position": [0.0, r * 0.1, -r * 1.1], "rotation": [-110.0, 0.0, 0.0], "scale": [1.0, 1.0, 0.3], "material": fur},
                wing(-1.0), wing(1.0),
                {"id": format!("{id}_bein_l"), "type": "cylinder", "radius": r * 0.05, "height": r * 0.6, "position": [-r * 0.3, -r * 1.0, 0.0], "material": {"color": "#ffb020"}},
                {"id": format!("{id}_bein_r"), "type": "cylinder", "radius": r * 0.05, "height": r * 0.6, "position": [r * 0.3, -r * 1.0, 0.0], "material": {"color": "#ffb020"}}
              ]});
            parts.push(body);
        }
        AnimalKind::Fish => {
            let r = u * 0.3;
            let body = json!({"id": format!("{id}_koerper"), "type": "group", "position": [0.0, r * 1.2, 0.0], "animate": rot(&|p: &Pose| [0.0, p.sway, 0.0]),
              "children": [
                {"id": format!("{id}_rumpf"), "type": "sphere", "radius": r, "scale": [0.55, 0.9, 1.4], "material": fur},
                {"id": format!("{id}_streifen"), "type": "torus", "radius": r * 0.82, "thickness": r * 0.12, "position": [0.0, 0.0, -r * 0.2], "rotation": [0.0, 0.0, 0.0], "scale": [0.68, 1.1, 1.0], "material": fur2},
                {"id": format!("{id}_schwanz"), "type": "group", "position": [0.0, 0.0, -r * 1.3], "animate": rot(&|p: &Pose| [0.0, p.tail[0], 0.0]),
                  "children": [{"id": format!("{id}_flosse"), "type": "cone", "radius": r * 0.6, "height": r * 0.8, "position": [0.0, 0.0, -r * 0.35], "rotation": [-90.0, 0.0, 0.0], "scale": [0.25, 1.0, 1.0], "material": fur}]},
                {"id": format!("{id}_rueckenflosse"), "type": "cone", "radius": r * 0.35, "height": r * 0.5, "position": [0.0, r * 0.95, -r * 0.1], "scale": [0.2, 1.0, 1.0], "material": fur},
                eye(-1.0, [-r * 0.42, r * 0.25, r * 0.8], r * 0.2), eye(1.0, [r * 0.42, r * 0.25, r * 0.8], r * 0.2)
              ]});
            parts.push(body);
        }
    }
    let mut root = json!({"id": format!("{id}_tier"), "type": "group", "children": parts});
    strip_nulls(&mut root);
    serde_json::from_value(root).expect("Tier ist ein gültiges Objekt")
}

fn strip_nulls(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.retain(|_, x| !x.is_null());
            for x in m.values_mut() {
                strip_nulls(x);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(strip_nulls),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Kind, Scene};

    #[test]
    fn all_animals_build_valid_groups() {
        for kind in ["dog", "cat", "bunny", "bird", "fish"] {
            let s = Scene::from_json(&format!(
                r#"{{"objects": [{{"id": "tier", "type": "animal", "kind": "{kind}", "outline": true, "actions": [{{"type": "wag", "start": 1, "end": 3}}],
                    "animate": {{"path": {{"points": [[0,0,0],[3,0,0]], "start": 3, "end": 5}}}}}}]}}"#
            ))
            .unwrap();
            let Kind::Animal(spec) = &s.objects[0].kind else { panic!() };
            let built = build_animal(&s.objects[0], spec);
            let probe = Scene { objects: vec![built], ..Scene::from_json("{}").unwrap() };
            assert!(probe.validate().is_empty(), "{kind}: {:?}", probe.validate());
        }
    }
}
