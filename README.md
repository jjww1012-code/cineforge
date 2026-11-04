# CineForge Studio

**KI-Video-Studio, in dem Claude mitarbeitet.** Szenen, 3D-Welten, Videos und Filme
entstehen als strukturierte Beschreibung und werden in Echtzeit auf der GPU gerendert,
bis 4K, für macOS und Windows.

> Status: **Etappe 1** – 3D-Welt aus einer Szenen-Datei, freie Flug-Kamera,
> Live-Reload, Screenshots. Siehe [Fahrplan](docs/FAHRPLAN.md).

![Demo-Szene](docs/bilder/wueste.png)

## Ausprobieren

1. Auf GitHub unter **Actions → Build → letzter Lauf** ganz unten unter *Artifacts*
   `CineForge-macOS` oder `CineForge-Windows` herunterladen und entpacken.
2. **Mac:** `CineForge.app` in den Programme-Ordner ziehen und doppelklicken.
   Solange die App noch nicht mit deinem Apple-Developer-Zertifikat signiert ist,
   meldet macOS beim ersten Start „kann nicht geöffnet werden“. Dann:
   *Systemeinstellungen → Datenschutz & Sicherheit → ganz unten „Trotzdem öffnen“*.
   Alternativ im Terminal: `xattr -dr com.apple.quarantine /Applications/CineForge.app`
3. **Windows:** `CineForge.exe` starten (bei der SmartScreen-Warnung „Weitere
   Informationen → Trotzdem ausführen“).

Beim ersten Start legt die App die Demo-Szene unter
`Dokumente/CineForge/Szenen/wueste.json` an und öffnet sie.
**Öffne diese Datei in einem Texteditor, ändere z. B. `"elevation": 7` auf `40`,
speichere – die Welt ändert sich sofort.**

### Steuerung

| Eingabe | Aktion |
|---|---|
| Rechte Maustaste halten + Maus | Umschauen |
| W A S D | Fliegen |
| E / Q | Hoch / Runter |
| Shift | Schneller |
| Mausrad | Fluggeschwindigkeit |
| C | Zurück zur Szenen-Kamera |
| R | Szene neu laden |
| H | Infos ein-/ausblenden |
| F12 | Screenshot (landet in `Dokumente/CineForge/Screenshots`) |
| JSON-Datei ins Fenster ziehen | Andere Szene öffnen |

## Für Entwickler

```bash
cargo run -p cineforge-studio -- scenes/wueste.json
cargo run -p cineforge-studio -- scenes/wueste.json --screenshot bild.png --size 1920x1080
cargo test --workspace
```

- [Architektur](docs/ARCHITEKTUR.md)
- [Szenen-Format](docs/SZENEN-FORMAT.md)
- [Fahrplan](docs/FAHRPLAN.md)
- [Mac-Signierung einrichten](docs/RELEASE.md)
