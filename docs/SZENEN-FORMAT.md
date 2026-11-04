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

## Beispiel

Siehe [`scenes/wueste.json`](../scenes/wueste.json).
