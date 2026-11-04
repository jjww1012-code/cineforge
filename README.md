# CineForge Studio

**KI-Video-Studio, in dem Claude mitarbeitet.** Szenen, 3D-Welten, Videos und Filme
entstehen als strukturierte Beschreibung und werden in Echtzeit auf der GPU gerendert,
bis 4K, für macOS und Windows.

> Status: Claude baut und ändert Szenen live in der App, prüft das Ergebnis per Kontrollbild,
> erzeugt eigene Sounds, Musik und 2D-Grafiken und rendert ganze Filme. Siehe [Fahrplan](docs/FAHRPLAN.md).

## Mit Claude arbeiten

App starten, **F2** drücken, Claude Desktop neu starten. Danach zum Beispiel schreiben:
„Bau mir in CineForge eine Unterwasserstadt mit leuchtenden Kuppeln.“
Details: [Claude verbinden](docs/CLAUDE-VERBINDEN.md)

![Die Oberfläche](docs/bilder/oberflaeche.png)

## Die Oberfläche

Aufgebaut wie ein Schnittprogramm (ähnlich CapCut):

- **Oben:** Projekt › Szene und ob Claude verbunden ist (mit der letzten Aktion).
- **Links:** *Szenen* (Storyboard, Klick öffnet die Szene), *Objekte* (alle Teile der Szene,
  Klick wählt aus), *Medien* (Audio, Clips, Bilder, 3D-Modelle des Projekts).
- **Mitte:** die Vorschau. Ein Klick auf ein Objekt wählt es aus und rahmt es ein –
  Claude weiß dann, was du mit „das hier“ meinst. Darunter Abspielen, Zeit, Schleife,
  Szenen-/freie Kamera und 16:9 / 9:16 (Hochformat für TikTok/Reels).
- **Rechts:** *Eigenschaften* des ausgewählten Objekts (oder der Szene) und *Was passiert* –
  was Claude gerade gemacht hat und der nächste Schritt.
- **Unten:** die Zeitleiste. Oben alle Shots des Films, darunter die Spuren der geöffneten
  Szene: Kamera, Texte, Ton und jedes animierte Objekt mit seinen Keyframes (◆).
  Klicken oder Ziehen springt an die Stelle.

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

Beim ersten Start legt die App das Demo-Projekt unter
`Dokumente/CineForge/Projekte/Demo/` an und öffnet die Szene `Szenen/wueste.json`.
Danach öffnet sie immer die zuletzt bearbeitete Szene.
**Öffne diese Datei in einem Texteditor, ändere z. B. `"elevation": 7` auf `40`,
speichere – die Welt ändert sich sofort.**

### Steuerung

| Eingabe | Aktion |
|---|---|
| Linke Maustaste in der Vorschau | Objekt auswählen |
| Rechte Maustaste halten + Maus | Umschauen (Maus über der Vorschau) |
| W A S D | Fliegen |
| E / Q | Hoch / Runter |
| Shift | Schneller |
| Mausrad | Fluggeschwindigkeit |
| C | Zurück zur Szenen-Kamera |
| R | Szene neu laden |
| Esc | Auswahl aufheben |
| F12 | Bild der Vorschau speichern (landet in `Dokumente/CineForge/Screenshots`) |
| Cmd/Strg + Z | Rückgängig (auch Änderungen von Claude) |
| F2 | Mit Claude Desktop verbinden |
| Leertaste | Animation abspielen / pausieren |
| ← / → (mit Shift feiner) | 1 s (0,1 s) springen |
| Pos1 / Ende | Anfang / Ende |
| L | Schleife an/aus |
| V | Durch die Szenen-Kamera schauen (Kamerafahrt) |
| JSON-Datei ins Fenster ziehen | Andere Szene öffnen |

## Video-Export

Claude exportiert mit `export_video` (eine Szene) oder `export_film` (ganzer Film, auch
1 Stunde+, mit Ton und Video-Clips) in `Dokumente/CineForge/Projekte/<Projekt>/Export/`.

**ffmpeg** (für Video und Ton) ist ab der aktuellen Version in der App enthalten. Falls nicht:
Mac `brew install ffmpeg`, Windows `winget install ffmpeg`. Die App nutzt automatisch den
Hardware-Encoder (Mac: VideoToolbox, Windows: NVIDIA/Intel/AMD).

**Eigene Medien** legst du in den Projektordner: `Audio/` (Musik, Geräusche, Stimmen),
`Clips/` (Videos, z. B. KI-Videos oder Handy-Aufnahmen), `Bilder/` (Logos, Fotos).

## Für Entwickler

```bash
cargo run -p cineforge-studio -- scenes/wueste.json
cargo run -p cineforge-studio -- scenes/wueste.json --screenshot bild.png --size 1920x1080
cargo test --workspace
```

- [Claude verbinden](docs/CLAUDE-VERBINDEN.md)
- [Architektur](docs/ARCHITEKTUR.md)
- [Szenen-Format](docs/SZENEN-FORMAT.md)
- [Fahrplan](docs/FAHRPLAN.md)
- [Mac-Signierung einrichten](docs/RELEASE.md)
