# CineForge – Hinweise für Claude (Entwicklung)

Natives KI-Film-Studio (macOS + Windows) in Rust/Bevy. Claude steuert die App über MCP.
Der Auftraggeber programmiert nicht: **alles auf Deutsch erklären**, UI-Texte und Doku auf Deutsch.

## Zuerst lesen

1. `docs/STAND.md` – aktueller Stand, was zuletzt passiert ist, **nächste Schritte**
2. `docs/FAHRPLAN.md` – Etappen
3. `docs/ARCHITEKTUR.md` – Aufbau

**Nach jedem abgeschlossenen Teilschritt `docs/STAND.md` aktualisieren, committen und pushen**,
damit eine neue Sitzung (z. B. nach Nutzungslimit) nahtlos weitermachen kann.

## Aufbau

- `crates/cf_scene` – Szenen-Dokument (JSON-Modell, Validierung), `ops` (Bearbeitung),
  `procedural` (Geometrie), `project` (Logbuch). Engine-unabhängig, gut testbar.
- `apps/studio` – Bevy-App: `source` (Datei, Live-Reload, Verlauf), `spawn` (Szene → Entities),
  `environment` (Himmel/Sonne/Kamera), `control` (MCP-HTTP-Server), `tools` (MCP-Werkzeuge),
  `bridge` (`cineforge mcp` stdio), `project` (Projektordner, zuletzt.json), `hud` (Schrift, Titel),
  `studio_ui` (Oberfläche wie ein Schnittprogramm: Listen, Vorschau als `ViewportNode`,
  Eigenschaften, Zeitleiste, Auswahl per Klick), `capture`.

## Befehle

```bash
cargo test --workspace
cargo build -p cineforge-studio
```

### Ohne Grafikkarte testen (Cloud-Container)

```bash
apt-get install -y mesa-vulkan-drivers xvfb libxkbcommon-x11-0 xdotool   # einmalig
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan
# Einzelbild:
xvfb-run -a ./target/debug/cineforge scenes/wueste.json --screenshot /tmp/x.png --size 960x540
# MCP Ende-zu-Ende (startet die App selbst; eigener Datenordner):
export CINEFORGE_HOME=/tmp/cf-test CINEFORGE_CAPTURE_FRAMES=12
Xvfb :79 & export DISPLAY=:79
python3 tools/mcp_test_client.py ./target/debug/cineforge /tmp/out steps.json
```

Bilder immer ansehen (Read auf PNG/JPG) – die Bildkontrolle ist Pflicht.
Oberfläche prüfen: `--hud` zeigt im Screenshot die komplette Oberfläche, `--select ID` wählt
ein Objekt vor. Klicks testen: App auf `Xvfb :81 -screen 0 1600x900x24` starten,
`xdotool mousemove X Y; xdotool mousedown 1; sleep 0.4; xdotool mouseup 1` (kein `--window`,
das wären synthetische Ereignisse), Bild mit `import -window root out.png`.
Software-Rendering ist langsam (~1 FPS, ~20 s pro Kontrollbild); auf echter GPU ist es schnell.
**Nie `rm -rf $HOME` o. ä.** – für Tests `CINEFORGE_HOME` auf einen frischen Ordner setzen.

## Bevy 0.20 – gelernte API-Details

- Rust ≥ 1.97.1 nötig (`rustup update stable`).
- Gepufferte Events heißen `Message` (`MessageWriter`, `MessageReader`, `add_message`).
- Eckenradius: `Node { border_radius: BorderRadius::all(px(8)), .. }` (kein eigenes Component).
- Atmosphäre: eigenes Entity `Atmosphere::earth(medium)` + auf der Kamera `AtmosphereSettings`,
  `AtmosphereEnvironmentMapLight`, `Exposure { ev100: 13.0 }`, Sonne `lux::RAW_SUNLIGHT`.
- Flug-Kamera: Feature `free_camera`, `FreeCameraPlugin` + `FreeCamera`.
- Offscreen: `Image::new_target_texture(w, h, Rgba8Unorm, Some(Rgba8UnormSrgb))`,
  `RenderTarget::Image(handle.into())`, `Screenshot::image(handle)` + `On<ScreenshotCaptured>`.
- Standardschrift ohne Umlaute → Inter wird in `hud::install_font` in den Default-Handle geladen.
- Shader-Pipelines kompilieren asynchron: vor Screenshots einige Frames warten.
- `Button`/`Interaction` sind veraltet: Klicks über `MessageReader<PointerClick>` (Picking),
  Hover über `Hovered`. 3D-Klicks brauchen CPU-Mesh-Daten (`RenderAssetUsages::default()`).
- Neben der Vorschau-Kamera (rendert in ein Bild) gibt es eine `Camera2d` nur für die UI.
  Gizmos liegen auf `RenderLayers` 7 – nur die Vorschau sieht sie, Exporte nicht.

## Regeln

- Szenen-Änderungen immer über `cf_scene::ops` (prüfen → erst dann übernehmen).
- Jede Claude-Aktion wird im Projekt-Logbuch protokolliert (`ActiveProject::log`).
- Neue MCP-Werkzeuge: Definition in `tools::definitions()`, Ausführung in `tools::execute`,
  Doku in `docs/CLAUDE-VERBINDEN.md`, Test im Ende-zu-Ende-Lauf.
- Commits mit Attribution laut System-Hinweis; auf `main` pushen (CI baut Mac/Windows).
