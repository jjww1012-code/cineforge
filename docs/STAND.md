# Aktueller Stand

> Diese Datei ist das Übergabe-Protokoll der Entwicklung. Nach jedem Teilschritt aktualisieren.

**Zuletzt aktualisiert:** 2026-11-04 · Zeit & Export fertig, als Nächstes Effekte

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

## Nächste Schritte (in dieser Reihenfolge)

1. **Effekte:** deterministische Partikel (Feuer, Rauch, Funken, Schnee, Regen).
2. **Film-Export:** Storyboard-Shots → zusammengesetztes Video.
3. ffmpeg mitliefern (Mac/Windows-Build), damit Nutzer nichts installieren müssen.
4. Danach laut Fahrplan: Klicken & sagen, Ton, Figuren-Animation, Produkt, Teams.

## Offene Punkte / Wissenswertes

- GitHub: `jjww1012-code/cineforge` (privat). CI-Status noch nicht von hier aus prüfbar
  (gh-Token ungültig) – der Nutzer sieht ihn unter „Actions“.
- Noch nie auf echter Mac-Hardware getestet.
- Export braucht ffmpeg (noch nicht gebündelt).
