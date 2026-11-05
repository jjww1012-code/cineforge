# Aktueller Stand

> Diese Datei ist das Übergabe-Protokoll der Entwicklung. Nach jedem Teilschritt aktualisieren.

**Zuletzt aktualisiert:** 2026-11-05 · Demo-Film, Dialog-Kamera folgt der Blickrichtung

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
- **Natur (`cf_scene::nature`):** `type: "tree"` (oak/pine/palm/birch/lollipop, Wind als
  Keyframes, Wald über `scatter` mit Baum-Varianten), `type: "water"` (3 Wellenzüge, Mesh jedes
  Bild aus der Zeit berechnet: `spawn::animate_water`), `environment.stars` (Stern-Mesh mit
  Vertex-Farben, additiv; Achtung Wicklung: Vorderseite zur Kamera), `environment.moon`
  (Richtungslicht 2500 lx bläulich + Mondscheibe).
- **Einblendungs-Effekte:** `effect` pop/slide_up/typewriter (Präfix-Meshes, links an der fertigen
  Zeile ausgerichtet), `shadow`, `background` (+Deckkraft); Ebenen über z-Versatz sortiert.
- **Ton in der Vorschau (`preview_audio.rs`):** Shot-Ton, Soundtrack-Ausschnitt (Film-Zeit −
  Shot-Beginn) und talk-Stimmen; Start beim Abspielen, Neustart bei Sprung (> 0,2 s Abweichung);
  Bevy-Features `wav`, `mp3`.
- **Export aus der Oberfläche:** Knöpfe Szene/Film exportieren, Abbrechen, Export-Ordner öffnen;
  laufen über denselben Werkzeug-Kanal wie Claude (`control::UiCommands`, Logbuch-Akteur „mensch“).
- **Figuren in der Oberfläche:** Aktionen lesbar (rechts) und als Balken in der Zeitleiste.
- Hinweis Entwicklung: `target/debug/deps/cineforge-*` (je ~2 GB) sammeln sich an – alte löschen,
  wenn der Platz knapp wird (siehe CLAUDE.md).
- **Mitgelieferte Bibliothek:** 12 Cartoon-Objekte (`apps/studio/assets/bibliothek.json`, erzeugt
  mit `tools/make_starter_library.py`), beim Start in `Dokumente/CineForge/Bibliothek/`
  installiert (nur fehlende Dateien), Test prüft alle Objekte.
- **Szenen-Vorlagen (`cf_scene::templates`):** 12 Vorlagen (inkl. schloss, unterwasser, stadt; Bild `docs/bilder/vorlagen.png`) für `new_scene` (`template`), alle
  gerendert und geprüft; Test validiert sie.
- **Einstellungsgrößen (`camera.frame`):** wide/full/medium/close/extreme_close, Winkel relativ zur
  Blickrichtung der Figur, Neigung; Abstand aus Bildwinkel und Figurenhöhe.
- **Kurzfilm Ende-zu-Ende über MCP** (new_project → generate_voice → create_music → new_scene mit
  Vorlagen → batch mit character/path/follow/frame/dof → update_plan mit Ton → check_film →
  export_film): 3 Shots, 9 s, Ton ok, 2:42 auf Software-Rendering. Ergebnis `docs/bilder/kurzfilm.mp4`.
- **Dialog-Werkzeug (`dialogue`):** Zeilen → Stimmen (synthesize, auf 48 kHz umgerechnet), Timing
  (start, gap, pause), je Sprecher `dlg_<szene>_<id>.wav`, Figuren: talk + talk-Aktionen (+ action),
  andere Objekte: animate.talk; Untertitel (y −0.82 + Box, ersetzt frühere), Szenenlänge, Shot-Ton.
- **Kamera-Schnitte (`camera.cuts`)** in einer Szene; `dialogue` mit `camera_cuts: true` setzt
  Schuss/Gegenschuss automatisch; Figuren-`look_at` (Kopf/Oberkörper zum Ziel, Startpositionen).
- **Mimik über die Zeit (`character.expressions`):** Brauen/Lippen/O-Mund je Ausdruck als Varianten
  mit `visible`-Sprüngen; Mundöffnung nur bei `talk`. Einstellungsgrößen an Figurenproportionen angepasst.
- **Tiere (`cf_scene::animal`, `type: "animal"`):** Hund, Katze, Hase, Vogel, Fisch mit Gelenken,
  Blinzeln, 8 Aktionen, automatischem Bewegen auf Pfaden.
- **Figuren-Accessoires:** Hüte (cap, beanie, top_hat, crown, party), Brille, Kleid, Schal, Rucksack.
- **Regie-Fahrplan:** MCP-Vorlage „Neuer Film“ führt in 13 Schritten durch Geschichte, Storyboard,
  Vorlagen, Figuren/Tiere, Dialog, Kamera, Licht, Prüfen, Ton, Titel, Export.
- **Shot-Vorschaubilder (`thumbs.rs`):** wenn eine Storyboard-Szene offen ist und Ruhe herrscht
  (4 s nach dem Laden, keine Aufträge), Still-Auftrag 192×108 aus der Shot-Mitte →
  `Export/.vorschau/<szene>.png`, Film-Leiste zeigt sie; schwarze Bilder werden verworfen (max. 3 Versuche).
- **Foley-Werkzeug (`foley`):** analysiert die geöffnete Szene (Figuren/Tiere: walk/run/jump/fly,
  Pfade mit Schrittfrequenz aus Tempo; Partikel fire/rain/snow/explosion; Titel pop/typewriter/
  slide_up) → `sfx_<szene>.wav` (normalisiert), im Shot eingetragen.
- **Export-Qualität `best`:** Supersampling ×2 (bis 4096 px Renderbreite), ffmpeg verkleinert mit
  Lanczos; auch für `export_film` (eigener Cache-Ordner je Qualität).
- **Menschenmengen/Herden:** `scatter` mit character/animal-Shape; `character::vary` (Farben, Frisur,
  Größe, Statur, Ausdruck, Hut/Brille, versetzte Aktionen).
- **Gegenstände halten:** `character.hold`/`hold_left` (beliebiges Objekt an der Hand, wird mitgeprüft).
- **Übergänge im Film:** `dissolve`, `wipe`, `slide`, `circle` über ffmpeg-xfade (0,6 s Überlappung,
  Shot-Startzeiten und Ton entsprechend verschoben, Video wird beim Zusammenfügen neu kodiert);
  `cut`/`fade` wie bisher verlustfrei. Test prüft Filterkette, Export geprüft (12,75 s).
- **Ton automatisch im Film:** `build_plan` nimmt Stimmen sprechender Figuren (talk) und
  `sfx_<szene>.wav` (foley) automatisch mit, wenn sie nicht im Shot stehen; Vorschau-Ton ebenso.
- **Vorzeige-Szene** `scenes/vorzeige.json` (Bild `docs/bilder/vorzeige.png`).
- **Start nie mehr stumm abbrechen:** Ist die zuletzt geöffnete Szene nicht ladbar, öffnet die App
  die Demo und zeigt den Grund (Banner, `resume`). Abstürze/Startprobleme landen in
  `Dokumente/CineForge/absturz.txt` (`crash.rs`, Panic-Hook mit Aufrufliste). Anlass: Nutzer
  konnte die neue Version auf dem Mac nicht öffnen (Ursache dort noch nicht bestätigt).
- **Demo-Film „Die Schatzsuche“** (4 Shots: Dorf-Dialog mit Schnitten + Hund, Wald mit
  Verfolgerkamera + Vogel, Schloss mit Titel, Lagerfeuer-Nacht) per MCP komplett erzeugt und
  exportiert (20 s). Dabei gefunden und behoben: Nahaufnahmen beachteten die Blickrichtung
  (`look_at`) nicht – Kamera stand hinter dem Kopf. Jetzt `motion::facing_yaw`/`look_yaw`
  (gemeinsam für Vorschau und Framing); Dialog-Schnitte: Seite je Sprecher (180°-Regel),
  kein Schnitt, wenn dieselbe Person weiterspricht. Wald: 8 Baum-Varianten im Wechsel (schneller).
- **Film-Übersicht:** `contact_sheet` mit `film: true` öffnet nacheinander alle Storyboard-Szenen und
  legt je Shot ein Bild (oder mehrere, `at: [0.1, 0.9]`) in einen Kontaktabzug; Wartezeit dafür 10 min.
- **Oberfläche aufgeräumt:** Tonspuren mit Wellenform (WAV, zwischengespeichert), überlappende
  Balken in Unter-Zeilen (Figur: Stimme / Aktionen), Ton-Spur zeigt auch die foley-Geräusche,
  Eigenschaften mit deutschen Namen, Farbfeldern, „ja/nein“ und übersetzten Werten, „Was passiert“
  mit kurzen Pfaden und zusammengefassten Wiederholungen, „Abbrechen“ nur während eines Exports,
  Film-Leiste nutzt die Szenenlänge bei Shots ohne Dauer. Layout-Fehler behoben: die Objektliste
  bzw. die Vorschau bestimmten die Mindesthöhe der Mitte → Zeitleiste rutschte aus dem Fenster
  (jetzt `flex_basis: 0`/`min_height: 0`, Vorschau absolut positioniert, Spurenbereich scrollbar).
- **Kamerafahrten mit `frame.to`:** Ziel-Einstellung (shot, angle, tilt, zoom, ease, start/end) –
  Heranfahren, Wegfahren, Bogen, Kran, Push-in; läuft bis zum nächsten Schnitt. Dialog-Nahaufnahmen
  mit sanftem Push-in (zoom 1.1). Nahaufnahme mit mehr Kopffreiheit.
- **Disney-Prinzipien bei Figuren:** Squash & Stretch (`Pose.squash`, Skalierung der ganzen Figur
  vom Boden aus, volumenerhaltend), Sprung mit Ausholen (Knie, Arme nach hinten) und gestauchter
  Landung; neue Aktionen `clap`, `bow`, `laugh`, `scared`, `look_around`, `shrug`, `sad`, `excited`.
- **Dialog mit Mimik:** `dialogue` nimmt je Zeile `expression` (neutral/happy/sad/angry/surprised) →
  `character.expressions`; ist die Szenen-Kamera noch Standard, beginnt der Dialog mit einer
  automatisch berechneten Zweier-Einstellung (alle Sprecher im Bild). Posen sad/scared so, dass das
  Gesicht in Nahaufnahmen sichtbar bleibt.
- **Umgebungsgeräusche:** neue Synthese-Vorlagen `birds`, `crickets`, `waves`, `stream`, `city`,
  `underwater` (deterministischer Zufall je Variante); `foley` erkennt die Umgebung (Bäume + Tag,
  Nacht, Wasser, Brunnen/Fontäne, Autos/Häuser, Fische + Nebel ohne Wasser) und mischt sie
  (`ambience`, `ambience_volume`). Getestet an allen Vorlagen (Pegel −20…−35 dB RMS).
- **Bearbeiten per Klick:** Eigenschaften-Bereich hat für das ausgewählte Objekt Knöpfe ← → nach
  hinten/vorn ↑ ↓ (0,25 m), ↺/↻ 15°, größer/kleiner (×1,1), Duplizieren, Verstecken/Zeigen, Löschen,
  Rückgängig – laufen als Werkzeug-Aufrufe „mensch“ (Logbuch, rückgängig machbar). Mit echten Klicks
  getestet (Xvfb + xdotool).
- **Musik nach Stimmung und Filmlänge:** `create_music` mit `mood` (happy, sad, adventure, tense,
  magical, calm, epic, funny, night) → Intro/Hauptteil/Ausklang; `duration` oder `film: true`
  (Länge = Film, gleich als Soundtrack mit Ein-/Ausblenden eingetragen). `music::fit_sections`, Test.
- **Demo-Film neu:** `docs/bilder/schatzsuche.mp4` + `.png` (Dialog mit Mimik, Zweier-Einstellung, dann
  Schuss/Gegenschuss; Umgebung; Abenteuer-Musik). Erster Dialogsatz läuft in der Eröffnungs-Einstellung.
- **Sicherer Modus nach Absturz (`crash.rs`):** Start-Markierung `.start_laeuft` (wird nach 8 s bzw.
  bei normalem Beenden gelöscht) und `.absturz_szene` (vom Panic-Hook geschrieben). Ist der letzte
  Start abgestürzt oder stürzte die Startszene ab → Demo-Szene, ohne SSAO/Tiefenschärfe/
  Bewegungsunschärfe/Lichtstrahlen, roter Hinweis mit Grund. Mit Xvfb getestet. `--select` wirkt
  jetzt auch im normalen Start.
- **Abspann:** Einblendungs-Effekt `scroll` (mehrzeiliger Text läuft von `start` bis `end` von unten
  nach oben durchs Bild). Getestet.
- **Ganzen Film in einem Schritt anlegen:** Storyboard-Einträge in `update_plan` dürfen `template`
  oder `copy_scene` enthalten → Szenen-Dateien werden gleich mit angelegt (Länge = duration).

## Nächste Schritte (in dieser Reihenfolge)

1. **Auf echter GPU prüfen** (Mac): SSAO, Tiefenschärfe, Lichtstrahlen, Bewegungsunschärfe im Export.
2. Oberfläche ausbauen: Farben per Palette ändern, Shots per Ziehen umsortieren, Objekte in der
   Vorschau ziehen.
3. Weitere Regie-Hilfen (z. B. Musik passend zur Stimmung je Shot, Kamerafahrten-Vorlagen).

## Offene Punkte / Wissenswertes

- **GitHub Actions startet keine Builds mehr (seit 2026-11-05 ~01:20 UTC):** „recent account payments have
  failed or your spending limit needs to be increased“. Letzter erfolgreicher Build: 00:21 UTC
  („App startet immer …“). Der Kontoinhaber muss unter GitHub → Settings → Billing and plans die
  Zahlung/das Ausgabenlimit für Actions prüfen, danach den letzten Build mit „Re-run“ starten.
  Workflow spart jetzt Minuten (concurrency mit Abbruch älterer Läufe, keine Builds für Doku-Änderungen).

- **Gegenlicht/Rim-Light:** mit einem zusätzlichen Richtungslicht getestet – wirkt in Bevy nicht
  (hellt nur den Boden auf, keine Kanten; Lichter lassen sich nicht auf Objekte beschränken).
  Echter Rim braucht einen eigenen Shader (`ExtendedMaterial` mit Fresnel-Term) – noch offen.

- GitHub: `jjww1012-code/cineforge` (privat). CI-Status noch nicht von hier aus prüfbar
  (gh-Token ungültig) – der Nutzer sieht ihn unter „Actions“.
- Export braucht ffmpeg (in CI mitgeliefert) – ohne ffmpeg PNG-Folge.
- Klänge kann Claude nicht anhören: Qualität der Sound-Vorlagen vom Nutzer beurteilen lassen.
- Prüfen von Videos bildgenau: `ffmpeg -i x.mp4 -vf "select=eq(n\,19)" -vsync 0 -frames:v 1 out.png`
  (`-ss` springt auf das nächste Bild *ab* dem Zeitpunkt).
