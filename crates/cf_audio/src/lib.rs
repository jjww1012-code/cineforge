//! # cf_audio
//!
//! Klangsynthese für CineForge: fertige Effekte ([`presets`]), eigene
//! Rezepte ([`synth::Recipe`]) und Musik ([`music::MusicSpec`]).
//! Alles deterministisch (gleiche Angaben ⇒ gleicher Klang), Ausgabe als
//! 48-kHz-Stereo-WAV.

pub mod dsp;
pub mod music;
pub mod presets;
pub mod synth;

use std::io::Write;
use std::path::Path;

pub use synth::Stereo;

/// Schreibt 16-Bit-PCM-Stereo-WAV.
pub fn write_wav(path: &Path, (l, r): &Stereo) -> std::io::Result<()> {
    let n = l.len().min(r.len());
    let data_len = (n * 4) as u32;
    let mut out = Vec::with_capacity(44 + n * 4);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&2u16.to_le_bytes()); // Stereo
    out.extend_from_slice(&48_000u32.to_le_bytes());
    out.extend_from_slice(&(48_000u32 * 4).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..n {
        for v in [l[i], r[i]] {
            let s = (v.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            out.extend_from_slice(&s.to_le_bytes());
        }
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::File::create(path)?;
    f.write_all(&out)
}

/// Kurze Klang-Analyse (Länge, Spitze, Lautheit) – hilft Claude beim Prüfen.
pub fn describe((l, r): &Stereo) -> String {
    let n = l.len().max(1);
    let peak = l.iter().chain(r.iter()).fold(0.0f32, |m, v| m.max(v.abs()));
    let rms = (l.iter().chain(r.iter()).map(|v| v * v).sum::<f32>() / (2 * n) as f32).sqrt();
    let db = |x: f32| if x > 1e-9 { 20.0 * x.log10() } else { -120.0 };
    format!("{:.2} s, Spitze {:.1} dBFS, Lautheit (RMS) {:.1} dBFS", n as f32 / dsp::SR, db(peak), db(rms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::{MusicSpec, Section, Style};
    use crate::presets::{preset, Tweak, NAMES};

    fn check(s: &Stereo, name: &str) {
        let (l, r) = s;
        assert!(!l.is_empty() && l.len() == r.len(), "{name}: leer");
        assert!(l.iter().chain(r.iter()).all(|v| v.is_finite()), "{name}: NaN");
        let peak = l.iter().chain(r.iter()).fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.5 && peak <= 1.0, "{name}: Spitze {peak}");
        // nicht nur Stille: Energie in der ersten Hälfte
        let energy: f32 = l.iter().take(l.len() / 2 + 1).map(|v| v * v).sum();
        assert!(energy > 1e-3, "{name}: keine Energie");
        // kein Knacken am Ende
        assert!(l.last().unwrap().abs() < 0.05, "{name}: Ende knackt");
    }

    #[test]
    fn all_presets_render_cleanly() {
        for (name, _) in NAMES {
            let r = preset(name, &Tweak::default()).unwrap();
            r.validate().unwrap();
            check(&r.render(), name);
        }
        assert!(preset("whosh", &Tweak::default()).unwrap_err().contains("whoosh"));
    }

    #[test]
    fn tweaks_change_the_sound() {
        let a = preset("pop", &Tweak::default()).unwrap().render();
        let b = preset("pop", &Tweak { length: 3.0, ..Tweak::default() }).unwrap().render();
        assert!(b.0.len() > a.0.len());
        // deterministisch
        let c = preset("pop", &Tweak::default()).unwrap().render();
        assert_eq!(a.0, c.0);
    }

    #[test]
    fn custom_recipe_from_json() {
        let r: synth::Recipe = serde_json::from_str(
            r#"{"layers":[{"wave":"saw","freq":[220,110],"length":0.3,"filter":{"type":"lowpass","cutoff":[3000,200]}}],"reverb":{"size":0.6,"mix":0.3}}"#,
        )
        .unwrap();
        check(&r.render(), "eigenes Rezept");
        assert!(serde_json::from_str::<synth::Recipe>(r#"{"layers":[{"wav":"saw"}]}"#).is_err());
    }

    #[test]
    fn music_grows_over_sections() {
        let spec = MusicSpec {
            bpm: 120.0,
            style: Style::Synthwave,
            chords: vec!["Am".into(), "F".into(), "C".into(), "G".into()],
            beats_per_chord: 4,
            sections: vec![
                Section { bars: 2, parts: vec!["pulse".into()], intensity: 0.6 },
                Section { bars: 2, parts: vec!["pad".into(), "bass".into(), "drums".into(), "arp".into()], intensity: 1.0 },
            ],
            seed: 1,
        };
        spec.validate().unwrap();
        assert!((spec.duration() - 8.0).abs() < 1e-3);
        let s = spec.render();
        check(&s, "Musik");
        let rms = |a: usize, b: usize| (s.0[a..b].iter().map(|v| v * v).sum::<f32>() / (b - a) as f32).sqrt();
        let quiet = rms(48_000, 3 * 48_000);
        let loud = rms(5 * 48_000, 7 * 48_000);
        assert!(loud > quiet * 1.8, "Track wächst: {quiet} → {loud}");
        let bad = MusicSpec { sections: vec![Section { bars: 1, parts: vec!["gitarre".into()], intensity: 1.0 }], ..spec };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn wav_roundtrip_header() {
        let s = preset("click", &Tweak::default()).unwrap().render();
        let p = std::env::temp_dir().join(format!("cf_audio_{}.wav", std::process::id()));
        write_wav(&p, &s).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(bytes.len(), 44 + s.0.len() * 4);
        std::fs::remove_file(p).ok();
    }
}

/// Liest eine WAV-Datei (PCM 8/16/24/32 Bit oder Float 32) und liefert (Abtastrate, Mono-Samples).
pub fn read_wav(bytes: &[u8]) -> Result<(u32, Vec<f32>), String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("keine WAV-Datei".into());
    }
    let (mut fmt, mut data): (Option<(u16, u16, u32, u16)>, Option<&[u8]>) = (None, None);
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        let body = &bytes[i + 8..(i + 8 + len).min(bytes.len())];
        if id == b"fmt " && body.len() >= 16 {
            let tag = u16::from_le_bytes([body[0], body[1]]);
            let ch = u16::from_le_bytes([body[2], body[3]]);
            let sr = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
            let bits = u16::from_le_bytes([body[14], body[15]]);
            // WAVE_FORMAT_EXTENSIBLE: eigentliches Format im Untertyp
            let tag = if tag == 0xFFFE && body.len() >= 26 { u16::from_le_bytes([body[24], body[25]]) } else { tag };
            fmt = Some((tag, ch.max(1), sr, bits));
        } else if id == b"data" {
            data = Some(body);
        }
        i += 8 + len + (len & 1);
    }
    let (tag, ch, sr, bits) = fmt.ok_or("WAV ohne fmt-Abschnitt")?;
    let data = data.ok_or("WAV ohne Daten")?;
    let bps = (bits / 8) as usize;
    if bps == 0 {
        return Err("ungültige Bittiefe".into());
    }
    let frame = bps * ch as usize;
    let sample = |b: &[u8]| -> f32 {
        match (tag, bits) {
            (3, 32) => f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            (_, 8) => (b[0] as f32 - 128.0) / 128.0,
            (_, 16) => i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0,
            (_, 24) => (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0,
            (_, 32) => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 2_147_483_648.0,
            _ => 0.0,
        }
    };
    if !matches!(tag, 1 | 3) {
        return Err(format!("WAV-Format {tag} wird nicht unterstützt"));
    }
    let mono = data
        .chunks_exact(frame)
        .map(|f| (0..ch as usize).map(|c| sample(&f[c * bps..])).sum::<f32>() / ch as f32)
        .collect();
    Ok((sr, mono))
}

/// Lautstärke-Hüllkurve für Lippen-Sync: `rate` Werte pro Sekunde, 0..1
/// (normiert auf laute Stellen, schnelles Öffnen, etwas langsameres Schließen).
pub fn envelope(samples: &[f32], sample_rate: u32, rate: f32) -> Vec<f32> {
    let hop = ((sample_rate as f32 / rate).round() as usize).max(1);
    let mut rms: Vec<f32> = samples
        .chunks(hop)
        .map(|c| (c.iter().map(|x| x * x).sum::<f32>() / c.len().max(1) as f32).sqrt())
        .collect();
    let mut sorted = rms.clone();
    sorted.sort_by(f32::total_cmp);
    let loud = sorted.get(sorted.len() * 95 / 100).copied().unwrap_or(0.0).max(1e-4);
    let gate = loud * 0.08; // Atem und Rauschen bleiben geschlossen
    let mut prev = 0.0;
    for v in &mut rms {
        let x = ((*v - gate) / (loud - gate)).clamp(0.0, 1.0);
        let x = x.sqrt(); // Mund öffnet schon bei mittlerer Lautstärke deutlich
        prev = if x > prev { prev + (x - prev) * 0.8 } else { prev + (x - prev) * 0.35 };
        *v = prev;
    }
    rms
}

#[cfg(test)]
mod wav_tests {
    use super::*;

    #[test]
    fn wav_roundtrip_and_envelope() {
        let sr = crate::dsp::SR as usize;
        let mut l = vec![0.0f32; sr];
        for (i, s) in l.iter_mut().enumerate().skip(sr / 4).take(sr / 2) {
            *s = (i as f32 * 0.05).sin() * 0.5;
        }
        let path = std::env::temp_dir().join(format!("cf_wav_test_{}.wav", std::process::id()));
        write_wav(&path, &(l.clone(), l.clone())).unwrap();
        let (rate, mono) = read_wav(&std::fs::read(&path).unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(rate, crate::dsp::SR as u32);
        assert_eq!(mono.len(), sr);
        assert!((mono[sr / 2] - l[sr / 2]).abs() < 1e-3);
        let env = envelope(&mono, rate, 100.0);
        assert_eq!(env.len(), 100);
        assert!(env[10] < 0.05 && env[50] > 0.8 && env[95] < 0.1, "{:?}", &env[..]);
    }
}
