//! 2D-Grafik aus SVG: Claude zeichnet Figuren, Logos und Hintergründe als SVG
//! (oder importiert SVG-Dateien). Jeder Pfad wird zu einer flachen (oder
//! extrudierten) Fläche mit eigener Farbe; Konturen werden zu Linien.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use usvg::tiny_skia_path::{PathSegment, Point};

use crate::color::Rgb;
use crate::modeling::{fill_contours, stroke_contour};
use crate::procedural::MeshData;

pub struct Shape2d {
    pub color: Rgb,
    pub opacity: f32,
    pub mesh: MeshData,
}

fn paint_color(p: &usvg::Paint) -> Rgb {
    let c = match p {
        usvg::Paint::Color(c) => *c,
        usvg::Paint::LinearGradient(g) => g.stops().first().map(|s| s.color()).unwrap_or(usvg::Color::black()),
        usvg::Paint::RadialGradient(g) => g.stops().first().map(|s| s.color()).unwrap_or(usvg::Color::black()),
        usvg::Paint::Pattern(_) => usvg::Color::black(),
    };
    Rgb::new(c.red as f32 / 255.0, c.green as f32 / 255.0, c.blue as f32 / 255.0)
}

/// Zerlegt Kurven in kurze Geraden. Liefert (Konturen, geschlossen?).
fn contours(path: &usvg::Path) -> Vec<(Vec<Point>, bool)> {
    let tf = path.abs_transform();
    let mut out: Vec<(Vec<Point>, bool)> = Vec::new();
    let mut cur: Vec<Point> = Vec::new();
    let mut last = Point::zero();
    let flush = |cur: &mut Vec<Point>, out: &mut Vec<(Vec<Point>, bool)>, closed: bool| {
        if cur.len() >= 2 {
            out.push((std::mem::take(cur), closed));
        } else {
            cur.clear();
        }
    };
    for seg in path.data().segments() {
        match seg {
            PathSegment::MoveTo(p) => {
                flush(&mut cur, &mut out, false);
                cur.push(p);
                last = p;
            }
            PathSegment::LineTo(p) => {
                cur.push(p);
                last = p;
            }
            PathSegment::QuadTo(c, p) => {
                for k in 1..=10 {
                    let t = k as f32 / 10.0;
                    let u = 1.0 - t;
                    cur.push(Point::from_xy(u * u * last.x + 2.0 * u * t * c.x + t * t * p.x, u * u * last.y + 2.0 * u * t * c.y + t * t * p.y));
                }
                last = p;
            }
            PathSegment::CubicTo(c1, c2, p) => {
                for k in 1..=14 {
                    let t = k as f32 / 14.0;
                    let u = 1.0 - t;
                    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    cur.push(Point::from_xy(
                        a * last.x + b * c1.x + c * c2.x + d * p.x,
                        a * last.y + b * c1.y + c * c2.y + d * p.y,
                    ));
                }
                last = p;
            }
            PathSegment::Close => flush(&mut cur, &mut out, true),
        }
    }
    flush(&mut cur, &mut out, false);
    for (pts, _) in &mut out {
        tf.map_points(pts);
    }
    out
}

fn collect<'a>(g: &'a usvg::Group, out: &mut Vec<&'a usvg::Path>) {
    for n in g.children() {
        match n {
            usvg::Node::Group(g) => collect(g, out),
            usvg::Node::Path(p) if p.is_visible() => out.push(p),
            // Text wird beim Einlesen in Pfade umgewandelt (Schrift Inter).
            usvg::Node::Text(t) => collect(t.flattened(), out),
            _ => {}
        }
    }
}

const FONT_REGULAR: &[u8] = include_bytes!("../../../apps/studio/assets/fonts/Inter-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../../apps/studio/assets/fonts/Inter-Bold.ttf");
const FONT_BLACK: &[u8] = include_bytes!("../../../apps/studio/assets/fonts/Inter-Black.ttf");

/// Einlese-Optionen mit eingebauter Schrift (Inter), damit `<text>` überall gleich aussieht.
fn options() -> usvg::Options<'static> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let db = DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        for f in [FONT_REGULAR, FONT_BOLD, FONT_BLACK] {
            db.load_font_data(f.to_vec());
        }
        db.set_sans_serif_family("Inter");
        db.set_serif_family("Inter");
        db.set_monospace_family("Inter");
        db.set_cursive_family("Inter");
        db.set_fantasy_family("Inter");
        Arc::new(db)
    });
    usvg::Options { font_family: "Inter".into(), fontdb: db.clone(), ..usvg::Options::default() }
}

fn parse(svg: &str) -> Result<usvg::Tree, String> {
    usvg::Tree::from_str(svg, &options()).map_err(|e| format!("SVG nicht lesbar: {e}"))
}

/// Wie ein SVG dargestellt wird.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SvgMode {
    /// Automatisch: „image“, wenn das SVG Verläufe, Filter (Unschärfe, Schatten), Masken,
    /// Muster, Text oder Bilder enthält, sonst „vector“. Mit `depth` > 0 immer „vector“.
    #[default]
    Auto,
    /// Jede Fläche wird ein eigenes 3D-Flächenstück: in jeder Größe scharf, `depth` möglich.
    Vector,
    /// Wie in einem Grafikprogramm gezeichnet (alle SVG-Möglichkeiten), als Bild auf einer Fläche.
    Image,
}

impl SvgMode {
    pub fn is_auto(&self) -> bool {
        *self == SvgMode::Auto
    }
}

/// Braucht dieses SVG die volle Darstellung (nicht nur einfarbige Flächen)?
pub fn needs_image(svg: &str) -> bool {
    ["Gradient", "<filter", "filter=", "<mask", "mask=", "<pattern", "<text", "<image", "clip-path", "<clipPath", "mix-blend-mode", "url(#"]
        .iter()
        .any(|k| svg.contains(k))
}

/// Tatsächlicher Modus (Vector oder Image).
pub fn resolve_mode(svg: &str, mode: SvgMode, depth: f32) -> SvgMode {
    match mode {
        _ if depth > 0.0 => SvgMode::Vector,
        SvgMode::Auto => {
            if needs_image(svg) {
                SvgMode::Image
            } else {
                SvgMode::Vector
            }
        }
        m => m,
    }
}

/// Prüft ein SVG (für die Validierung): lesbar und – im Vektor-Modus – mit sichtbaren Flächen.
pub fn check(svg: &str, width: f32, depth: f32, mode: SvgMode) -> Result<(), String> {
    match resolve_mode(svg, mode, depth) {
        SvgMode::Image => parse(svg).map(|_| ()),
        _ => svg_shapes(svg, width, depth).map(|_| ()),
    }
}

/// Gerastertes SVG: RGBA mit vormultipliziertem Alpha (keine dunklen Ränder beim Filtern).
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Zeichnet das SVG mit resvg (Verläufe, Filter, Masken, Text …); lange Seite = `max_side` Pixel.
pub fn rasterize(svg: &str, max_side: u32) -> Result<Raster, String> {
    let tree = parse(svg)?;
    let size = tree.size();
    let (sw, sh) = (size.width().max(1e-3), size.height().max(1e-3));
    let scale = max_side.clamp(16, 8192) as f32 / sw.max(sh);
    let (w, h) = (((sw * scale).round() as u32).max(1), ((sh * scale).round() as u32).max(1));
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or("Bild zu groß")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    Ok(Raster { width: w, height: h, rgba: pixmap.take() })
}

/// Seitenverhältnis Höhe/Breite des SVG.
pub fn aspect(svg: &str) -> Result<f32, String> {
    let s = parse(svg)?.size();
    Ok(s.height() / s.width().max(1e-3))
}

/// SVG → Flächen. Breite `width` in Metern, zentriert, y nach oben; `depth` > 0 extrudiert.
pub fn svg_shapes(svg: &str, width: f32, depth: f32) -> Result<Vec<Shape2d>, String> {
    let tree = parse(svg)?;
    let size = tree.size();
    let (sw, sh) = (size.width().max(1e-3), size.height().max(1e-3));
    let scale = width / sw;
    let height = sh * scale;
    let map = |p: &Point| [p.x * scale - width / 2.0, (sh - p.y) * scale - height / 2.0];
    let mut paths = Vec::new();
    collect(tree.root(), &mut paths);
    let mut shapes = Vec::new();
    // Malreihenfolge: spätere Pfade liegen minimal weiter vorne (kein Flimmern).
    // Der Abstand muss über der Tiefen-Genauigkeit der 2D-Kamera liegen (~0,1 mm).
    let layer_step = (width * 0.001).max(0.001);
    for (i, p) in paths.iter().enumerate() {
        let cs = contours(p);
        if cs.is_empty() {
            continue;
        }
        let z = i as f32 * layer_step;
        let tf = p.abs_transform();
        let tf_scale = ((tf.sx * tf.sy - tf.kx * tf.ky).abs()).sqrt().max(1e-6);
        if let Some(fill) = p.fill() {
            let rings: Vec<Vec<[f32; 2]>> = cs.iter().map(|(pts, _)| pts.iter().map(map).collect()).collect();
            let mut mesh = fill_contours(&rings, depth);
            for v in &mut mesh.positions {
                v[2] += z;
            }
            if !mesh.indices.is_empty() {
                shapes.push(Shape2d { color: paint_color(fill.paint()), opacity: fill.opacity().get(), mesh });
            }
        }
        if let Some(stroke) = p.stroke() {
            let w = stroke.width().get() * tf_scale * scale;
            let mut mesh = MeshData::default();
            for (pts, closed) in &cs {
                let ring: Vec<[f32; 2]> = pts.iter().map(map).collect();
                stroke_contour(&mut mesh, &ring, *closed, w, z + layer_step * 0.5 + depth / 2.0);
            }
            if !mesh.indices.is_empty() {
                shapes.push(Shape2d { color: paint_color(stroke.paint()), opacity: stroke.opacity().get(), mesh });
            }
        }
    }
    if shapes.is_empty() {
        return Err("SVG enthält keine sichtbaren Flächen oder Linien".into());
    }
    Ok(shapes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_cartoon_svg() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <rect x="0" y="0" width="200" height="100" fill="#87ceeb"/>
            <circle cx="50" cy="50" r="30" fill="#ffcc00" stroke="#aa7700" stroke-width="4"/>
            <path d="M120 80 C 140 20, 180 20, 190 80 Z" fill="#2e8b57" fill-opacity="0.8"/>
            <path d="M 100 10 L 150 10" stroke="black" stroke-width="2" fill="none"/>
        </svg>"##;
        let shapes = svg_shapes(svg, 4.0, 0.0).unwrap();
        assert_eq!(shapes.len(), 5, "Himmel, Sonne (+Kontur), Hügel, Linie");
        let sun = &shapes[1];
        assert!((sun.color.r - 1.0).abs() < 0.01 && (sun.color.g - 0.8).abs() < 0.01);
        assert!((shapes[3].opacity - 0.8).abs() < 0.01);
        // Größe: 4 m breit, 2 m hoch, zentriert
        let all: Vec<[f32; 3]> = shapes.iter().flat_map(|s| s.mesh.positions.clone()).collect();
        let (minx, maxx) = all.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
        let (miny, maxy) = all.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[1]), b.max(p[1])));
        assert!((minx + 2.0).abs() < 0.05 && (maxx - 2.0).abs() < 0.05, "{minx}..{maxx}");
        assert!((miny + 1.0).abs() < 0.05 && (maxy - 1.0).abs() < 0.05, "{miny}..{maxy}");
        // Sonne liegt vor dem Himmel
        let z_sky = shapes[0].mesh.positions[0][2];
        let z_sun = shapes[1].mesh.positions[0][2];
        assert!(z_sun > z_sky);
        assert!(svg_shapes("<svg", 1.0, 0.0).is_err());
    }

    #[test]
    fn gradients_filters_and_text_render_as_image() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <defs>
              <linearGradient id="g" x1="0" x2="1"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient>
              <filter id="b"><feGaussianBlur stdDeviation="4"/></filter>
            </defs>
            <rect width="200" height="100" fill="url(#g)"/>
            <circle cx="100" cy="50" r="20" fill="#000" filter="url(#b)"/>
            <text x="10" y="90" font-size="20" fill="#fff">Hallo Ü</text>
        </svg>"##;
        assert_eq!(resolve_mode(svg, SvgMode::Auto, 0.0), SvgMode::Image);
        assert_eq!(resolve_mode(svg, SvgMode::Auto, 0.2), SvgMode::Vector);
        let r = rasterize(svg, 400).unwrap();
        assert_eq!((r.width, r.height), (400, 200));
        let px = |x: u32, y: u32| &r.rgba[((y * r.width + x) * 4) as usize..][..4];
        assert!(px(2, 2)[0] > 200 && px(2, 2)[2] < 60, "links rot");
        assert!(px(397, 2)[2] > 200 && px(397, 2)[0] < 60, "rechts blau");
        // Text wurde gezeichnet: helle Pixel unten links
        let bright = (20..120).flat_map(|x| (150..190).map(move |y| (x, y))).filter(|&(x, y)| px(x, y)[1] > 200).count();
        assert!(bright > 50, "Text fehlt ({bright})");
        // Text funktioniert auch im Vektor-Modus
        let only_text = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 30"><text x="5" y="22" font-size="20" font-weight="bold">Hi</text></svg>"#;
        assert!(!svg_shapes(only_text, 1.0, 0.0).unwrap().is_empty());
    }

    #[test]
    fn extruded_logo() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M5 0 L10 10 L0 10 Z M5 4 L7 8 L3 8 Z" fill="white" fill-rule="evenodd"/></svg>"#;
        let s = svg_shapes(svg, 1.0, 0.2).unwrap();
        assert_eq!(s.len(), 1);
        assert!(s[0].mesh.positions.iter().any(|p| p[2] > 0.09) && s[0].mesh.positions.iter().any(|p| p[2] < -0.09), "Tiefe");
    }
}
