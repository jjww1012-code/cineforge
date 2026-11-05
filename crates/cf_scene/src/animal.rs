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
    Cow,
    Pig,
    Sheep,
    Horse,
    Chicken,
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
        AnimalKind::Cow => 1.6,
        AnimalKind::Pig => 0.9,
        AnimalKind::Sheep => 0.9,
        AnimalKind::Horse => 1.8,
        AnimalKind::Chicken => 0.45,
    });
    let u = size;
    let (c1, c2) = match kind {
        AnimalKind::Dog => (Rgb::new(0.78, 0.55, 0.32), Rgb::new(0.97, 0.92, 0.84)),
        AnimalKind::Cat => (Rgb::new(0.55, 0.55, 0.58), Rgb::new(0.95, 0.95, 0.95)),
        AnimalKind::Bunny => (Rgb::new(0.92, 0.9, 0.88), Rgb::new(1.0, 0.8, 0.85)),
        AnimalKind::Bird => (Rgb::new(0.25, 0.55, 0.95), Rgb::new(1.0, 0.85, 0.3)),
        AnimalKind::Fish => (Rgb::new(1.0, 0.55, 0.15), Rgb::new(1.0, 1.0, 1.0)),
        AnimalKind::Cow => (Rgb::new(0.97, 0.96, 0.94), Rgb::new(0.12, 0.11, 0.11)),
        AnimalKind::Pig => (Rgb::new(0.98, 0.72, 0.72), Rgb::new(0.9, 0.52, 0.56)),
        AnimalKind::Sheep => (Rgb::new(0.97, 0.96, 0.92), Rgb::new(0.24, 0.21, 0.2)),
        AnimalKind::Horse => (Rgb::new(0.56, 0.34, 0.19), Rgb::new(0.17, 0.11, 0.08)),
        AnimalKind::Chicken => (Rgb::new(0.98, 0.97, 0.94), Rgb::new(0.95, 0.93, 0.88)),
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
        AnimalKind::Dog | AnimalKind::Cat | AnimalKind::Bunny | AnimalKind::Cow | AnimalKind::Pig | AnimalKind::Sheep | AnimalKind::Horse => {
            // Proportionen je Art: (Beinlänge, Körperradius, Körperlänge, Kopfradius) relativ zur Größe
            let (leg_f, body_f, len_f, head_f) = match kind {
                AnimalKind::Bunny => (0.18, 0.18, 0.45, 0.2),
                AnimalKind::Cat => (0.32, 0.15, 0.45, 0.17),
                AnimalKind::Cow => (0.3, 0.2, 0.55, 0.15),
                AnimalKind::Pig => (0.16, 0.22, 0.5, 0.16),
                AnimalKind::Sheep => (0.22, 0.2, 0.45, 0.13),
                AnimalKind::Horse => (0.42, 0.16, 0.55, 0.12),
                _ => (0.32, 0.18, 0.45, 0.17),
            };
            let farm = matches!(kind, AnimalKind::Cow | AnimalKind::Pig | AnimalKind::Sheep | AnimalKind::Horse);
            let leg_h = u * leg_f;
            let body_r = u * body_f;
            let body_len = u * len_f;
            let hip_y = leg_h + body_r * 0.6;
            let leg_r = u * if farm { 0.045 } else { 0.055 };
            let leg_mat = if kind == AnimalKind::Sheep { fur2.clone() } else { fur.clone() };
            let leg = |k: usize, x: f32, z: f32| {
                let f: Box<dyn Fn(&Pose) -> [f32; 3]> = Box::new(move |p: &Pose| [p.legs[k], 0.0, 0.0]);
                // Hufe (Bauernhoftiere) bzw. Pfoten
                let foot = if farm {
                    json!({"id": format!("{id}_huf_{k}"), "type": "cylinder", "radius": leg_r * 1.1, "height": u * 0.05, "position": [0.0, -leg_h + u * 0.025, 0.0], "material": fur2})
                } else {
                    json!({"id": format!("{id}_pfote_{k}"), "type": "sphere", "radius": u * 0.065, "position": [0.0, -leg_h + u * 0.03, u * 0.02], "scale": [1.0, 0.7, 1.2], "material": fur2})
                };
                json!({"id": format!("{id}_bein_{k}"), "type": "group", "position": [x, 0.0, z], "animate": rot(&*f),
                  "children": [
                    {"id": format!("{id}_beinform_{k}"), "type": "capsule", "radius": leg_r, "length": (leg_h - leg_r * 2.0).max(0.01), "position": [0.0, -leg_h / 2.0, 0.0], "material": leg_mat},
                    foot]})
            };
            let hx = body_r * 0.6;
            let hz = body_len * 0.5;
            let head_r = u * head_f;
            let head_mat = if kind == AnimalKind::Sheep { fur2.clone() } else { fur.clone() };
            let mut head = vec![
                json!({"id": format!("{id}_kopfform"), "type": "sphere", "radius": head_r, "scale": if kind == AnimalKind::Horse { [0.85, 0.95, 1.5] } else { [1.0, 1.0, 1.0] }, "material": head_mat}),
                eye(-1.0, [-head_r * 0.42, head_r * 0.22, head_r * 0.78], head_r * 0.24),
                eye(1.0, [head_r * 0.42, head_r * 0.22, head_r * 0.78], head_r * 0.24),
            ];
            if !farm {
                head.push(json!({"id": format!("{id}_schnauze"), "type": "sphere", "radius": head_r * 0.55, "position": [0.0, -head_r * 0.3, head_r * 0.75], "scale": [1.1, 0.8, 1.0], "material": fur2}));
                head.push(json!({"id": format!("{id}_nase"), "type": "sphere", "radius": head_r * 0.16, "position": [0.0, -head_r * 0.15, head_r * 1.25],
                       "material": {"color": if kind == AnimalKind::Dog { "#1a1a1a" } else { "#f08aa0" }, "roughness": 0.2}}));
            }
            let nostrils = |head: &mut Vec<Value>, y: f32, z: f32, dx: f32| {
                for s in [-1.0f32, 1.0] {
                    head.push(json!({"id": format!("{id}_nasenloch_{s}"), "type": "sphere", "radius": head_r * 0.07, "position": [s * dx, y, z], "material": {"color": "#3a2020", "unlit": true}}));
                }
            };
            match kind {
                AnimalKind::Cow => {
                    head.push(json!({"id": format!("{id}_maul"), "type": "sphere", "radius": head_r * 0.62, "position": [0.0, -head_r * 0.42, head_r * 0.78], "scale": [1.25, 0.85, 0.95], "material": mat(Rgb::new(0.96, 0.72, 0.72), 0.5)}));
                    nostrils(&mut head, -head_r * 0.35, head_r * 1.35, head_r * 0.25);
                    for s in [-1.0f32, 1.0] {
                        head.push(json!({"id": format!("{id}_horn_{s}"), "type": "cone", "radius": head_r * 0.12, "height": head_r * 0.45, "position": [s * head_r * 0.55, head_r * 0.85, -head_r * 0.1], "rotation": [0.0, 0.0, -s * 35.0], "material": {"color": "#efe3c4", "roughness": 0.4}}));
                        head.push(json!({"id": format!("{id}_ohr_{s}"), "type": "capsule", "radius": head_r * 0.16, "length": head_r * 0.5, "position": [s * head_r * 1.05, head_r * 0.35, -head_r * 0.15], "rotation": [0.0, 0.0, s * 75.0], "scale": [1.0, 1.0, 0.5], "material": fur}));
                    }
                }
                AnimalKind::Pig => {
                    head.push(json!({"id": format!("{id}_ruessel"), "type": "cylinder", "radius": head_r * 0.36, "height": head_r * 0.35, "position": [0.0, -head_r * 0.12, head_r * 1.0], "rotation": [90.0, 0.0, 0.0], "material": fur2}));
                    nostrils(&mut head, -head_r * 0.12, head_r * 1.18, head_r * 0.13);
                    for s in [-1.0f32, 1.0] {
                        head.push(json!({"id": format!("{id}_ohr_{s}"), "type": "cone", "radius": head_r * 0.3, "height": head_r * 0.5, "position": [s * head_r * 0.55, head_r * 0.8, head_r * 0.1], "rotation": [35.0, 0.0, -s * 20.0], "scale": [1.0, 1.0, 0.45], "material": fur}));
                    }
                }
                AnimalKind::Sheep => {
                    for (k, (x, y, z)) in [(0.0, 0.85, 0.0), (-0.35, 0.75, -0.1), (0.35, 0.75, -0.1), (0.0, 0.8, -0.35)].iter().enumerate() {
                        head.push(json!({"id": format!("{id}_lockenkopf_{k}"), "type": "sphere", "radius": head_r * 0.45, "position": [x * head_r, y * head_r, z * head_r], "material": fur}));
                    }
                    for s in [-1.0f32, 1.0] {
                        head.push(json!({"id": format!("{id}_ohr_{s}"), "type": "capsule", "radius": head_r * 0.18, "length": head_r * 0.55, "position": [s * head_r * 1.05, head_r * 0.15, -head_r * 0.1], "rotation": [0.0, 0.0, s * 80.0], "scale": [1.0, 1.0, 0.5], "material": fur2}));
                    }
                }
                AnimalKind::Horse => {
                    head.push(json!({"id": format!("{id}_maul"), "type": "sphere", "radius": head_r * 0.6, "position": [0.0, -head_r * 0.3, head_r * 1.15], "scale": [0.95, 0.85, 1.0], "material": mat(Rgb::new(c1.r * 0.8, c1.g * 0.75, c1.b * 0.7), 0.7)}));
                    nostrils(&mut head, -head_r * 0.3, head_r * 1.7, head_r * 0.22);
                    for s in [-1.0f32, 1.0] {
                        head.push(json!({"id": format!("{id}_ohr_{s}"), "type": "cone", "radius": head_r * 0.18, "height": head_r * 0.55, "position": [s * head_r * 0.4, head_r * 0.95, -head_r * 0.35], "rotation": [-10.0, 0.0, -s * 10.0], "material": fur}));
                    }
                }
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
                AnimalKind::Sheep => json!({"id": format!("{id}_schwanzform"), "type": "sphere", "radius": u * 0.07, "position": [0.0, 0.0, -u * 0.03], "material": fur}),
                AnimalKind::Pig => json!({"id": format!("{id}_schwanzform"), "type": "tube", "radius": u * 0.018,
                    "path": [[0.0, 0.0, 0.0], [0.0, u * 0.04, -u * 0.05], [u * 0.04, u * 0.08, -u * 0.04], [u * 0.02, u * 0.1, -u * 0.0], [-u * 0.02, u * 0.08, -u * 0.03]], "material": fur}),
                AnimalKind::Cow => json!({"id": format!("{id}_schwanzform"), "type": "group", "children": [
                    {"id": format!("{id}_schwanzseil"), "type": "tube", "radius": u * 0.015, "path": [[0.0, 0.0, 0.0], [0.0, -u * 0.12, -u * 0.06], [0.0, -u * 0.3, -u * 0.07]], "material": fur},
                    {"id": format!("{id}_schwanzquaste"), "type": "sphere", "radius": u * 0.035, "position": [0.0, -u * 0.32, -u * 0.07], "scale": [0.8, 1.4, 0.8], "material": fur2}]}),
                AnimalKind::Horse => json!({"id": format!("{id}_schwanzform"), "type": "tube", "radius": u * 0.045, "radius_end": u * 0.02,
                    "path": [[0.0, 0.0, 0.0], [0.0, -u * 0.05, -u * 0.12], [0.0, -u * 0.25, -u * 0.18], [0.0, -u * 0.42, -u * 0.17]], "material": fur2}),
                AnimalKind::Cat => json!({"id": format!("{id}_schwanzform"), "type": "tube", "radius": u * 0.035, "radius_end": u * 0.03,
                    "path": [[0.0, 0.0, 0.0], [0.0, u * 0.15, -u * 0.15], [0.0, u * 0.4, -u * 0.12]], "material": fur}),
                _ => json!({"id": format!("{id}_schwanzform"), "type": "tube", "radius": u * 0.04, "radius_end": u * 0.015,
                    "path": [[0.0, 0.0, 0.0], [0.0, u * 0.1, -u * 0.12], [0.0, u * 0.22, -u * 0.18]], "material": fur}),
            };
            // Kopf-Position: Pferd auf langem Hals, Kuh tiefer vorne, sonst knapp über dem Körper
            let head_pos = match kind {
                AnimalKind::Horse => [0.0, body_r * 1.35 + head_r * 0.6, hz + head_r * 1.5],
                AnimalKind::Cow => [0.0, body_r * 0.6, hz + head_r * 0.85],
                AnimalKind::Pig => [0.0, body_r * 0.35, hz + head_r * 0.55],
                AnimalKind::Sheep => [0.0, body_r * 0.7, hz + head_r * 0.8],
                _ => [0.0, body_r * 0.9 + head_r * 0.4, hz + head_r * 0.5],
            };
            let belly_mat = if farm { fur.clone() } else { fur2.clone() };
            let mut extras: Vec<Value> = Vec::new();
            match kind {
                AnimalKind::Cow => {
                    // Flecken und Euter
                    for (k, (x, y, z, r)) in [(0.95, 0.3, 0.2, 0.45), (-0.95, 0.1, -0.25, 0.5), (0.5, 0.85, -0.45, 0.35), (-0.4, 0.9, 0.35, 0.3), (0.9, -0.1, -0.5, 0.3)].iter().enumerate() {
                        extras.push(json!({"id": format!("{id}_fleck_{k}"), "type": "sphere", "radius": body_r * r, "position": [x * body_r, y * body_r, z * body_len], "scale": [0.35, 1.0, 1.2], "rotation": [0.0, 0.0, -x * 25.0], "material": fur2}));
                    }
                    extras.push(json!({"id": format!("{id}_euter"), "type": "sphere", "radius": body_r * 0.35, "position": [0.0, -body_r * 0.85, -body_len * 0.2], "scale": [1.0, 0.7, 1.0], "material": mat(Rgb::new(0.96, 0.72, 0.72), 0.5)}));
                }
                AnimalKind::Sheep => {
                    // Wolle: viele weiche Kugeln um den Körper
                    let mut k = 0;
                    for zi in [-0.45f32, -0.15, 0.15, 0.45] {
                        for (x, y) in [(-0.55f32, 0.25f32), (0.55, 0.25), (0.0, 0.7), (-0.4, -0.35), (0.4, -0.35)] {
                            extras.push(json!({"id": format!("{id}_wolle_{k}"), "type": "sphere", "radius": body_r * 0.62, "position": [x * body_r, y * body_r, zi * body_len], "material": fur}));
                            k += 1;
                        }
                    }
                }
                AnimalKind::Horse => {
                    // Hals und Mähne
                    let top = [0.0, body_r * 0.5, hz * 0.85];
                    let neck_end = [0.0, head_pos[1] - head_r * 0.3, head_pos[2] - head_r * 0.4];
                    extras.push(json!({"id": format!("{id}_hals"), "type": "tube", "radius": body_r * 0.55, "radius_end": head_r * 0.7, "path": [top, neck_end], "material": fur}));
                    for k in 0..6 {
                        let f = k as f32 / 5.0;
                        let p = [0.0, top[1] + (neck_end[1] - top[1]) * f + body_r * 0.45, top[2] + (neck_end[2] - top[2]) * f - body_r * 0.25];
                        extras.push(json!({"id": format!("{id}_maehne_{k}"), "type": "sphere", "radius": body_r * 0.2, "position": p, "scale": [0.5, 1.2, 1.0], "material": fur2}));
                    }
                }
                _ => {}
            }
            let mut body_children = vec![
                json!({"id": format!("{id}_rumpf"), "type": "capsule", "radius": body_r, "length": body_len, "rotation": [90.0, 0.0, 0.0], "material": fur, "animate": {"breathe": 0.02}}),
                json!({"id": format!("{id}_bauch"), "type": "sphere", "radius": body_r * 0.85, "position": [0.0, -body_r * 0.3, body_len * 0.2], "scale": [0.9, 0.8, 1.3], "material": belly_mat}),
                json!({"id": format!("{id}_kopf"), "type": "group", "position": head_pos, "animate": rot(&|p: &Pose| [p.head[0], p.head[1], 0.0]), "children": head}),
                json!({"id": format!("{id}_schwanz"), "type": "group", "position": [0.0, body_r * 0.3, -hz - body_r * 0.7], "animate": rot(&|p: &Pose| [p.tail[1], p.tail[0], 0.0]), "children": [tail]}),
                leg(0, -hx, hz * 0.8), leg(1, hx, hz * 0.8), leg(2, -hx, -hz * 0.8), leg(3, hx, -hz * 0.8),
            ];
            body_children.extend(extras);
            let body = json!({"id": format!("{id}_koerper"), "type": "group", "position": [0.0, hip_y, 0.0],
              "animate": {"position": Value::Array(times.iter().zip(&poses).map(|(t, p)| json!({"t": t, "v": [0.0, hip_y + p.body_y * u, 0.0], "ease": "linear"})).collect()),
                          "rotation": Value::Array(times.iter().zip(&poses).map(|(t, p)| json!({"t": t, "v": [p.body_pitch, 0.0, 0.0], "ease": "linear"})).collect())},
              "children": body_children});
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
        AnimalKind::Bird | AnimalKind::Chicken => {
            let r = u * 0.3;
            let chicken = kind == AnimalKind::Chicken;
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
                    eye(-1.0, [-r * 0.28, r * 0.15, r * 0.45], r * 0.15), eye(1.0, [r * 0.28, r * 0.15, r * 0.45], r * 0.15),
                    // Huhn: roter Kamm und Kehllappen
                    if chicken { json!({"id": format!("{id}_kamm"), "type": "sphere", "radius": r * 0.16, "position": [0.0, r * 0.6, r * 0.05], "scale": [0.5, 1.0, 1.0],
                        "repeat": {"count": 3, "offset": [0.0, -r * 0.04, -r * 0.18]}, "material": {"color": "#e0262c", "roughness": 0.4}}) } else { Value::Null },
                    if chicken { json!({"id": format!("{id}_kehllappen"), "type": "sphere", "radius": r * 0.1, "position": [0.0, -r * 0.35, r * 0.55], "scale": [0.6, 1.3, 0.6], "material": {"color": "#e0262c", "roughness": 0.4}}) } else { Value::Null }]},
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
        Value::Array(a) => {
            a.retain(|x| !x.is_null()); // optionale Teile (z. B. Hühnerkamm) als null eingetragen
            a.iter_mut().for_each(strip_nulls)
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Kind, Scene};

    #[test]
    fn all_animals_build_valid_groups() {
        for kind in ["dog", "cat", "bunny", "bird", "fish", "cow", "pig", "sheep", "horse", "chicken"] {
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
