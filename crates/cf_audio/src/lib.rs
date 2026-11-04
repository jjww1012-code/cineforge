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
