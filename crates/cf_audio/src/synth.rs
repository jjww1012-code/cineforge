//! Klang-Rezepte: Schichten aus Oszillatoren/Rauschen mit Hüllkurve, Tonhöhen-
//! und Filterverlauf, Verzerrung, FM, Wiederholungen, dazu Hall und Echo.

use serde::{Deserialize, Serialize};

use crate::dsp::{envelope, saturate, sweep, FilterKind, Osc, Reverb, Svf, Wave, SR};

fn d_len() -> f32 {
    0.2
}
fn d_freq() -> [f32; 2] {
    [440.0, 440.0]
}
fn d_true() -> bool {
    true
}
fn d_attack() -> f32 {
    0.003
}
fn d_decay() -> f32 {
    0.2
}
fn d_release() -> f32 {
    0.05
}
fn d_one() -> f32 {
    1.0
}
fn d_q() -> f32 {
    0.707
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    #[serde(rename = "type", default)]
    pub kind: FilterKind,
    /// Grenzfrequenz [Anfang, Ende] in Hz.
    pub cutoff: [f32; 2],
    /// Resonanz (0.7 neutral, 3–10 pfeifend).
    #[serde(default = "d_q")]
    pub q: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Fm {
    /// Frequenzverhältnis Modulator/Träger (ganzzahlig = harmonisch, krumm = glockig/metallisch).
    pub ratio: f32,
    /// Modulationsstärke [Anfang, Ende].
    pub index: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Repeat {
    pub count: u32,
    /// Abstand in Sekunden.
    pub interval: f32,
    /// Lautstärke-Faktor je Wiederholung.
    #[serde(default = "d_one")]
    pub decay: f32,
    /// Tonhöhen-Faktor je Wiederholung.
    #[serde(default = "d_one")]
    pub pitch: f32,
    /// Zufällige Abweichung im Timing (s) – wirkt menschlicher.
    #[serde(default)]
    pub jitter: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    #[serde(default)]
    pub wave: Wave,
    /// Einsatz in Sekunden.
    #[serde(default)]
    pub start: f32,
    /// Haltedauer bis zum Ausklingen (s).
    #[serde(default = "d_len")]
    pub length: f32,
    /// Tonhöhe [Anfang, Ende] in Hz (bei Rauschen ohne Wirkung).
    #[serde(default = "d_freq")]
    pub freq: [f32; 2],
    /// Exponentieller Tonhöhenverlauf (klingt natürlicher).
    #[serde(default = "d_true")]
    pub exp: bool,
    #[serde(default = "d_attack")]
    pub attack: f32,
    #[serde(default = "d_decay")]
    pub decay: f32,
    #[serde(default)]
    pub sustain: f32,
    #[serde(default = "d_release")]
    pub release: f32,
    #[serde(default = "d_one")]
    pub gain: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    /// Sättigung 0..1.
    #[serde(default)]
    pub drive: f32,
    /// Stereo -1 (links) .. 1 (rechts).
    #[serde(default)]
    pub pan: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fm: Option<Fm>,
    /// Zweiter, leicht verstimmter Oszillator (Cent) – breiter, wärmer.
    #[serde(default)]
    pub detune: f32,
    /// Vibrato [Hz, Halbtöne].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibrato: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<Repeat>,
}

impl Default for Layer {
    fn default() -> Self {
        serde_json::from_str("{}").expect("Standard-Schicht")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReverbSpec {
    /// Raumgröße 0..1.
    #[serde(default = "d_half")]
    pub size: f32,
    /// Anteil 0..1.
    #[serde(default = "d_mix")]
    pub mix: f32,
    /// Dämpfung der Höhen 0..1.
    #[serde(default = "d_half")]
    pub damp: f32,
}
fn d_half() -> f32 {
    0.5
}
fn d_mix() -> f32 {
    0.2
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EchoSpec {
    pub time: f32,
    #[serde(default = "d_fb")]
    pub feedback: f32,
    #[serde(default = "d_mix")]
    pub mix: f32,
}
fn d_fb() -> f32 {
    0.35
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    /// Gesamtlänge (s); Standard: bis alle Schichten und der Hall verklungen sind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f32>,
    pub layers: Vec<Layer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverb: Option<ReverbSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo: Option<EchoSpec>,
    /// Ziel-Spitzenpegel 0..1 (Standard 0.89 ≈ -1 dB).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peak: Option<f32>,
    #[serde(default)]
    pub seed: u32,
}

/// Stereo-Puffer (links, rechts).
pub type Stereo = (Vec<f32>, Vec<f32>);

impl Recipe {
    pub fn validate(&self) -> Result<(), String> {
        if self.layers.is_empty() {
            return Err("layers ist leer".into());
        }
        if self.layers.len() > 64 {
            return Err("zu viele Schichten (max. 64)".into());
        }
        let total = self.natural_length();
        if total > 600.0 {
            return Err(format!("Klang wäre {total:.0} s lang (max. 600 s)"));
        }
        for (i, l) in self.layers.iter().enumerate() {
            if l.length < 0.0 || l.attack < 0.0 || l.release < 0.0 {
                return Err(format!("layers[{i}]: Zeiten dürfen nicht negativ sein"));
            }
            if l.repeat.as_ref().is_some_and(|r| r.count > 2000) {
                return Err(format!("layers[{i}].repeat.count zu groß"));
            }
        }
        Ok(())
    }

    fn natural_length(&self) -> f32 {
        let layers_end = self
            .layers
            .iter()
            .map(|l| {
                let reps = l.repeat.as_ref().map(|r| (r.count.max(1) - 1) as f32 * r.interval + r.jitter).unwrap_or(0.0);
                l.start + reps + l.attack.max(0.0) + l.length + l.release
            })
            .fold(0.0, f32::max);
        let tail = self.reverb.as_ref().map(|r| 0.4 + r.size * 2.5).unwrap_or(0.0)
            + self.echo.as_ref().map(|e| e.time * 6.0 * e.feedback.clamp(0.0, 0.9)).unwrap_or(0.0);
        self.duration.unwrap_or(layers_end + tail)
    }

    pub fn render(&self) -> Stereo {
        let total = self.natural_length().max(0.01);
        let n = (total * SR) as usize;
        let (mut left, mut right) = (vec![0.0f32; n], vec![0.0f32; n]);
        let mut rng = crate::dsp::Rng::new(self.seed ^ 0x51ED);
        for (li, layer) in self.layers.iter().enumerate() {
            let (count, interval, decay, pitch, jitter) = match &layer.repeat {
                Some(r) => (r.count.max(1), r.interval, r.decay, r.pitch, r.jitter),
                None => (1, 0.0, 1.0, 1.0, 0.0),
            };
            let mut g = layer.gain;
            let mut p = 1.0;
            for k in 0..count {
                let offset = layer.start + k as f32 * interval + jitter * rng.unit();
                render_layer(layer, offset, g, p, self.seed.wrapping_add(li as u32 * 977 + k * 31), &mut left, &mut right);
                g *= decay;
                p *= pitch;
            }
        }
        if let Some(r) = &self.reverb {
            let (mut rl, mut rr) = (Reverb::new(r.size, r.damp, 0), Reverb::new(r.size, r.damp, 23));
            let mix = r.mix.clamp(0.0, 1.0);
            for i in 0..n {
                let m = (left[i] + right[i]) * 0.5;
                let (wl, wr) = (rl.process(m), rr.process(m));
                left[i] = left[i] * (1.0 - mix * 0.5) + wl * mix * 3.0;
                right[i] = right[i] * (1.0 - mix * 0.5) + wr * mix * 3.0;
            }
        }
        if let Some(e) = &self.echo {
            let d = ((e.time.max(0.01)) * SR) as usize;
            let fb = e.feedback.clamp(0.0, 0.95);
            let mix = e.mix.clamp(0.0, 1.0);
            let (mut bl, mut br) = (vec![0.0f32; d], vec![0.0f32; d]);
            for i in 0..n {
                let j = i % d;
                let (yl, yr) = (bl[j], br[j]);
                // Ping-Pong
                bl[j] = left[i] + yr * fb;
                br[j] = right[i] + yl * fb;
                left[i] += yl * mix;
                right[i] += yr * mix;
            }
        }
        finish(&mut left, &mut right, self.peak.unwrap_or(0.89));
        (left, right)
    }
}

pub(crate) fn render_layer(l: &Layer, offset: f32, gain: f32, pitch: f32, seed: u32, left: &mut [f32], right: &mut [f32]) {
    let start = (offset.max(0.0) * SR) as usize;
    let span = l.attack + l.length + l.release;
    let n = ((span * SR) as usize).min(left.len().saturating_sub(start));
    if n == 0 {
        return;
    }
    let mut osc = Osc::new(l.wave, seed, (seed % 1000) as f32 / 1000.0);
    let mut osc2 = Osc::new(l.wave, seed ^ 0xBEEF, 0.37);
    let mut modu = Osc::new(Wave::Sine, seed ^ 0x1234, 0.0);
    let mut filt = Svf::default();
    let mut filt2 = Svf::default();
    let pan = l.pan.clamp(-1.0, 1.0);
    let (pl, pr) = (((1.0 - pan) * std::f32::consts::FRAC_PI_4).cos(), ((1.0 + pan) * std::f32::consts::FRAC_PI_4).cos());
    let hold = l.attack + l.length;
    let detune = 2f32.powf(l.detune / 1200.0);
    let mut fm_phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / SR;
        let prog = (t / span.max(1e-4)).clamp(0.0, 1.0);
        let env = envelope(t, l.attack, l.decay, l.sustain, l.release, hold);
        if env <= 0.0 && t > hold {
            break;
        }
        let mut f = sweep(l.freq[0], l.freq[1], (t / hold.max(1e-4)).min(1.0), l.exp) * pitch;
        if let Some([rate, depth]) = l.vibrato {
            f *= 2f32.powf((t * rate * std::f32::consts::TAU).sin() * depth / 12.0);
        }
        let mut s = match (&l.fm, l.wave) {
            (Some(fm), Wave::Sine) => {
                let idx = sweep(fm.index[0], fm.index[1], prog, false);
                let m = modu.next(f * fm.ratio);
                fm_phase = (fm_phase + f / SR).fract();
                ((fm_phase * std::f32::consts::TAU) + idx * m).sin()
            }
            _ => osc.next(f),
        };
        if l.detune != 0.0 {
            s = (s + osc2.next(f * detune)) * 0.6;
        }
        if let Some(fl) = &l.filter {
            let c = sweep(fl.cutoff[0], fl.cutoff[1], prog, true);
            s = filt.process(s, fl.kind, c, fl.q);
            // zweite Stufe für steilere Flanke (12 → 24 dB/Okt.)
            if fl.kind != FilterKind::Bandpass {
                s = filt2.process(s, fl.kind, c, 0.6);
            }
        }
        s = saturate(s, l.drive) * env * gain;
        left[start + i] += s * pl;
        right[start + i] += s * pr;
    }
}

/// Gleichanteil entfernen, Ende weich ausblenden, auf Zielpegel normalisieren.
pub fn finish(left: &mut [f32], right: &mut [f32], peak: f32) {
    let n = left.len();
    // DC-Blocker
    for buf in [&mut *left, &mut *right] {
        let (mut x1, mut y1) = (0.0f32, 0.0f32);
        for v in buf.iter_mut() {
            let y = *v - x1 + 0.9995 * y1;
            x1 = *v;
            y1 = y;
            *v = y;
        }
    }
    let fade = (0.004 * SR) as usize;
    for i in 0..fade.min(n) {
        let g = i as f32 / fade as f32;
        left[n - 1 - i] *= g;
        right[n - 1 - i] *= g;
    }
    let max = left.iter().chain(right.iter()).fold(0.0f32, |m, v| m.max(v.abs()));
    if max > 1e-6 {
        let g = peak.clamp(0.05, 1.0) / max;
        for v in left.iter_mut().chain(right.iter_mut()) {
            *v *= g;
        }
    }
}
