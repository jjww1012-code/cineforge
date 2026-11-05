//! Absturzbericht: Beim Start per Doppelklick sieht man keine Fehlermeldungen.
//! Deshalb landen Abstürze und Startprobleme in `Dokumente/CineForge/absturz.txt`.

use std::io::Write;

/// Fängt Abstürze ab und schreibt sie (mit Aufrufliste) in den Bericht.
pub fn install() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        note(&format!("ABSTURZ: {info}\n{bt}"));
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
