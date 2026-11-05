# Szenen-Format (Version 1)

Eine Szene ist eine JSON-Datei. Alles außer `objects[].id` und `objects[].type` ist
optional und hat sinnvolle Standardwerte. **Einheiten:** Meter, Grad, Sekunden.
**Achsen:** Y zeigt nach oben, -Z ist „Norden“ (vorne).

Die App lädt die Datei bei jeder Änderung automatisch neu. Ist die neue Fassung
fehlerhaft, bleibt die alte sichtbar und der Fehler wird angezeigt.

## Schnellstart mit Vorlagen

`new_scene` mit `"template"` legt eine fertig ausgeleuchtete Umgebung an: `wiese`, `dorf`, `wald`,
`strand`, `nacht`, `studio`, `weltraum`, `zimmer`, `cartoon_2d`. Danach nur noch Figuren
(`type: "character"`), Handlung (`actions`, `path`) und Kamera (`frame`, `follow`) ergänzen.

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
  "objects": [ ... ],
  "overlays": [ ... ]
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
| `ao` | auto | Kontaktschatten in Ecken, Fugen, unter Objekten (wirkt sofort realistischer). Standard: an im Film-Stil, aus im Cartoon-Stil |
| `stars` | 0 | **Sternenhimmel** (1 = normal, 2 = sehr viele) – erscheint nur nachts (Sonne unter dem Horizont, `elevation` < 0, oder dunkler Farb-Himmel) |
| `moon` | – | **Mond**: `{"azimuth": 120, "elevation": 30, "intensity": 1, "size": 1}` – kühles Mondlicht mit Schatten + Mondscheibe (`size` 3 = Märchenmond). Für Nacht: Sonne `elevation` −10, `stars` 1, `moon`, warme `point_light`s (Fenster, Feuer) |
| `light_rays` | 0 | Sichtbare Sonnenstrahlen im Dunst (0.3 dezent, 1 stark) – ideal bei tiefer Sonne, Wald, Ruinen |
| `grade` | – | Farbkorrektur: `{"saturation": 1.15, "vignette": 0.35}` (Sättigung 1 = normal; Vignette = dunklere Bildränder) |
| `look` | auto | `"film"` = natürliche, filmische Tonwerte (Farben etwas gedämpft) · `"cartoon"` = Farben **exakt** wie angegeben (Cartoons, 2D, Grafiken, Logos). Ohne Angabe: `cartoon` bei 2D-Kamera, sonst `film` |

## camera

| Feld | Standard | Bedeutung |
|---|---|---|
| `position` | [0,5,20] | Standort der Kamera |
| `look_at` | [0,0,0] | Punkt, auf den sie schaut |
| `fov` | 50 | Bildwinkel in Grad (klein = Tele/Zoom, groß = Weitwinkel) |
| `dof` | – | **Tiefenschärfe** wie im Kino: `{"focus": "held", "aperture": 2}` – `focus` = id eines Objekts (Schärfe folgt ihm) oder Entfernung in Metern; `aperture` = Blende (1.4 sehr weicher Hintergrund, 2.8 Standard, 8 fast alles scharf) |
| `motion_blur` | – | **Bewegungsunschärfe** (Verschlusswinkel 0–1, 0.5 = Kino-Standard). Wirkt im Export und beim Abspielen |
| `orthographic` | – | **2D-Kamera** ohne Perspektive: sichtbare Bildhöhe in Metern (z. B. `7`). Ideal für Cartoons, Erklärvideos, Seitenansichten. Weiter entfernte Objekte werden nicht kleiner – Tiefe (z) bestimmt nur, was vorne liegt |
| `animate` | – | Kamerafahrt, siehe unten |
| `target` | – | **Immer auf dieses Objekt schauen** (id) – ersetzt `look_at`, folgt dem Objekt auch wenn es sich bewegt |
| `target_offset` | [0,0,0] | Blickpunkt relativ zum Objekt, z. B. `[0, 1, 0]` = Kopfhöhe einer 1,2-m-Figur |
| `follow` | – | **Kamera folgt einem Objekt**: `{"target": "max", "offset": [2, 1.5, 4], "smooth": 0.5}` – Position = Objekt + `offset` (Welt), weich nachgezogen (`smooth` in s); der Blick bleibt auf dem Objekt |
| `frame` | – | **Einstellungsgröße wie im Film**: `{"target": "max", "shot": "close", "angle": -30, "tilt": 10}` – `shot`: `wide` (Totale), `full` (ganze Figur), `medium` (halbnah), `close` (nah), `extreme_close` (Großaufnahme); `angle` = Blickwinkel um die Figur (0 = von vorne, berücksichtigt ihre Drehung), `tilt` = von oben (+) / unten (−). Rechnet die Kamera automatisch aus, auch wenn die Figur läuft |
| `orbit` | – | **Kamera kreist**: `{"target": "auto", "radius": 6, "height": 2, "speed": 20, "start_angle": 0}` (oder `center` statt `target`) – 360°-Fahrt um ein Objekt |

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
| `subdivide` | 0 | Feiner unterteilen (1–5, jede Stufe ×4 Dreiecke) – nötig für Verformungen |
| `rounded` | false | Beim Unterteilen abrunden (Loop-Glättung): Würfel → Kiesel, Kanten → weiche Rundungen. Ohne `subdivide` = 2 Stufen |
| `deform` | – | Verformungen, siehe unten |
| `billboard` | false | Dreht sich immer zur Kamera (nur um die senkrechte Achse) – für 2D-Figuren, Bäume, Bilder in 3D-Welten („Pappaufsteller“) |

### deform (Verformen – macht aus Grundformen detaillierte Modelle)

Liste, wird der Reihe nach angewendet. Alles bezieht sich auf das Objekt selbst (vor `rotation`/`scale`).
Damit eine Verformung weich aussieht, braucht die Form genug Punkte: `subdivide` 3–4 bei Boxen.

| type | Felder (Standard) | Wirkung / Beispiel |
|---|---|---|
| `twist` | `angle` 90, `axis` y | Verdrehen: gedrehte Säule, Bohrer, Horn, Eistüte |
| `bend` | `angle` 45, `axis` y | Biegen (unten fest, Richtung +X): Ast, Rüssel, Schwanz, gebogenes Rohr |
| `taper` | `scale` 0.5, `axis` y | Verjüngen (oben `scale`, 0 = Spitze): Baumstamm, Karotte, Zahn, Kralle |
| `wave` | `amplitude` 0.1, `wavelength` 1, `axis` x, `phase` 0 | Wellen: Fahne, Stoff, Wasser, Schlange |
| `inflate` | `amount` 0.1 | Aufblähen (negativ = einziehen) |
| `noise` | `amount` 0.1, `scale` 1, `octaves` 3, `seed` 0 | Unregelmäßige Oberfläche: Felsen, Erde, Rinde, Haut, Wolken |

Rezepte:
- **Felsen:** `sphere` + `"subdivide": 2` + `noise` (amount 0.3×Radius, scale 1.5, octaves 4) + `material.flat`
- **Kiesel / weiche Formen:** `box` + `"rounded": true, "subdivide": 3`
  (Achtung: `rounded` schrumpft spitze Formen stark – Pyramiden/Dächer werden zu flachen Klumpen.
  Für leicht gerundete Kanten lieber `rounded_box` nehmen.)
- **Figuren-Körper (Pixar-Stil):** `capsule`/`sphere`/`rounded_box` mit `rounded` + leichtem `noise` (amount 0.01) gegen das „Computer-Perfekte“
- **Ast:** `cylinder` + `subdivide` 3 + `taper` 0.35 + `bend` 60
- **Fahne:** `box` [2, 1.2, 0.02] + `subdivide` 4 + `wave` (axis x)

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
| `outline` | – | **Cartoon-Umriss**: `{"width": 0.02, "color": "#000000"}` – schwarze Linie um 3D-Objekte wie gezeichnet (passt 3D an 2D-Figuren an) |
| `texture` | – | Bild als Oberfläche (Datei in `Bilder/`), z. B. Plakat, Bildschirm, Logo auf einer Kiste |

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
| `text` | `text` (Zeilen mit \n), `size` 1, `depth` 0.2 (0 = flach, z. B. für Bildschirm-Inhalte und Einblendungen), `align` center/left/right, `letter_spacing` 0, `weight` regular/bold/black, `italic` false – steht in der XY-Ebene, schaut nach +Z, Grundlinie y = 0 |
| `instance` | `asset` (Name in der Bibliothek), `material_override` (optional) |
| `image` | `file` (im Ordner `Bilder/` des Projekts), `width` 1 – Bildfläche mit dem Seitenverhältnis des Bildes (PNG mit Transparenz möglich), XY-Ebene, schaut nach +Z. Für exakte Farben `"material": {"unlit": true}` |
| `svg` | `svg` (SVG-Text direkt im Feld) **oder** `file` (SVG-Datei in `Bilder/`), `width` 1, `depth` 0, `mode` auto, `resolution` auto – **2D-Grafik**, mittig, XY-Ebene, schaut nach +Z. `mode`: `"vector"` = jede Fläche wird ein eigenes Flächenstück (in jeder Größe scharf, mit `depth` > 0 ein 3D-Relief), `"image"` = volle SVG-Darstellung wie in einem Grafikprogramm (Farbverläufe, `feGaussianBlur`-Schatten, Masken, Text mit Inter, Transparenz). `auto` wählt `image`, sobald das SVG Verläufe, Filter, Masken, Muster oder Text enthält. `resolution` = Pixel der langen Seite (Standard 400 pro Meter, 1024–4096). Siehe „Cartoons und 2D“ |
| `tree` | `kind` (`oak`, `pine`, `palm`, `birch`, `lollipop` = Cartoon-Kugelbaum), `height` 5, `leaves` (Farbe; Herbst `#d9822b`, Blüte `#f4b6c8`), `trunk`, `wind` 0 (0.3 leichtes Wiegen), `seed`, `low_poly` – **fertiger Baum**. Wald: `scatter` mit `"shape": {"type": "tree", "kind": "pine", "height": 8}` (jeder Baum eine andere Variante, max. 1500) |
| `water` | `size` [40,40], `wave_height` 0.15, `wavelength` 4, `speed` 1, `direction` 0, `color`, `resolution` 96 – **Wasserfläche mit laufenden Wellen** (See, Meer, Pool); etwas über den Grund legen |
| `model` | `file` (glTF/GLB-Datei im Ordner `Modelle/`) – **3D-Modell importieren** (z. B. von Sketchfab, Blender, KI-3D-Generatoren) mit eigenen Materialien und Texturen. Größe/Lage über `position`, `rotation`, `scale` |

## Figuren (`type: "character"`) – fertige Cartoon-Figuren im Pixar-Stil

Eine Zeile reicht für eine komplette Figur mit Gelenken, großen Augen (blinzeln von selbst),
Brauen, Mund (Lippen-Sync), Frisur und Kleidung. Vorderseite schaut nach +Z, Füße auf y = 0.

```json
{"id": "max", "type": "character", "height": 1.2, "hair": "spiky", "hair_color": "#3b2412",
 "shirt": "#ff8a3d", "pants": "#2d3a5a", "shoes": "#e53935", "skin": "#f2c29b", "eye_color": "#3a6fd8",
 "expression": "happy", "outline": false,
 "talk": {"audio": "max_satz1.wav", "start": 1.0},
 "actions": [{"type": "wave", "start": 0.5, "end": 2.5}, {"type": "talk", "start": 1.0, "end": 4.0}],
 "animate": {"path": {"points": [[-4,0,0],[0,0,1],[4,0,0]], "start": 4, "end": 8}}}
```

| Feld | Standard | Werte |
|---|---|---|
| `height` | 1.2 | Größe in Metern (Kind 1.0–1.3, Erwachsene 1.6–1.9) |
| `build` | normal | `slim`, `normal`, `round` |
| `hair` | short | `short`, `spiky`, `long`, `bun`, `curly`, `bald` |
| `skin`, `hair_color`, `shirt`, `pants`, `shoes`, `eye_color` | – | Farben |
| `expression` | happy | `neutral`, `happy`, `sad`, `angry`, `surprised` |
| `outline` | false | gezeichnete Umrisslinie |
| `blink` | true | Blinzeln |
| `talk` | – | Lippen-Sync wie `animate.talk` (WAV aus `Audio/`) |
| `actions` | – | `idle`, `walk`, `run`, `wave`, `jump`, `cheer`, `dance`, `point`, `nod`, `shake_head`, `talk` (Gesten beim Sprechen), `sit`, `think` – je mit `start`, `end`, `speed` |
| `look_at` | – | Kopf und Oberkörper drehen sich zu diesem Objekt (z. B. Gesprächspartner) |
| `auto_walk` | true | Läuft die Figur einen `animate.path` entlang, bewegen sich Beine und Arme automatisch im passenden Tempo (schnell ⇒ rennen) |

**Dialoge in einem Schritt:** Werkzeug `dialogue` mit
`{"lines": [{"who": "max", "text": "Hallo Lena!", "action": "wave"}, {"who": "lena", "text": "Hi Max!", "pause": 0.3}], "voices": {"lena": "Anna"}}`
– erzeugt die Stimmen, legt die Sätze hintereinander, gibt jeder Figur ihre Tonspur (Lippen-Sync +
Sprech-Gesten), setzt Untertitel und trägt den Ton im Storyboard-Shot ein (Szene wird bei Bedarf länger).

Tipps: Gesichtsausdruck je Shot wählen (zwei Fassungen derselben Figur in verschiedenen Szenen),
Kamera mit `follow`/`target` auf die Figur, `dof` mit `focus` = Figur. Aktionen dürfen sich
überlappen (z. B. `walk` + `wave`). Übergänge zwischen Aktionen werden automatisch weich.

## Cartoons und 2D

Für Zeichentrick-Look: **2D-Kamera** (`camera.orthographic`) + **SVG-Grafiken** (`type: "svg"`).
Farben werden dann automatisch exakt wiedergegeben (`look: "cartoon"`).

```json
"environment": {"sky": {"type": "color", "color": "#9ad8ff"}, "bloom": 0},
"camera": {"position": [0, 2.5, 20], "look_at": [0, 2.5, 0], "orthographic": 7},
"objects": [
  {"id": "hintergrund", "type": "svg", "width": 18, "position": [0, 2.5, -3], "svg": "<svg …>…</svg>"},
  {"id": "junge", "type": "svg", "width": 1.6, "position": [0, 1.4, 0], "svg": "<svg viewBox='0 0 100 150'>…</svg>"}
]
```

- **Ebenen über z:** Hintergrund z = -3, Bäume z = -1, Figuren z = 0, Vordergrund z = 1.
  Innerhalb eines SVG liegt jedes spätere Element vor dem vorherigen (wie beim Zeichnen).
- **Im SVG:** `rect`, `circle`, `ellipse`, `path` (auch Kurven), `polygon`, `stroke` (Konturen,
  `stroke-linecap: round`), `fill-opacity`, `transform`, Gruppen. Verläufe werden als einfarbige
  Fläche (erste Farbe) gezeichnet. Einfache Hochkommas `'` im SVG sparen Escape-Zeichen im JSON.
- **Bewegung:** Figuren aus Teilen bauen (Körper, Kopf, Arme je als eigenes `svg`); Laufen =
  `position` mit kleinem Auf und Ab. Ein `svg` dreht sich um seine Mitte. **Gelenke:** das Teil
  (Arm) in eine eigene `group` legen, die am Gelenk (Schulter) sitzt, den Arm darin so verschieben,
  dass die Schulter im Ursprung liegt, und die Gruppe mit `animate.rotation` (um z) drehen.
- **Detaillierte 2D-Figuren:** Farbverläufe für Rundungen (`radialGradient` mit hellem Lichtpunkt
  oben links), weiche Schatten (`<filter><feGaussianBlur stdDeviation='3'/></filter>`), dunkle
  Konturen (`stroke` 2–4, `stroke-linejoin='round'`), Glanzpunkte in den Augen, Wangenröte mit
  halbtransparentem Verlauf. Mehrere Schichten: Schatten → Körper → Details → Lichter.
- **Sprechende Figuren:** Stimme mit `generate_voice` erzeugen, Figur als `group` aus Kopf, Augen,
  Mund (je ein `svg`); Mund mit `"animate": {"talk": {"audio": "…wav", "start": …}}`, Augen mit
  `blink`, die ganze Gruppe mit `breathe`. Die Stimme zusätzlich als `audio` im Shot eintragen,
  damit man sie im Video hört (gleicher Startzeitpunkt).
- **2D und 3D mischen:** SVG-Figuren mit `"billboard": true` in eine 3D-Welt stellen (sie schauen
  immer zur Kamera), 3D-Objekte mit `material.outline` bekommen gezeichnete Umrisse, `look: "cartoon"`
  sorgt für exakte, kräftige Farben. Umgekehrt dürfen 3D-Objekte in 2D-Szenen vorkommen.
- Beispiel: [`scenes/cartoon.json`](../scenes/cartoon.json)

## Einblendungen (`overlays`)

Texte, die **fest im Bild** stehen – unabhängig von der Kamera (Untertitel, Bauchbinden,
Daten wie „06:14 · Noah, 17 · Rating 1232“). Sie passen sich 16:9 und 9:16 automatisch an.

```json
"overlays": [
  {"text": "06:14 · Noah, 17 · Rating 1232", "x": -0.92, "y": -0.85, "size": 0.04, "italic": true, "align": "left"},
  {"text": "Ein Tag. Eine Zahl.", "y": 0.7, "size": 0.08, "weight": "black", "start": 1, "end": 4}
]
```

| Feld | Standard | Bedeutung |
|---|---|---|
| `text` | – | Text (Zeilen mit \n) |
| `x`, `y` | 0, -0.8 | Position: -1 = linker/unterer Rand, 1 = rechter/oberer Rand (y = Grundlinie) |
| `size` | 0.05 | Anteil der **kürzeren** Bildseite |
| `color`, `weight`, `italic`, `align` | weiß, regular, false, center | |
| `start`, `end`, `fade` | 0, Ende, 0.3 | Sichtbarkeit (s) mit weichem Ein-/Ausblenden |
| `effect` | fade | Auftritt: `pop` (ploppt federnd auf – Titel), `slide_up` (gleitet herein), `typewriter` (Buchstabe für Buchstabe, 22 pro Sekunde) |
| `shadow` | false | Weicher dunkler Schatten – lesbar auf hellen Bildern |
| `background`, `background_opacity` | –, 0.6 | Box hinter dem Text, z. B. Untertitel: `"background": "#000000"` |

## Partikel-Effekte (`type: "particles"`)

```json
{"id": "feuer", "type": "particles", "preset": "fire", "position": [0, 0.2, 0]}
```

Voreinstellungen (`preset`): `fire`, `smoke`, `sparks`, `snow`, `rain`, `explosion`, `magic`,
`dust`, `fountain`. Alles lässt sich anpassen:

| Feld | Bedeutung |
|---|---|
| `rate` | Partikel pro Sekunde |
| `burst` | Alle auf einmal bei `start` (Explosionen, Konfetti) |
| `start`, `end` | Zeitraum der Erzeugung (s) |
| `lifetime` | [min, max] Lebensdauer (s) |
| `speed`, `direction`, `spread` | Startgeschwindigkeit [min,max], Hauptrichtung, Streuwinkel (180 = alle Richtungen) |
| `gravity`, `drag`, `turbulence` | Beschleunigung [x,y,z], Luftwiderstand, Taumeln (m) |
| `size` | [Anfang, Ende] in Metern |
| `color_start`, `color_end`, `opacity`, `glow` | Farbverlauf, Deckkraft, Leuchten |
| `area` | Entstehungsfläche [Breite, Höhe, Tiefe] (Schnee/Regen: z. B. [30, 0.5, 30], hoch oben platzieren) |
| `prewarm` | Dauerhafte Effekte laufen beim Start schon (Standard: an) |
| `max`, `seed` | Obergrenze, Variante |

Tipps: Explosion = `explosion` + `smoke` mit `burst` und etwas späterem `start` + ein
`point_light` mit Blitz (`animate.intensity`). Lagerfeuer = `fire` + `smoke` darüber + `sparks` +
warmes `point_light` mit `"animate": {"flicker": 0.4}`.

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
| `intensity` | Lumen | Nur `point_light`: Lichtstärke über die Zeit (Blitz, Ein-/Ausschalten) |
| `draw` | 0–1 | Nur `tube`: gezeichneter Anteil – die Linie „zeichnet sich“ (Logos, Routen, Unterschriften) |
| `flicker` | 0–1 | Nur `point_light`: Flackern (0.3 Kerze/Feuer) |
| `spin` | [Grad/s x,y,z] | Dauerdrehung, z. B. `[0, 90, 0]` |
| `smooth_path` | true/false | Position als weiche Kurve durch alle Keyframes |
| `path` | `{"points": [[0,0,0],[4,0,2],[8,0,0]], "start": 0, "end": 5}` | **Weg entlang laufen/fahren/fliegen:** weiche Kurve durch die Punkte mit gleichmäßigem Tempo; `face` (true) dreht das Objekt in Laufrichtung (Vorderseite +Z), `ease` (in_out), `closed` (Runde). Ersetzt `position` |
| `talk` | `{"audio": "max_satz1.wav", "start": 1.2}` | **Lippen-Sync:** der Mund öffnet sich mit der Lautstärke der Stimme (WAV aus `Audio/`, z. B. von `generate_voice`). `start` = wann die Datei in der Szene beginnt; `closed` (0.15) = Höhe des geschlossenen Mundes; `gain` (1) = Verstärkung. Gehört auf das Mund-Objekt (Ursprung = Mundmitte) |
| `blink` | `{"every": 4, "seed": 1}` | **Blinzeln** in natürlichen, zufälligen Abständen (beide Augen derselbe `seed`, andere Figur anderer `seed`) |
| `breathe` | 0.015 | **Atmen:** sanftes Heben/Senken – Figuren wirken nie eingefroren |

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
- `export_video` – die geöffnete Szene als MP4 bis 4K (`export_status` für Fortschritt)
- `export_film` – der ganze Film: alle Storyboard-Shots (je `scene` + `duration`) hintereinander,
  mit harten Schnitten oder Abblenden (`transition: "fade"`)

## Kino-Qualität (Pixar-/Disney-Look) – so geht's

1. **Licht zuerst:** tiefe Sonne (`elevation` 8–25) für lange Schatten und warmes Licht, `sun.color`
   leicht warm (`#ffe2c0`) am Abend, kühl (`#dfe8ff`) am Morgen; für Innenräume mehrere `point_light`
   (Hauptlicht warm, Fülllicht schwach und kühl, Kantenlicht hinter der Figur).
2. **Kamera wie im Film:** `fov` 30–40 (leichtes Tele wirkt edel), `dof` mit Fokus auf die
   Hauptfigur und `aperture` 2–2.8, langsame Kamerafahrten mit `smooth_path` und `ease: in_out`,
   `motion_blur` 0.5 bei schnellen Bewegungen.
3. **Atmosphäre:** `fog` für Tiefe, `light_rays` 0.3–0.6 bei Gegenlicht, `bloom` 0.15–0.3 für
   leuchtende Dinge, Partikel (Staub, Glühwürmchen) für Leben im Bild.
4. **Formen weich und organisch:** `rounded` + `subdivide` für Figuren und alles Natürliche,
   `rounded_box` für Gebautes, kleine Unregelmäßigkeiten (`noise` mit sehr kleinem `amount`).
5. **Materialien:** wenig reines Weiß/Schwarz, Farben leicht entsättigt mit einem kräftigen
   Akzent, Glanz über `roughness` (Haut 0.6, Lack 0.2 + `clearcoat`), Metall `metallic` 1.
6. **Bewegung mit Charakter:** `out_back` (Überschwingen) und `out_bounce` (Abprallen) statt
   linear, Vorbereitung vor großen Bewegungen (kurz zurück, dann los), nichts steht ganz still
   (leichtes Atmen über `scale` 1 → 1.02).
7. **Farbkorrektur zum Schluss:** `grade` mit `saturation` 1.1–1.2 und `vignette` 0.3.
8. **Prüfen:** `contact_sheet` über die ganze Szene, Nahaufnahmen per `screenshot` mit `position`.

## Mitgelieferte Bibliothek

Sofort einsetzbar mit `{"id": "haus_1", "type": "instance", "asset": "haus", "position": [x, 0, z]}`
(Ursprung unten in der Mitte, Vorderseite +Z): `haus`, `auto`, `laterne` (mit Licht), `bank`, `zaun`
(4 m, mit `repeat` verlängern), `tisch`, `stuhl`, `wolke`, `brunnen` (sprudelt), `blume`, `busch`,
`felsen`. `list_assets` zeigt alle mit Beschreibung; `get_asset` liefert den Bauplan zum Abwandeln
(z. B. andere Farbe → mit neuem Namen per `save_asset` speichern).

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
- **Organisch statt eckig:** `rounded` + `subdivide` für alles Natürliche, `deform` für Biegungen,
  Verdrehungen und unregelmäßige Oberflächen (siehe „deform“).
- **Prüfen:** Nach dem Bauen `screenshot` mit `position`/`look_at` als Nahaufnahme.

## Beispiel

- [`scenes/wueste.json`](../scenes/wueste.json) – Landschaft mit Gelände und Scatter
- [`scenes/werkstatt.json`](../scenes/werkstatt.json) – alle Modellier-Werkzeuge und Muster
- [`scenes/cartoon.json`](../scenes/cartoon.json) – 2D-Cartoon mit SVG-Grafiken und 2D-Kamera
- [`scenes/vorzeige.json`](../scenes/vorzeige.json) – 2D-Figur in 3D-Welt: Verläufe, Blinzeln, Atmen,
  Cartoon-Umrisse, verformter Baum, Tiefenschärfe, Lichtstrahlen, Farbkorrektur, Kamerafahrt
