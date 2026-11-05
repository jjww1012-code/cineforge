//! Absturzbericht: Beim Start per Doppelklick sieht man keine Fehlermeldungen.
//! Deshalb landen Abstürze und Startprobleme in `Dokumente/CineForge/absturz.txt`.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Sicherer Modus: ohne Kino-Effekte (SSAO, Tiefenschärfe, Bewegungsunschärfe, Lichtstrahlen),
/// nach einem Absturz beim letzten Start.
pub static SAFE_MODE: AtomicBool = AtomicBool::new(false);

pub fn safe_mode() -> bool {
    SAFE_MODE.load(Ordering::Relaxed)
}

/// Gerade geöffnete Szene (für den Absturzbericht und den nächsten Start).
static CURRENT_SCENE: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

pub fn set_current_scene(p: &std::path::Path) {
    if let Ok(mut c) = CURRENT_SCENE.lock() {
        *c = Some(p.to_path_buf());
    }
}

fn marker() -> Option<std::path::PathBuf> {
    crate::project::root_dir().ok().map(|r| r.join(".start_laeuft"))
}

fn crash_scene_file() -> Option<std::path::PathBuf> {
    crate::project::root_dir().ok().map(|r| r.join(".absturz_szene"))
}

/// Beim Start aufrufen: Liefert einen Grund, falls der letzte Start (oder die zuletzt geöffnete
/// Szene) abgestürzt ist – dann im sicheren Modus starten. Setzt die Start-Markierung.
pub fn check_last_start(startup_scene: Option<&std::path::Path>) -> Option<String> {
    let mut reason = None;
    if let Some(m) = marker() {
        if m.exists() {
            reason = Some("Der letzte Start von CineForge ist abgestürzt.".to_string());
        }
        if let Some(d) = m.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(&m, crate::project::now());
    }
    if let (Some(f), Some(scene)) = (crash_scene_file(), startup_scene) {
        if let Ok(p) = std::fs::read_to_string(&f) {
            if std::path::Path::new(p.trim()) == scene {
                reason = Some(format!("Beim letzten Mal ist CineForge mit der Szene „{}“ abgestürzt.", scene.file_stem().unwrap_or_default().to_string_lossy()));
            }
        }
        let _ = std::fs::remove_file(&f);
    }
    reason
}

/// Nach einigen Sekunden ohne Absturz: Start-Markierung entfernen.
pub fn startup_ok() {
    if let Some(m) = marker() {
        let _ = std::fs::remove_file(m);
    }
}

/// Fängt Abstürze ab und schreibt sie (mit Aufrufliste) in den Bericht.
pub fn install() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        let scene = CURRENT_SCENE.lock().ok().and_then(|c| c.clone());
        note(&format!("ABSTURZ: {info}\nSzene: {}\n{bt}", scene.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "–".into())));
        // Merken, mit welcher Szene es abgestürzt ist: beim nächsten Start nicht wieder öffnen
        if let (Some(f), Some(p)) = (crash_scene_file(), scene) {
            let _ = std::fs::write(f, p.display().to_string());
        }
        default(info);
    }));
}

/// Hängt eine Meldung mit Zeitstempel an den Bericht an (Datei bleibt klein).
pub fn note(msg: &str) {
    let Ok(root) = crate::project::root_dir() else { return };
    let _ = std::fs::create_dir_all(&root);
    let path = root.join("absturz.txt");
    if std::fs::metadata(&path).map(|m| m.len() > 512 * 1024).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(
            f,
            "=== {} · CineForge {} · {} ===\n{msg}\n",
            crate::project::now(),
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS
        );
    }
}
