//! 2D-Grafik aus SVG: Claude zeichnet Figuren, Logos und Hintergründe als SVG
//! (oder importiert SVG-Dateien). Jeder Pfad wird zu einer flachen (oder
//! extrudierten) Fläche mit eigener Farbe; Konturen werden zu Linien.

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
            _ => {}
        }
    }
}

/// SVG → Flächen. Breite `width` in Metern, zentriert, y nach oben; `depth` > 0 extrudiert.
pub fn svg_shapes(svg: &str, width: f32, depth: f32) -> Result<Vec<Shape2d>, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| format!("SVG nicht lesbar: {e}"))?;
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
    fn extruded_logo() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M5 0 L10 10 L0 10 Z M5 4 L7 8 L3 8 Z" fill="white" fill-rule="evenodd"/></svg>"#;
        let s = svg_shapes(svg, 1.0, 0.2).unwrap();
        assert_eq!(s.len(), 1);
        assert!(s[0].mesh.positions.iter().any(|p| p[2] > 0.09) && s[0].mesh.positions.iter().any(|p| p[2] < -0.09), "Tiefe");
    }
}
