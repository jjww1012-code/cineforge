//! Cartoon-Figuren-Generator (Pixar-/Disney-Stil): aus wenigen Angaben entsteht eine
//! fertige 3D-Figur mit Gelenken (Hüfte, Beine, Arme, Kopf), Gesicht (Augen mit Blinzeln,
//! Brauen je Ausdruck, Mund mit Lippen-Sync), Frisur und Kleidung – plus Aktionen wie
//! gehen, rennen, winken, springen, tanzen. Läuft die Figur einen `path` entlang, bewegen
//! sich die Beine automatisch passend zum Tempo.
//!
//! Ergebnis ist eine normale Gruppe aus Grundformen mit Keyframes – die App zeigt sie wie
//! jedes andere Objekt.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::animation::Talk;
use crate::color::Rgb;
use crate::model::Object;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Build {
    Slim,
    #[default]
    Normal,
    Round,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Hair {
    #[default]
    Short,
    Spiky,
    Long,
    Bun,
    Curly,
    Bald,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Expression {
    Neutral,
    #[default]
    Happy,
    Sad,
    Angry,
    Surprised,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Idle,
    Walk,
    Run,
    Wave,
    Jump,
    Cheer,
    Dance,
    Point,
    Nod,
    ShakeHead,
    Talk,
    Sit,
    Think,
    /// In die Hände klatschen
    Clap,
    /// Verbeugen
    Bow,
    /// Lachen (zurücklehnen, Schultern beben)
    Laugh,
    /// Erschrecken (ducken, Arme schützend hoch, zittern)
    Scared,
    /// Sich umschauen
    LookAround,
    /// Schultern zucken („keine Ahnung“)
    Shrug,
    /// Traurig (Kopf hängen lassen)
    Sad,
    /// Aufgeregt hüpfen
    Excited,
}

/// Eine Aktion im Zeitablauf.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Action {
    #[serde(rename = "type")]
    pub kind: ActionKind,
    #[serde(default)]
    pub start: f32,
    /// Ende (Standard: start + 2 s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<f32>,
    /// Tempo-Faktor (1 = normal).
    #[serde(default = "one")]
    pub speed: f32,
}

fn one() -> f32 {
    1.0
}
fn yes() -> bool {
    true
}
fn d_height() -> f32 {
    1.2
}
fn c_skin() -> Rgb {
    Rgb::new(0.98, 0.80, 0.66)
}
fn c_hair() -> Rgb {
    Rgb::new(0.36, 0.22, 0.12)
}
fn c_shirt() -> Rgb {
    Rgb::new(0.20, 0.55, 0.90)
}
fn c_pants() -> Rgb {
    Rgb::new(0.22, 0.26, 0.36)
}
fn c_shoes() -> Rgb {
    Rgb::new(0.85, 0.25, 0.22)
}
fn c_eyes() -> Rgb {
    Rgb::new(0.25, 0.45, 0.25)
}

/// Angaben für eine Figur.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CharacterSpec {
    /// Körpergröße in Metern (Kind ≈ 1.0–1.3, Erwachsene ≈ 1.6–1.9).
    #[serde(default = "d_height")]
    pub height: f32,
    #[serde(default)]
    pub build: Build,
    #[serde(default = "c_skin")]
    pub skin: Rgb,
    #[serde(default)]
    pub hair: Hair,
    #[serde(default = "c_hair")]
    pub hair_color: Rgb,
    #[serde(default = "c_shirt")]
    pub shirt: Rgb,
    #[serde(default = "c_pants")]
    pub pants: Rgb,
    #[serde(default = "c_shoes")]
    pub shoes: Rgb,
    #[serde(default = "c_eyes")]
    pub eye_color: Rgb,
    #[serde(default)]
    pub expression: Expression,
    /// Gezeichnete Umrisslinie (Cartoon-Look).
    #[serde(default)]
    pub outline: bool,
    /// Blinzeln (Standard an).
    #[serde(default = "yes")]
    pub blink: bool,
    /// Aktionen im Zeitablauf (gehen, winken, springen …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    /// Lippen-Sync zur Stimme (WAV in Audio/).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub talk: Option<Talk>,
    /// Beim Laufen auf einem `path` die Beine automatisch bewegen (Standard an).
    #[serde(default = "yes")]
    pub auto_walk: bool,
    /// Zu diesem Objekt schauen (Kopf und Oberkörper drehen sich hin), z. B. zum Gesprächspartner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look_at: Option<String>,
    /// Gesichtsausdruck wechselt im Lauf der Szene: [{"t": 3, "expression": "surprised"}, …].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expressions: Vec<ExpressionChange>,
    /// Kopfbedeckung.
    #[serde(default, skip_serializing_if = "Hat::is_none")]
    pub hat: Hat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hat_color: Option<Rgb>,
    /// Brille.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub glasses: bool,
    /// Kleidung: "shirt" (Standard) oder "dress" (Kleid in Hemdfarbe).
    #[serde(default, skip_serializing_if = "Outfit::is_shirt")]
    pub outfit: Outfit,
    /// Schal (Farbe).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scarf: Option<Rgb>,
    /// Rucksack (Farbe).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backpack: Option<Rgb>,
    /// Hält ein Objekt in der rechten Hand (beliebiges Objekt, z. B. Ballon, Blume, Eis, Buch,
    /// oder `{"id": "x", "type": "instance", "asset": "blume"}`). Ursprung = Handmitte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold: Option<Box<Object>>,
    /// Hält ein Objekt in der linken Hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_left: Option<Box<Object>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Hat {
    #[default]
    None,
    Cap,
    Beanie,
    TopHat,
    Crown,
    Party,
}

impl Hat {
    pub fn is_none(&self) -> bool {
        *self == Hat::None
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Outfit {
    #[default]
    Shirt,
    Dress,
}

impl Outfit {
    pub fn is_shirt(&self) -> bool {
        *self == Outfit::Shirt
    }
}

/// Wechsel des Gesichtsausdrucks ab Zeitpunkt `t`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExpressionChange {
    pub t: f32,
    pub expression: Expression,
}

impl CharacterSpec {
    pub fn validate(&self, id: &str, p: &mut Vec<String>) {
        if !(0.2..=20.0).contains(&self.height) {
            p.push(format!("\"{id}\": height zwischen 0.2 und 20 Metern"));
        }
        for a in &self.actions {
            if a.end.is_some_and(|e| e <= a.start) {
                p.push(format!("\"{id}\": Aktion {:?}: end muss nach start liegen", a.kind));
            }
            if !(a.speed > 0.0 && a.speed <= 10.0) {
                p.push(format!("\"{id}\": Aktion {:?}: speed zwischen 0 und 10", a.kind));
            }
        }
    }
}

/// Gelenk-Winkel und Hüft-Versatz einer Pose (Grad bzw. Anteil der Größe).
#[derive(Default, Clone, Copy)]
struct Pose {
    hip_y: f32,
    hip_x: f32,
    hip_rot_x: f32,
    hip_rot_y: f32,
    leg_l: f32,
    leg_r: f32,
    arm_l: [f32; 2], // [x, z]
    arm_r: [f32; 2],
    head: [f32; 3], // x, y, z
    /// Stauchen (−) / Strecken (+) des ganzen Körpers, volumenerhaltend (Disney: Squash & Stretch).
    squash: f32,
}

impl Pose {
    fn add(&mut self, o: &Pose, w: f32) {
        self.hip_y += o.hip_y * w;
        self.hip_x += o.hip_x * w;
        self.hip_rot_x += o.hip_rot_x * w;
        self.hip_rot_y += o.hip_rot_y * w;
        self.leg_l += o.leg_l * w;
        self.leg_r += o.leg_r * w;
        for k in 0..2 {
            self.arm_l[k] += o.arm_l[k] * w;
            self.arm_r[k] += o.arm_r[k] * w;
        }
        for k in 0..3 {
            self.head[k] += o.head[k] * w;
        }
        self.squash += o.squash * w;
    }
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Pose einer Aktion zur lokalen Zeit `lt` (Sekunden seit Start).
fn action_pose(kind: ActionKind, lt: f32, speed: f32, len: f32) -> Pose {
    use std::f32::consts::TAU;
    let mut p = Pose::default();
    match kind {
        ActionKind::Idle => {
            let s = (lt * TAU / 4.0).sin();
            p.arm_l[1] = -2.0 * s;
            p.arm_r[1] = 2.0 * s;
            p.head = [2.0 * (lt * TAU / 5.0).sin(), 12.0 * (lt * TAU / 7.0).sin(), 0.0];
        }
        ActionKind::Walk | ActionKind::Run => {
            let run = kind == ActionKind::Run;
            let f = if run { 2.6 } else { 1.7 } * speed;
            let ph = lt * TAU * f / 2.0; // ein Zyklus = zwei Schritte
            let (leg, arm) = if run { (45.0, 55.0) } else { (28.0, 24.0) };
            p.leg_l = leg * ph.sin();
            p.leg_r = -leg * ph.sin();
            p.arm_l[0] = -arm * ph.sin();
            p.arm_r[0] = arm * ph.sin();
            p.hip_y = if run { 0.035 } else { 0.015 } * (1.0 - (2.0 * ph).cos()) * 0.5;
            p.hip_rot_x = if run { 10.0 } else { 3.0 };
            p.hip_rot_y = 4.0 * ph.sin();
            p.head[0] = if run { -6.0 } else { -1.0 };
        }
        ActionKind::Wave => {
            p.arm_r = [-10.0, 150.0 + 18.0 * (lt * TAU * 2.2 * speed).sin()];
            p.head = [-4.0, 0.0, -6.0];
        }
        ActionKind::Point => {
            p.arm_r = [-85.0, 5.0];
            p.head[0] = -3.0;
        }
        ActionKind::Jump => {
            // Ausholen (in die Knie, Arme nach hinten) → Absprung gestreckt → Flugbogen → Landung gestaucht
            let u = (lt / (0.9 / speed)).clamp(0.0, 1.0);
            let crouch = if u < 0.25 { -0.07 * smoothstep(u / 0.25) } else if u < 0.32 { -0.07 * (1.0 - smoothstep((u - 0.25) / 0.07)) } else { 0.0 };
            let air = if (0.25..0.85).contains(&u) { ((u - 0.25) / 0.6 * std::f32::consts::PI).sin() * 0.35 } else { 0.0 };
            let land = if u >= 0.85 { -0.06 * (1.0 - smoothstep((u - 0.85) / 0.15)) } else { 0.0 };
            p.hip_y = crouch + air + land;
            p.squash = if u < 0.25 {
                -0.12 * smoothstep(u / 0.25)
            } else if u < 0.55 {
                // Absprung: von gestaucht schnell auf gestreckt, bis zum Scheitelpunkt wieder normal
                let k = (u - 0.25) / 0.3;
                0.14 * (k * std::f32::consts::PI).sin().max(0.0) - 0.12 * (1.0 - smoothstep(k * 4.0))
            } else if u < 0.85 {
                0.05 * smoothstep((u - 0.6) / 0.25) // im Fall leicht gestreckt
            } else {
                -0.16 * (1.0 - smoothstep((u - 0.85) / 0.15))
            };
            let up = smoothstep((u - 0.22) / 0.1) * (1.0 - smoothstep((u - 0.82) / 0.12));
            let back = smoothstep(u / 0.2) * (1.0 - smoothstep((u - 0.22) / 0.06));
            p.arm_l = [35.0 * back, -150.0 * up];
            p.arm_r = [35.0 * back, 150.0 * up];
            p.leg_l = -15.0 * up;
            p.leg_r = 10.0 * up;
            p.hip_rot_x = 12.0 * back;
            p.head[0] = 8.0 * back - 6.0 * up;
        }
        ActionKind::Cheer => {
            let s = (lt * TAU * 2.0 * speed).sin();
            p.arm_l[1] = -155.0 - 10.0 * s;
            p.arm_r[1] = 155.0 + 10.0 * s;
            p.hip_y = 0.04 * s.abs();
            p.head[0] = -8.0;
        }
        ActionKind::Dance => {
            let ph = lt * TAU * 1.0 * speed;
            p.hip_x = 0.03 * ph.sin();
            p.hip_rot_y = 15.0 * ph.sin();
            p.hip_y = 0.02 * (2.0 * ph).sin().abs();
            p.arm_l = [-30.0 * ph.sin(), -60.0 - 40.0 * ph.cos()];
            p.arm_r = [30.0 * ph.sin(), 60.0 - 40.0 * ph.cos()];
            p.leg_l = 12.0 * ph.sin();
            p.leg_r = -12.0 * ph.sin();
            p.head = [0.0, 0.0, 10.0 * ph.sin()];
        }
        ActionKind::Nod => {
            p.head[0] = 14.0 * (lt * TAU * 1.6 * speed).sin().max(-0.3);
        }
        ActionKind::ShakeHead => {
            p.head[1] = 22.0 * (lt * TAU * 1.8 * speed).sin();
        }
        ActionKind::Talk => {
            let a = (lt * TAU * 0.7 * speed).sin();
            let b = (lt * TAU * 1.13 * speed + 1.0).sin();
            p.arm_r = [-25.0 - 15.0 * a, 15.0 + 10.0 * b];
            p.arm_l = [-10.0 * b, -10.0];
            p.head = [3.0 * b, 8.0 * a, 4.0 * a];
        }
        ActionKind::Sit => {
            p.hip_y = -0.32;
            p.leg_l = -85.0;
            p.leg_r = -85.0;
            p.arm_l[0] = -20.0;
            p.arm_r[0] = -20.0;
        }
        ActionKind::Think => {
            p.arm_r = [-120.0, 30.0];
            p.head = [-6.0, -10.0, 8.0];
        }
        ActionKind::Clap => {
            let c = (lt * TAU * 2.6 * speed).sin();
            // Hände treffen sich vor der Brust (Schulterbreite ≈ Armlänge × sin 45°)
            p.arm_l = [-58.0, 42.0 + 9.0 * c];
            p.arm_r = [-58.0, -42.0 - 9.0 * c];
            p.head[0] = -4.0;
            p.squash = 0.015 * c;
        }
        ActionKind::Bow => {
            p.hip_rot_x = 38.0;
            p.head[0] = 12.0;
            p.arm_l = [-12.0, -4.0];
            p.arm_r = [-12.0, 4.0];
        }
        ActionKind::Laugh => {
            let b = (lt * TAU * 5.5 * speed).sin();
            p.hip_rot_x = -8.0;
            p.head = [-16.0 + 3.0 * b, 0.0, 4.0 * (lt * TAU * 0.8).sin()];
            p.hip_y = 0.008 * b;
            p.squash = 0.02 * b;
            p.arm_l = [-35.0, 18.0];
            p.arm_r = [-35.0, -18.0];
        }
        ActionKind::Scared => {
            let tr = (lt * TAU * 11.0).sin();
            // Gesicht bleibt sichtbar (Nahaufnahmen): Arme vor der Brust, Kopf nur leicht eingezogen
            p.hip_y = -0.03;
            p.hip_x = 0.004 * tr;
            p.hip_rot_x = -4.0;
            p.arm_l = [-80.0, 30.0];
            p.arm_r = [-80.0, -30.0];
            p.head = [-4.0, 3.0 * tr, 0.0];
            p.squash = -0.04;
        }
        ActionKind::LookAround => {
            p.head = [-4.0, 55.0 * (lt * TAU * 0.35 * speed).sin(), 0.0];
            p.hip_rot_y = 12.0 * (lt * TAU * 0.35 * speed - 0.4).sin();
        }
        ActionKind::Shrug => {
            p.arm_l = [-35.0, -38.0];
            p.arm_r = [-35.0, 38.0];
            p.head = [-3.0, 0.0, 12.0];
            p.hip_y = 0.012;
            p.squash = 0.03;
        }
        ActionKind::Sad => {
            p.head = [12.0, 0.0, 4.0];
            p.hip_rot_x = 5.0;
            p.hip_y = -0.01;
            p.arm_l = [4.0, -2.0];
            p.arm_r = [4.0, 2.0];
            p.squash = -0.03;
        }
        ActionKind::Excited => {
            let ph = (lt * TAU * 2.4 * speed).sin();
            let hop = ph.abs();
            p.hip_y = 0.07 * hop;
            p.squash = 0.08 * hop - 0.06 * (1.0 - hop).powi(4);
            p.arm_l = [-20.0, -110.0 - 25.0 * ph];
            p.arm_r = [-20.0, 110.0 + 25.0 * ph];
            p.head[0] = -8.0;
        }
    }
    let _ = len;
    p
}

fn total_pose(actions: &[(ActionKind, f32, f32, f32)], t: f32) -> Pose {
    // Grundhaltung: Arme leicht vom Körper weg
    let mut pose = Pose { arm_l: [0.0, -8.0], arm_r: [0.0, 8.0], ..Default::default() };
    for &(kind, start, end, speed) in actions {
        if t < start - 0.3 || t > end + 0.3 {
            continue;
        }
        let ramp = 0.25_f32.min((end - start) / 3.0);
        let w = smoothstep((t - start) / ramp + 1.0).min(smoothstep((end - t) / ramp + 1.0));
        if w <= 0.0 {
            continue;
        }
        let a = action_pose(kind, (t - start).max(0.0), speed, end - start);
        pose.add(&a, w);
    }
    pose
}

/// Zufällige Variante einer Figur (für Menschenmengen): Frisur, Farben, Größe, Statur, Ausdruck.
pub fn vary(spec: &CharacterSpec, seed: u32) -> CharacterSpec {
    let mut r = crate::noise::Rng::new(seed.wrapping_mul(2654435761) ^ 0x5eed);
    let pick = |r: &mut crate::noise::Rng, n: usize| (r.next_u32() as usize) % n;
    let skins = ["#f9d7c0", "#f2c29b", "#e0ac84", "#c68863", "#8d5b3b", "#5e3b26"];
    let hairs = ["#2b1a0f", "#5d3a1a", "#a0522d", "#e0a33a", "#d9d2c5", "#1c1c1c", "#b23a2a"];
    let clothes = ["#e63946", "#2a9d8f", "#f4a261", "#457b9d", "#8338ec", "#ffbe0b", "#3a86ff", "#06d6a0", "#ef476f", "#6c757d"];
    let pants = ["#2d3a5a", "#3d405b", "#5c4033", "#264653", "#6b705c"];
    let mut s = spec.clone();
    s.skin = Rgb::parse_hex(skins[pick(&mut r, skins.len())]).unwrap_or(s.skin);
    s.hair_color = Rgb::parse_hex(hairs[pick(&mut r, hairs.len())]).unwrap_or(s.hair_color);
    s.shirt = Rgb::parse_hex(clothes[pick(&mut r, clothes.len())]).unwrap_or(s.shirt);
    s.pants = Rgb::parse_hex(pants[pick(&mut r, pants.len())]).unwrap_or(s.pants);
    s.shoes = Rgb::parse_hex(clothes[pick(&mut r, clothes.len())]).unwrap_or(s.shoes);
    s.hair = [Hair::Short, Hair::Spiky, Hair::Long, Hair::Bun, Hair::Curly, Hair::Bald][pick(&mut r, 6)];
    s.build = [Build::Slim, Build::Normal, Build::Normal, Build::Round][pick(&mut r, 4)];
    s.height = spec.height * r.range(0.85, 1.25);
    s.expression = [Expression::Happy, Expression::Happy, Expression::Neutral, Expression::Surprised][pick(&mut r, 4)];
    s.outfit = if s.hair == Hair::Long && pick(&mut r, 2) == 0 { Outfit::Dress } else { Outfit::Shirt };
    s.hat = [Hat::None, Hat::None, Hat::None, Hat::Cap, Hat::Beanie][pick(&mut r, 5)];
    s.glasses = pick(&mut r, 5) == 0;
    s.hat_color = Rgb::parse_hex(clothes[pick(&mut r, clothes.len())]);
    s.talk = None;
    // Nicht alle im Gleichtakt: Aktionen leicht versetzt und etwas unterschiedlich schnell
    let shift = r.range(0.0, 0.6);
    for act in &mut s.actions {
        act.start += shift;
        act.end = act.end.map(|e| e + shift);
        act.speed *= r.range(0.88, 1.12);
    }
    s
}

/// Baut die Figur als Gruppe (Kinder-ids: `<id>_…`). `obj` = das Szenen-Objekt (für `path`).
pub fn build(obj: &Object, spec: &CharacterSpec) -> Object {
    build_looking(obj, spec, 0.0)
}

/// Wie [`build`], mit Blickrichtung: `look_yaw` = Winkel (Grad) vom Körper zum Ziel.
pub fn build_looking(obj: &Object, spec: &CharacterSpec, look_yaw: f32) -> Object {
    let look = look_yaw.clamp(-80.0, 80.0);
    let u = spec.height / 1.2; // Maßstab: Entwurf für 1,2 m
    let bw = match spec.build {
        Build::Slim => 0.85,
        Build::Normal => 1.0,
        Build::Round => 1.28,
    };
    let id = &obj.id;
    let ol = if spec.outline { json!({"width": 0.01 * u}) } else { Value::Null };
    let mat = |c: Rgb, rough: f32| {
        let mut m = json!({"color": c.to_hex(), "roughness": rough});
        if !ol.is_null() {
            m["outline"] = ol.clone();
        }
        m
    };
    let skin = mat(spec.skin, 0.55);
    // Gesichts-Details ohne Umriss (sonst Pünktchen an kleinen Formen)
    let skin_dark = json!({"color": Rgb::new(spec.skin.r * 0.9, spec.skin.g * 0.78, spec.skin.b * 0.72).to_hex(), "roughness": 0.55});
    let skin_plain = json!({"color": spec.skin.to_hex(), "roughness": 0.55});

    // ---- Aktionen sammeln (inkl. automatisches Gehen auf dem Pfad) ----
    let mut acts: Vec<(ActionKind, f32, f32, f32)> =
        spec.actions.iter().map(|a| (a.kind, a.start, a.end.unwrap_or(a.start + 2.0), a.speed)).collect();
    if spec.auto_walk {
        if let Some(path) = obj.animate.as_ref().and_then(|a| a.path.as_ref()) {
            let covered = acts.iter().any(|(k, s, e, _)| matches!(k, ActionKind::Walk | ActionKind::Run) && *s < path.end && *e > path.start);
            if !covered {
                let v = path.length() / (path.end - path.start).max(0.1); // m/s
                let (kind, base) = if v > 2.6 * u { (ActionKind::Run, 2.6) } else { (ActionKind::Walk, 1.7) };
                // Schrittfrequenz passend zur Geschwindigkeit (Schrittlänge ≈ 0,45 × Größe)
                let freq = v / (0.45 * spec.height);
                acts.push((kind, path.start, path.end, (freq / base).clamp(0.3, 3.0)));
            }
        }
    }
    let t_end = acts.iter().map(|a| a.2).fold(0.0_f32, f32::max) + 0.5;
    let fps = 15.0;
    let n = if acts.is_empty() { 0 } else { (t_end * fps).ceil() as usize + 1 };
    let times: Vec<f32> = (0..n).map(|i| i as f32 / fps).collect();
    let poses: Vec<Pose> = times
        .iter()
        .map(|&t| {
            let mut p = total_pose(&acts, t);
            p.head[1] += look * 0.7;
            p.hip_rot_y += look * 0.3;
            p
        })
        .collect();
    let keys3 = |f: &dyn Fn(&Pose) -> [f32; 3]| -> Value {
        Value::Array(times.iter().zip(&poses).map(|(t, p)| json!({"t": t, "v": f(p), "ease": "linear"})).collect())
    };
    let anim_rot = |f: &dyn Fn(&Pose) -> [f32; 3]| -> Value {
        if n == 0 { Value::Null } else { json!({"rotation": keys3(f)}) }
    };

    // ---- Kopf ----
    let hr = 0.24 * u; // Kopfradius (großer Kopf = niedlich)
    let eye_x = 0.085 * u;
    let blink = if spec.blink {
        let seed = id.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
        json!({"blink": {"every": 3.6, "seed": seed % 1000}})
    } else {
        Value::Null
    };
    let eye = |side: f32| {
        json!({"id": format!("{id}_auge_{}", if side < 0.0 { "l" } else { "r" }), "type": "group",
          "position": [side * eye_x, 0.035 * u, 0.19 * u], "animate": blink,
          "children": [
            {"id": format!("{id}_augapfel_{side}"), "type": "sphere", "radius": 0.07 * u, "scale": [1.0, 1.18, 0.62], "material": {"color": "#ffffff", "roughness": 0.2}},
            {"id": format!("{id}_iris_{side}"), "type": "sphere", "radius": 0.044 * u, "position": [0.0, -0.006 * u, 0.03 * u], "scale": [1.0, 1.12, 0.5], "material": {"color": spec.eye_color.to_hex(), "roughness": 0.25}},
            {"id": format!("{id}_pupille_{side}"), "type": "sphere", "radius": 0.026 * u, "position": [0.0, -0.006 * u, 0.046 * u], "scale": [1.0, 1.12, 0.45], "material": {"color": "#0b0b10", "roughness": 0.1, "unlit": true}},
            {"id": format!("{id}_glanz_{side}"), "type": "sphere", "radius": 0.012 * u, "position": [0.016 * u, 0.02 * u, 0.06 * u], "material": {"color": "#ffffff", "unlit": true}}
          ]})
    };
    // Ausdrucks-abhängige Teile: Brauen, Lippenlinie, ggf. „O“-Mund
    let face_parts = |expr: Expression, suffix: &str| -> Vec<Value> {
        let (brow_rot, brow_y, mouth_w, mouth_y): (f32, f32, f32, f32) = match expr {
            Expression::Neutral => (0.0, 0.125, 0.9, -0.1),
            Expression::Happy => (-6.0, 0.135, 1.2, -0.1),
            Expression::Sad => (16.0, 0.13, 0.8, -0.11),
            Expression::Angry => (-22.0, 0.115, 1.0, -0.105),
            Expression::Surprised => (-4.0, 0.15, 0.6, -0.105),
        };
        let brow = |side: f32| {
            json!({"id": format!("{id}_braue_{side}{suffix}"), "type": "capsule", "radius": 0.011 * u, "length": 0.06 * u,
              "position": [side * eye_x, brow_y * u, 0.215 * u], "rotation": [0.0, 0.0, 90.0 + side * brow_rot],
              "material": {"color": spec.hair_color.to_hex(), "roughness": 0.7}})
        };
        let curve: f32 = match expr {
            Expression::Happy => 0.035,
            Expression::Sad => -0.03,
            Expression::Angry => -0.012,
            Expression::Neutral | Expression::Surprised => 0.004,
        };
        let half_w = 0.07 * u * mouth_w.min(1.3);
        let lip_points: Vec<[f32; 3]> = (0..=8)
            .map(|i| {
                let x = -half_w + 2.0 * half_w * i as f32 / 8.0;
                let k = x / half_w;
                let y = mouth_y * u + curve * u * k * k;
                let z = ((hr * hr - x * x - y * y).max(0.0)).sqrt() * 0.95 + 0.004 * u;
                [x, y, z]
            })
            .collect();
        let mut parts = vec![brow(-1.0), brow(1.0)];
        if expr == Expression::Surprised {
            parts.push(json!({"id": format!("{id}_o_mund{suffix}"), "type": "sphere", "radius": 0.04 * u,
                "position": [0.0, (mouth_y - 0.01) * u, 0.205 * u], "scale": [0.8, 1.0, 0.45], "material": {"color": "#4a1018", "roughness": 0.6}}));
        } else {
            parts.push(json!({"id": format!("{id}_lippen{suffix}"), "type": "tube", "path": lip_points, "radius": 0.009 * u, "radius_end": 0.009 * u,
                "segments": 8, "material": {"color": "#5b1a22", "roughness": 0.5}}));
        }
        parts
    };
    let mouth_y = -0.1;
    let open_base = [1.0, 0.12, 0.45];
    let mut mouth = json!({"id": format!("{id}_mund"), "type": "sphere", "radius": 0.05 * u,
        "position": [0.0, (mouth_y - 0.012) * u, 0.2 * u], "scale": open_base,
        "material": {"color": "#4a1018", "roughness": 0.6}});
    if let Some(t) = &spec.talk {
        let mut t = t.clone();
        // Geschlossen = nur ein Strich; offen = bis zur vollen Höhe (Faktor auf die Grundhöhe 0.12)
        mouth["scale"] = json!([1.0, 0.8, 0.45]);
        t.closed = t.closed.min(0.12);
        mouth["animate"] = json!({"talk": t});
    }
    let mut head_children = vec![
        json!({"id": format!("{id}_schaedel"), "type": "sphere", "radius": hr, "scale": [1.0, 0.96, 0.95], "material": skin}),
        json!({"id": format!("{id}_ohr_l"), "type": "sphere", "radius": 0.05 * u, "position": [-hr * 0.97, -0.01 * u, -0.01 * u], "scale": [0.6, 1.0, 0.8], "material": skin_plain}),
        json!({"id": format!("{id}_ohr_r"), "type": "sphere", "radius": 0.05 * u, "position": [hr * 0.97, -0.01 * u, -0.01 * u], "scale": [0.6, 1.0, 0.8], "material": skin_plain}),
        json!({"id": format!("{id}_nase"), "type": "sphere", "radius": 0.034 * u, "position": [0.0, -0.025 * u, 0.232 * u], "scale": [1.0, 0.9, 0.9], "material": skin_dark}),
        json!({"id": format!("{id}_wange_l"), "type": "sphere", "radius": 0.035 * u, "position": [-0.13 * u, -0.06 * u, 0.175 * u], "scale": [1.0, 0.7, 0.4], "material": {"color": "#f59a9a", "roughness": 0.7}}),
        json!({"id": format!("{id}_wange_r"), "type": "sphere", "radius": 0.035 * u, "position": [0.13 * u, -0.06 * u, 0.175 * u], "scale": [1.0, 0.7, 0.4], "material": {"color": "#f59a9a", "roughness": 0.7}}),
        eye(-1.0),
        eye(1.0),
    ];
    // Mundöffnung nur, wenn die Figur spricht (sonst sähe man einen zweiten Strich)
    if spec.talk.is_some() {
        head_children.push(mouth);
    }
    if spec.expressions.is_empty() {
        head_children.extend(face_parts(spec.expression, ""));
    } else {
        let mut changes = spec.expressions.clone();
        changes.sort_by(|a, b| a.t.total_cmp(&b.t));
        let mut variants = vec![spec.expression];
        for c in &changes {
            if !variants.contains(&c.expression) {
                variants.push(c.expression);
            }
        }
        for (k, v) in variants.iter().enumerate() {
            let mut keys = vec![json!({"t": 0.0, "v": spec.expression == *v, "ease": "step"})];
            keys.extend(changes.iter().map(|c| json!({"t": c.t, "v": c.expression == *v, "ease": "step"})));
            for mut part in face_parts(*v, &format!("_v{k}")) {
                part["animate"] = json!({"visible": keys});
                head_children.push(part);
            }
        }
    }
    let hair = mat(spec.hair_color, 0.65);
    let cap = |extra: Value| {
        let mut o = json!({"id": format!("{id}_haar"), "type": "sphere", "radius": hr * 1.06, "position": [0.0, 0.07 * u, -0.045 * u],
           "scale": [1.0, 0.88, 1.0], "material": hair.clone()});
        if let Value::Object(m) = extra {
            for (k, v) in m {
                o[k] = v;
            }
        }
        o
    };
    match spec.hair {
        Hair::Bald => {}
        Hair::Short => head_children.push(cap(json!({}))),
        Hair::Curly => head_children.push(cap(json!({"radius": hr * 1.1, "subdivide": 2,
            "deform": [{"type": "noise", "amount": 0.035 * u, "scale": 9.0 / u, "octaves": 2, "seed": 3}]}))),
        Hair::Spiky => {
            head_children.push(cap(json!({})));
            for (i, (x, z, rx, rz)) in [(0.0, 0.0, -15.0, 0.0), (-0.1, -0.03, -10.0, 25.0), (0.1, -0.03, -10.0, -25.0), (-0.05, -0.12, -40.0, 12.0), (0.05, -0.12, -40.0, -12.0)].iter().enumerate() {
                head_children.push(json!({"id": format!("{id}_zacke_{i}"), "type": "cone", "radius": 0.06 * u, "height": 0.16 * u,
                    "position": [x * u, 0.27 * u, z * u], "rotation": [*rx, 0.0, *rz], "material": hair.clone()}));
            }
        }
        Hair::Long => {
            head_children.push(cap(json!({})));
            head_children.push(json!({"id": format!("{id}_haar_lang"), "type": "capsule", "radius": 0.2 * u, "length": 0.22 * u,
                "position": [0.0, -0.12 * u, -0.1 * u], "scale": [1.05, 1.0, 0.55], "material": hair.clone()}));
        }
        Hair::Bun => {
            head_children.push(cap(json!({})));
            head_children.push(json!({"id": format!("{id}_dutt"), "type": "sphere", "radius": 0.095 * u,
                "position": [0.0, 0.25 * u, -0.1 * u], "material": hair.clone()}));
        }
    }
    // ---- Accessoires am Kopf ----
    let hat_c = spec.hat_color.unwrap_or(match spec.hat {
        Hat::Crown => Rgb::new(1.0, 0.8, 0.2),
        Hat::TopHat => Rgb::new(0.12, 0.12, 0.14),
        _ => Rgb::new(0.85, 0.2, 0.25),
    });
    let hm = json!({"color": hat_c.to_hex(), "roughness": if spec.hat == Hat::Crown { 0.25 } else { 0.7 }, "metallic": if spec.hat == Hat::Crown { 1.0 } else { 0.0 }});
    let top = hr * 0.88;
    match spec.hat {
        Hat::None => {}
        Hat::Cap => {
            // höher und nach hinten versetzt, damit die Augen frei bleiben
            head_children.push(json!({"id": format!("{id}_kappe"), "type": "sphere", "radius": hr * 1.05, "position": [0.0, hr * 0.5, -hr * 0.22], "scale": [1.0, 0.72, 1.0], "material": hm}));
            head_children.push(json!({"id": format!("{id}_schirm"), "type": "cylinder", "radius": hr * 0.5, "height": 0.015 * u, "position": [0.0, hr * 0.52, hr * 0.78], "scale": [1.0, 1.0, 0.75], "rotation": [-10.0, 0.0, 0.0], "material": hm}));
        }
        Hat::Beanie => {
            head_children.push(json!({"id": format!("{id}_muetze"), "type": "sphere", "radius": hr * 1.08, "position": [0.0, hr * 0.45, -hr * 0.27], "scale": [1.0, 0.8, 1.0], "material": hm}));
            head_children.push(json!({"id": format!("{id}_bommel"), "type": "sphere", "radius": hr * 0.22, "position": [0.0, top + hr * 0.32, -0.03 * u], "material": {"color": "#ffffff", "roughness": 0.9}}));
        }
        Hat::TopHat => {
            head_children.push(json!({"id": format!("{id}_krempe"), "type": "cylinder", "radius": hr * 1.15, "height": 0.02 * u, "position": [0.0, top - hr * 0.05, 0.0], "material": hm}));
            head_children.push(json!({"id": format!("{id}_zylinder"), "type": "cylinder", "radius": hr * 0.7, "height": hr * 1.0, "position": [0.0, top + hr * 0.45, 0.0], "material": hm}));
        }
        Hat::Crown => {
            head_children.push(json!({"id": format!("{id}_kronenreif"), "type": "cylinder", "radius": hr * 0.62, "height": hr * 0.3, "position": [0.0, top + hr * 0.05, 0.0], "material": hm}));
            head_children.push(json!({"id": format!("{id}_zacke"), "type": "cone", "radius": hr * 0.14, "height": hr * 0.32, "position": [hr * 0.55, top + hr * 0.32, 0.0],
                "repeat": {"count": 6, "rotation": [0.0, 60.0, 0.0], "pivot": [-hr * 0.55, 0.0, 0.0]}, "material": hm}));
        }
        Hat::Party => {
            head_children.push(json!({"id": format!("{id}_partyhut"), "type": "cone", "radius": hr * 0.45, "height": hr * 1.1, "position": [0.0, top + hr * 0.45, 0.0], "rotation": [0.0, 0.0, 10.0],
                "material": {"color": hat_c.to_hex(), "roughness": 0.6, "pattern": {"type": "stripes", "color2": "#ffe14d", "scale": 6.0}}}));
        }
    }
    if spec.glasses {
        let gm = json!({"color": "#1f1f24", "roughness": 0.3, "metallic": 0.6});
        for side in [-1.0f32, 1.0] {
            head_children.push(json!({"id": format!("{id}_brillenglas_{side}"), "type": "torus", "radius": 0.075 * u, "thickness": 0.008 * u,
                "position": [side * eye_x, 0.035 * u, 0.235 * u], "rotation": [90.0, 0.0, 0.0], "material": gm}));
        }
        head_children.push(json!({"id": format!("{id}_brillensteg"), "type": "tube", "radius": 0.006 * u,
            "path": [[-0.012 * u, 0.04 * u, 0.24 * u], [0.0, 0.05 * u, 0.245 * u], [0.012 * u, 0.04 * u, 0.24 * u]], "material": gm}));
    }
    let head_y = 0.75 * u; // Hals-Gelenk
    let head = json!({"id": format!("{id}_kopf"), "type": "group", "position": [0.0, head_y, 0.0],
        "rotation": [0.0, if n == 0 { look * 0.7 } else { 0.0 }, 0.0],
        "animate": anim_rot(&|p: &Pose| p.head),
        "children": [{"id": format!("{id}_kopf_mitte"), "type": "group", "position": [0.0, hr * 0.92, 0.0], "children": head_children}]});

    // ---- Körper ----
    let shirt = mat(spec.shirt, 0.8);
    let pants = mat(spec.pants, 0.85);
    let shoes = mat(spec.shoes, 0.45);
    let hip_y = 0.36 * u;
    let leg = |side: f32| {
        let f: Box<dyn Fn(&Pose) -> [f32; 3]> = if side < 0.0 { Box::new(|p: &Pose| [p.leg_l, 0.0, 0.0]) } else { Box::new(|p: &Pose| [p.leg_r, 0.0, 0.0]) };
        json!({"id": format!("{id}_bein_{}", if side < 0.0 { "l" } else { "r" }), "type": "group",
          "position": [side * 0.085 * u * bw, 0.0, 0.0], "animate": anim_rot(&*f),
          "children": [
            {"id": format!("{id}_hosenbein_{side}"), "type": "capsule", "radius": 0.065 * u * bw.sqrt(), "length": 0.2 * u, "position": [0.0, -0.15 * u, 0.0], "material": pants},
            {"id": format!("{id}_schuh_{side}"), "type": "rounded_box", "size": [0.12 * u, 0.08 * u, 0.2 * u], "radius": 0.035 * u, "position": [0.0, -0.32 * u, 0.035 * u], "material": shoes}
          ]})
    };
    let arm = |side: f32| {
        let f: Box<dyn Fn(&Pose) -> [f32; 3]> = if side < 0.0 { Box::new(|p: &Pose| [p.arm_l[0], 0.0, p.arm_l[1]]) } else { Box::new(|p: &Pose| [p.arm_r[0], 0.0, p.arm_r[1]]) };
        let rest = if n == 0 { json!([0.0, 0.0, side * 8.0]) } else { json!([0.0, 0.0, 0.0]) };
        let mut children = vec![
            json!({"id": format!("{id}_aermel_{side}"), "type": "capsule", "radius": 0.05 * u, "length": 0.17 * u, "position": [0.0, -0.12 * u, 0.0], "material": shirt}),
            json!({"id": format!("{id}_hand_{side}"), "type": "sphere", "radius": 0.055 * u, "position": [0.0, -0.29 * u, 0.0], "material": skin}),
        ];
        let held = if side > 0.0 { spec.hold.as_ref() } else { spec.hold_left.as_ref() };
        if let Some(h) = held {
            // Gegenstand sitzt in der Hand; die Hand-Gruppe gleicht die Armhaltung nicht aus –
            // so schwingt er natürlich mit.
            let mut obj = serde_json::to_value(h.as_ref()).unwrap_or(Value::Null);
            if let Value::Object(m) = &mut obj {
                let base = m.get("position").cloned().unwrap_or(json!([0.0, 0.0, 0.0]));
                let b = |i: usize| base.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32;
                m.insert("position".into(), json!([b(0), -0.32 * u + b(1), 0.03 * u + b(2)]));
                let old = m.get("id").and_then(Value::as_str).unwrap_or("gegenstand").to_string();
                m.insert("id".into(), json!(format!("{id}_haelt_{side}_{old}")));
            }
            children.push(obj);
        }
        json!({"id": format!("{id}_arm_{}", if side < 0.0 { "l" } else { "r" }), "type": "group",
          "position": [side * 0.2 * u * bw, 0.62 * u - hip_y, 0.0], "rotation": rest, "animate": anim_rot(&*f),
          "children": children})
    };
    let torso = json!({"id": format!("{id}_oberkoerper"), "type": "capsule", "radius": 0.165 * u * bw, "length": 0.12 * u,
        "position": [0.0, 0.53 * u - hip_y, 0.0], "scale": [1.0, 1.0, 0.82], "material": shirt, "animate": {"breathe": 0.015}});
    let belly = json!({"id": format!("{id}_hose"), "type": "sphere", "radius": 0.16 * u * bw, "position": [0.0, 0.4 * u - hip_y, 0.0],
        "scale": [1.0, 0.6, 0.84], "material": pants});
    let neck = json!({"id": format!("{id}_hals"), "type": "cylinder", "radius": 0.05 * u, "height": 0.08 * u, "position": [0.0, head_y - hip_y - 0.02 * u, 0.0], "material": skin});
    let mut head_rel = head;
    head_rel["position"] = json!([0.0, head_y - hip_y, 0.0]);
    let hip_anim = if n == 0 {
        Value::Null
    } else {
        json!({
          "position": Value::Array(times.iter().zip(&poses).map(|(t, p)| json!({"t": t, "v": [p.hip_x * u, hip_y + p.hip_y * u, 0.0], "ease": "linear"})).collect()),
          "rotation": keys3(&|p: &Pose| [p.hip_rot_x, p.hip_rot_y, 0.0])
        })
    };
    let mut body_children = vec![leg(-1.0), leg(1.0), belly, torso, neck, arm(-1.0), arm(1.0), head_rel];
    if spec.outfit == Outfit::Dress {
        // Kleid: Glocke von der Taille bis über die Knie
        body_children.push(json!({"id": format!("{id}_kleid"), "type": "lathe", "segments": 32,
            "profile": [[0.0, 0.47 * u - hip_y], [0.15 * u * bw, 0.47 * u - hip_y], [0.2 * u * bw, 0.33 * u - hip_y], [0.27 * u * bw, 0.17 * u - hip_y], [0.26 * u * bw, 0.15 * u - hip_y], [0.0, 0.15 * u - hip_y]],
            "material": mat(spec.shirt, 0.75)}));
    }
    if let Some(c) = spec.scarf {
        body_children.push(json!({"id": format!("{id}_schal"), "type": "torus", "radius": 0.085 * u, "thickness": 0.035 * u,
            "position": [0.0, head_y - hip_y - 0.03 * u, 0.0], "material": mat(c, 0.9)}));
        body_children.push(json!({"id": format!("{id}_schalende"), "type": "rounded_box", "size": [0.06 * u, 0.16 * u, 0.03 * u], "radius": 0.012 * u,
            "position": [0.05 * u, head_y - hip_y - 0.12 * u, 0.12 * u], "rotation": [10.0, 0.0, 8.0], "material": mat(c, 0.9)}));
    }
    if let Some(c) = spec.backpack {
        body_children.push(json!({"id": format!("{id}_rucksack"), "type": "rounded_box", "size": [0.24 * u * bw, 0.26 * u, 0.12 * u], "radius": 0.04 * u,
            "position": [0.0, 0.56 * u - hip_y, -0.17 * u * bw], "material": mat(c, 0.7)}));
    }
    let hips = json!({"id": format!("{id}_huefte"), "type": "group", "position": [0.0, hip_y, 0.0], "animate": hip_anim,
        "rotation": [0.0, if n == 0 { look * 0.3 } else { 0.0 }, 0.0],
        "children": body_children});
    // Stauchen & Strecken: ganze Figur vom Boden aus skalieren (Füße bleiben am Boden, Volumen bleibt)
    let squash_anim = if n > 0 && poses.iter().any(|p| p.squash.abs() > 1e-3) {
        json!({"scale": Value::Array(times.iter().zip(&poses).map(|(t, p)| {
            let sy = (1.0 + p.squash).clamp(0.6, 1.5);
            let sx = 1.0 / sy.sqrt();
            json!({"t": t, "v": [sx, sy, sx], "ease": "linear"})
        }).collect())})
    } else {
        Value::Null
    };
    let root = json!({"id": format!("{id}_figur"), "type": "group", "animate": squash_anim, "children": [hips]});
    let mut v = root;
    strip_nulls(&mut v);
    serde_json::from_value(v).expect("Figur ist ein gültiges Objekt")
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

    fn count(o: &Object) -> usize {
        1 + match &o.kind {
            Kind::Group { children } => children.iter().map(count).sum(),
            _ => 0,
        }
    }

    #[test]
    fn builds_valid_character_with_actions_and_auto_walk() {
        let s = Scene::from_json(
            r#"{"duration": 8, "objects": [{"id": "max", "type": "character", "hair": "spiky", "expression": "happy", "outline": true,
                 "actions": [{"type": "wave", "start": 5, "end": 7}],
                 "talk": {"audio": "x.wav", "start": 5},
                 "animate": {"path": {"points": [[0,0,0],[4,0,0]], "start": 0, "end": 4}}}]}"#,
        )
        .unwrap();
        let o = &s.objects[0];
        let Kind::Character(spec) = &o.kind else { panic!() };
        let fig = build(o, spec);
        assert!(count(&fig) > 25, "viele Teile: {}", count(&fig));
        // Als eigene Szene gültig (alle ids eindeutig, alle Felder erlaubt)
        let probe = Scene { objects: vec![fig.clone()], ..Scene::from_json("{}").unwrap() };
        assert!(probe.validate().is_empty(), "{:?}", probe.validate());
        // Beine bewegen sich während des Weges (automatisches Gehen)
        let json = serde_json::to_string(&fig).unwrap();
        assert!(json.contains("max_bein_l") && json.contains("\"talk\""));
        let p_walk = total_pose(&[(ActionKind::Walk, 0.0, 4.0, 1.0)], 1.13);
        assert!(p_walk.leg_l.abs() > 5.0);
        let p_rest = total_pose(&[(ActionKind::Walk, 0.0, 4.0, 1.0)], 6.0);
        assert!(p_rest.leg_l.abs() < 1e-3);
    }
}
