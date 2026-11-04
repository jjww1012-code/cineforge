use std::path::PathBuf;

pub const USAGE: &str = "\
CineForge Studio

Aufruf:
  cineforge [SZENE.json] [Optionen]

Ohne SZENE wird die Demo-Szene aus Dokumente/CineForge/Szenen geöffnet
(beim ersten Start wird sie dort angelegt).

Optionen:
  --screenshot DATEI.png   Szene rendern, Bild speichern und beenden
  --size BREITExHÖHE       Fenster-/Bildgröße (Standard 1600x900)
  --frames N               Frames warten vor dem Screenshot (Standard 60)
  --hud                    Einblendungen auch im Screenshot zeigen
  -h, --help               Diese Hilfe

Steuerung im Fenster:
  Rechte Maustaste halten + Maus   Umschauen
  W A S D                          Fliegen
  E / Q                            Hoch / Runter
  Shift                            Schneller
  Mausrad                          Fluggeschwindigkeit
  C                                Zurück zur Szenen-Kamera
  R                                Szene neu laden
  H                                Infos ein-/ausblenden
  F12                              Screenshot speichern
";

#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub scene: Option<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub size: (u32, u32),
    pub frames: u32,
    pub hud: bool,
    pub help: bool,
}

impl Args {
    pub fn parse(mut it: impl Iterator<Item = String>) -> Result<Args, String> {
        let mut a = Args { scene: None, screenshot: None, size: (1600, 900), frames: 60, hud: false, help: false };
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "-h" | "--help" => a.help = true,
                "--hud" => a.hud = true,
                "--screenshot" => {
                    a.screenshot = Some(it.next().ok_or("--screenshot braucht einen Dateinamen")?.into())
                }
                "--size" => {
                    let v = it.next().ok_or("--size braucht z. B. 1920x1080")?;
                    let (w, h) = v.split_once('x').ok_or("--size im Format BREITExHÖHE")?;
                    a.size = (
                        w.parse().map_err(|_| "ungültige Breite")?,
                        h.parse().map_err(|_| "ungültige Höhe")?,
                    );
                }
                "--frames" => {
                    a.frames = it.next().and_then(|v| v.parse().ok()).ok_or("--frames braucht eine Zahl")?
                }
                // macOS übergibt beim Start per Doppelklick manchmal "-psn_…"
                s if s.starts_with("-psn_") => {}
                s if s.starts_with('-') => return Err(format!("Unbekannte Option: {s}")),
                _ => {
                    if a.scene.is_some() {
                        return Err("Bitte nur eine Szene angeben".into());
                    }
                    a.scene = Some(arg.into());
                }
            }
        }
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(v: &[&str]) -> Result<Args, String> {
        Args::parse(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_all_options() {
        let a = p(&["x.json", "--screenshot", "o.png", "--size", "3840x2160", "--frames", "5"]).unwrap();
        assert_eq!(a.scene, Some("x.json".into()));
        assert_eq!(a.screenshot, Some("o.png".into()));
        assert_eq!(a.size, (3840, 2160));
        assert_eq!(a.frames, 5);
    }

    #[test]
    fn rejects_garbage() {
        assert!(p(&["--size", "groß"]).is_err());
        assert!(p(&["--wat"]).is_err());
        assert!(p(&["a.json", "b.json"]).is_err());
        assert!(p(&["-psn_0_12345"]).is_ok());
    }
}
