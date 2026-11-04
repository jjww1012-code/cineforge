//! Fremdmaterial: Bilder, Audio- und Videodateien.
//!
//! Ordner im Projekt: `Bilder/`, `Audio/`, `Clips/`. Dateien werden nur
//! innerhalb des CineForge-Ordners gesucht (kein Zugriff auf andere Orte).

use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::project::{root_dir, ActiveProject};

pub const IMAGES: &str = "Bilder";
pub const AUDIO: &str = "Audio";
pub const CLIPS: &str = "Clips";
pub const MODELS: &str = "Modelle";

/// Findet eine Mediendatei: Projekt/<Ordner>/datei, Projekt/datei, CineForge/<Ordner>/datei.
pub fn resolve(project: &ActiveProject, folder: &str, file: &str) -> Result<PathBuf, String> {
    let rel = Path::new(file);
    if rel.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(format!("„{file}“: '..' ist in Dateinamen nicht erlaubt"));
    }
    let root = root_dir()?;
    if rel.is_absolute() {
        return if rel.starts_with(&root) && rel.is_file() {
            Ok(rel.to_path_buf())
        } else {
            Err(format!("„{file}“: Dateien müssen im Ordner {} liegen", root.display()))
        };
    }
    let mut candidates = Vec::new();
    if let Some(d) = &project.dir {
        candidates.push(d.join(folder).join(rel));
        candidates.push(d.join(rel));
    }
    candidates.push(root.join(folder).join(rel));
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| {
            let where_ = project.dir.as_ref().map(|d| d.join(folder)).unwrap_or_else(|| root.join(folder));
            format!("Datei „{file}“ nicht gefunden (bitte in {} ablegen)", where_.display())
        })
}

/// Prüft die Datei-Verweise einer Szene (3D-Modelle, Bilder, SVG-Dateien, Texturen),
/// damit Claude fehlende oder kaputte Dateien sofort erfährt – nicht erst im Bild.
pub fn check_files(project: &ActiveProject, scene: &cf_scene::Scene) -> Vec<String> {
    use cf_scene::Kind;
    let mut out = Vec::new();
    scene.walk(&mut |o| {
        let mut need = |folder: &str, file: &str| -> Option<PathBuf> {
            resolve(project, folder, file).map_err(|e| out.push(format!("\"{}\": {e}", o.id))).ok()
        };
        let svg_problem = match &o.kind {
            Kind::Model { file } => need(MODELS, file).and(None),
            Kind::Image { file, .. } => need(IMAGES, file).and(None),
            Kind::Svg { svg: None, file: Some(file), width, depth, mode, .. } => need(IMAGES, file).and_then(|path| {
                std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|text| cf_scene::svg::check(&text, *width, *depth, *mode))
                    .err()
                    .map(|e| format!("\"{}\": {file}: {e}", o.id))
            }),
            _ => None,
        };
        if let Some(t) = &o.material.texture {
            need(IMAGES, t);
        }
        out.extend(svg_problem);
    });
    out
}

/// Listet Dateien eines Medien-Ordners im Projekt.
pub fn list(project: &ActiveProject, folder: &str) -> (PathBuf, Vec<String>) {
    let dir = project.dir.as_ref().map(|d| d.join(folder)).unwrap_or_else(|| root_dir().unwrap_or_default().join(folder));
    let mut v: Vec<String> = std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();
    v.sort();
    (dir, v)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaInfo {
    pub duration: Option<f32>,
    pub has_audio: bool,
    pub has_video: bool,
}

/// Länge und Spuren einer Audio-/Videodatei (über ffmpeg).
pub fn probe(path: &Path) -> Result<MediaInfo, String> {
    let ff = crate::video::find_ffmpeg().ok_or("ffmpeg nicht gefunden – nötig für Audio und Video-Clips")?;
    let out = std::process::Command::new(ff)
        .args(["-hide_banner", "-i"])
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(parse_probe(&String::from_utf8_lossy(&out.stderr)))
}

pub fn parse_probe(stderr: &str) -> MediaInfo {
    let duration = stderr.find("Duration: ").and_then(|i| {
        let t = &stderr[i + 10..];
        let t = t.split(',').next()?.trim();
        let parts: Vec<f32> = t.split(':').filter_map(|x| x.parse().ok()).collect();
        (parts.len() == 3).then(|| parts[0] * 3600.0 + parts[1] * 60.0 + parts[2])
    });
    let streams: Vec<&str> = stderr.lines().filter(|l| l.trim_start().starts_with("Stream #")).collect();
    MediaInfo {
        duration,
        has_audio: streams.iter().any(|l| l.contains("Audio:")),
        has_video: streams.iter().any(|l| l.contains("Video:") && !l.contains("attached pic")),
    }
}

/// Geladene Bilder (mit Prüfung auf Änderung der Datei).
#[derive(Resource, Default)]
pub struct ImageCache {
    images: HashMap<PathBuf, (Option<SystemTime>, LoadedImage)>,
}

#[derive(Clone)]
pub struct LoadedImage {
    pub handle: Handle<Image>,
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
}

impl ImageCache {
    pub fn load(&mut self, path: &Path, images: &mut Assets<Image>) -> Result<LoadedImage, String> {
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        if let Some((m, img)) = self.images.get(path) {
            if *m == modified {
                return Ok(img.clone());
            }
        }
        let decoded = image::ImageReader::open(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .with_guessed_format()
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| format!("Bild „{}“ nicht lesbar: {e}", path.display()))?;
        let rgba = decoded.to_rgba8();
        let (w, h) = rgba.dimensions();
        let has_alpha = rgba.pixels().any(|p| p.0[3] < 250);
        let mut img = Image::new(
            Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            TextureDimension::D2,
            rgba.into_raw(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            anisotropy_clamp: 1,
            ..default()
        });
        let loaded = LoadedImage { handle: images.add(img), width: w, height: h, has_alpha };
        self.images.insert(path.to_path_buf(), (modified, loaded.clone()));
        Ok(loaded)
    }
}

/// Text-zu-Sprache mit den eingebauten Stimmen des Betriebssystems (kostenlos, offline).
/// Mac: `say`, Windows: System.Speech, Linux: espeak-ng. Ergebnis: WAV-Datei.
pub fn synthesize(text: &str, voice: Option<&str>, rate: Option<u32>, out_wav: &Path) -> Result<(), String> {
    if let Some(dir) = out_wav.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let status = if cfg!(target_os = "macos") {
        let aiff = out_wav.with_extension("aiff");
        let mut c = std::process::Command::new("say");
        c.arg("-o").arg(&aiff);
        if let Some(v) = voice {
            c.args(["-v", v]);
        }
        if let Some(r) = rate {
            c.args(["-r", &r.to_string()]);
        }
        let st = c.arg(text).status().map_err(|e| format!("say: {e}"))?;
        if st.success() {
            convert_to_wav(&aiff, out_wav)?;
            let _ = std::fs::remove_file(&aiff);
        }
        st
    } else if cfg!(windows) {
        let esc = |s: &str| s.replace('\'', "''");
        let mut script = String::from("Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; ");
        if let Some(v) = voice {
            script += &format!("$s.SelectVoice('{}'); ", esc(v));
        }
        if let Some(r) = rate {
            script += &format!("$s.Rate = {}; ", ((r as i32 - 180) / 20).clamp(-10, 10));
        }
        script += &format!("$s.SetOutputToWaveFile('{}'); $s.Speak('{}'); $s.Dispose()", esc(&out_wav.to_string_lossy()), esc(text));
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .map_err(|e| format!("PowerShell: {e}"))?
    } else {
        let mut c = std::process::Command::new("espeak-ng");
        c.args(["-v", voice.unwrap_or("de"), "-w"]).arg(out_wav);
        if let Some(r) = rate {
            c.args(["-s", &r.to_string()]);
        }
        c.arg(text).status().map_err(|e| format!("espeak-ng: {e}"))?
    };
    if status.success() && out_wav.is_file() {
        Ok(())
    } else {
        Err("Sprachausgabe fehlgeschlagen (Stimme vorhanden?)".into())
    }
}

fn convert_to_wav(input: &Path, out: &Path) -> Result<(), String> {
    match crate::video::find_ffmpeg() {
        Some(ff) => {
            let st = std::process::Command::new(ff)
                .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
                .arg(input)
                .arg(out)
                .status()
                .map_err(|e| e.to_string())?;
            if st.success() { Ok(()) } else { Err("Umwandlung in WAV fehlgeschlagen".into()) }
        }
        None => std::fs::copy(input, out.with_extension("aiff")).map(|_| ()).map_err(|e| e.to_string()),
    }
}

/// Verfügbare Systemstimmen (für die Auswahl durch Claude).
pub fn list_voices() -> String {
    let out = if cfg!(target_os = "macos") {
        std::process::Command::new("say").args(["-v", "?"]).output().ok()
    } else if cfg!(windows) {
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Add-Type -AssemblyName System.Speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).GetInstalledVoices() | % { $_.VoiceInfo.Name + ' – ' + $_.VoiceInfo.Culture }"])
            .output()
            .ok()
    } else {
        std::process::Command::new("espeak-ng").arg("--voices=de").output().ok()
    };
    out.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_else(|| "keine Stimmen gefunden".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_parsing() {
        let s = "Input #0, mov,mp4\n  Duration: 00:01:05.50, start: 0.0, bitrate: 1 kb/s\n  Stream #0:0[0x1](und): Video: h264, yuv420p\n  Stream #0:1[0x2](und): Audio: aac, 48000 Hz\n";
        let i = parse_probe(s);
        assert_eq!(i.duration, Some(65.5));
        assert!(i.has_audio && i.has_video);
        let only_audio = parse_probe("  Duration: 00:00:02.00, start\n  Stream #0:0: Audio: pcm_s16le\n");
        assert!(only_audio.has_audio && !only_audio.has_video);
    }

    #[test]
    fn resolve_rejects_escapes() {
        let p = ActiveProject { dir: None, data: Default::default() };
        assert!(resolve(&p, AUDIO, "../geheim.wav").is_err());
        assert!(resolve(&p, AUDIO, "/etc/passwd").is_err());
    }
}
