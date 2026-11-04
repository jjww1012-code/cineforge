//! Prozedurale, nahtlos kachelbare Texturen für Material-Muster.
//!
//! Ergebnis sind sRGB-Farbpixel (RGBA8) und optional eine Normal-Map
//! (Tangentenraum) für plastische Oberflächen.

use crate::color::Rgb;
use crate::model::{Pattern, PatternKind};
use crate::noise::Rng;

/// Kachelbares Wert-Rauschen mit Periode `period` Zellen.
struct TileNoise {
    values: Vec<f32>,
    size: usize,
}

impl TileNoise {
    fn new(seed: u32) -> Self {
        let size = 256;
        let mut rng = Rng::new(seed ^ 0xA5A5_1234);
        Self { values: (0..size * size).map(|_| rng.f32()).collect(), size }
    }

    fn cell(&self, x: i64, y: i64, period: i64) -> f32 {
        let xi = x.rem_euclid(period) as usize % self.size;
        let yi = y.rem_euclid(period) as usize % self.size;
        self.values[yi * self.size + xi]
    }

    /// u, v in 0..1 (eine Kachel); Ergebnis 0..1.
    fn value(&self, u: f32, v: f32, period: i64) -> f32 {
        let (x, y) = (u * period as f32, v * period as f32);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let s = |t: f32| t * t * (3.0 - 2.0 * t);
        let (sx, sy) = (s(fx), s(fy));
        let (x0, y0) = (x0 as i64, y0 as i64);
        let a = self.cell(x0, y0, period);
        let b = self.cell(x0 + 1, y0, period);
        let c = self.cell(x0, y0 + 1, period);
        let d = self.cell(x0 + 1, y0 + 1, period);
        let top = a + (b - a) * sx;
        let bot = c + (d - c) * sx;
        top + (bot - top) * sy
    }

    fn fbm(&self, u: f32, v: f32, base_period: i64, octaves: u32) -> f32 {
        let (mut sum, mut amp, mut norm, mut period) = (0.0, 1.0, 0.0, base_period);
        for _ in 0..octaves {
            sum += self.value(u, v, period) * amp;
            norm += amp;
            amp *= 0.5;
            period *= 2;
        }
        sum / norm
    }
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Liefert (Mischfaktor zwischen Farbe 1 und 2, Höhe) für einen Punkt der Kachel.
fn sample(kind: PatternKind, n: &TileNoise, u: f32, v: f32) -> (f32, f32) {
    match kind {
        PatternKind::Noise => {
            let h = n.fbm(u, v, 8, 5);
            (smoothstep(0.25, 0.75, h), h)
        }
        PatternKind::Checker => {
            let c = ((u * 2.0).floor() as i32 + (v * 2.0).floor() as i32).rem_euclid(2) as f32;
            (c, 0.5 + 0.02 * n.fbm(u, v, 16, 3))
        }
        PatternKind::Stripes => {
            let s = 0.5 + 0.5 * (u * std::f32::consts::TAU * 2.0).sin();
            let m = smoothstep(0.4, 0.6, s);
            (m, m)
        }
        PatternKind::Bricks => {
            let rows = 4.0;
            let row = (v * rows).floor();
            let shift = if (row as i32) % 2 == 1 { 0.25 } else { 0.0 };
            let bu = (u * 2.0 + shift).rem_euclid(1.0);
            let bv = (v * rows).rem_euclid(1.0);
            let mortar_u = 0.035 * 2.0;
            let mortar_v = 0.035 * rows;
            let edge = smoothstep(0.0, mortar_u, bu.min(1.0 - bu)).min(smoothstep(0.0, mortar_v, bv.min(1.0 - bv)));
            let brick_id = ((u * 2.0 + shift).floor() as i64, row as i64);
            let variation = n.cell(brick_id.0 * 7 + 3, brick_id.1 * 13 + 5, 256) * 0.25;
            let grain = n.fbm(u, v, 32, 3) * 0.15;
            let mortar = 1.0 - edge;
            ((mortar + (1.0 - mortar) * (variation + grain - 0.1)).clamp(0.0, 1.0), edge * (0.85 + grain))
        }
        PatternKind::Grid => {
            let line = |t: f32| {
                let f = (t * 2.0).rem_euclid(1.0);
                1.0 - smoothstep(0.0, 0.04, f.min(1.0 - f))
            };
            let l = line(u).max(line(v));
            (l, 1.0 - l * 0.8)
        }
        PatternKind::Dots => {
            let (fu, fv) = ((u * 4.0).rem_euclid(1.0) - 0.5, (v * 4.0).rem_euclid(1.0) - 0.5);
            let d = (fu * fu + fv * fv).sqrt();
            let dot = 1.0 - smoothstep(0.26, 0.3, d);
            (dot, 0.5 + dot * 0.5)
        }
        PatternKind::Marble => {
            let w = n.fbm(u, v, 4, 5);
            let s = (0.5 + 0.5 * ((u + v) * std::f32::consts::TAU * 2.0 + w * 9.0).sin()).powf(6.0);
            (s, 0.5 + 0.1 * w)
        }
        PatternKind::Wood => {
            let w = n.fbm(u, v * 0.25, 4, 4);
            let ring = ((u * 8.0 + w * 3.0).rem_euclid(1.0) * 2.0 - 1.0).abs();
            let r = smoothstep(0.55, 1.0, ring) * 0.8 + n.fbm(u * 8.0, v, 64, 2) * 0.2;
            (r.clamp(0.0, 1.0), 1.0 - r * 0.5)
        }
    }
}

pub struct PatternImages {
    pub size: u32,
    /// sRGB-Farben, RGBA8
    pub color: Vec<u8>,
    /// Normal-Map (linear), RGBA8 – nur wenn `bump > 0`
    pub normal: Option<Vec<u8>>,
}

pub fn pattern_images(p: &Pattern, base: Rgb, size: u32) -> PatternImages {
    let size = size.clamp(16, 2048);
    let n = TileNoise::new(p.seed);
    let s = size as usize;
    let mut heights = vec![0.0f32; s * s];
    let mut color = Vec::with_capacity(s * s * 4);
    let to8 = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    for y in 0..s {
        for x in 0..s {
            let (u, v) = ((x as f32 + 0.5) / size as f32, (y as f32 + 0.5) / size as f32);
            let (mix, h) = sample(p.kind, &n, u, v);
            heights[y * s + x] = h;
            let c = [
                base.r + (p.color2.r - base.r) * mix,
                base.g + (p.color2.g - base.g) * mix,
                base.b + (p.color2.b - base.b) * mix,
            ];
            color.extend_from_slice(&[to8(c[0]), to8(c[1]), to8(c[2]), 255]);
        }
    }
    let normal = (p.bump > 0.0).then(|| {
        let strength = p.bump * size as f32 / 64.0;
        let mut out = Vec::with_capacity(s * s * 4);
        let h = |x: isize, y: isize| heights[(y.rem_euclid(s as isize) as usize) * s + x.rem_euclid(s as isize) as usize];
        for y in 0..s as isize {
            for x in 0..s as isize {
                let dx = (h(x + 1, y) - h(x - 1, y)) * strength;
                let dy = (h(x, y + 1) - h(x, y - 1)) * strength;
                let l = (dx * dx + dy * dy + 1.0).sqrt();
                let nn = [-dx / l, -dy / l, 1.0 / l];
                out.extend_from_slice(&[to8(nn[0] * 0.5 + 0.5), to8(nn[1] * 0.5 + 0.5), to8(nn[2] * 0.5 + 0.5), 255]);
            }
        }
        out
    });
    PatternImages { size, color, normal }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pat(kind: PatternKind, bump: f32) -> Pattern {
        Pattern { kind, color2: Rgb::BLACK, scale: 1.0, bump, seed: 1 }
    }

    #[test]
    fn all_patterns_produce_contrast_and_tile() {
        for kind in [
            PatternKind::Noise,
            PatternKind::Checker,
            PatternKind::Stripes,
            PatternKind::Bricks,
            PatternKind::Grid,
            PatternKind::Dots,
            PatternKind::Marble,
            PatternKind::Wood,
        ] {
            let img = pattern_images(&pat(kind, 0.5), Rgb::WHITE, 64);
            assert_eq!(img.color.len(), 64 * 64 * 4);
            let lum: Vec<u8> = img.color.chunks(4).map(|c| c[0]).collect();
            let (lo, hi) = (*lum.iter().min().unwrap(), *lum.iter().max().unwrap());
            assert!(hi - lo > 60, "{kind:?} hat zu wenig Kontrast ({lo}..{hi})");
            // Nahtlos: Der Sprung am Kachelrand (Spalte 63 → 0) ist nicht größer
            // als der größte Sprung zwischen zwei Nachbarspalten im Inneren.
            let col_diff = |a: usize, b: usize| -> u32 {
                (0..64).map(|y| (lum[y * 64 + a] as i32 - lum[y * 64 + b] as i32).unsigned_abs()).sum::<u32>() / 64
            };
            let inner = (0..63).map(|x| col_diff(x, x + 1)).max().unwrap();
            let seam = col_diff(63, 0);
            assert!(seam <= inner + 12, "{kind:?} kachelt nicht nahtlos (Rand {seam}, innen max {inner})");
            assert!(img.normal.is_some());
        }
        assert!(pattern_images(&pat(PatternKind::Noise, 0.0), Rgb::WHITE, 32).normal.is_none());
    }
}
