//! Das Szenen-Dokument. Es ist die einzige Wahrheit über eine Szene:
//! Die Engine rendert es, Claude liest und schreibt es, die UI zeigt es an.
//!
//! Gestaltungsregeln:
//! - Jedes Objekt hat eine stabile `id`, damit Claude es gezielt ändern kann.
//! - Alles hat sinnvolle Standardwerte, damit kurze Beschreibungen genügen.
//! - Einheiten: Meter, Grad, Sekunden. Y zeigt nach oben.

use serde::{Deserialize, Serialize};

use crate::color::Rgb;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub name: String,
    /// Klartext: Was passiert in dieser Szene? Wird in der UI angezeigt.
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub environment: Environment,
    #[serde(default)]
    pub camera: CameraSpec,
    #[serde(default)]
    pub objects: Vec<Object>,
}

fn default_version() -> u32 {
    FORMAT_VERSION
}

// ---------------------------------------------------------------------------
// Umgebung
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Environment {
    pub sky: Sky,
    pub sun: Sun,
    pub fog: Option<Fog>,
    /// Helligkeit des Umgebungslichts (0 = aus, 1 = normal).
    pub ambient: f32,
    /// Belichtungskorrektur in Blendenstufen (+1 = doppelt so hell).
    pub exposure: f32,
    /// Glühen heller Stellen (0 = aus, ~0.15 = natürlich, 0.5 = stark).
    pub bloom: f32,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            sky: Sky::default(),
            sun: Sun::default(),
            fog: None,
            ambient: 1.0,
            exposure: 0.0,
            bloom: 0.15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Sky {
    /// Physikalisch simulierter Himmel; Farbe ergibt sich aus dem Sonnenstand.
    #[default]
    Atmosphere,
    /// Einfarbiger Hintergrund (z. B. Studio, Weltraum, Motion Graphics).
    Color { color: Rgb },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Sun {
    /// Himmelsrichtung in Grad (0 = Norden/-Z, 90 = Osten/+X).
    pub azimuth: f32,
    /// Höhe über dem Horizont in Grad (0 = Horizont, 90 = Zenit).
    pub elevation: f32,
    /// Stärke relativ zu normalem Sonnenlicht.
    pub intensity: f32,
    pub color: Rgb,
    pub shadows: bool,
}

impl Default for Sun {
    fn default() -> Self {
        Self {
            azimuth: 135.0,
            elevation: 35.0,
            intensity: 1.0,
            color: Rgb::WHITE,
            shadows: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Fog {
    /// Sichtweite in Metern.
    pub visibility: f32,
    #[serde(default = "default_fog_color")]
    pub color: Rgb,
}

fn default_fog_color() -> Rgb {
    Rgb::new(0.7, 0.75, 0.85)
}

// ---------------------------------------------------------------------------
// Kamera
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct CameraSpec {
    pub position: [f32; 3],
    pub look_at: [f32; 3],
    /// Vertikaler Bildwinkel in Grad.
    pub fov: f32,
}

impl Default for CameraSpec {
    fn default() -> Self {
        Self {
            position: [0.0, 5.0, 20.0],
            look_at: [0.0, 0.0, 0.0],
            fov: 50.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Objekte
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Object {
    /// Eindeutige, stabile Kennung, z. B. `"pyramid_main"`.
    pub id: String,
    /// Optionaler Anzeigename (sonst wird die id verwendet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    #[serde(default)]
    pub position: [f32; 3],
    /// Drehung in Grad um X, Y, Z.
    #[serde(default)]
    pub rotation: [f32; 3],
    #[serde(default)]
    pub scale: Scale,
    #[serde(default)]
    pub material: Material,
    #[serde(flatten)]
    pub kind: Kind,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Object {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Scale {
    Uniform(f32),
    PerAxis([f32; 3]),
}

impl Default for Scale {
    fn default() -> Self {
        Scale::Uniform(1.0)
    }
}

impl Scale {
    pub fn to_array(self) -> [f32; 3] {
        match self {
            Scale::Uniform(s) => [s, s, s],
            Scale::PerAxis(a) => a,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Material {
    pub color: Rgb,
    /// 0 = spiegelglatt, 1 = matt.
    pub roughness: f32,
    /// 0 = Kunststoff/Stein, 1 = Metall.
    pub metallic: f32,
    /// Selbstleuchtende Farbe (schwarz = aus).
    pub emissive: Rgb,
    /// Leuchtstärke der `emissive`-Farbe.
    pub emissive_strength: f32,
    /// 1 = deckend, 0 = unsichtbar.
    pub opacity: f32,
    /// Flächenschattierung (facettiert) statt weicher Übergänge.
    pub flat: bool,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            color: Rgb::new(0.8, 0.8, 0.8),
            roughness: 0.6,
            metallic: 0.0,
            emissive: Rgb::BLACK,
            emissive_strength: 1.0,
            opacity: 1.0,
            flat: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Kind {
    /// Quader. `size` = Breite, Höhe, Tiefe.
    Box {
        #[serde(default = "one3")]
        size: [f32; 3],
    },
    Sphere {
        #[serde(default = "half")]
        radius: f32,
    },
    Cylinder {
        #[serde(default = "half")]
        radius: f32,
        #[serde(default = "one")]
        height: f32,
    },
    Cone {
        #[serde(default = "half")]
        radius: f32,
        #[serde(default = "one")]
        height: f32,
    },
    Capsule {
        #[serde(default = "quarter")]
        radius: f32,
        #[serde(default = "one")]
        length: f32,
    },
    Torus {
        #[serde(default = "half")]
        radius: f32,
        #[serde(default = "tenth")]
        thickness: f32,
    },
    /// Flache Ebene. `size` = Breite, Tiefe.
    Plane {
        #[serde(default = "ten2")]
        size: [f32; 2],
    },
    /// Pyramide mit rechteckiger Grundfläche, Basis bei y = 0.
    Pyramid {
        #[serde(default = "one3")]
        size: [f32; 3],
    },
    /// Prozedurales Gelände.
    Terrain(TerrainSpec),
    /// Viele Kopien einer Form, zufällig verteilt (Steine, Bäume, Sterne …).
    Scatter(ScatterSpec),
    /// Unsichtbarer Container; Kinder erben die Transformation.
    Group { children: Vec<Object> },
    PointLight(LightSpec),
}

fn one() -> f32 {
    1.0
}
fn half() -> f32 {
    0.5
}
fn quarter() -> f32 {
    0.25
}
fn tenth() -> f32 {
    0.1
}
fn one3() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn ten2() -> [f32; 2] {
    [10.0, 10.0]
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TerrainStyle {
    #[default]
    Hills,
    Dunes,
    Mountains,
    Flat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TerrainSpec {
    #[serde(default = "default_terrain_size")]
    pub size: [f32; 2],
    /// Gitterpunkte pro Seite (mehr = detaillierter, langsamer).
    #[serde(default = "default_terrain_resolution")]
    pub resolution: u32,
    /// Maximale Höhe in Metern.
    #[serde(default = "default_terrain_height")]
    pub height: f32,
    #[serde(default)]
    pub style: TerrainStyle,
    /// Größe der Strukturen in Metern (größer = weitläufiger).
    #[serde(default = "default_feature_size")]
    pub feature_size: f32,
    #[serde(default)]
    pub seed: u32,
    /// Bereich um den Mittelpunkt, der flach bleibt (z. B. für Gebäude).
    #[serde(default)]
    pub flat_center_radius: f32,
}

fn default_terrain_size() -> [f32; 2] {
    [200.0, 200.0]
}
fn default_terrain_resolution() -> u32 {
    256
}
fn default_terrain_height() -> f32 {
    8.0
}
fn default_feature_size() -> f32 {
    40.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScatterSpec {
    /// Die Form, die vervielfältigt wird (ohne id; Material kommt vom Scatter-Objekt).
    pub shape: Box<ShapeOnly>,
    pub count: u32,
    /// Fläche (Breite, Tiefe) um die Position des Scatter-Objekts.
    #[serde(default = "default_scatter_area")]
    pub area: [f32; 2],
    #[serde(default)]
    pub seed: u32,
    /// Zufällige Größe zwischen min und max.
    #[serde(default = "default_scale_range")]
    pub scale_range: [f32; 2],
    #[serde(default = "default_true")]
    pub random_rotation: bool,
    /// id eines Terrain-Objekts: Kopien werden auf dessen Oberfläche gesetzt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_terrain: Option<String>,
    /// Wie tief Kopien in den Boden einsinken (Anteil ihrer Größe).
    #[serde(default)]
    pub sink: f32,
    /// Keine Kopien innerhalb dieses Radius um die Mitte.
    #[serde(default)]
    pub keep_clear_radius: f32,
}

fn default_scatter_area() -> [f32; 2] {
    [50.0, 50.0]
}
fn default_scale_range() -> [f32; 2] {
    [0.7, 1.3]
}
fn default_true() -> bool {
    true
}

/// Eine Form ohne eigene id/Transformation, für Scatter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShapeOnly {
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LightSpec {
    #[serde(default = "default_light_color")]
    pub color: Rgb,
    /// Lichtstärke in Lumen (Glühbirne ≈ 800, Flutlicht ≈ 100000).
    #[serde(default = "default_light_intensity")]
    pub intensity: f32,
    /// Reichweite in Metern.
    #[serde(default = "default_light_range")]
    pub range: f32,
    #[serde(default)]
    pub shadows: bool,
}

fn default_light_color() -> Rgb {
    Rgb::WHITE
}
fn default_light_intensity() -> f32 {
    10_000.0
}
fn default_light_range() -> f32 {
    30.0
}

// ---------------------------------------------------------------------------
// Laden & Prüfen
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum SceneError {
    Parse(String),
    Invalid(Vec<String>),
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneError::Parse(e) => write!(f, "Szene konnte nicht gelesen werden: {e}"),
            SceneError::Invalid(errs) => {
                writeln!(f, "Szene ist ungültig:")?;
                for e in errs {
                    writeln!(f, "  - {e}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for SceneError {}

impl Scene {
    pub fn from_json(text: &str) -> Result<Scene, SceneError> {
        let scene: Scene =
            serde_json::from_str(text).map_err(|e| SceneError::Parse(e.to_string()))?;
        let problems = scene.validate();
        if problems.is_empty() {
            Ok(scene)
        } else {
            Err(SceneError::Invalid(problems))
        }
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("Szene ist immer serialisierbar")
    }

    /// Liefert alle Probleme als lesbare Sätze (leer = alles gut).
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.version > FORMAT_VERSION {
            problems.push(format!(
                "Format-Version {} ist neuer als unterstützt ({FORMAT_VERSION})",
                self.version
            ));
        }
        let mut ids = std::collections::HashSet::new();
        let mut terrains = std::collections::HashSet::new();
        self.walk(&mut |o| {
            if o.id.trim().is_empty() {
                problems.push("Ein Objekt hat eine leere id".into());
            } else if !ids.insert(o.id.clone()) {
                problems.push(format!("id \"{}\" ist doppelt vergeben", o.id));
            }
            if matches!(o.kind, Kind::Terrain(_)) {
                terrains.insert(o.id.clone());
            }
        });
        self.walk(&mut |o| check_object(o, &terrains, &mut problems));
        problems
    }

    /// Besucht alle Objekte inklusive Gruppen-Kindern.
    pub fn walk<'a>(&'a self, f: &mut dyn FnMut(&'a Object)) {
        fn rec<'a>(objs: &'a [Object], f: &mut dyn FnMut(&'a Object)) {
            for o in objs {
                f(o);
                if let Kind::Group { children } = &o.kind {
                    rec(children, f);
                }
            }
        }
        rec(&self.objects, f);
    }

    pub fn find(&self, id: &str) -> Option<&Object> {
        let mut found = None;
        self.walk(&mut |o| {
            if found.is_none() && o.id == id {
                found = Some(o);
            }
        });
        found
    }
}

fn check_object(o: &Object, terrains: &std::collections::HashSet<String>, p: &mut Vec<String>) {
    let id = &o.id;
    let positive = |name: &str, v: f32, p: &mut Vec<String>| {
        if !(v > 0.0) || !v.is_finite() {
            p.push(format!("\"{id}\": {name} muss größer als 0 sein (ist {v})"));
        }
    };
    for (i, s) in o.scale.to_array().iter().enumerate() {
        if *s == 0.0 || !s.is_finite() {
            p.push(format!("\"{id}\": scale[{i}] darf nicht 0 sein"));
        }
    }
    if !(0.0..=1.0).contains(&o.material.opacity) {
        p.push(format!("\"{id}\": material.opacity muss zwischen 0 und 1 liegen"));
    }
    match &o.kind {
        Kind::Terrain(t) => {
            if !(2..=2048).contains(&t.resolution) {
                p.push(format!("\"{id}\": resolution muss zwischen 2 und 2048 liegen"));
            }
            positive("size[0]", t.size[0], p);
            positive("size[1]", t.size[1], p);
            positive("feature_size", t.feature_size, p);
        }
        Kind::Scatter(s) => {
            if s.count > 200_000 {
                p.push(format!("\"{id}\": count {} ist zu groß (max. 200000)", s.count));
            }
            if matches!(s.shape.kind, Kind::Scatter(_) | Kind::Group { .. } | Kind::Terrain(_)) {
                p.push(format!("\"{id}\": scatter.shape muss eine einfache Form sein"));
            }
            if s.scale_range[0] > s.scale_range[1] {
                p.push(format!("\"{id}\": scale_range min ist größer als max"));
            }
            if let Some(t) = &s.on_terrain {
                if !terrains.contains(t) {
                    p.push(format!("\"{id}\": on_terrain verweist auf unbekanntes Terrain \"{t}\""));
                }
            }
        }
        Kind::Sphere { radius } => positive("radius", *radius, p),
        Kind::Cylinder { radius, height } | Kind::Cone { radius, height } => {
            positive("radius", *radius, p);
            positive("height", *height, p);
        }
        _ => {}
    }
}
