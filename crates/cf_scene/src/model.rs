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
    /// Länge der Szene in Sekunden.
    #[serde(default = "default_duration")]
    pub duration: f32,
    /// Bilder pro Sekunde beim Export.
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default)]
    pub environment: Environment,
    #[serde(default)]
    pub camera: CameraSpec,
    #[serde(default)]
    pub objects: Vec<Object>,
    /// Einblendungen, die fest im Bild stehen (Untertitel, Bauchbinden, Daten).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlays: Vec<Overlay>,
}

/// Text, der fest im Bild steht – unabhängig von der Kamera.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Overlay {
    pub text: String,
    /// Waagerechte Position: -1 = linker Rand, 0 = Mitte, 1 = rechter Rand.
    #[serde(default)]
    pub x: f32,
    /// Senkrechte Position (Grundlinie): -1 = unten, 0 = Mitte, 1 = oben.
    #[serde(default = "default_overlay_y")]
    pub y: f32,
    /// Schriftgröße als Anteil der kürzeren Bildseite (0.05 = 5 %; im Querformat die Höhe, im Hochformat die Breite).
    #[serde(default = "default_overlay_size")]
    pub size: f32,
    #[serde(default = "white")]
    pub color: Rgb,
    #[serde(default)]
    pub weight: TextWeight,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub align: TextAlign,
    /// Sichtbar ab (s).
    #[serde(default)]
    pub start: f32,
    /// Sichtbar bis (s); Standard: bis zum Ende.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<f32>,
    /// Ein-/Ausblendzeit (s).
    #[serde(default = "default_overlay_fade")]
    pub fade: f32,
    /// Auftritt: "fade" (Standard), "pop" (ploppt auf), "slide_up" (gleitet von unten herein),
    /// "typewriter" (erscheint Buchstabe für Buchstabe).
    #[serde(default, skip_serializing_if = "OverlayEffect::is_fade")]
    pub effect: OverlayEffect,
    /// Weicher dunkler Schatten hinter der Schrift (besser lesbar auf hellen Bildern).
    #[serde(default, skip_serializing_if = "is_false")]
    pub shadow: bool,
    /// Hintergrund-Box (z. B. für Untertitel): Farbe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Rgb>,
    /// Deckkraft der Hintergrund-Box (Standard 0.6).
    #[serde(default = "default_bg_opacity")]
    pub background_opacity: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverlayEffect {
    #[default]
    Fade,
    Pop,
    SlideUp,
    Typewriter,
}

impl OverlayEffect {
    pub fn is_fade(&self) -> bool {
        *self == OverlayEffect::Fade
    }
}

fn default_bg_opacity() -> f32 {
    0.6
}


fn default_overlay_y() -> f32 {
    -0.8
}
fn default_overlay_size() -> f32 {
    0.05
}
fn default_overlay_fade() -> f32 {
    0.3
}
fn white() -> Rgb {
    Rgb::WHITE
}

fn default_duration() -> f32 {
    5.0
}
fn default_fps() -> u32 {
    30
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
    /// Bildstil: "film" (natürliche Tonwerte) oder "cartoon" (Farben exakt wie angegeben).
    /// Ohne Angabe: "cartoon" bei 2D-Kamera (`camera.orthographic`), sonst "film".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub look: Option<Look>,
    /// Kontaktschatten in Ecken und Fugen (Umgebungsverdeckung). Standard: an im Film-Stil, aus im Cartoon-Stil.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ao: Option<bool>,
    /// Sternenhimmel für Nachtszenen (0 = aus, 1 = normal, 2 = sehr viele). Nur bei Nacht sichtbar
    /// (Sonne unter dem Horizont oder dunkler Farb-Himmel).
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub stars: f32,
    /// Mond: kühles Mondlicht mit Schatten + sichtbare Mondscheibe (für Nachtszenen).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moon: Option<Moon>,
    /// Sichtbare Lichtstrahlen der Sonne im Dunst: 0 = aus, 0.3 = dezent, 1 = stark.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub light_rays: f32,
    /// Farbkorrektur (Sättigung, Vignette). Farbstimmung warm/kalt: über `sun.color` und den Himmel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade: Option<Grade>,
    /// Änderungen über die Zeit (Sonnenstand, Belichtung).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animate: Option<crate::animation::EnvironmentAnimation>,
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
            look: None,
            ao: None,
            stars: 0.0,
            moon: None,
            light_rays: 0.0,
            grade: None,
            animate: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Look {
    /// Filmische Tonwertkurve: natürlich bei hellem Sonnenlicht, Farben etwas gedämpft.
    Film,
    /// Farben exakt wie angegeben (Cartoons, 2D, Grafiken, Logos).
    Cartoon,
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
    /// 2D-Kamera ohne Perspektive: sichtbare Höhe in Metern (z. B. 10). Ideal für Cartoons/2D.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orthographic: Option<f32>,
    /// Kamerafahrt über die Zeit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animate: Option<crate::animation::CameraAnimation>,
    /// Tiefenschärfe wie bei einer Kinokamera (Hintergrund/Vordergrund weich).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dof: Option<DepthOfFieldSpec>,
    /// Bewegungsunschärfe: Verschlusswinkel 0..1 (0.5 = 180°, Kino-Standard).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion_blur: Option<f32>,
    /// Immer auf dieses Objekt schauen (ersetzt look_at).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Blickpunkt relativ zum Objekt (z. B. [0, 1.5, 0] = Kopfhöhe einer Figur).
    #[serde(skip_serializing_if = "is_zero3")]
    pub target_offset: [f32; 3],
    /// Dem Objekt folgen (Position = Objekt + offset, weich nachgezogen).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow: Option<crate::motion::Follow>,
    /// Um ein Objekt oder einen Punkt kreisen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orbit: Option<crate::motion::Orbit>,
    /// Einstellungsgröße auf ein Objekt (Totale, Halbnah, Nah, Großaufnahme …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<crate::motion::Frame>,
    /// Harte Schnitte innerhalb der Szene (Schuss/Gegenschuss): ab `t` gilt die Einstellung.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cuts: Vec<crate::motion::CameraCut>,
}

fn is_zero3(v: &[f32; 3]) -> bool {
    *v == [0.0; 3]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DepthOfFieldSpec {
    /// Worauf scharf gestellt wird: Entfernung in Metern oder id eines Objekts (folgt ihm).
    pub focus: Focus,
    /// Blende: 1.4 = sehr weicher Hintergrund, 2.8 = Kino-Standard, 8 = fast alles scharf.
    #[serde(default = "default_aperture")]
    pub aperture: f32,
}

fn default_aperture() -> f32 {
    2.8
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Focus {
    Distance(f32),
    Object(String),
}

/// Mond am Nachthimmel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Moon {
    pub azimuth: f32,
    pub elevation: f32,
    /// Helligkeit des Mondlichts (1 = filmische Nacht, 2 = Vollmond-hell).
    pub intensity: f32,
    pub color: Rgb,
    /// Größe der Mondscheibe (1 = normal, 3 = riesig wie im Märchenfilm).
    pub size: f32,
}

impl Default for Moon {
    fn default() -> Self {
        Self { azimuth: 120.0, elevation: 30.0, intensity: 1.0, color: Rgb::new(0.72, 0.8, 1.0), size: 1.0 }
    }
}

/// Farbkorrektur („Grading“) für den Look des Films.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Grade {
    /// Farbsättigung (1 = normal, 1.2 = kräftig, 0 = schwarzweiß).
    pub saturation: f32,
    /// Abgedunkelte Bildränder (0 = aus, 0.3 = dezent, 1 = stark).
    pub vignette: f32,
}

impl Default for Grade {
    fn default() -> Self {
        Self { saturation: 1.0, vignette: 0.0 }
    }
}

impl Default for CameraSpec {
    fn default() -> Self {
        Self {
            position: [0.0, 5.0, 20.0],
            look_at: [0.0, 0.0, 0.0],
            fov: 50.0,
            orthographic: None,
            animate: None,
            dof: None,
            motion_blur: None,
            target: None,
            target_offset: [0.0; 3],
            follow: None,
            orbit: None,
            frame: None,
            cuts: Vec::new(),
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
    /// Wiederholt das Objekt (Reihen, Kreise, Treppen, Spiralen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<Repeat>,
    /// Animation über die Zeit (Keyframes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animate: Option<crate::animation::Animation>,
    /// Feinere Unterteilung der Oberfläche (0–5, jede Stufe ×4 Dreiecke) – Grundlage für
    /// Verformungen und glatte, organische Formen.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub subdivide: u32,
    /// Beim Unterteilen abrunden: aus Würfeln werden Kiesel, aus Kanten Rundungen.
    #[serde(default, skip_serializing_if = "is_false")]
    pub rounded: bool,
    /// Verformungen in dieser Reihenfolge (verdrehen, biegen, verjüngen, Welle, aufblähen, Rauschen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deform: Vec<crate::shaping::Deform>,
    /// Schaut immer zur Kamera (2D-Figuren und Bilder in 3D-Welten, wie Pappaufsteller).
    #[serde(default, skip_serializing_if = "is_false")]
    pub billboard: bool,
    #[serde(flatten)]
    pub kind: Kind,
}

fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

/// Wiederholung: Kopie i bekommt i-mal den Schritt angewendet:
/// um `pivot` drehen (`rotation`), skalieren (`scale`), dann um `offset` verschieben.
///
/// - Reihe: `offset: [2, 0, 0]`
/// - Kreis (z. B. 12 Säulen, Radius 5): `pivot: [-5, 0, 0]`, `rotation: [0, 30, 0]`
/// - Wendeltreppe: `pivot: [-1.5, 0, 0]`, `rotation: [0, 20, 0]`, `offset: [0, 0.2, 0]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Repeat {
    pub count: u32,
    #[serde(default)]
    pub offset: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 3],
    #[serde(default = "one")]
    pub scale: f32,
    /// Drehpunkt relativ zur Position des Objekts (Standard: das Objekt selbst).
    #[serde(default)]
    pub pivot: [f32; 3],
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
    /// Durchsichtigkeit wie Glas/Wasser (0 = aus, 1 = klares Glas).
    pub transmission: f32,
    /// Brechungsindex (Glas 1.5, Wasser 1.33, Diamant 2.4).
    pub ior: f32,
    /// Klarlack-Schicht (Autolack, lackiertes Holz), 0..1.
    pub clearcoat: f32,
    /// Ohne Beleuchtung (reine Farbe, z. B. für Grafiken/Hologramme).
    pub unlit: bool,
    /// Beide Seiten sichtbar (Blätter, Stoff, offene Flächen).
    pub double_sided: bool,
    /// Oberflächen-Muster (Ziegel, Raster, Rauschen …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<Pattern>,
    /// Bild als Oberfläche (Datei im Ordner Bilder/ des Projekts), z. B. Logo, Foto, Plakat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    /// Cartoon-Umrisslinie um das Objekt (wie gezeichnet), z. B. `{"width": 0.02}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<Outline>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    #[default]
    Noise,
    Checker,
    Stripes,
    Bricks,
    Grid,
    Dots,
    Marble,
    Wood,
}

/// Prozedurales Muster: mischt `color` (Material) und `color2`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Pattern {
    #[serde(rename = "type")]
    pub kind: PatternKind,
    #[serde(default = "default_color2")]
    pub color2: Rgb,
    /// Wiederholungen pro Meter bzw. pro Fläche (größer = feiner).
    #[serde(default = "one")]
    pub scale: f32,
    /// Plastische Struktur (0 = glatt, 1 = deutlich).
    #[serde(default)]
    pub bump: f32,
    #[serde(default)]
    pub seed: u32,
}

fn default_color2() -> Rgb {
    Rgb::new(0.2, 0.2, 0.2)
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
            transmission: 0.0,
            ior: 1.5,
            clearcoat: 0.0,
            unlit: false,
            double_sided: false,
            pattern: None,
            texture: None,
            outline: None,
        }
    }
}

/// Umrisslinie für den Cartoon-Look.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Outline {
    /// Linienstärke in Metern (am Objekt gemessen).
    #[serde(default = "default_outline_width")]
    pub width: f32,
    #[serde(default = "black")]
    pub color: Rgb,
}

fn default_outline_width() -> f32 {
    0.02
}

fn black() -> Rgb {
    Rgb::BLACK
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
    /// Quader mit abgerundeten Kanten.
    RoundedBox {
        #[serde(default = "one3")]
        size: [f32; 3],
        #[serde(default = "tenth")]
        radius: f32,
        #[serde(default = "four")]
        segments: u32,
    },
    /// Umriss (in der XZ-Ebene, von oben gesehen) nach oben gezogen, Basis bei y = 0.
    Extrude(ExtrudeSpec),
    /// Profil [Radius, Höhe] um die Y-Achse gedreht (Vasen, Säulen, Kuppeln).
    Lathe(LatheSpec),
    /// Röhre entlang eines Pfads (Kabel, Rohre, Äste).
    Tube(TubeSpec),
    /// 3D-Schrift in der XY-Ebene, Blickrichtung +Z, Grundlinie bei y = 0.
    Text(TextSpec),
    /// Bildfläche (Datei im Ordner Bilder/), Seitenverhältnis wie das Bild; steht in der XY-Ebene, schaut nach +Z.
    Image {
        file: String,
        /// Breite in Metern (Höhe ergibt sich aus dem Bild).
        #[serde(default = "one")]
        width: f32,
    },
    /// 2D-Grafik aus SVG (inline als `svg` oder Datei in Bilder/): Figuren, Logos, Hintergründe.
    /// Steht in der XY-Ebene, schaut nach +Z, zentriert. `depth` > 0 macht sie dreidimensional.
    Svg {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        svg: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file: Option<String>,
        /// Breite in Metern.
        #[serde(default = "one")]
        width: f32,
        #[serde(default)]
        depth: f32,
        /// "auto", "vector" (Flächen, `depth` möglich) oder "image" (volle SVG-Darstellung).
        #[serde(default, skip_serializing_if = "crate::svg::SvgMode::is_auto")]
        mode: crate::svg::SvgMode,
        /// Pixel der langen Seite im Bild-Modus (Standard: automatisch nach Größe).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resolution: Option<u32>,
    },
    /// Fertige Cartoon-Figur mit Gelenken, Gesicht und Aktionen (siehe `character`).
    Character(crate::character::CharacterSpec),
    /// Tier (Hund, Katze, Hase, Vogel, Fisch) mit Bewegungen.
    Animal(crate::animal::AnimalSpec),
    /// Baum (Laubbaum, Tanne, Palme, Birke, Cartoon) mit Wind.
    Tree(crate::nature::TreeSpec),
    /// Wasserfläche mit Wellen.
    Water(crate::nature::WaterSpec),
    /// 3D-Modell (GLB/glTF, Datei im Ordner Modelle/), z. B. aus Blender, Sketchfab oder einem KI-3D-Generator.
    Model { file: String },
    /// Partikel-Effekt (Feuer, Rauch, Funken, Schnee, Regen, Explosion, Magie, Staub, Fontäne).
    Particles(crate::particles::ParticleSpec),
    /// Objekt aus der Bibliothek.
    Instance {
        asset: String,
        /// Ersetzt das Material aller Teile (optional).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        material_override: Option<Material>,
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
fn four() -> u32 {
    4
}
fn one3() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn ten2() -> [f32; 2] {
    [10.0, 10.0]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtrudeSpec {
    /// Umriss-Punkte [x, z] (mind. 3), Reihenfolge egal.
    pub profile: Vec<[f32; 2]>,
    /// Löcher im Umriss, jeweils eine Punkt-Liste [x, z].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<[f32; 2]>>,
    #[serde(default = "one")]
    pub height: f32,
    /// Größe der Oberseite relativ zur Unterseite (1 = gerade, 0.5 = verjüngt, 0 = Spitze).
    #[serde(default = "one")]
    pub taper: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LatheSpec {
    /// Profil-Punkte [Radius, Höhe] von unten nach oben.
    pub profile: Vec<[f32; 2]>,
    #[serde(default = "default_lathe_segments")]
    pub segments: u32,
    /// Drehwinkel in Grad (360 = geschlossen).
    #[serde(default = "default_full_angle")]
    pub angle: f32,
}

fn default_lathe_segments() -> u32 {
    48
}
fn default_full_angle() -> f32 {
    360.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TubeSpec {
    /// Pfad-Punkte [x, y, z] (mind. 2).
    pub path: Vec<[f32; 3]>,
    #[serde(default = "tenth")]
    pub radius: f32,
    /// Radius am Ende (für spitz zulaufende Formen wie Äste).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_end: Option<f32>,
    /// Anzahl der Kanten im Querschnitt.
    #[serde(default = "default_tube_segments")]
    pub segments: u32,
    /// Pfad weich als Kurve durch die Punkte führen.
    #[serde(default = "default_true")]
    pub smooth: bool,
    /// Ende mit Anfang verbinden (Ring).
    #[serde(default)]
    pub closed: bool,
}

fn default_tube_segments() -> u32 {
    16
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TextSpec {
    /// Text; Zeilenumbrüche mit \n.
    pub text: String,
    /// Schriftgröße in Metern (ungefähr Höhe der Großbuchstaben × 1.4).
    #[serde(default = "one")]
    pub size: f32,
    /// Tiefe der Buchstaben in Metern.
    #[serde(default = "default_text_depth")]
    pub depth: f32,
    #[serde(default)]
    pub align: TextAlign,
    /// Zusätzlicher Buchstabenabstand (Anteil der Schriftgröße).
    #[serde(default)]
    pub letter_spacing: f32,
    /// Schriftstärke: regular, bold, black.
    #[serde(default)]
    pub weight: TextWeight,
    /// Kursiv (schräg gestellt).
    #[serde(default)]
    pub italic: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextWeight {
    #[default]
    Regular,
    Bold,
    Black,
}

fn default_text_depth() -> f32 {
    0.2
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
    /// Kontaktschatten (SSAO) an? Standard: im Film-Stil ja, im Cartoon-Stil nein.
    pub fn ao_enabled(&self) -> bool {
        self.environment.ao.unwrap_or(!self.exact_colors())
    }

    /// Sollen Farben exakt wiedergegeben werden (Cartoon-Stil)?
    pub fn exact_colors(&self) -> bool {
        match self.environment.look {
            Some(Look::Cartoon) => true,
            Some(Look::Film) => false,
            None => self.camera.orthographic.is_some(),
        }
    }

    pub fn from_json(text: &str) -> Result<Scene, SceneError> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| SceneError::Parse(format!("Kein gültiges JSON: {e}")))?;
        let scene = crate::diagnose::parse_scene(value).map_err(SceneError::Parse)?;
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
        if !(self.duration > 0.0 && self.duration <= 3.0 * 3600.0) {
            problems.push(format!("duration muss zwischen 0 und 10800 Sekunden liegen (ist {})", self.duration));
        }
        if let Some(d) = &self.camera.dof {
            if !(d.aperture > 0.1 && d.aperture <= 64.0) {
                problems.push("camera.dof.aperture muss zwischen 0.1 und 64 liegen".into());
            }
            match &d.focus {
                Focus::Distance(x) if !(*x > 0.0) => problems.push("camera.dof.focus muss größer als 0 sein".into()),
                Focus::Object(id) if crate::ops::find(self, id).is_none() => {
                    problems.push(format!("camera.dof.focus: Objekt „{id}“ gibt es nicht"))
                }
                _ => {}
            }
        }
        let cam_ids = self
            .camera
            .target
            .iter()
            .chain(self.camera.follow.as_ref().map(|f| &f.target))
            .chain(self.camera.orbit.as_ref().and_then(|o| o.target.as_ref()))
            .chain(self.camera.frame.as_ref().map(|f| &f.target))
            .chain(self.camera.cuts.iter().flat_map(|c| {
                c.target.iter().chain(c.frame.as_ref().map(|f| &f.target)).chain(c.follow.as_ref().map(|f| &f.target)).chain(c.orbit.as_ref().and_then(|o| o.target.as_ref()))
            }));
        for id in cam_ids {
            if crate::ops::find(self, id).is_none() {
                problems.push(format!("camera: Objekt „{id}“ gibt es nicht"));
            }
        }
        let mut look_targets = Vec::new();
        let mut held: Vec<Object> = Vec::new();
        self.walk(&mut |o| {
            if let Kind::Character(c) = &o.kind {
                if let Some(t) = &c.look_at {
                    look_targets.push((o.id.clone(), t.clone()));
                }
                held.extend(c.hold.iter().chain(c.hold_left.iter()).map(|h| (**h).clone()));
            }
        });
        // Gehaltene Gegenstände wie normale Objekte prüfen
        let no_terrains = std::collections::HashSet::new();
        for h in &held {
            check_object(h, &no_terrains, &mut problems);
        }
        for (who, t) in look_targets {
            if crate::ops::find(self, &t).is_none() {
                problems.push(format!("\"{who}\": look_at „{t}“ gibt es nicht"));
            }
        }
        if let Some(o) = &self.camera.orbit {
            if o.target.is_none() && o.center.is_none() {
                problems.push("camera.orbit: target (id) oder center angeben".into());
            }
        }
        if self.camera.motion_blur.is_some_and(|m| !(0.0..=1.0).contains(&m)) {
            problems.push("camera.motion_blur muss zwischen 0 und 1 liegen (0.5 = Kino-Standard)".into());
        }
        if !(0.0..=3.0).contains(&self.environment.light_rays) {
            problems.push("environment.light_rays muss zwischen 0 und 3 liegen".into());
        }
        if let Some(g) = &self.environment.grade {
            if !(0.0..=3.0).contains(&g.saturation) || !(0.0..=1.5).contains(&g.vignette) {
                problems.push("environment.grade: saturation 0–3, vignette 0–1.5".into());
            }
        }
        if !(1..=120).contains(&self.fps) {
            problems.push(format!("fps muss zwischen 1 und 120 liegen (ist {})", self.fps));
        }
        if let Some(a) = &self.camera.animate {
            a.validate(&mut problems);
        }
        if let Some(a) = &self.environment.animate {
            a.validate(&mut problems);
        }
        for (i, o) in self.overlays.iter().enumerate() {
            if o.text.trim().is_empty() {
                problems.push(format!("overlays[{i}]: text ist leer"));
            }
            if !(o.size > 0.0 && o.size <= 1.0) {
                problems.push(format!("overlays[{i}]: size muss zwischen 0 und 1 liegen (Anteil der kürzeren Bildseite)"));
            }
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
    if o.subdivide > crate::shaping::MAX_SUBDIVIDE {
        p.push(format!("\"{id}\": subdivide höchstens {} (jede Stufe vervierfacht die Dreiecke)", crate::shaping::MAX_SUBDIVIDE));
    }
    let shapes_mesh = !matches!(
        o.kind,
        Kind::Group { .. } | Kind::Instance { .. } | Kind::Particles(_) | Kind::PointLight(_) | Kind::Scatter(_) | Kind::Model { .. } | Kind::Image { .. } | Kind::Svg { .. } | Kind::Character(_) | Kind::Tree(_) | Kind::Water(_) | Kind::Animal(_)
    );
    if (o.subdivide > 0 || o.rounded || !o.deform.is_empty()) && !shapes_mesh {
        p.push(format!("\"{id}\": subdivide/rounded/deform gehen nur bei Formen (box, sphere, extrude, lathe, tube, text …), nicht bei diesem Typ"));
    }
    if let Some(ol) = &o.material.outline {
        if !(ol.width > 0.0 && ol.width < 10.0) {
            p.push(format!("\"{id}\": outline.width muss zwischen 0 und 10 liegen"));
        }
    }
    for d in &o.deform {
        if let Some(e) = d.validate() {
            p.push(format!("\"{id}\": deform {e}"));
        }
    }
    if !(0.0..=1.0).contains(&o.material.transmission) {
        p.push(format!("\"{id}\": material.transmission muss zwischen 0 und 1 liegen"));
    }
    if let Some(pt) = &o.material.pattern {
        if !(pt.scale > 0.0 && pt.scale <= 1000.0) {
            p.push(format!("\"{id}\": pattern.scale muss zwischen 0 und 1000 liegen"));
        }
    }
    if let Some(a) = &o.animate {
        a.validate(id, p);
    }
    if let Some(r) = &o.repeat {
        if r.count == 0 || r.count > 10_000 {
            p.push(format!("\"{id}\": repeat.count muss zwischen 1 und 10000 liegen"));
        }
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
            if matches!(s.shape.kind, Kind::Scatter(_) | Kind::Group { .. } | Kind::Terrain(_) | Kind::Instance { .. } | Kind::Particles(_) | Kind::Image { .. } | Kind::Svg { .. } | Kind::Model { .. } | Kind::Water(_)) {
                p.push(format!("\"{id}\": scatter.shape muss eine einfache Form, ein tree, character oder animal sein"));
            }
            if matches!(s.shape.kind, Kind::Tree(_)) && s.count > 1500 {
                p.push(format!("\"{id}\": höchstens 1500 Bäume pro scatter"));
            }
            if matches!(s.shape.kind, Kind::Character(_) | Kind::Animal(_)) && s.count > 300 {
                p.push(format!("\"{id}\": höchstens 300 Figuren/Tiere pro scatter"));
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
        Kind::RoundedBox { size, radius, segments } => {
            for (i, v) in size.iter().enumerate() {
                positive(&format!("size[{i}]"), *v, p);
            }
            let min = size.iter().cloned().fold(f32::MAX, f32::min);
            if *radius < 0.0 || *radius > min / 2.0 {
                p.push(format!("\"{id}\": radius muss zwischen 0 und der halben kleinsten Kante ({}) liegen", min / 2.0));
            }
            if *segments == 0 || *segments > 32 {
                p.push(format!("\"{id}\": segments muss zwischen 1 und 32 liegen"));
            }
        }
        Kind::Extrude(e) => {
            if e.profile.len() < 3 {
                p.push(format!("\"{id}\": extrude.profile braucht mindestens 3 Punkte"));
            }
            if e.holes.iter().any(|h| h.len() < 3) {
                p.push(format!("\"{id}\": jedes Loch braucht mindestens 3 Punkte"));
            }
            positive("height", e.height, p);
            if !(0.0..=10.0).contains(&e.taper) {
                p.push(format!("\"{id}\": taper muss zwischen 0 und 10 liegen"));
            }
        }
        Kind::Lathe(l) => {
            if l.profile.len() < 2 {
                p.push(format!("\"{id}\": lathe.profile braucht mindestens 2 Punkte"));
            }
            if l.profile.iter().any(|q| q[0] < 0.0) {
                p.push(format!("\"{id}\": lathe-Radien dürfen nicht negativ sein"));
            }
            if !(3..=256).contains(&l.segments) {
                p.push(format!("\"{id}\": segments muss zwischen 3 und 256 liegen"));
            }
            if !(1.0..=360.0).contains(&l.angle) {
                p.push(format!("\"{id}\": angle muss zwischen 1 und 360 liegen"));
            }
        }
        Kind::Tube(t) => {
            if t.path.len() < 2 {
                p.push(format!("\"{id}\": tube.path braucht mindestens 2 Punkte"));
            }
            positive("radius", t.radius, p);
            if !(3..=64).contains(&t.segments) {
                p.push(format!("\"{id}\": segments muss zwischen 3 und 64 liegen"));
            }
        }
        Kind::Text(t) => {
            if t.text.trim().is_empty() {
                p.push(format!("\"{id}\": text darf nicht leer sein"));
            }
            if t.text.chars().count() > 500 {
                p.push(format!("\"{id}\": text ist zu lang (max. 500 Zeichen)"));
            }
            positive("size", t.size, p);
            if t.depth < 0.0 {
                p.push(format!("\"{id}\": depth darf nicht negativ sein"));
            }
        }
        Kind::Particles(ps) => ps.validate(id, p),
        Kind::Svg { svg, file, width, depth, mode, resolution } => {
            if resolution.is_some_and(|r| !(16..=8192).contains(&r)) {
                p.push(format!("\"{id}\": resolution muss zwischen 16 und 8192 liegen"));
            }
            positive("width", *width, p);
            if *depth < 0.0 {
                p.push(format!("\"{id}\": depth darf nicht negativ sein"));
            }
            match (svg, file) {
                (Some(_), Some(_)) => p.push(format!("\"{id}\": entweder svg oder file angeben, nicht beides")),
                (None, None) => p.push(format!("\"{id}\": svg (Markup) oder file fehlt")),
                (Some(s), None) => {
                    if let Err(e) = crate::svg::check(s, *width, *depth, *mode) {
                        p.push(format!("\"{id}\": {e}"));
                    }
                }
                _ => {}
            }
        }
        Kind::Character(c) => c.validate(id, p),
        Kind::Tree(t) => t.validate(id, p),
        Kind::Animal(a) => a.validate(id, p),
        Kind::Water(w) => w.validate(id, p),
        Kind::Model { file } => {
            let lower = file.to_lowercase();
            if !(lower.ends_with(".glb") || lower.ends_with(".gltf")) {
                p.push(format!("\"{id}\": Modelle müssen .glb oder .gltf sein (z. B. aus Blender exportieren)"));
            }
        }
        Kind::Image { file, width } => {
            positive("width", *width, p);
            if file.trim().is_empty() {
                p.push(format!("\"{id}\": image.file fehlt"));
            }
        }
        Kind::Instance { asset, .. } => {
            if crate::project::safe_name(asset).is_err() {
                p.push(format!("\"{id}\": ungültiger Bibliotheks-Name \"{asset}\""));
            }
        }
        Kind::Cylinder { radius, height } | Kind::Cone { radius, height } => {
            positive("radius", *radius, p);
            positive("height", *height, p);
        }
        _ => {}
    }
}
