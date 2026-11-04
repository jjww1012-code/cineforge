//! Partikel-Effekte (Feuer, Rauch, Funken, Schnee, Regen, Explosion …).
//!
//! Vollständig deterministisch: Zustand jedes Partikels ist eine reine
//! Funktion von Seed und Zeit. Dadurch sieht jedes Bild in Vorschau,
//! Kontrollbild und Export identisch aus, und man kann beliebig springen.

use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::noise::Rng;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParticlePreset {
    #[default]
    Fire,
    Smoke,
    Sparks,
    Snow,
    Rain,
    Explosion,
    Magic,
    Dust,
    Fountain,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Blend {
    /// Leuchtend, addiert sich (Feuer, Funken, Magie)
    Add,
    /// Normal deckend/transparent (Rauch, Schnee, Regen)
    Alpha,
}

/// Angaben in der Szene: Voreinstellung + optionale Anpassungen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ParticleSpec {
    #[serde(default)]
    pub preset: ParticlePreset,
    /// Partikel pro Sekunde.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<f32>,
    /// Alle auf einmal zum Zeitpunkt `start` (Explosion). Ersetzt `rate`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burst: Option<u32>,
    /// Lebensdauer [min, max] in Sekunden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifetime: Option<[f32; 2]>,
    /// Startgeschwindigkeit [min, max] in m/s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<[f32; 2]>,
    /// Hauptrichtung.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<[f32; 3]>,
    /// Streuwinkel um die Hauptrichtung in Grad (180 = alle Richtungen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<f32>,
    /// Beschleunigung in m/s² (Schwerkraft [0, -9.8, 0], Auftrieb positiv).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gravity: Option<[f32; 3]>,
    /// Luftwiderstand (0 = keiner, 2 = stark).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drag: Option<f32>,
    /// Zufälliges Schweben/Taumeln in Metern.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turbulence: Option<f32>,
    /// Größe [am Anfang, am Ende] in Metern.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_start: Option<Rgb>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_end: Option<Rgb>,
    /// Grund-Deckkraft 0..1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    /// Leuchtstärke (0 = kein Leuchten).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glow: Option<f32>,
    /// Fläche, in der Partikel entstehen [Breite, Höhe, Tiefe].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<[f32; 3]>,
    /// Ab wann (s) Partikel entstehen.
    #[serde(default)]
    pub start: f32,
    /// Bis wann (s) Partikel entstehen (Standard: bis zum Ende).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<f32>,
    #[serde(default)]
    pub seed: u32,
    /// Obergrenze gleichzeitig sichtbarer Partikel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    /// Effekt läuft beim Start schon „eingeschwungen“ (Standard bei dauerhaften Effekten).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prewarm: Option<bool>,
}

/// Voll aufgelöste Werte (Voreinstellung + Anpassungen).
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub rate: f32,
    pub burst: Option<u32>,
    pub lifetime: [f32; 2],
    pub speed: [f32; 2],
    pub direction: [f32; 3],
    pub spread: f32,
    pub gravity: [f32; 3],
    pub drag: f32,
    pub turbulence: f32,
    pub size: [f32; 2],
    pub color_start: Rgb,
    pub color_end: Rgb,
    pub opacity: f32,
    pub glow: f32,
    pub area: [f32; 3],
    pub start: f32,
    pub end: f32,
    pub seed: u32,
    pub max: u32,
    pub blend: Blend,
    /// Partikel werden in Flugrichtung gestreckt (Regen, Funken).
    pub stretch: f32,
    /// Erzeugung beginnt so viel früher (Vorlaufen), sichtbar erst ab `start`.
    pub prewarm: f32,
}

fn hex(s: &str) -> Rgb {
    Rgb::parse_hex(s).expect("gültige Farbe")
}

impl ParticleSpec {
    pub fn resolve(&self) -> Resolved {
        use ParticlePreset::*;
        let up = [0.0, 1.0, 0.0];
        let down = [0.0, -1.0, 0.0];
        let g = [0.0, -9.81, 0.0];
        // (rate, burst, lifetime, speed, dir, spread, gravity, drag, turb, size, c0, c1, opacity, glow, area, blend, stretch)
        let base = match self.preset {
            Fire => Resolved {
                rate: 70.0, burst: None, lifetime: [0.6, 1.2], speed: [0.8, 1.8], direction: up, spread: 12.0,
                gravity: [0.0, 2.0, 0.0], drag: 0.8, turbulence: 0.08, size: [0.7, 0.15],
                color_start: hex("#ffd27a"), color_end: hex("#ff2a00"), opacity: 0.9, glow: 6.0,
                area: [0.5, 0.1, 0.5], start: 0.0, end: f32::MAX, seed: 0, max: 600, blend: Blend::Add, stretch: 0.0, prewarm: 0.0,
            },
            Smoke => Resolved {
                rate: 14.0, burst: None, lifetime: [3.0, 5.0], speed: [0.4, 0.9], direction: up, spread: 18.0,
                gravity: [0.15, 0.35, 0.0], drag: 0.4, turbulence: 0.25, size: [0.6, 3.0],
                color_start: hex("#6e6e6e"), color_end: hex("#3a3a3a"), opacity: 0.45, glow: 0.0,
                area: [0.6, 0.2, 0.6], start: 0.0, end: f32::MAX, seed: 0, max: 400, blend: Blend::Alpha, stretch: 0.0, prewarm: 0.0,
            },
            Sparks => Resolved {
                rate: 90.0, burst: None, lifetime: [0.4, 1.1], speed: [3.0, 7.0], direction: up, spread: 55.0,
                gravity: g, drag: 0.15, turbulence: 0.0, size: [0.06, 0.02],
                color_start: hex("#fff4b0"), color_end: hex("#ff6a00"), opacity: 1.0, glow: 25.0,
                area: [0.1, 0.1, 0.1], start: 0.0, end: f32::MAX, seed: 0, max: 800, blend: Blend::Add, stretch: 0.06, prewarm: 0.0,
            },
            Snow => Resolved {
                rate: 220.0, burst: None, lifetime: [7.0, 9.0], speed: [0.0, 0.2], direction: down, spread: 25.0,
                gravity: [0.0, -1.2, 0.0], drag: 1.5, turbulence: 0.6, size: [0.07, 0.07],
                color_start: hex("#ffffff"), color_end: hex("#f2f6ff"), opacity: 0.95, glow: 0.0,
                area: [40.0, 0.5, 40.0], start: 0.0, end: f32::MAX, seed: 0, max: 2500, blend: Blend::Alpha, stretch: 0.0, prewarm: 0.0,
            },
            Rain => Resolved {
                rate: 900.0, burst: None, lifetime: [0.9, 1.1], speed: [10.0, 12.0], direction: down, spread: 3.0,
                gravity: g, drag: 0.0, turbulence: 0.0, size: [0.015, 0.015],
                color_start: hex("#b9cdf0"), color_end: hex("#b9cdf0"), opacity: 0.45, glow: 0.0,
                area: [40.0, 0.5, 40.0], start: 0.0, end: f32::MAX, seed: 0, max: 1500, blend: Blend::Alpha, stretch: 0.045, prewarm: 0.0,
            },
            Explosion => Resolved {
                rate: 0.0, burst: Some(320), lifetime: [0.5, 1.6], speed: [4.0, 14.0], direction: up, spread: 180.0,
                gravity: [0.0, -3.0, 0.0], drag: 1.8, turbulence: 0.1, size: [1.3, 0.2],
                color_start: hex("#fff2a0"), color_end: hex("#ff2a00"), opacity: 0.95, glow: 14.0,
                area: [0.5, 0.5, 0.5], start: 0.0, end: f32::MAX, seed: 0, max: 600, blend: Blend::Add, stretch: 0.0, prewarm: 0.0,
            },
            Magic => Resolved {
                rate: 45.0, burst: None, lifetime: [1.0, 2.2], speed: [0.2, 0.9], direction: up, spread: 180.0,
                gravity: [0.0, 0.5, 0.0], drag: 0.6, turbulence: 0.5, size: [0.14, 0.0],
                color_start: hex("#9be7ff"), color_end: hex("#c77dff"), opacity: 1.0, glow: 12.0,
                area: [1.0, 1.0, 1.0], start: 0.0, end: f32::MAX, seed: 0, max: 400, blend: Blend::Add, stretch: 0.0, prewarm: 0.0,
            },
            Dust => Resolved {
                rate: 30.0, burst: None, lifetime: [5.0, 8.0], speed: [0.02, 0.15], direction: up, spread: 180.0,
                gravity: [0.0, 0.0, 0.0], drag: 0.5, turbulence: 0.35, size: [0.035, 0.035],
                color_start: hex("#fff6dc"), color_end: hex("#fff6dc"), opacity: 0.5, glow: 1.5,
                area: [10.0, 4.0, 10.0], start: 0.0, end: f32::MAX, seed: 0, max: 400, blend: Blend::Add, stretch: 0.0, prewarm: 0.0,
            },
            Fountain => Resolved {
                rate: 160.0, burst: None, lifetime: [1.4, 1.9], speed: [6.0, 7.0], direction: up, spread: 7.0,
                gravity: g, drag: 0.05, turbulence: 0.0, size: [0.09, 0.05],
                color_start: hex("#d6f1ff"), color_end: hex("#a9dcff"), opacity: 0.75, glow: 0.0,
                area: [0.15, 0.0, 0.15], start: 0.0, end: f32::MAX, seed: 0, max: 600, blend: Blend::Alpha, stretch: 0.03, prewarm: 0.0,
            },
        };
        let mut r = base;
        macro_rules! ov {
            ($f:ident) => {
                if let Some(v) = self.$f {
                    r.$f = v;
                }
            };
        }
        ov!(rate);
        ov!(lifetime);
        ov!(speed);
        ov!(direction);
        ov!(spread);
        ov!(gravity);
        ov!(drag);
        ov!(turbulence);
        ov!(size);
        ov!(color_start);
        ov!(color_end);
        ov!(opacity);
        ov!(glow);
        ov!(area);
        ov!(max);
        if self.burst.is_some() {
            r.burst = self.burst;
        } else if self.rate.is_some() {
            r.burst = None;
        }
        r.start = self.start;
        r.end = self.end.unwrap_or(f32::MAX);
        let continuous = r.burst.is_none();
        r.prewarm = if self.prewarm.unwrap_or(continuous) && continuous { r.lifetime[1] } else { 0.0 };
        r.seed = self.seed;
        r.lifetime = [r.lifetime[0].max(0.01), r.lifetime[1].max(r.lifetime[0].max(0.01))];
        r.max = r.max.min(20_000);
        r
    }

    pub fn validate(&self, id: &str, p: &mut Vec<String>) {
        let r = self.resolve();
        if r.rate < 0.0 || r.rate > 20_000.0 {
            p.push(format!("\"{id}\": rate muss zwischen 0 und 20000 liegen"));
        }
        if self.burst.is_some_and(|b| b > 20_000) {
            p.push(format!("\"{id}\": burst ist zu groß (max. 20000)"));
        }
        if !(0.0..=1.0).contains(&r.opacity) {
            p.push(format!("\"{id}\": opacity muss zwischen 0 und 1 liegen"));
        }
        if r.lifetime[1] > 120.0 {
            p.push(format!("\"{id}\": lifetime ist zu lang (max. 120 s)"));
        }
    }
}

/// Zustand eines lebenden Partikels (lokale Koordinaten des Emitters).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    pub index: u32,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub size: f32,
    /// Alter relativ zur Lebensdauer (0..1).
    pub age: f32,
}

impl Resolved {
    /// Wie viele Partikel höchstens gleichzeitig leben (für den Entity-Pool).
    pub fn pool_size(&self) -> u32 {
        let n = match self.burst {
            Some(b) => b,
            None => (self.rate * self.lifetime[1]).ceil() as u32 + 1,
        };
        n.min(self.max).max(1)
    }

    /// Alle lebenden Partikel zum Zeitpunkt `t`.
    pub fn particles_at(&self, t: f32) -> Vec<Particle> {
        let mut out = Vec::new();
        if t < self.start {
            return out;
        }
        let (first, last) = match self.burst {
            Some(b) => (0u32, b),
            None => {
                if self.rate <= 0.0 {
                    return out;
                }
                let origin = self.start - self.prewarm;
                let emit_end = t.min(self.end);
                let oldest = (t - self.lifetime[1]).max(origin);
                let first = ((oldest - origin) * self.rate).floor().max(0.0) as u32;
                let last = (((emit_end - origin) * self.rate).floor() as i64 + 1).max(0) as u32;
                (first, last)
            }
        };
        for i in first..last {
            if let Some(p) = self.particle(i, t) {
                out.push(p);
                if out.len() as u32 >= self.max {
                    break;
                }
            }
        }
        out
    }

    fn particle(&self, i: u32, t: f32) -> Option<Particle> {
        let mut rng = Rng::new(self.seed.wrapping_mul(0x9E37_79B1) ^ i.wrapping_mul(0x85EB_CA6B) ^ 0x1234_5678);
        let birth = match self.burst {
            Some(_) => self.start,
            None => self.start - self.prewarm + (i as f32 + rng.f32() * 0.999) / self.rate,
        };
        if birth > t || birth > self.end {
            return None;
        }
        let life = rng.range(self.lifetime[0], self.lifetime[1]);
        let a = t - birth;
        if a >= life {
            return None;
        }
        // Startpunkt in der Fläche
        let p0 = [
            rng.range(-0.5, 0.5) * self.area[0],
            rng.range(-0.5, 0.5) * self.area[1],
            rng.range(-0.5, 0.5) * self.area[2],
        ];
        // Richtung im Kegel um die Hauptrichtung (gleichmäßig auf der Kugelkappe)
        let dir = normalize(self.direction);
        let cos_max = self.spread.clamp(0.0, 180.0).to_radians().cos();
        let cos_t = rng.range(cos_max, 1.0);
        let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
        let phi = rng.range(0.0, std::f32::consts::TAU);
        let (u, v) = basis(dir);
        let d = [
            dir[0] * cos_t + (u[0] * phi.cos() + v[0] * phi.sin()) * sin_t,
            dir[1] * cos_t + (u[1] * phi.cos() + v[1] * phi.sin()) * sin_t,
            dir[2] * cos_t + (u[2] * phi.cos() + v[2] * phi.sin()) * sin_t,
        ];
        let speed = rng.range(self.speed[0], self.speed[1]);
        let v0 = [d[0] * speed, d[1] * speed, d[2] * speed];
        // Bewegung mit Luftwiderstand k: v(t) = (v0 - g/k) e^{-kt} + g/k
        let k = self.drag.max(0.0);
        let g = self.gravity;
        let (pos, vel) = if k < 1e-4 {
            (
                std::array::from_fn(|j| p0[j] + v0[j] * a + 0.5 * g[j] * a * a),
                std::array::from_fn(|j| v0[j] + g[j] * a),
            )
        } else {
            let e = (-k * a).exp();
            (
                std::array::from_fn(|j| p0[j] + g[j] / k * a + (v0[j] - g[j] / k) * (1.0 - e) / k),
                std::array::from_fn(|j| (v0[j] - g[j] / k) * e + g[j] / k),
            )
        };
        let mut pos: [f32; 3] = pos;
        if self.turbulence > 0.0 {
            let ph = [rng.range(0.0, 6.28), rng.range(0.0, 6.28), rng.range(0.0, 6.28)];
            let f = rng.range(0.6, 1.4);
            pos[0] += self.turbulence * (a * 1.3 * f + ph[0]).sin();
            pos[1] += self.turbulence * 0.5 * (a * 0.9 * f + ph[1]).sin();
            pos[2] += self.turbulence * (a * 1.1 * f + ph[2]).cos();
        }
        let age = a / life;
        let size = self.size[0] + (self.size[1] - self.size[0]) * age;
        Some(Particle { index: i, position: pos, velocity: vel, size: size.max(0.0), age })
    }

    /// Farbe und Deckkraft für ein Alter (0..1): weich ein- und ausblenden.
    pub fn look_at_age(&self, age: f32) -> (Rgb, f32) {
        let fade_in = smooth(0.0, 0.08, age);
        let fade_out = 1.0 - smooth(0.6, 1.0, age);
        let c = Rgb::new(
            self.color_start.r + (self.color_end.r - self.color_start.r) * age,
            self.color_start.g + (self.color_end.g - self.color_start.g) * age,
            self.color_start.b + (self.color_end.b - self.color_start.b) * age,
        );
        (c, self.opacity * fade_in * fade_out)
    }
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l < 1e-9 { [0.0, 1.0, 0.0] } else { [v[0] / l, v[1] / l, v[2] / l] }
}

fn basis(n: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let a = if n[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let u = normalize([n[1] * a[2] - n[2] * a[1], n[2] * a[0] - n[0] * a[2], n[0] * a[1] - n[1] * a[0]]);
    let v = [n[1] * u[2] - n[2] * u[1], n[2] * u[0] - n[0] * u[2], n[0] * u[1] - n[1] * u[0]];
    (u, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(preset: ParticlePreset) -> ParticleSpec {
        ParticleSpec { preset, ..Default::default() }
    }

    #[test]
    fn deterministic_and_within_pool() {
        for preset in [
            ParticlePreset::Fire,
            ParticlePreset::Smoke,
            ParticlePreset::Sparks,
            ParticlePreset::Snow,
            ParticlePreset::Rain,
            ParticlePreset::Explosion,
            ParticlePreset::Magic,
            ParticlePreset::Dust,
            ParticlePreset::Fountain,
        ] {
            let r = spec(preset).resolve();
            for t in [0.5, 2.0, 7.3] {
                let a = r.particles_at(t);
                let b = r.particles_at(t);
                assert_eq!(a, b, "{preset:?} nicht deterministisch");
                assert!(a.len() as u32 <= r.pool_size(), "{preset:?}: {} > Pool {}", a.len(), r.pool_size());
                assert!(a.iter().all(|p| (0.0..1.0).contains(&p.age)));
            }
            // Explosionen sind nach ihrer Lebensdauer verglüht → kurz nach dem Start prüfen.
            let probe = if r.burst.is_some() { 0.3 } else { 2.0 };
            assert!(!r.particles_at(probe).is_empty(), "{preset:?} erzeugt nichts");
        }
    }

    #[test]
    fn gravity_drag_and_timing() {
        let fountain = spec(ParticlePreset::Fountain).resolve();
        // Fontäne: steigt, fällt wieder – nach ~1.4 s sind die ersten Tropfen unten
        let ps = fountain.particles_at(3.0);
        let max_y = ps.iter().map(|p| p.position[1]).fold(f32::MIN, f32::max);
        assert!(max_y > 1.5 && max_y < 3.0, "Höhe {max_y}");
        // Feuer steigt
        let fire = spec(ParticlePreset::Fire).resolve();
        assert!(fire.particles_at(3.0).iter().all(|p| p.velocity[1] > 0.0));
        // Vor dem Start nichts, Explosion verglüht
        let ex = ParticleSpec { start: 2.0, ..spec(ParticlePreset::Explosion) }.resolve();
        assert!(ex.particles_at(1.9).is_empty());
        assert_eq!(ex.particles_at(2.05).len(), 320);
        assert!(ex.particles_at(4.0).is_empty(), "nach max. Lebensdauer erloschen");
        // Ende der Erzeugung
        let stop = ParticleSpec { end: Some(1.0), ..spec(ParticlePreset::Fire) }.resolve();
        assert!(stop.particles_at(3.0).is_empty());
        // Vorlaufen: Schnee ist schon bei t = 0 über die ganze Höhe verteilt
        let snow = spec(ParticlePreset::Snow).resolve();
        let at0 = snow.particles_at(0.0);
        assert!(at0.len() > 1000, "eingeschwungen: {}", at0.len());
        let lowest = at0.iter().map(|p| p.position[1]).fold(f32::MAX, f32::min);
        assert!(lowest < -4.0, "Flocken sind schon gefallen: {lowest}");
        let cold = ParticleSpec { prewarm: Some(false), ..spec(ParticlePreset::Snow) }.resolve();
        assert!(cold.particles_at(0.0).len() < 5);
    }

    #[test]
    fn overrides_and_look() {
        let s: ParticleSpec = serde_json::from_str(r##"{"preset":"smoke","rate":5,"color_start":"#ff0000","size":[1,2]}"##).unwrap();
        let r = s.resolve();
        assert_eq!(r.rate, 5.0);
        assert_eq!(r.size, [1.0, 2.0]);
        assert_eq!(r.blend, Blend::Alpha);
        let (c, a) = r.look_at_age(0.0);
        assert_eq!(c.to_hex(), "#ff0000");
        assert!(a < 0.01, "blendet ein");
        assert!(r.look_at_age(0.3).1 > 0.4);
        assert!(r.look_at_age(0.999).1 < 0.01, "blendet aus");
        assert!(serde_json::from_str::<ParticleSpec>(r#"{"preset":"plasma"}"#).is_err());
    }
}
