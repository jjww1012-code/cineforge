# Aktueller Stand

> Diese Datei ist das Übergabe-Protokoll der Entwicklung. Nach jedem Teilschritt aktualisieren.

**Zuletzt aktualisiert:** 2026-11-04 · Effekte + Film-Export fertig

## Fertig

- **Etappe 1:** Szenen-Format, 3D-Studio (Bevy 0.20), Flug-Kamera, Live-Reload, Screenshots,
  CI für macOS/Windows.
- **Etappe 2:** MCP-Verbindung (lokaler HTTP-Server + stdio-Brücke `cineforge mcp`),
  Werkzeuge zum Bauen/Ändern/Prüfen, Kontrollbilder über unsichtbare Kamera, Rückgängig,
  F2 = Claude Desktop einrichten.
- **Fortsetzungs-System:** Projekte (`Dokumente/CineForge/Projekte/<Name>/`) mit Logbuch
  (`projekt.json`: Ziel, Stil, Storyboard, Aufgaben, Notizen, Protokoll, nächster Schritt),
  Werkzeuge `resume`, `update_plan`, `update_tasks`, `add_note`, `list_projects`,
  `open_project`, `new_project`; MCP-Vorlagen „Weitermachen“ und „Neuer Film“;
  App öffnet beim Start die zuletzt geöffnete Szene. Ende-zu-Ende getestet
  (Sitzung → harter Abbruch → neue Sitzung → `resume`).

- **Detaillierte Objekte:** `rounded_box`, `extrude` (Löcher, Verjüngung), `lathe`, `tube`,
  `text` (3D-Schrift, Inter), `repeat` mit Drehpunkt, Material: Glas, Klarlack, unbeleuchtet,
  beidseitig, 8 kachelbare Muster mit Struktur (Normal-Map) und Mipmaps, UVs in Metern.
  Objekt-Bibliothek (Projekt `Objekte/`, global `Bibliothek/`, Typ `instance`,
  `material_override`), MCP `save_asset`/`list_assets`/`get_asset`. Vorführ-Szene
  `scenes/werkstatt.json`. Ende-zu-Ende getestet.

- **Zeit & Export:** `duration`/`fps`, Keyframes mit 7 Bewegungskurven für Objekte
  (Position, Drehung, Größe, Farbe, Leuchten, Deckkraft, Sichtbarkeit, `spin`, `smooth_path`),
  Kamerafahrten (weiche Kurven, Zoom, Handkamera-Wackeln), Sonnenstand/Belichtung über die Zeit.
  Wiedergabe in der App (Leertaste, Pfeiltasten, V = Szenen-Kamera, Zeitleiste).
  Render-Aufträge (`render.rs`): `screenshot` mit `time`, `contact_sheet`, `export_video`
  (Fließband, ffmpeg mit getestetem Hardware-Encoder, PNG-Fallback), `export_status`,
  `cancel_export`; Szene während Export gesperrt. Kamera-Rücksprung-Fehler behoben.
  Ende-zu-Ende getestet (MP4 geprüft mit ffprobe).

- **Effekte:** deterministische Partikel (`cf_scene::particles`, App `fx.rs`) mit 9
  Voreinstellungen, Vorlaufen, Billboards zur rendernden Kamera; Licht-Animation
  (`intensity`, `flicker`). Testszene `scenes/effekte.json`.
- **Film-Export:** `export_film` setzt alle Storyboard-Shots (Szene + Dauer) zu einem MP4
  zusammen, harte Schnitte oder Abblenden; Export intern als Abschnitte (`render::Segment`).
  Ende-zu-Ende geprüft (Bildgenaue Übergänge).

- **Erster Test auf echtem Mac (M2):** läuft mit 76 FPS, Claude-Desktop-Verbindung funktioniert.
  Danach behoben: fehlende Schriftzeichen (❚ ⟲ → Text, Test prüft Zeichen), „1 Schritt“,
  Horizont: Atmosphären-Bodenfarbe = Geländefarbe + automatischer 40-km-Horizont-Boden.
  Hinweis: Nutzer müssen die App aus dem Programme-Ordner starten (sonst App Translocation).

- **Lange Filme / Medien:** Film-Pipeline (`film.rs`) mit Shot-Cache (Fingerabdruck aus
  Szene, Bibliothek, Bildern, Format, Blenden; `Export/.cache/`), Fortsetzen nach Abbruch,
  verlustfreies Zusammenfügen (concat), Ton-Mischung (soundtrack, audio je Shot, Clip-Originalton),
  Video-Clips als Shots (skaliert, Bildrate angepasst, Blenden), Bilder (`image`, `texture`),
  Einblendungen (`overlays`, Größe relativ zur kürzeren Bildseite), Systemstimmen
  (`generate_voice`), `check_film`, `list_shots`, `list_media`, kompakte Logbuch-Übersicht
  für Hunderte Shots. ffmpeg wird in CI mitgeliefert (Mac: martin-riedl.de-Build,
  Windows: BtbN LGPL; Schritt darf fehlschlagen). Ende-zu-Ende getestet inkl. Ton-Positionen
  und Cache-Wiederverwendung. Fehler behoben: Screenshot-Modus überschrieb „zuletzt“.
  Lizenz-Hinweis: Mac-ffmpeg-Build vor dem Verkauf auf LGPL prüfen.

## Nächste Schritte (in dieser Reihenfolge)

1. **Ton-Vorschau in der App** (Wiedergabe synchron zur Zeitleiste) und Lippen-/Beat-Sync.
2. **Klicken & sagen:** Objekt im Fenster anklicken → hervorheben → Claude bekommt die id.
3. **Ton:** lokale KI-Stimmen (Piper), Musik, Untertitel, Ton im Export.
4. Danach laut Fahrplan: Klicken & sagen, Ton, Figuren-Animation, Produkt, Teams.

## Offene Punkte / Wissenswertes

- GitHub: `jjww1012-code/cineforge` (privat). CI-Status noch nicht von hier aus prüfbar
  (gh-Token ungültig) – der Nutzer sieht ihn unter „Actions“.
- Noch nie auf echter Mac-Hardware getestet.
- Export braucht ffmpeg (noch nicht gebündelt) – ohne ffmpeg PNG-Folge.
- Prüfen von Videos bildgenau: `ffmpeg -i x.mp4 -vf "select=eq(n\,19)" -vsync 0 -frames:v 1 out.png`
  (`-ss` springt auf das nächste Bild *ab* dem Zeitpunkt).
