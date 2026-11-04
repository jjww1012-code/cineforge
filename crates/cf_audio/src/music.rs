//! Einfacher, aber musikalischer Track-Generator: Akkordfolge, Abschnitte
//! (der Track kann „wachsen“), Stile mit passenden Rhythmen und Klängen.

use serde::{Deserialize, Serialize};

use crate::dsp::{FilterKind, Reverb, Wave, SR};
use crate::synth::{finish, render_layer, Filter, Layer, Stereo};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    #[default]
    Synthwave,
    Edm,
    Lofi,
    Cinematic,
    Ambient,
    Hiphop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub bars: u32,
    /// Instrumente: pulse, pad, bass, kick, snare, hats, drums (= kick+snare+hats), arp
    pub parts: Vec<String>,
    /// Lautstärke/Energie 0..1.
    #[serde(default = "one")]
    pub intensity: f32,
}

fn one() -> f32 {
    1.0
}
fn d_bpm() -> f32 {
    100.0
}
fn d_chords() -> Vec<String> {
    ["Am", "F", "C", "G"].map(String::from).to_vec()
}
fn d_beats() -> u32 {
    4
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MusicSpec {
    #[serde(default = "d_bpm")]
    pub bpm: f32,
    #[serde(default)]
    pub style: Style,
    /// Akkorde, z. B. ["Am","F","C","G"] (Dur, m, 7, maj7, m7, sus2, sus4, dim).
    #[serde(default = "d_chords")]
    pub chords: Vec<String>,
    /// Schläge pro Akkord (4 = ein Takt).
    #[serde(default = "d_beats")]
    pub beats_per_chord: u32,
    pub sections: Vec<Section>,
    #[serde(default)]
    pub seed: u32,
}


/// Akkord → MIDI-Noten (Grundton in Oktave 3).
pub fn parse_chord(c: &str) -> Result<Vec<i32>, String> {
    let c = c.trim();
    let mut chars = c.chars();
    let letter = chars.next().ok_or("leerer Akkord")?.to_ascii_uppercase();
    let mut root: i32 = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' | 'H' => 11,
        _ => return Err(format!("Akkord „{c}“: unbekannter Grundton")),
    };
    let mut rest: String = chars.collect();
    if rest.starts_with('#') {
        root += 1;
        rest.remove(0);
    } else if rest.starts_with('b') && !rest.is_empty() {
        root -= 1;
        rest.remove(0);
    }
    let intervals: &[i32] = match rest.as_str() {
        "" | "maj" => &[0, 4, 7],
        "m" | "min" => &[0, 3, 7],
        "7" => &[0, 4, 7, 10],
        "maj7" => &[0, 4, 7, 11],
        "m7" => &[0, 3, 7, 10],
        "sus2" => &[0, 2, 7],
        "sus4" => &[0, 5, 7],
        "dim" => &[0, 3, 6],
        "add9" => &[0, 4, 7, 14],
        other => return Err(format!("Akkord „{c}“: unbekannte Art „{other}“ (m, 7, maj7, m7, sus2, sus4, dim, add9)")),
    };
    let base = 48 + root.rem_euclid(12);
    Ok(intervals.iter().map(|i| base + i).collect())
}

fn hz(midi: i32) -> f32 {
    440.0 * 2f32.powf((midi as f32 - 69.0) / 12.0)
}

impl MusicSpec {
    pub fn validate(&self) -> Result<(), String> {
        if !(40.0..=220.0).contains(&self.bpm) {
            return Err("bpm muss zwischen 40 und 220 liegen".into());
        }
        if self.sections.is_empty() {
            return Err("sections ist leer".into());
        }
        if self.chords.is_empty() {
            return Err("chords ist leer".into());
        }
        for c in &self.chords {
            parse_chord(c)?;
        }
        let bars: u32 = self.sections.iter().map(|s| s.bars).sum();
        if bars == 0 || self.duration() > 900.0 {
            return Err("Länge muss zwischen 1 Takt und 15 Minuten liegen".into());
        }
        const KNOWN: [&str; 8] = ["pulse", "pad", "bass", "kick", "snare", "hats", "drums", "arp"];
        for s in &self.sections {
            for p in &s.parts {
                if !KNOWN.contains(&p.as_str()) {
                    return Err(format!("Unbekanntes Instrument „{p}“ (erlaubt: {})", KNOWN.join(", ")));
                }
            }
        }
        Ok(())
    }

    pub fn duration(&self) -> f32 {
        let bars: u32 = self.sections.iter().map(|s| s.bars).sum();
        bars as f32 * 4.0 * 60.0 / self.bpm
    }

    pub fn render(&self) -> Stereo {
        let beat = 60.0 / self.bpm;
        let tail = 3.0;
        let n = ((self.duration() + tail) * SR) as usize;
        let mut dry = (vec![0.0f32; n], vec![0.0f32; n]);
        let mut wet = (vec![0.0f32; n], vec![0.0f32; n]);
        let mut kicks: Vec<f32> = Vec::new();
        let chords: Vec<Vec<i32>> = self.chords.iter().map(|c| parse_chord(c).unwrap_or_else(|_| vec![57, 60, 64])).collect();
        let swing = if self.style == Style::Lofi { 0.12 } else { 0.0 };
        let mut rng = crate::dsp::Rng::new(self.seed ^ 0xCAFE);
        let mut bar0 = 0u32;
        for sec in &self.sections {
            let has = |p: &str| sec.parts.iter().any(|x| x == p) || (["kick", "snare", "hats"].contains(&p) && sec.parts.iter().any(|x| x == "drums"));
            let e = sec.intensity.clamp(0.0, 1.5);
            for b in 0..sec.bars {
                let bar = bar0 + b;
                let t_bar = bar as f32 * 4.0 * beat;
                for step in 0..16u32 {
                    let beat_pos = bar * 4 + step / 4;
                    let chord = &chords[((beat_pos / self.beats_per_chord.max(1)) as usize) % chords.len()];
                    let mut t = t_bar + step as f32 * beat / 4.0;
                    if step % 2 == 1 {
                        t += swing * beat / 2.0;
                    }
                    let seed = bar * 16 + step + self.seed;
                    // --- Schlagzeug
                    let (kick_on, snare_on, hat_on, open_hat) = match self.style {
                        Style::Edm => (step % 4 == 0, step == 4 || step == 12, step % 2 == 0, step % 4 == 2),
                        Style::Hiphop => ([0, 7, 10].contains(&step), step == 4 || step == 12, step % 2 == 0, false),
                        Style::Lofi => ([0, 6, 10].contains(&step), step == 4 || step == 12, step % 2 == 0, false),
                        Style::Cinematic => (step == 0 || step == 10, step == 8, false, false),
                        Style::Ambient => (false, false, false, false),
                        Style::Synthwave => (step == 0 || step == 8, step == 4 || step == 12, step % 2 == 0, false),
                    };
                    if has("kick") && kick_on {
                        kicks.push(t);
                        let deep = self.style == Style::Cinematic;
                        put(&mut dry, t, 0.95 * e, seed, &Layer {
                            wave: Wave::Sine,
                            freq: if deep { [110.0, 38.0] } else { [150.0, 45.0] },
                            length: 0.015,
                            decay: if deep { 0.5 } else { 0.22 },
                            release: 0.05,
                            drive: 0.35,
                            ..Layer::default()
                        });
                        put(&mut dry, t, 0.25 * e, seed + 1, &noise(0.001, 0.008, FilterKind::Highpass, 3000.0, 0.7));
                    }
                    if has("snare") && snare_on {
                        put(&mut dry, t, 0.55 * e, seed, &noise(0.004, 0.14, FilterKind::Bandpass, 1900.0, 0.8));
                        put(&mut dry, t, 0.3 * e, seed, &Layer { wave: Wave::Triangle, freq: [200.0, 165.0], length: 0.01, decay: 0.07, ..Layer::default() });
                        put(&mut wet, t, 0.12 * e, seed, &noise(0.004, 0.1, FilterKind::Bandpass, 1900.0, 0.8));
                    }
                    if has("hats") && hat_on {
                        let vel = if step % 4 == 0 { 0.22 } else { 0.14 } * (0.85 + 0.3 * rng.unit());
                        put(&mut dry, t, vel * e, seed, &noise(0.001, if open_hat { 0.16 } else { 0.035 }, FilterKind::Highpass, 7500.0, 0.7));
                    }
                    if has("hats") && open_hat && self.style == Style::Edm {
                        put(&mut dry, t, 0.12 * e, seed + 5, &noise(0.001, 0.18, FilterKind::Highpass, 6500.0, 0.7));
                    }
                    // --- Bass (Achtel oder Viertel, Grundton)
                    if has("bass") {
                        let pattern = match self.style {
                            Style::Synthwave | Style::Edm => step % 2 == 0,
                            Style::Cinematic => step % 2 == 0,
                            _ => step % 4 == 0 || step == 14,
                        };
                        if pattern {
                            let f = hz(chord[0] - 12);
                            let len = beat / 2.0 * 0.8;
                            put(&mut dry, t, 0.5 * e, seed, &Layer {
                                wave: Wave::Saw,
                                freq: [f, f],
                                length: len,
                                decay: 0.25,
                                sustain: 0.5,
                                release: 0.04,
                                filter: Some(Filter { kind: FilterKind::Lowpass, cutoff: [500.0 + 600.0 * e, 220.0], q: 1.1 }),
                                ..Layer::default()
                            });
                            put(&mut dry, t, 0.35 * e, seed, &Layer { wave: Wave::Sine, freq: [f, f], length: len, decay: 0.3, sustain: 0.7, release: 0.04, ..Layer::default() });
                        }
                    }
                    // --- Arpeggio (16tel durch die Akkordtöne)
                    if has("arp") {
                        let note = chord[(step as usize) % chord.len()] + 12 + if step % 8 >= 4 { 12 } else { 0 };
                        let f = hz(note);
                        put(&mut wet, t, 0.16 * e, seed, &Layer {
                            wave: Wave::Saw,
                            freq: [f, f],
                            length: 0.02,
                            decay: 0.16,
                            release: 0.05,
                            detune: 6.0,
                            filter: Some(Filter { kind: FilterKind::Lowpass, cutoff: [3500.0, 700.0], q: 1.4 }),
                            pan: if step % 2 == 0 { -0.35 } else { 0.35 },
                            ..Layer::default()
                        });
                    }
                }
                // --- Flächen je Akkordwechsel
                let chord_len = self.beats_per_chord.max(1) as f32 * beat;
                let per_bar = (4.0 * beat / chord_len).max(1.0) as u32;
                for c in 0..per_bar {
                    let t = t_bar + c as f32 * chord_len;
                    let beat_pos = bar * 4 + (c * self.beats_per_chord);
                    let chord = &chords[((beat_pos / self.beats_per_chord.max(1)) as usize) % chords.len()];
                    if has("pad") {
                        for (i, m) in chord.iter().enumerate() {
                            let f = hz(*m + 12);
                            put(&mut wet, t, 0.13 * e, bar * 7 + i as u32, &Layer {
                                wave: Wave::Saw,
                                freq: [f, f],
                                attack: (chord_len * 0.25).min(0.8),
                                length: chord_len * 0.9,
                                decay: 1.0,
                                sustain: 0.85,
                                release: chord_len * 0.5,
                                detune: 11.0,
                                filter: Some(Filter {
                                    kind: FilterKind::Lowpass,
                                    cutoff: if self.style == Style::Lofi { [700.0, 600.0] } else { [900.0 + 900.0 * e, 1200.0] },
                                    q: 0.8,
                                }),
                                pan: (i as f32 - 1.0) * 0.4,
                                ..Layer::default()
                            });
                        }
                    }
                    if has("pulse") {
                        // ein einzelner, gedämpfter Ton (wie aus einem Nebenraum)
                        let f = hz(chord[0]);
                        put(&mut wet, t, 0.2 * e, bar, &Layer {
                            wave: Wave::Triangle,
                            freq: [f, f],
                            attack: 0.4,
                            length: chord_len * 0.9,
                            decay: 1.0,
                            sustain: 0.9,
                            release: 1.0,
                            filter: Some(Filter { kind: FilterKind::Lowpass, cutoff: [500.0, 500.0], q: 0.7 }),
                            ..Layer::default()
                        });
                    }
                }
            }
            bar0 += sec.bars;
        }
        // Sidechain: Flächen und Bass weichen der Bassdrum aus (EDM/Synthwave)
        if matches!(self.style, Style::Edm | Style::Synthwave) && !kicks.is_empty() {
            let depth = if self.style == Style::Edm { 0.65 } else { 0.35 };
            let mut k = 0;
            for i in 0..n {
                let t = i as f32 / SR;
                while k + 1 < kicks.len() && kicks[k + 1] <= t {
                    k += 1;
                }
                if kicks[k] <= t {
                    let g = 1.0 - depth * (-(t - kicks[k]) / 0.12).exp();
                    wet.0[i] *= g;
                    wet.1[i] *= g;
                }
            }
        }
        // Hall auf die nasse Gruppe
        let size = match self.style {
            Style::Ambient | Style::Cinematic => 0.9,
            Style::Lofi => 0.5,
            _ => 0.7,
        };
        let (mut rl, mut rr) = (Reverb::new(size, 0.5, 0), Reverb::new(size, 0.5, 23));
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        for i in 0..n {
            let m = (wet.0[i] + wet.1[i]) * 0.5;
            l[i] = dry.0[i] + wet.0[i] + rl.process(m) * 1.2;
            r[i] = dry.1[i] + wet.1[i] + rr.process(m) * 1.2;
        }
        finish(&mut l, &mut r, 0.89);
        (l, r)
    }
}

fn noise(length: f32, decay: f32, kind: FilterKind, cutoff: f32, q: f32) -> Layer {
    Layer {
        wave: Wave::Noise,
        attack: 0.0005,
        length,
        decay,
        release: 0.02,
        filter: Some(Filter { kind, cutoff: [cutoff, cutoff], q }),
        ..Layer::default()
    }
}

fn put(buf: &mut Stereo, t: f32, gain: f32, seed: u32, layer: &Layer) {
    let (l, r) = buf;
    render_layer(layer, t, gain, 1.0, seed, l, r);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords() {
        assert_eq!(parse_chord("C").unwrap(), vec![48, 52, 55]);
        assert_eq!(parse_chord("Am").unwrap(), vec![57, 60, 64]);
        assert_eq!(parse_chord("F#m7").unwrap(), vec![54, 57, 61, 64]);
        assert_eq!(parse_chord("Bb").unwrap(), vec![58, 62, 65]);
        assert!(parse_chord("Xm").is_err());
        assert!(parse_chord("Cblub").is_err());
    }
}
