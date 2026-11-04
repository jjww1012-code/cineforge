use std::path::PathBuf;

pub const USAGE: &str = "\
CineForge Studio

Aufruf:
  cineforge [SZENE.json] [Optionen]
  cineforge mcp [--port N]     Brücke für Claude Desktop (stdio)

Ohne SZENE wird die zuletzt bearbeitete Szene geöffnet, beim ersten Start
das Demo-Projekt (Dokumente/CineForge/Projekte/Demo).

Optionen:
  --screenshot DATEI.png   Szene rendern, Bild speichern und beenden
  --size BREITExHÖHE       Fenster-/Bildgröße (Standard 1600x900)
  --frames N               Frames warten vor dem Screenshot (Standard 60)
  --hud                    Einblendungen auch im Screenshot zeigen
  --time SEKUNDEN          Zeitpunkt für den Screenshot (Standard 0)
  --port N                 Port für die Claude-Verbindung (Standard 47800)
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
  Cmd/Strg + Z                     Rückgängig
  F2                               Mit Claude Desktop verbinden
  Leertaste                        Abspielen / Pause
  ← / →  (mit Shift feiner)        1 s (0,1 s) springen
  Pos1 / Ende                      Anfang / Ende
  L                                Schleife an/aus
  V                                Durch die Szenen-Kamera schauen (Kamerafahrt)

Claude verbinden:
  Claude Code:    claude mcp add --transport http cineforge http://127.0.0.1:47800/mcp
  Claude Desktop: siehe docs/CLAUDE-VERBINDEN.md
";

#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub scene: Option<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub size: (u32, u32),
    pub frames: u32,
    pub hud: bool,
    pub time: f32,
    pub port: u16,
    /// `cineforge mcp`: stdio-Brücke statt Fenster.
    pub mcp_bridge: bool,
    pub help: bool,
}

impl Args {
    pub fn parse(mut it: impl Iterator<Item = String>) -> Result<Args, String> {
        let mut a = Args {
            scene: None, screenshot: None, size: (1600, 900), frames: 60, hud: false, time: 0.0,
            port: crate::control::DEFAULT_PORT, mcp_bridge: false, help: false,
        };
        let mut first = true;
        while let Some(arg) = it.next() {
            let is_first = std::mem::replace(&mut first, false);
            match arg.as_str() {
                "mcp" if is_first => a.mcp_bridge = true,
                "--port" => a.port = it.next().and_then(|v| v.parse().ok()).ok_or("--port braucht eine Zahl")?,
                "-h" | "--help" => a.help = true,
                "--hud" => a.hud = true,
                "--time" => a.time = it.next().and_then(|v| v.parse().ok()).ok_or("--time braucht Sekunden")?,
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

    #[test]
    fn mcp_subcommand_only_in_first_position() {
        let a = p(&["mcp", "--port", "5000"]).unwrap();
        assert!(a.mcp_bridge);
        assert_eq!(a.port, 5000);
        let a = p(&["szene.json"]).unwrap();
        assert!(!a.mcp_bridge);
        // Eine Szene namens "mcp" an zweiter Stelle bleibt eine Szene.
        assert!(p(&["--hud", "mcp"]).unwrap().scene.is_some());
    }
}
