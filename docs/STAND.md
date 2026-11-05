# Aktueller Stand

> Diese Datei ist das Übergabe-Protokoll der Entwicklung. Nach jedem Teilschritt aktualisieren.

**Zuletzt aktualisiert:** 2026-11-05 · 3D-Verformen, 2D in voller Qualität, 2D/3D mischen, Kino-Look, Lippen-Sync, Vorzeige-Szene

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
  Cache je Format/Qualität in eigenem Unterordner, veraltete Shots werden nach erfolgreichem
  Export entfernt. Belastungstest: 30 Shots, Abbruch bei Shot 13, Fortsetzen → 12 aus Cache,
  18 neu, Ergebnis bildgenau (180 Bilder, 18,0 s, mit Ton).
  Speicherbedarf grob: 1 h in 1080p ≈ 3–7 GB Cache + gleich viel für das fertige Video.

- **Schneller arbeiten & Fehler sofort beheben:** Fehlermeldungen mit genauer Stelle
  (`objects[3].material.color`) und „Meintest du …?“ (`cf_scene::diagnose`), Erkennung von
  Tippfehlern in Feldnamen, `batch` (viele Änderungen, alles oder nichts), `duplicate_object`.
- **Mehrere Agenten:** `claim_task` (Aufgabe mit Besitzer), Parameter `scene` bearbeitet andere
  Szenen ohne sie zu öffnen, `agent` im Logbuch; MCP-Vorlage „Als weiterer Agent mitarbeiten“.
- **Klangsynthese (`crates/cf_audio`):** eigene Geräusche ohne KI (Oszillatoren, Rauschen,
  Filter, Hüllkurven, FM, Hall, Echo), 26 Vorlagen + Rezepte aus Schichten (`create_sound`),
  Musik-Generator mit Stilen, Akkorden, Abschnitten (`create_music`), WAV in `Audio/`.
- **2D & Import:** `type: "svg"` (SVG-Text inline oder Datei aus `Bilder/`, jede Fläche/Kontur
  mit eigener Farbe, `depth` = 3D-Relief; `cf_scene::svg`), 2D-Kamera `camera.orthographic`,
  Bildstil `environment.look` (`cartoon` = exakte Farben ohne Tonwertkurve, automatisch bei
  2D-Kamera), `type: "model"` (glTF/GLB aus `Modelle/`, Asset-Quelle `cineforge://`; Render-
  Aufträge warten, bis Modelle geladen sind). Fehlende/kaputte Dateien werden in der Antwort
  an Claude sofort gemeldet (`media::check_files`). Neue Projekte bekommen die Ordner
  `Audio/ Clips/ Bilder/ Modelle/`. Beispiel `scenes/cartoon.json`. Behoben: Ebenen-Flimmern
  bei 2D-Kamera (Tiefenbereich -50…2000 m, SVG-Ebenenabstand ≥ 1 mm).

- **Inkrementelle Updates:** Beim Laden werden nur geänderte oberste Objekte neu gebaut
  (`spawn::BuiltScene`, Fingerabdruck aus Objekt-JSON + Abhängigkeiten: Gelände unter Scatter,
  Bibliotheks-Dateien, Bilder/SVG/Modelle mit Änderungszeit). Umgebung (Himmel, Sonne) wird nur
  bei Änderung neu aufgebaut. Wüste: voller Aufbau 878 ms, Einzeländerung 0 ms. HUD zeigt „(N neu)“.

- **Oberfläche wie ein Schnittprogramm (`studio_ui.rs`):** Kopfzeile (Projekt › Szene,
  Claude-Status, Export-Fortschritt), links Szenen/Storyboard, Objekte (Baum), Medien;
  Mitte Vorschau als `ViewportNode` (Hauptkamera rendert in ein Bild, 16:9 oder 9:16) mit
  Wiedergabe-Leiste; rechts Eigenschaften (Objekt oder Szene) und „Was passiert“ (Logbuch,
  nächster Schritt); unten Film-Leiste (Shots, Klick öffnet Szene) und Spuren der Szene
  (Kamera, Text, Ton, animierte Objekte mit Keyframes), Klicken/Ziehen springt.
  **Klicken & sagen:** Klick in Liste oder Vorschau wählt aus (Mesh-Picking, Rahmen per Gizmo
  auf eigener Render-Ebene – nicht in Exporten); MCP `get_selection` / `select_object`,
  Auswahl steht auch in `list_objects` und `resume`. F12 speichert nur die Vorschau.
  CLI `--hud` zeigt die Oberfläche im Screenshot, `--select ID`. Getestet mit xdotool auf Xvfb.

- **Detaillierte 3D-Formen (`cf_scene::shaping`):** `subdivide` (Loop, bitgenau
  deterministisch), `rounded` (Glättung), `deform` (twist, bend, taper, wave, inflate, noise mit
  3D-Perlin `noise::Noise3`); verschweißte Punkte ⇒ keine Risse, Normalen mit Knickwinkel
  (Würfelkanten bleiben scharf). Dreiecks-Obergrenze 1,5 Mio. App: Mesh-Cache für gleiche Formen,
  CPU-Mesh-Daten bleiben für Picking erhalten.

- **2D in voller Qualität:** SVG `mode` auto/vector/image; Bild-Modus rastert mit resvg
  (Verläufe, Filter, Masken, Text mit eingebauter Inter), vormultipliziertes Alpha + Mipmaps,
  `resolution`. Text funktioniert auch im Vektor-Modus.
- **2D/3D mischen:** `billboard` (dreht sich parallel zur Blickrichtung der rendernden Kamera),
  `material.outline` (Cartoon-Umriss über vergrößerte Hülle mit Vorderseiten-Culling),
  Cartoon-Stil belichtet beleuchtete Flächen +1 EV dunkler (kein Ausbrennen ohne Tonwertkurve).
- **Kino-Look:** `camera.dof` (Gauß; Bokeh lieferte auf lavapipe Bildmüll), Fokus folgt Objekt;
  `camera.motion_blur` (Vorschau nur beim Abspielen); `environment.ao` (SSAO + SMAA, MSAA aus;
  Standard an im Film-Stil); `light_rays` (VolumetricLight + dünnes FogVolume 400×120×400 m);
  `grade` (Sättigung, Vignette). Bevys Kontrast/Weißabgleich verfälschten im Test die Farben
  (rot/schwarz) – bewusst weggelassen, ggf. später eigener Filter. Doku „Kino-Qualität“.

- **Lebendige Figuren:** `animate.talk` (Lippen-Sync: WAV → Lautstärke-Hüllkurve 100/s,
  `cf_audio::read_wav`/`envelope`, Mund-Höhe skaliert), `animate.blink` (deterministisch je
  `seed`), `animate.breathe`. Fehlende Stimme wird Claude sofort gemeldet.
  Hinweis Tests: mehrere Software-Renderings parallel ⇒ Shader evtl. beim Foto noch nicht fertig
  (leeres Bild) – Kontrollbilder einzeln oder mit `--frames 60` rendern.

- **Bewegung & Kamera-Automatik (`cf_scene::motion`):** `animate.path` (Catmull-Rom, gleichmäßiges
  Tempo über Bogenlänge, `face` dreht in Laufrichtung), `world_position` (auch in animierten
  Gruppen), `camera.target`/`target_offset`/`follow` (deterministisches Nachziehen über
  Zeitfenster-Mittel, Blick bleibt auf aktueller Position)/`orbit`; App nutzt `camera_shot`.
- **Figuren-Generator (`cf_scene::character`, `type: "character"`):** Gruppe aus Grundformen mit
  Hüfte/Beinen/Armen/Kopf-Gelenken, Augen (Iris, Pupille, Glanz, Blinzeln je id), Brauen und
  Lippenlinie je Ausdruck, Mundöffnung mit Lippen-Sync, 6 Frisuren, 3 Statur-Typen, Umriss;
  13 Aktionen als prozedurale Posen → Keyframes (15/s) mit weichen Übergängen; `auto_walk`
  passt Schrittfrequenz an die Pfad-Geschwindigkeit an. Teile heißen `<id>/…` (Klick wählt Figur).
- **Vorzeige-Szene** `scenes/vorzeige.json` (Bild `docs/bilder/vorzeige.png`).
- **Start nie mehr stumm abbrechen:** Ist die zuletzt geöffnete Szene nicht ladbar, öffnet die App
  die Demo und zeigt den Grund (Banner, `resume`). Abstürze/Startprobleme landen in
  `Dokumente/CineForge/absturz.txt` (`crash.rs`, Panic-Hook mit Aufrufliste). Anlass: Nutzer
  konnte die neue Version auf dem Mac nicht öffnen (Ursache dort noch nicht bestätigt).

## Nächste Schritte (in dieser Reihenfolge)

1. **Auf echter GPU prüfen** (Mac): SSAO, Tiefenschärfe, Lichtstrahlen, Bewegungsunschärfe im Export.
2. **Cartoon-Werkzeuge:** Figuren aus SVG-Teilen mit Gelenken (Vorlagen), Lippen-Sync zur Stimme.
3. **Ton-Vorschau in der App** (synchron zur Zeitleiste, Wellenform in der Ton-Spur).
4. Oberfläche ausbauen: Werte direkt im Eigenschaften-Bereich ändern, Shots per Ziehen
   umsortieren, Vorschaubilder der Shots in der Film-Leiste.

## Offene Punkte / Wissenswertes

- GitHub: `jjww1012-code/cineforge` (privat). CI-Status noch nicht von hier aus prüfbar
  (gh-Token ungültig) – der Nutzer sieht ihn unter „Actions“.
- Export braucht ffmpeg (in CI mitgeliefert) – ohne ffmpeg PNG-Folge.
- Klänge kann Claude nicht anhören: Qualität der Sound-Vorlagen vom Nutzer beurteilen lassen.
- Prüfen von Videos bildgenau: `ffmpeg -i x.mp4 -vf "select=eq(n\,19)" -vsync 0 -frames:v 1 out.png`
  (`-ss` springt auf das nächste Bild *ab* dem Zeitpunkt).
