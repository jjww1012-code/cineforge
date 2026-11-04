//! Deterministisches 2D-Gradient-Rauschen (Perlin-artig). Gleicher Seed ⇒
//! gleiche Welt auf jedem Rechner, wichtig für reproduzierbare Renderings.

pub struct Noise2 {
    perm: [u8; 512],
}

impl Noise2 {
    pub fn new(seed: u32) -> Self {
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        let mut rng = Rng::new(seed ^ 0x9e37_79b9);
        for i in (1..256).rev() {
            let j = (rng.next_u32() % (i as u32 + 1)) as usize;
            p.swap(i, j);
        }
        let perm = std::array::from_fn(|i| p[i & 255]);
        Self { perm }
    }

    /// Wert ungefähr in -1..1.
    pub fn get(&self, x: f32, y: f32) -> f32 {
        let xi = x.floor();
        let yi = y.floor();
        let xf = x - xi;
        let yf = y - yi;
        let xi = (xi as i32 & 255) as usize;
        let yi = (yi as i32 & 255) as usize;
        let h = |i: usize, j: usize| self.perm[self.perm[i] as usize + j];
        let g = |hash: u8, x: f32, y: f32| match hash & 7 {
            0 => x + y,
            1 => -x + y,
            2 => x - y,
            3 => -x - y,
            4 => x,
            5 => -x,
            6 => y,
            _ => -y,
        };
        let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let u = fade(xf);
        let v = fade(yf);
        let a = g(h(xi, yi), xf, yf);
        let b = g(h(xi + 1, yi), xf - 1.0, yf);
        let c = g(h(xi, yi + 1), xf, yf - 1.0);
        let d = g(h(xi + 1, yi + 1), xf - 1.0, yf - 1.0);
        let l1 = a + u * (b - a);
        let l2 = c + u * (d - c);
        (l1 + v * (l2 - l1)) * 0.9
    }

    /// Fraktales Rauschen aus mehreren Oktaven, Ergebnis ungefähr in -1..1.
    pub fn fbm(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
        for o in 0..octaves {
            let off = o as f32 * 17.31;
            sum += self.get(x * freq + off, y * freq - off) * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }

    /// Gratiges Rauschen für Gebirge, Ergebnis in 0..1.
    pub fn ridged(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
        for o in 0..octaves {
            let off = o as f32 * 31.7;
            let n = 1.0 - self.get(x * freq + off, y * freq + off).abs();
            sum += n * n * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }
}

/// Deterministisches 3D-Gradient-Rauschen (für Oberflächen-Details an Modellen).
pub struct Noise3 {
    perm: [u8; 512],
}

impl Noise3 {
    pub fn new(seed: u32) -> Self {
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        let mut rng = Rng::new(seed ^ 0x51ed_270b);
        for i in (1..256).rev() {
            let j = (rng.next_u32() % (i as u32 + 1)) as usize;
            p.swap(i, j);
        }
        Self { perm: std::array::from_fn(|i| p[i & 255]) }
    }

    /// Wert ungefähr in -1..1.
    pub fn get(&self, x: f32, y: f32, z: f32) -> f32 {
        let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
        let (xf, yf, zf) = (x - xi, y - yi, z - zi);
        let (xi, yi, zi) = ((xi as i32 & 255) as usize, (yi as i32 & 255) as usize, (zi as i32 & 255) as usize);
        let p = &self.perm;
        let h = |i: usize, j: usize, k: usize| p[p[p[i] as usize + j] as usize + k];
        let g = |hash: u8, x: f32, y: f32, z: f32| {
            let hh = hash & 15;
            let u = if hh < 8 { x } else { y };
            let v = if hh < 4 { y } else if hh == 12 || hh == 14 { x } else { z };
            (if hh & 1 == 0 { u } else { -u }) + (if hh & 2 == 0 { v } else { -v })
        };
        let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let (u, v, w) = (fade(xf), fade(yf), fade(zf));
        let lerp = |t: f32, a: f32, b: f32| a + t * (b - a);
        let x1 = lerp(u, g(h(xi, yi, zi), xf, yf, zf), g(h(xi + 1, yi, zi), xf - 1.0, yf, zf));
        let x2 = lerp(u, g(h(xi, yi + 1, zi), xf, yf - 1.0, zf), g(h(xi + 1, yi + 1, zi), xf - 1.0, yf - 1.0, zf));
        let x3 = lerp(u, g(h(xi, yi, zi + 1), xf, yf, zf - 1.0), g(h(xi + 1, yi, zi + 1), xf - 1.0, yf, zf - 1.0));
        let x4 = lerp(u, g(h(xi, yi + 1, zi + 1), xf, yf - 1.0, zf - 1.0), g(h(xi + 1, yi + 1, zi + 1), xf - 1.0, yf - 1.0, zf - 1.0));
        lerp(w, lerp(v, x1, x2), lerp(v, x3, x4)) * 0.95
    }

    /// Fraktales Rauschen (mehrere Oktaven), ungefähr in -1..1.
    pub fn fbm(&self, x: f32, y: f32, z: f32, octaves: u32) -> f32 {
        let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
        for o in 0..octaves.max(1) {
            let off = o as f32 * 19.17;
            sum += self.get(x * freq + off, y * freq - off, z * freq + off * 0.5) * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }
}

/// Kleiner, schneller, deterministischer Zufallsgenerator (PCG-artig).
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u32) -> Self {
        let mut r = Rng((seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03);
        r.next_u32();
        r
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let x = ((self.0 >> 18) ^ self.0) >> 27;
        let rot = (self.0 >> 59) as u32;
        (x as u32).rotate_right(rot)
    }
    /// Gleichverteilt in 0..1.
    pub fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }
}
