//! Grundbausteine der Klangerzeugung (48 kHz, f32).

pub const SR: f32 = 48_000.0;

/// Deterministischer Zufall (gleicher Seed ⇒ gleicher Klang).
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u32) -> Self {
        let mut r = Rng((seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA076_1D64_78BD_642F);
        r.next();
        r
    }
    pub fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let x = ((self.0 >> 18) ^ self.0) >> 27;
        (x as u32).rotate_right((self.0 >> 59) as u32)
    }
    /// -1..1
    pub fn bipolar(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
    /// 0..1
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Wave {
    #[default]
    Sine,
    Triangle,
    Saw,
    Square,
    /// Weißes Rauschen (hell, zischend)
    Noise,
    /// Rosa Rauschen (natürlich, wie Wind/Meer)
    Pink,
    /// Braunes Rauschen (dunkel, Rumpeln)
    Brown,
}

fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// Oszillator mit bandbegrenzten Sägezahn/Rechteck (keine harten Aliasing-Artefakte).
pub struct Osc {
    wave: Wave,
    phase: f32,
    rng: Rng,
    pink: [f32; 7],
    brown: f32,
    tri: f32,
}

impl Osc {
    pub fn new(wave: Wave, seed: u32, phase: f32) -> Self {
        Self { wave, phase: phase.fract(), rng: Rng::new(seed), pink: [0.0; 7], brown: 0.0, tri: 0.0 }
    }

    pub fn next(&mut self, freq: f32) -> f32 {
        let dt = (freq / SR).clamp(0.0, 0.49);
        let t = self.phase;
        let out = match self.wave {
            Wave::Sine => (t * std::f32::consts::TAU).sin(),
            Wave::Saw => (2.0 * t - 1.0) - poly_blep(t, dt),
            Wave::Square => {
                let mut v = if t < 0.5 { 1.0 } else { -1.0 };
                v += poly_blep(t, dt);
                v -= poly_blep((t + 0.5).fract(), dt);
                v
            }
            Wave::Triangle => {
                // integriertes Rechteck
                let mut sq = if t < 0.5 { 1.0 } else { -1.0 };
                sq += poly_blep(t, dt);
                sq -= poly_blep((t + 0.5).fract(), dt);
                self.tri = dt * sq * 4.0 + (1.0 - dt * 0.05) * self.tri;
                self.tri
            }
            Wave::Noise => self.rng.bipolar(),
            Wave::Pink => {
                // Paul Kellet
                let w = self.rng.bipolar();
                let b = &mut self.pink;
                b[0] = 0.99886 * b[0] + w * 0.0555179;
                b[1] = 0.99332 * b[1] + w * 0.0750759;
                b[2] = 0.96900 * b[2] + w * 0.1538520;
                b[3] = 0.86650 * b[3] + w * 0.3104856;
                b[4] = 0.55000 * b[4] + w * 0.5329522;
                b[5] = -0.7616 * b[5] - w * 0.0168980;
                let v = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + w * 0.5362;
                b[6] = w * 0.115926;
                v * 0.11
            }
            Wave::Brown => {
                self.brown = (self.brown + self.rng.bipolar() * 0.02).clamp(-1.0, 1.0) * 0.998;
                self.brown * 3.5
            }
        };
        self.phase = (self.phase + dt).fract();
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FilterKind {
    #[default]
    Lowpass,
    Highpass,
    Bandpass,
}

/// Zustandsvariables Filter (Cytomic/TPT) – stabil auch bei schnellen Sweeps.
#[derive(Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}

impl Svf {
    pub fn process(&mut self, x: f32, kind: FilterKind, cutoff: f32, q: f32) -> f32 {
        let g = (std::f32::consts::PI * (cutoff / SR).clamp(0.0001, 0.49)).tan();
        let k = 1.0 / q.max(0.05);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        match kind {
            FilterKind::Lowpass => v2,
            FilterKind::Bandpass => v1,
            FilterKind::Highpass => x - k * v1 - v2,
        }
    }
}

/// Verlauf zwischen zwei Werten (exponentiell für Frequenzen = natürlicher Klang).
pub fn sweep(a: f32, b: f32, t: f32, exp: bool) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if exp && a > 0.0 && b > 0.0 {
        a * (b / a).powf(t)
    } else {
        a + (b - a) * t
    }
}

/// ADSR-Hüllkurve mit exponentiellem Abklingen. `len` = Haltedauer bis Release.
pub fn envelope(t: f32, attack: f32, decay: f32, sustain: f32, release: f32, len: f32) -> f32 {
    if t < 0.0 {
        return 0.0;
    }
    let level_at = |t: f32| -> f32 {
        if t < attack {
            // weicher Anstieg
            let x = t / attack.max(1e-5);
            x * x * (3.0 - 2.0 * x)
        } else {
            let d = t - attack;
            sustain + (1.0 - sustain) * (-d / decay.max(1e-5) * 4.6).exp()
        }
    };
    if t < len {
        level_at(t)
    } else {
        let r = t - len;
        if r > release {
            0.0
        } else {
            level_at(len) * (-r / release.max(1e-5) * 4.6).exp() * (1.0 - r / release.max(1e-5))
        }
    }
}

/// Freeverb-artiger Stereo-Hall.
pub struct Reverb {
    combs: Vec<(Vec<f32>, usize, f32)>,
    allpass: Vec<(Vec<f32>, usize)>,
    feedback: f32,
    damp: f32,
}

impl Reverb {
    pub fn new(size: f32, damp: f32, spread: usize) -> Self {
        let comb_lens = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        let ap_lens = [556, 441, 341, 225];
        let scale = SR / 44_100.0;
        Self {
            combs: comb_lens.iter().map(|l| (vec![0.0; ((*l + spread) as f32 * scale) as usize], 0, 0.0)).collect(),
            allpass: ap_lens.iter().map(|l| (vec![0.0; ((*l + spread) as f32 * scale) as usize], 0)).collect(),
            feedback: 0.7 + size.clamp(0.0, 1.0) * 0.28,
            damp: damp.clamp(0.0, 1.0) * 0.4,
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let input = x * 0.015;
        let mut out = 0.0;
        for (buf, idx, store) in &mut self.combs {
            let y = buf[*idx];
            *store = y * (1.0 - self.damp) + *store * self.damp;
            buf[*idx] = input + *store * self.feedback;
            *idx = (*idx + 1) % buf.len();
            out += y;
        }
        for (buf, idx) in &mut self.allpass {
            let b = buf[*idx];
            buf[*idx] = out + b * 0.5;
            out = b - out;
            *idx = (*idx + 1) % buf.len();
        }
        out
    }
}

/// Weiche Sättigung (warm statt digital hart).
pub fn saturate(x: f32, drive: f32) -> f32 {
    if drive <= 0.0 {
        return x;
    }
    let d = 1.0 + drive * 8.0;
    (x * d).tanh() / d.tanh()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oscillators_stay_in_range() {
        for w in [Wave::Sine, Wave::Triangle, Wave::Saw, Wave::Square, Wave::Noise, Wave::Pink, Wave::Brown] {
            let mut o = Osc::new(w, 1, 0.0);
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for _ in 0..48_000 {
                let v = o.next(440.0);
                assert!(v.is_finite());
                lo = lo.min(v);
                hi = hi.max(v);
            }
            assert!(hi - lo > 0.3 && hi < 2.5 && lo > -2.5, "{w:?}: {lo}..{hi}");
        }
    }

    #[test]
    fn lowpass_removes_highs() {
        let mut o = Osc::new(Wave::Noise, 3, 0.0);
        let mut f = Svf::default();
        let (mut raw, mut filt) = (0.0, 0.0);
        let mut prev = (0.0, 0.0);
        for _ in 0..48_000 {
            let x = o.next(0.0);
            let y = f.process(x, FilterKind::Lowpass, 300.0, 0.707);
            raw += (x - prev.0).abs();
            filt += (y - prev.1).abs();
            prev = (x, y);
        }
        assert!(filt < raw * 0.15, "Tiefpass dämpft Höhen: {filt} vs {raw}");
    }

    #[test]
    fn envelope_shape() {
        assert_eq!(envelope(-0.1, 0.01, 0.1, 0.5, 0.2, 1.0), 0.0);
        assert!((envelope(0.01, 0.01, 0.1, 0.5, 0.2, 1.0) - 1.0).abs() < 0.01);
        assert!((envelope(0.9, 0.01, 0.1, 0.5, 0.2, 1.0) - 0.5).abs() < 0.01);
        assert_eq!(envelope(1.3, 0.01, 0.1, 0.5, 0.2, 1.0), 0.0);
    }
}
