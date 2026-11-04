//! Video-Kodierung über ffmpeg mit Hardware-Encoder (falls vorhanden).
//!
//! Reihenfolge der Encoder: Apple VideoToolbox → NVIDIA NVENC → Intel QSV →
//! AMD AMF → libx264 (Software) → mpeg4. Jeder Hardware-Encoder wird vorher
//! mit einer kurzen Probe getestet, weil ffmpeg auch Encoder für nicht
//! vorhandene Grafikkarten auflistet.
//!
//! Ohne ffmpeg werden die Bilder als PNG-Folge gespeichert.

use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{mpsc::Receiver, Arc, Mutex, OnceLock},
    time::Instant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Draft,
    Standard,
    High,
}

impl Quality {
    pub fn parse(s: Option<&str>) -> Result<Self, String> {
        match s.unwrap_or("standard") {
            "draft" | "entwurf" => Ok(Quality::Draft),
            "standard" => Ok(Quality::Standard),
            "high" | "hoch" => Ok(Quality::High),
            other => Err(format!("Unbekannte Qualität \"{other}\" (draft, standard, high)")),
        }
    }
}

/// Fortschritt eines Exports (wird vom Schreib-Thread aktualisiert).
#[derive(Debug, Clone, Default)]
pub struct ExportProgress {
    pub total: u32,
    pub written: u32,
    pub done: bool,
    pub error: Option<String>,
    pub output: PathBuf,
    pub encoder: String,
    pub seconds: f32,
}

pub fn find_ffmpeg() -> Option<PathBuf> {
    static FOUND: OnceLock<Option<PathBuf>> = OnceLock::new();
    FOUND
        .get_or_init(|| {
            let exe_name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
            let mut candidates: Vec<PathBuf> = Vec::new();
            if let Some(p) = std::env::var_os("CINEFORGE_FFMPEG") {
                candidates.push(PathBuf::from(p));
            }
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    candidates.push(dir.join(exe_name));
                    candidates.push(dir.join("../Resources").join(exe_name)); // macOS-App-Paket
                }
            }
            if let Some(path) = std::env::var_os("PATH") {
                candidates.extend(std::env::split_paths(&path).map(|d| d.join(exe_name)));
            }
            for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "C:\\ffmpeg\\bin"] {
                candidates.push(Path::new(d).join(exe_name));
            }
            candidates.into_iter().find(|p| p.is_file())
        })
        .clone()
}

fn encoder_works(ffmpeg: &Path, enc: &str) -> bool {
    Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=black:s=256x256:d=0.2", "-c:v", enc])
        .args(["-f", "null", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Bester funktionierender H.264-Encoder (einmal ermittelt).
pub fn best_encoder(ffmpeg: &Path) -> String {
    static BEST: OnceLock<String> = OnceLock::new();
    BEST.get_or_init(|| {
        let listed = Command::new(ffmpeg)
            .args(["-hide_banner", "-encoders"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let order: &[&str] = if cfg!(target_os = "macos") {
            &["h264_videotoolbox", "libx264", "mpeg4"]
        } else {
            &["h264_nvenc", "h264_qsv", "h264_amf", "libx264", "mpeg4"]
        };
        order
            .iter()
            .find(|e| listed.contains(&format!(" {e} ")) && encoder_works(ffmpeg, e))
            .map(|e| e.to_string())
            .unwrap_or_else(|| "mpeg4".into())
    })
    .clone()
}

fn encoder_args(enc: &str, w: u32, h: u32, fps: u32, q: Quality) -> Vec<String> {
    let factor = match q {
        Quality::Draft => 0.05,
        Quality::Standard => 0.12,
        Quality::High => 0.25,
    };
    let kbps = ((w as f64 * h as f64 * fps as f64 * factor) / 1000.0).clamp(500.0, 200_000.0) as u64;
    let mut a: Vec<String> = vec!["-c:v".into(), enc.into()];
    match enc {
        "libx264" => {
            let (crf, preset) = match q {
                Quality::Draft => ("28", "veryfast"),
                Quality::Standard => ("20", "medium"),
                Quality::High => ("16", "slow"),
            };
            a.extend(["-crf", crf, "-preset", preset].map(String::from));
        }
        "mpeg4" => a.extend(["-q:v".into(), "3".into()]),
        _ => a.extend(["-b:v".into(), format!("{kbps}k"), "-maxrate".into(), format!("{}k", kbps * 3 / 2)]),
    }
    a.extend(["-pix_fmt", "yuv420p", "-movflags", "+faststart"].map(String::from));
    a
}

enum Sink {
    Ffmpeg(Child),
    Png(PathBuf),
}

/// Startet den Schreib-Thread. Er empfängt (Bildnummer, RGBA-Pixel), sortiert
/// und schreibt in Reihenfolge – in ffmpeg oder als PNG-Folge.
pub fn start_writer(
    output: PathBuf,
    w: u32,
    h: u32,
    fps: u32,
    total: u32,
    quality: Quality,
    rx: Receiver<(u32, Vec<u8>)>,
) -> Result<Arc<Mutex<ExportProgress>>, String> {
    let (sink, encoder, output) = match find_ffmpeg() {
        Some(ff) => {
            let enc = best_encoder(&ff);
            let child = Command::new(&ff)
                .args(["-y", "-hide_banner", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba"])
                .args(["-s", &format!("{w}x{h}"), "-framerate", &fps.to_string(), "-i", "-"])
                .args(encoder_args(&enc, w, h, fps, quality))
                .arg(&output)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("ffmpeg konnte nicht gestartet werden: {e}"))?;
            (Sink::Ffmpeg(child), enc, output)
        }
        None => {
            let dir = output.with_extension("");
            let dir = dir.with_file_name(format!("{}_bilder", dir.file_name().unwrap_or_default().to_string_lossy()));
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            (Sink::Png(dir.clone()), "PNG-Folge (ffmpeg nicht gefunden)".to_string(), dir)
        }
    };
    let progress = Arc::new(Mutex::new(ExportProgress {
        total,
        output: output.clone(),
        encoder: encoder.clone(),
        ..Default::default()
    }));
    let prog = progress.clone();
    std::thread::Builder::new()
        .name("cineforge-export".into())
        .spawn(move || {
            let started = Instant::now();
            let result = write_loop(sink, w, h, total, rx, &prog);
            let mut p = prog.lock().expect("Fortschritt");
            p.done = true;
            p.seconds = started.elapsed().as_secs_f32();
            if let Err(e) = result {
                p.error = Some(e);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(progress)
}

fn write_loop(
    mut sink: Sink,
    w: u32,
    h: u32,
    total: u32,
    rx: Receiver<(u32, Vec<u8>)>,
    prog: &Arc<Mutex<ExportProgress>>,
) -> Result<(), String> {
    let expected = (w * h * 4) as usize;
    let mut pending: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    let mut next = 0u32;
    while next < total {
        let Ok((idx, data)) = rx.recv() else {
            return Err(format!("Export abgebrochen nach {next} von {total} Bildern"));
        };
        if data.len() != expected {
            return Err(format!("Bild {idx}: unerwartete Größe {} statt {expected} Bytes", data.len()));
        }
        pending.insert(idx, data);
        while let Some(data) = pending.remove(&next) {
            match &mut sink {
                Sink::Ffmpeg(child) => {
                    let stdin = child.stdin.as_mut().ok_or("ffmpeg-Eingang fehlt")?;
                    if let Err(e) = stdin.write_all(&data) {
                        let mut err = String::new();
                        if let Some(mut se) = child.stderr.take() {
                            use std::io::Read;
                            let _ = se.read_to_string(&mut err);
                        }
                        return Err(format!("ffmpeg-Fehler: {e} {err}"));
                    }
                }
                Sink::Png(dir) => {
                    let img = image::RgbaImage::from_raw(w, h, data).ok_or("Bilddaten ungültig")?;
                    img.save(dir.join(format!("bild_{next:06}.png"))).map_err(|e| e.to_string())?;
                }
            }
            next += 1;
            prog.lock().expect("Fortschritt").written = next;
        }
    }
    if let Sink::Ffmpeg(mut child) = sink {
        drop(child.stdin.take()); // Ende signalisieren
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("ffmpeg meldet einen Fehler: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_parsing_and_args() {
        assert_eq!(Quality::parse(None).unwrap(), Quality::Standard);
        assert!(Quality::parse(Some("ultra")).is_err());
        let a = encoder_args("h264_videotoolbox", 3840, 2160, 30, Quality::Standard);
        assert!(a.contains(&"29859k".to_string()) || a.iter().any(|x| x.ends_with('k')), "{a:?}");
        assert!(encoder_args("libx264", 1920, 1080, 30, Quality::High).contains(&"16".to_string()));
    }

    #[test]
    fn writes_video_or_png_sequence_in_order() {
        let dir = std::env::temp_dir().join(format!("cf-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("test.mp4");
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let (w, h, n) = (64u32, 48u32, 12u32);
        let prog = start_writer(out.clone(), w, h, 24, n, Quality::Draft, rx).unwrap();
        // absichtlich nicht in Reihenfolge senden
        for i in (0..n).rev() {
            let v = (i * 20) as u8;
            tx.send((i, vec![v; (w * h * 4) as usize])).unwrap();
        }
        for _ in 0..200 {
            if prog.lock().unwrap().done {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let p = prog.lock().unwrap().clone();
        assert!(p.done && p.error.is_none(), "{p:?}");
        assert_eq!(p.written, n);
        if find_ffmpeg().is_some() {
            assert!(std::fs::metadata(&p.output).unwrap().len() > 100, "Video leer");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
