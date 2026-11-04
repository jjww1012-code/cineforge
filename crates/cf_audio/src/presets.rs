//! Fertige Klänge, wie Sounddesigner sie aus Synthese bauen.
//! Jeder Klang lässt sich über [`Tweak`] anpassen (Tonhöhe, Länge, Stärke, Helligkeit, Variante).

use serde::{Deserialize, Serialize};

use crate::dsp::{FilterKind, Wave};
use crate::synth::{EchoSpec, Filter, Fm, Layer, Recipe, Repeat, ReverbSpec};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Tweak {
    /// Tonhöhe (1 = normal, 2 = Oktave höher, 0.5 = tiefer).
    pub pitch: f32,
    /// Länge (1 = normal).
    pub length: f32,
    /// Stärke/Härte 0..1.
    pub intensity: f32,
    /// Helligkeit (1 = normal, <1 dumpfer, >1 heller).
    pub brightness: f32,
    /// Hall 0..1 (None = Standard der Voreinstellung).
    pub reverb: Option<f32>,
    /// Variante (anderer Zufall).
    pub seed: u32,
}

impl Default for Tweak {
    fn default() -> Self {
        Self { pitch: 1.0, length: 1.0, intensity: 0.5, brightness: 1.0, reverb: None, seed: 0 }
    }
}

pub const NAMES: &[(&str, &str)] = &[
    ("click", "kurzer, trockener UI-Klick"),
    ("tap", "weicher Finger-Tap aufs Display"),
    ("pop", "Blasen-Pop, verspielt"),
    ("whoosh", "Luftzug, z. B. Übergang oder schnelle Bewegung"),
    ("swoosh", "kurzer, heller Wisch"),
    ("impact", "Treffer/Schlag (Boxhandschuh auf Pratze)"),
    ("boom", "tiefer Einschlag mit Nachhall"),
    ("riser", "aufsteigende Spannung vor einem Höhepunkt"),
    ("notification", "zweitöniger Benachrichtigungs-Gong"),
    ("confirm", "positive Bestätigung (gebucht, geschafft)"),
    ("error", "dumpfer Fehlerton"),
    ("haptic", "Handy-Vibration (zwei kurze Summer)"),
    ("heartbeat", "Herzschlag (lub-dub)"),
    ("footstep", "Schritt auf hartem Boden"),
    ("wind", "Wind/Atmosphäre (Länge über length)"),
    ("rain", "Regen (Länge über length)"),
    ("fire", "knisterndes Feuer"),
    ("typing", "Tippen auf der Tastatur"),
    ("shutter", "Kamera-Auslöser"),
    ("glass", "helles Glas-Klingen"),
    ("success", "kleines Erfolgs-Arpeggio (Level geschafft)"),
    ("coin", "Münze/Punkt"),
    ("swipe", "Wischgeste über Glas"),
    ("rope", "Seilspringen (Schwung und Aufprall)"),
    ("crowd_cheer", "kurzer Jubel einer kleinen Gruppe"),
    ("drone", "dunkle, ruhige Fläche (Länge über length)"),
];

fn l() -> Layer {
    Layer::default()
}

fn filt(kind: FilterKind, a: f32, b: f32, q: f32) -> Option<Filter> {
    Some(Filter { kind, cutoff: [a, b], q })
}

pub fn preset(name: &str, t: &Tweak) -> Result<Recipe, String> {
    let p = t.pitch.clamp(0.1, 8.0);
    let len = t.length.clamp(0.1, 60.0);
    let br = t.brightness.clamp(0.1, 8.0);
    let int = t.intensity.clamp(0.0, 1.0);
    let rev = |d: f32, size: f32| t.reverb.or(Some(d)).filter(|m| *m > 0.0).map(|mix| ReverbSpec { size, mix, damp: 0.5 });
    use FilterKind::*;
    use Wave::*;
    let r = match name {
        "click" => Recipe {
            layers: vec![
                Layer { wave: Noise, length: 0.002, decay: 0.008, release: 0.004, filter: filt(Bandpass, 4500.0 * br, 3000.0 * br, 1.2), ..l() },
                Layer { wave: Sine, freq: [2400.0 * p, 1800.0 * p], length: 0.004, decay: 0.012, gain: 0.5, ..l() },
            ],
            reverb: rev(0.05, 0.2),
            ..Default::default()
        },
        "tap" => Recipe {
            layers: vec![
                Layer { wave: Sine, freq: [520.0 * p, 300.0 * p], length: 0.01, decay: 0.04, release: 0.02, ..l() },
                Layer { wave: Noise, length: 0.002, decay: 0.01, gain: 0.25, filter: filt(Lowpass, 2500.0 * br, 1200.0 * br, 0.7), ..l() },
            ],
            reverb: rev(0.06, 0.25),
            ..Default::default()
        },
        "pop" => Recipe {
            layers: vec![
                Layer { wave: Sine, freq: [900.0 * p, 180.0 * p], length: 0.03 * len, decay: 0.06 * len, release: 0.03, ..l() },
                Layer { wave: Noise, length: 0.001, decay: 0.006, gain: 0.3, filter: filt(Highpass, 2000.0 * br, 2000.0 * br, 0.7), ..l() },
            ],
            reverb: rev(0.1, 0.3),
            ..Default::default()
        },
        "whoosh" | "swoosh" => {
            let (d, f0, f1) = if name == "swoosh" { (0.28, 900.0, 5000.0) } else { (0.75, 250.0, 2800.0) };
            let d = d * len;
            Recipe {
                layers: vec![
                    Layer {
                        wave: Pink,
                        attack: d * 0.55,
                        length: 0.0,
                        decay: d * 0.4,
                        release: d * 0.45,
                        filter: filt(Bandpass, f0 * br, f1 * br, 1.4 + int * 2.0),
                        pan: -0.4,
                        ..l()
                    },
                    Layer {
                        wave: Noise,
                        attack: d * 0.6,
                        length: 0.0,
                        decay: d * 0.35,
                        release: d * 0.35,
                        gain: 0.35,
                        filter: filt(Bandpass, f0 * 2.0 * br, f1 * 1.5 * br, 2.5),
                        pan: 0.4,
                        ..l()
                    },
                ],
                reverb: rev(0.18, 0.4),
                ..Default::default()
            }
        }
        "impact" => Recipe {
            layers: vec![
                Layer { wave: Sine, freq: [160.0 * p, 48.0 * p], length: 0.02, decay: 0.16 * len, release: 0.08, drive: 0.3 + int * 0.5, ..l() },
                Layer { wave: Noise, length: 0.004, decay: 0.05, release: 0.03, gain: 0.8, filter: filt(Lowpass, 3500.0 * br, 900.0 * br, 0.8), ..l() },
                Layer { wave: Noise, length: 0.001, decay: 0.012, gain: 0.5 + int * 0.3, filter: filt(Bandpass, 2200.0 * br, 1800.0 * br, 1.5), ..l() },
            ],
            reverb: rev(0.12, 0.35),
            ..Default::default()
        },
        "boom" => Recipe {
            layers: vec![
                Layer { wave: Sine, freq: [90.0 * p, 28.0 * p], length: 0.05, decay: 0.9 * len, release: 0.5, drive: 0.4 + int * 0.4, ..l() },
                Layer { wave: Brown, length: 0.1, decay: 1.2 * len, release: 0.6, gain: 0.9, filter: filt(Lowpass, 1800.0 * br, 120.0, 0.7), ..l() },
                Layer { wave: Noise, length: 0.005, decay: 0.08, gain: 0.5, filter: filt(Lowpass, 5000.0 * br, 800.0, 0.7), ..l() },
            ],
            reverb: rev(0.35, 0.85),
            ..Default::default()
        },
        "riser" => {
            let d = 2.0 * len;
            Recipe {
                layers: vec![
                    Layer { wave: Noise, attack: d, length: 0.0, decay: 0.1, release: 0.05, filter: filt(Highpass, 300.0 * br, 6000.0 * br, 1.0), gain: 0.6, ..l() },
                    Layer { wave: Saw, freq: [180.0 * p, 1400.0 * p], attack: d, length: 0.0, decay: 0.1, release: 0.05, detune: 14.0, gain: 0.35, filter: filt(Lowpass, 600.0 * br, 7000.0 * br, 2.0), ..l() },
                ],
                reverb: rev(0.25, 0.6),
                ..Default::default()
            }
        }
        "notification" => Recipe {
            layers: vec![
                Layer { wave: Sine, freq: [1318.5 * p; 2], length: 0.02, decay: 0.45, release: 0.2, fm: Some(Fm { ratio: 3.01, index: [1.2, 0.0] }), ..l() },
                Layer { wave: Sine, start: 0.12, freq: [1975.5 * p; 2], length: 0.02, decay: 0.6, release: 0.3, fm: Some(Fm { ratio: 3.01, index: [1.0, 0.0] }), gain: 0.8, ..l() },
            ],
            reverb: rev(0.2, 0.45),
            ..Default::default()
        },
        "confirm" => Recipe {
            layers: vec![
                Layer { wave: Triangle, freq: [880.0 * p; 2], length: 0.04, decay: 0.12, release: 0.05, ..l() },
                Layer { wave: Triangle, start: 0.07, freq: [1318.5 * p; 2], length: 0.06, decay: 0.25, release: 0.1, ..l() },
                Layer { wave: Sine, start: 0.07, freq: [2637.0 * p; 2], length: 0.02, decay: 0.2, gain: 0.25, ..l() },
            ],
            reverb: rev(0.15, 0.35),
            ..Default::default()
        },
        "error" => Recipe {
            layers: vec![Layer {
                wave: Square,
                freq: [240.0 * p, 160.0 * p],
                length: 0.16,
                decay: 0.2,
                sustain: 0.6,
                release: 0.06,
                filter: filt(Lowpass, 1400.0 * br, 700.0 * br, 0.8),
                repeat: Some(Repeat { count: 2, interval: 0.2, decay: 0.8, pitch: 0.9, jitter: 0.0 }),
                ..l()
            }],
            reverb: rev(0.08, 0.3),
            ..Default::default()
        },
        "haptic" => Recipe {
            layers: vec![Layer {
                wave: Square,
                freq: [165.0 * p; 2],
                length: 0.045,
                attack: 0.004,
                decay: 0.05,
                sustain: 0.8,
                release: 0.02,
                filter: filt(Lowpass, 500.0 * br, 500.0 * br, 0.9),
                repeat: Some(Repeat { count: 2, interval: 0.12, decay: 1.0, pitch: 1.0, jitter: 0.0 }),
                ..l()
            }],
            ..Default::default()
        },
        "heartbeat" => Recipe {
            layers: vec![
                Layer { wave: Sine, freq: [70.0 * p, 45.0 * p], length: 0.02, decay: 0.12, release: 0.05, drive: 0.2, ..l() },
                Layer { wave: Sine, start: 0.22, freq: [60.0 * p, 40.0 * p], length: 0.02, decay: 0.1, release: 0.05, gain: 0.7, drive: 0.2, ..l() },
            ],
            ..Default::default()
        },
        "footstep" => Recipe {
            layers: vec![
                Layer { wave: Noise, length: 0.01, decay: 0.07, release: 0.03, filter: filt(Lowpass, 1600.0 * br, 500.0 * br, 0.8), ..l() },
                Layer { wave: Sine, freq: [110.0 * p, 60.0 * p], length: 0.005, decay: 0.05, gain: 0.6, ..l() },
                Layer { wave: Noise, start: 0.03, length: 0.005, decay: 0.03, gain: 0.25, filter: filt(Bandpass, 3000.0 * br, 2500.0 * br, 1.0), ..l() },
            ],
            reverb: rev(0.12, 0.3),
            seed: t.seed,
            ..Default::default()
        },
        "wind" => Recipe {
            duration: Some(4.0 * len),
            layers: vec![
                Layer { wave: Pink, attack: 0.8, length: 4.0 * len - 1.6, decay: 1.0, sustain: 1.0, release: 0.8, filter: filt(Bandpass, 350.0 * br, 900.0 * br, 1.8), vibrato: None, ..l() },
                Layer { wave: Brown, attack: 1.0, length: 4.0 * len - 2.0, decay: 1.0, sustain: 1.0, release: 1.0, gain: 0.6, filter: filt(Lowpass, 400.0 * br, 250.0 * br, 0.7), ..l() },
            ],
            ..Default::default()
        },
        "rain" => {
            let d = 4.0 * len;
            Recipe {
                duration: Some(d),
                layers: vec![
                    Layer { wave: Pink, attack: 0.4, length: d - 0.8, sustain: 1.0, release: 0.4, gain: 0.7, filter: filt(Highpass, 900.0 * br, 900.0 * br, 0.7), ..l() },
                    Layer {
                        wave: Noise,
                        length: 0.001,
                        decay: 0.006,
                        gain: 0.35,
                        filter: filt(Bandpass, 4000.0 * br, 3000.0 * br, 2.0),
                        repeat: Some(Repeat { count: (d * 60.0) as u32, interval: 1.0 / 60.0, decay: 1.0, pitch: 1.0, jitter: 0.02 }),
                        ..l()
                    },
                ],
                seed: t.seed,
                ..Default::default()
            }
        }
        "fire" => {
            let d = 3.0 * len;
            Recipe {
                duration: Some(d),
                layers: vec![
                    Layer { wave: Brown, attack: 0.3, length: d - 0.6, sustain: 1.0, release: 0.3, gain: 0.8, filter: filt(Lowpass, 600.0 * br, 600.0 * br, 0.7), ..l() },
                    Layer {
                        wave: Noise,
                        length: 0.0015,
                        decay: 0.01,
                        gain: 0.6,
                        filter: filt(Bandpass, 2500.0 * br, 1800.0 * br, 1.2),
                        repeat: Some(Repeat { count: (d * 14.0) as u32, interval: 1.0 / 14.0, decay: 1.0, pitch: 1.0, jitter: 0.06 }),
                        ..l()
                    },
                ],
                seed: t.seed,
                ..Default::default()
            }
        }
        "typing" => Recipe {
            layers: vec![Layer {
                wave: Noise,
                length: 0.002,
                decay: 0.02,
                filter: filt(Bandpass, 3200.0 * br, 2600.0 * br, 1.5),
                repeat: Some(Repeat { count: (12.0 * len) as u32, interval: 0.11, decay: 1.0, pitch: 1.0, jitter: 0.05 }),
                ..l()
            }],
            reverb: rev(0.05, 0.2),
            seed: t.seed,
            ..Default::default()
        },
        "shutter" => Recipe {
            layers: vec![
                Layer { wave: Noise, length: 0.003, decay: 0.02, filter: filt(Bandpass, 3500.0 * br, 2500.0 * br, 1.0), ..l() },
                Layer { wave: Noise, start: 0.07, length: 0.003, decay: 0.03, gain: 0.8, filter: filt(Bandpass, 2500.0 * br, 1800.0 * br, 1.0), ..l() },
                Layer { wave: Sine, start: 0.07, freq: [180.0 * p, 120.0 * p], length: 0.005, decay: 0.03, gain: 0.3, ..l() },
            ],
            reverb: rev(0.08, 0.25),
            ..Default::default()
        },
        "glass" => Recipe {
            layers: vec![
                Layer { wave: Sine, freq: [2793.0 * p; 2], length: 0.01, decay: 1.2 * len, release: 0.4, fm: Some(Fm { ratio: 2.76, index: [2.0, 0.0] }), ..l() },
                Layer { wave: Sine, freq: [4186.0 * p; 2], length: 0.01, decay: 0.7 * len, gain: 0.4, ..l() },
            ],
            reverb: rev(0.25, 0.5),
            ..Default::default()
        },
        "success" | "coin" => {
            let notes: &[f32] = if name == "coin" { &[987.8, 1318.5] } else { &[523.25, 659.25, 783.99, 1046.5] };
            let step = if name == "coin" { 0.07 } else { 0.08 };
            Recipe {
                layers: notes
                    .iter()
                    .enumerate()
                    .map(|(i, f)| Layer {
                        wave: Square,
                        start: i as f32 * step,
                        freq: [*f * p; 2],
                        length: 0.03,
                        decay: 0.18,
                        release: 0.08,
                        gain: 0.6,
                        filter: filt(Lowpass, 5000.0 * br, 2500.0 * br, 0.8),
                        ..l()
                    })
                    .collect(),
                reverb: rev(0.15, 0.35),
                echo: Some(EchoSpec { time: 0.12, feedback: 0.25, mix: 0.15 }),
                ..Default::default()
            }
        }
        "swipe" => Recipe {
            layers: vec![Layer {
                wave: Noise,
                attack: 0.05 * len,
                length: 0.0,
                decay: 0.12 * len,
                release: 0.05,
                gain: 0.7,
                filter: filt(Bandpass, 2500.0 * br, 6000.0 * br, 2.0),
                ..l()
            }],
            ..Default::default()
        },
        "rope" => Recipe {
            layers: vec![
                Layer { wave: Pink, attack: 0.12, length: 0.0, decay: 0.08, release: 0.04, gain: 0.5, filter: filt(Bandpass, 700.0 * br, 2200.0 * br, 2.0), ..l() },
                Layer { wave: Noise, start: 0.2, length: 0.002, decay: 0.03, gain: 0.8, filter: filt(Lowpass, 2500.0 * br, 900.0 * br, 0.8), ..l() },
            ],
            reverb: rev(0.12, 0.35),
            ..Default::default()
        },
        "crowd_cheer" => {
            let mut layers = Vec::new();
            for i in 0..14u32 {
                let f = 180.0 + (i as f32 * 37.0) % 220.0;
                layers.push(Layer {
                    wave: Saw,
                    start: (i as f32 * 0.031) % 0.25,
                    freq: [f * p, f * 1.35 * p],
                    attack: 0.15,
                    length: 0.6 * len,
                    decay: 0.5,
                    sustain: 0.7,
                    release: 0.5,
                    gain: 0.12,
                    vibrato: Some([5.0 + i as f32 * 0.3, 0.6]),
                    filter: filt(Bandpass, 900.0 * br, 1400.0 * br, 1.2),
                    pan: (i as f32 / 13.0) * 1.6 - 0.8,
                    ..l()
                });
            }
            layers.push(Layer { wave: Pink, attack: 0.2, length: 0.6 * len, decay: 0.5, sustain: 0.6, release: 0.6, gain: 0.5, filter: filt(Bandpass, 1200.0 * br, 1500.0 * br, 0.8), ..l() });
            Recipe { layers, reverb: rev(0.3, 0.55), ..Default::default() }
        }
        "drone" => {
            let d = 6.0 * len;
            Recipe {
                duration: Some(d),
                layers: [55.0f32, 82.41, 110.0]
                    .iter()
                    .enumerate()
                    .map(|(i, f)| Layer {
                        wave: Saw,
                        freq: [*f * p; 2],
                        attack: d * 0.3,
                        length: d * 0.4,
                        decay: 1.0,
                        sustain: 1.0,
                        release: d * 0.3,
                        detune: 9.0 + i as f32 * 3.0,
                        gain: 0.4,
                        filter: filt(Lowpass, 300.0 * br, 700.0 * br, 0.9),
                        pan: (i as f32 - 1.0) * 0.5,
                        ..l()
                    })
                    .collect(),
                reverb: rev(0.35, 0.9),
                ..Default::default()
            }
        }
        other => {
            let names: Vec<&str> = NAMES.iter().map(|(n, _)| *n).collect();
            let hint = suggest(other, &names).map(|s| format!(" → Meintest du „{s}“?")).unwrap_or_default();
            return Err(format!("Unbekannter Klang „{other}“{hint}. Verfügbar: {}", names.join(", ")));
        }
    };
    let mut r = r;
    if r.seed == 0 {
        r.seed = t.seed;
    }
    // Stärke: lauter angeschlagene Schichten etwas härter
    for layer in &mut r.layers {
        layer.drive = (layer.drive + int * 0.15).min(1.0);
    }
    Ok(r)
}

fn suggest<'a>(w: &str, names: &[&'a str]) -> Option<&'a str> {
    names.iter().copied().min_by_key(|n| {
        let common = n.chars().zip(w.chars()).filter(|(a, b)| a == b).count();
        (n.len().max(w.len()) - common) as i32
    }).filter(|n| n.chars().next() == w.chars().next())
}
