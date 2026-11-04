# Szenen-Format (Version 1)

Eine Szene ist eine JSON-Datei. Alles außer `objects[].id` und `objects[].type` ist
optional und hat sinnvolle Standardwerte. **Einheiten:** Meter, Grad, Sekunden.
**Achsen:** Y zeigt nach oben, -Z ist „Norden“ (vorne).

Die App lädt die Datei bei jeder Änderung automatisch neu. Ist die neue Fassung
fehlerhaft, bleibt die alte sichtbar und der Fehler wird angezeigt.

## Grundgerüst

```json
{
  "version": 1,
  "name": "Meine Szene",
  "description": "Klartext: was passiert hier?",
  "duration": 5,
  "fps": 30,
  "environment": { ... },
  "camera": { "position": [0, 5, 20], "look_at": [0, 0, 0], "fov": 50 },
  "objects": [ ... ]
}
```

## environment

| Feld | Standard | Bedeutung |
|---|---|---|
| `sky` | `{"type":"atmosphere"}` | Physikalischer Himmel; oder `{"type":"color","color":"#101018"}` |
| `sun.azimuth` | 135 | Himmelsrichtung in Grad (0 = Norden, 90 = Osten, 180 = Süden, 270 = Westen) |
| `sun.elevation` | 35 | Höhe über dem Horizont (0–90). Unter 10 ⇒ Sonnenuntergang |
| `sun.intensity` | 1 | Stärke relativ zur normalen Sonne |
| `sun.color` | `"#ffffff"` | Lichtfarbe |
| `sun.shadows` | true | Schatten an/aus |
| `fog` | keiner | `{"visibility": 500, "color": "#c0c8d8"}` – Sichtweite in Metern |
| `ambient` | 1 | Umgebungslicht (0 = harte Schatten) |
| `exposure` | 0 | Belichtung in Blendenstufen (+1 = doppelt so hell) |
| `bloom` | 0.15 | Glühen heller Stellen (0 = aus) |

## Gemeinsame Felder aller Objekte

| Feld | Standard | Bedeutung |
|---|---|---|
| `id` | – (Pflicht) | Eindeutig, z. B. `"tower_left"` |
| `name` | id | Anzeigename |
| `type` | – (Pflicht) | siehe unten |
| `position` | [0,0,0] | |
| `rotation` | [0,0,0] | Grad um X, Y, Z |
| `scale` | 1 | Zahl oder [x,y,z] |
| `hidden` | false | |
| `material` | grau | siehe unten |
| `repeat` | – | Wiederholung, siehe unten |

### repeat (Wiederholung – funktioniert mit jedem Objekt, auch Gruppen und Instanzen)

Kopie *i* bekommt *i*-mal den Schritt: um `pivot` drehen, skalieren, um `offset` verschieben.

| Feld | Standard | Bedeutung |
|---|---|---|
| `count` | – | Anzahl Kopien (inkl. Original) |
| `offset` | [0,0,0] | Verschiebung pro Schritt |
| `rotation` | [0,0,0] | Drehung pro Schritt (Grad) |
| `scale` | 1 | Größenfaktor pro Schritt |
| `pivot` | [0,0,0] | Drehpunkt relativ zum Objekt |

Beispiele:
- Säulenreihe: `{"count": 8, "offset": [4, 0, 0]}`
- Säulenkreis, Radius 10: `{"count": 12, "rotation": [0, 30, 0], "pivot": [-10, 0, 0]}`
- Wendeltreppe: `{"count": 20, "offset": [0, 0.2, 0], "rotation": [0, 18, 0], "pivot": [-1.5, 0, 0]}`
- Fensterraster: Fenster mit `repeat` in X in eine Gruppe, die Gruppe mit `repeat` in Y

## material

| Feld | Standard | Bedeutung |
|---|---|---|
| `color` | `"#cccccc"` | Hex oder [r,g,b] (0–1) |
| `roughness` | 0.6 | 0 = spiegelglatt, 1 = matt |
| `metallic` | 0 | 0 = Stein/Plastik, 1 = Metall |
| `emissive` | `"#000000"` | Leuchtfarbe |
| `emissive_strength` | 1 | Leuchtstärke (10–100 für deutliches Glühen) |
| `opacity` | 1 | 1 = deckend, <1 = durchsichtig |
| `flat` | false | Facetten-Look (Low-Poly) |
| `transmission` | 0 | Glas/Wasser: 0.9 = klares Glas (mit `roughness` ≈ 0.05) |
| `ior` | 1.5 | Brechung: Wasser 1.33, Glas 1.5, Diamant 2.4 |
| `clearcoat` | 0 | Klarlack (Autolack, lackiertes Holz), 0–1 |
| `unlit` | false | Ohne Licht/Schatten (Grafiken, Hologramme, Bildschirme) |
| `double_sided` | false | Beide Seiten sichtbar (Blätter, Stoff, Glas) |
| `pattern` | – | Oberflächen-Muster, siehe unten |

### pattern (Oberflächen-Muster)

`{"type": "bricks", "color2": "#cfc6b8", "scale": 0.5, "bump": 0.8, "seed": 1}`

| Feld | Standard | Bedeutung |
|---|---|---|
| `type` | noise | `noise`, `checker`, `stripes`, `bricks`, `grid`, `dots`, `marble`, `wood` |
| `color2` | dunkelgrau | Zweite Farbe (erste = `material.color`) |
| `scale` | 1 | Wiederholungen pro Meter (Ziegel: 0.5 ≈ normale Ziegelgröße) |
| `bump` | 0 | Plastische Struktur (0 = glatt, 0.3 = leicht, 1 = stark) |
| `seed` | 0 | Variante |

Muster-Größen sind bei `box`, `rounded_box`, `extrude`, `lathe`, `tube`, `text` und
`terrain` in Metern, bei `sphere`, `cylinder`, `cone`, `capsule`, `torus` pro Umfang.

## Objekt-Typen

| type | Felder (Standard) |
|---|---|
| `box` | `size` [1,1,1] |
| `sphere` | `radius` 0.5 |
| `cylinder` | `radius` 0.5, `height` 1 |
| `cone` | `radius` 0.5, `height` 1 |
| `capsule` | `radius` 0.25, `length` 1 |
| `torus` | `radius` 0.5, `thickness` 0.1 |
| `plane` | `size` [10,10] |
| `pyramid` | `size` [1,1,1] – Basis bei y = 0 |
| `terrain` | `size` [200,200], `resolution` 256, `height` 8, `style` (`hills`, `dunes`, `mountains`, `flat`), `feature_size` 40, `seed` 0, `flat_center_radius` 0 |
| `scatter` | `shape` (eine Form), `count`, `area` [50,50], `seed`, `scale_range` [0.7,1.3], `random_rotation` true, `on_terrain` (id), `sink` 0, `keep_clear_radius` 0 |
| `group` | `children` [Objekte] – Kinder bewegen sich mit |
| `point_light` | `color`, `intensity` (Lumen, 10000), `range` 30, `shadows` false |
| `rounded_box` | `size` [1,1,1], `radius` 0.1, `segments` 4 |
| `extrude` | `profile` [[x,z],…] Umriss von oben, `holes` [[[x,z],…]], `height` 1, `taper` 1 (Oberseite relativ, 0 = Spitze) – Basis bei y = 0 |
| `lathe` | `profile` [[Radius, Höhe],…] von unten nach oben, `segments` 48, `angle` 360 |
| `tube` | `path` [[x,y,z],…], `radius` 0.1, `radius_end` (optional, spitz zulaufend), `segments` 16, `smooth` true, `closed` false |
| `text` | `text` (Zeilen mit \n), `size` 1, `depth` 0.2, `align` center/left/right, `letter_spacing` 0 – steht in der XY-Ebene, schaut nach +Z, Grundlinie y = 0 |
| `instance` | `asset` (Name in der Bibliothek), `material_override` (optional) |

## Animation

Szene: `duration` (Sekunden, Standard 5) und `fps` (Standard 30).

Jede animierbare Eigenschaft ist eine Liste von **Keyframes** `{"t": Sekunde, "v": Wert, "ease": Kurve}`.
`ease` beschreibt den Weg **zum** Keyframe: `linear`, `in`, `out`, `in_out` (Standard, filmisch),
`out_back` (überschwingen), `out_bounce` (abprallen), `step` (springen).
Vor dem ersten Keyframe gilt dessen Wert, nach dem letzten der letzte.

### Objekte: `animate`

| Feld | Wert | Beispiel |
|---|---|---|
| `position` | [x,y,z] | `[{"t":0,"v":[0,0,0]},{"t":2,"v":[0,5,0]}]` |
| `rotation` | [Grad x,y,z] | 0 → 360 = eine Umdrehung |
| `scale` | Zahl oder [x,y,z] | Aufploppen: `[{"t":0,"v":0.01},{"t":0.8,"v":1,"ease":"out_back"}]` |
| `color` | Farbe | Farbwechsel |
| `emissive_strength` | Zahl | Aufleuchten |
| `opacity` | 0–1 | Ein-/Ausblenden |
| `visible` | true/false | Erscheinen/Verschwinden (springt) |
| `spin` | [Grad/s x,y,z] | Dauerdrehung, z. B. `[0, 90, 0]` |
| `smooth_path` | true/false | Position als weiche Kurve durch alle Keyframes |

Hinweise: Skalierung wirkt um den Ursprung des Objekts (Kugel/Box/Zylinder: Mitte; Pyramide,
Extrusion, Drehkörper: Boden). Zum „Hochwachsen“ vom Boden aus eignen sich `extrude`/`lathe`
oder ein Objekt in einer Gruppe, deren Ursprung am Boden liegt. Animationen funktionieren auch
in Gruppen, mit `repeat` und in Bibliotheks-Objekten.

### Kamerafahrt: `camera.animate`

```json
"camera": {
  "position": [0, 4, 14], "look_at": [0, 1, 0], "fov": 45,
  "animate": {
    "smooth_path": true,
    "position": [{"t":0,"v":[-10,3,12]}, {"t":3,"v":[0,5,13],"ease":"linear"}, {"t":6,"v":[10,2,9],"ease":"out"}],
    "look_at": [{"t":0,"v":[0,1,0]}],
    "fov": [{"t":0,"v":45}, {"t":6,"v":30}],
    "shake": 0.3
  }
}
```

`smooth_path` führt die Kamera als weiche Kurve durch alle Punkte (ideal für Flüge).
`shake` = Handkamera-Wackeln (0.2–0.5 dezent). Zoom über `fov`.

### Umgebung: `environment.animate`

`sun_elevation`, `sun_azimuth`, `sun_intensity`, `exposure` – z. B. Sonnenuntergang im Zeitraffer:
`"sun_elevation": [{"t":0,"v":35},{"t":10,"v":-2,"ease":"linear"}]`

### Prüfen und exportieren

- `screenshot` mit `time` – ein Zeitpunkt
- `contact_sheet` – mehrere Zeitpunkte in einem Bild (Bewegung, Timing, Kamerafahrt prüfen)
- `export_video` – MP4 bis 4K in den Export-Ordner des Projekts (`export_status` für Fortschritt)

## Detaillierte Objekte bauen – Tipps

- **Aus Teilen zusammensetzen:** Ein Objekt ist meist eine `group` aus vielen Teilen
  (Raumschiff = Rumpf `lathe` + Flügel `extrude` + Triebwerke `lathe` + Leitungen `tube` …).
- **Drehkörper (`lathe`)** für alles Runde: Vasen, Flaschen, Säulen, Kuppeln, Triebwerke,
  Raketen, Lampen, Räder (um 90° kippen), Becher, Türme.
- **Extrusion (`extrude`)** für alles mit Grundriss: Gebäude, Wände mit Tür-/Fenster-Löchern,
  Zahnräder, Sterne, Logos, Flügel, Treppenstufen; `taper` < 1 für Dächer und Türme.
- **Röhren (`tube`)** für Kabel, Rohre, Äste, Geländer, Tentakel, Haare, Schienen.
- **`rounded_box`** statt `box` für alles Gefertigte (Möbel, Geräte, Fahrzeugteile) –
  abgerundete Kanten fangen Licht und wirken sofort realistischer.
- **Details durch Wiederholung:** Fenster, Nieten, Rippen, Zaunlatten, Stufen mit `repeat`.
- **Material macht viel aus:** Metall (`metallic` 1, `roughness` 0.2–0.4), Kunststoff
  (`clearcoat`), Glas (`transmission`), Leuchtstreifen (`emissive`), Oberflächen (`pattern`).
- **Wiederverwenden:** Fertige Objekte mit `save_asset` in die Bibliothek legen und per
  `instance` einsetzen – so bleiben Figuren und Fahrzeuge in allen Szenen gleich.
- **Prüfen:** Nach dem Bauen `screenshot` mit `position`/`look_at` als Nahaufnahme.

## Beispiel

- [`scenes/wueste.json`](../scenes/wueste.json) – Landschaft mit Gelände und Scatter
- [`scenes/werkstatt.json`](../scenes/werkstatt.json) – alle Modellier-Werkzeuge und Muster
