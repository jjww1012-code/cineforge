# CineForge – Architektur

CineForge ist ein natives Programm (macOS + Windows), in dem Claude Szenen, Videos
und Filme als Code bzw. strukturierte Dokumente erstellt und sofort sieht, was dabei
herauskommt.

## Leitidee

```
 Mensch ──┐                         ┌──► Live-Vorschau (Echtzeit, GPU)
          ├──► Szenen-Dokument ─────┤
 Claude ──┘    (eine Wahrheit)      ├──► Bildkontrolle für Claude (Screenshots, Contact Sheets)
                                    └──► Export (MP4/ProRes bis 4K, Hardware-Encoder)
```

1. **Ein Dokument, viele Bearbeiter.** Szene = strukturiertes Dokument mit stabilen
   Objekt-ids. Mensch (per Klick) und Claude (per Werkzeug) ändern dasselbe Dokument.
2. **Echtzeit-Engine statt Frame-Screenshots.** Gerendert wird wie in einem Spiel auf
   der GPU; der Export schreibt Frames direkt in den Hardware-Encoder.
3. **Claude sieht mit.** Jede Änderung kann sofort als Bild geprüft werden.
4. **Deterministisch.** Gleicher Seed ⇒ gleiche Welt auf jedem Rechner.

## Bausteine

| Baustein | Ort | Status |
|---|---|---|
| Szenen-Dokument (Modell, Validierung, prozedurale Geometrie) | `crates/cf_scene` | ✅ Etappe 1 |
| Studio-App (Fenster, Vorschau, Flug-Kamera, Live-Reload, Screenshots) | `apps/studio` | ✅ Etappe 1 |
| Zeitleiste & Animation (Keyframes, Kamerafahrten) | `crates/cf_timeline` | Etappe 3 |
| Export (Hardware-Encoding, parallele Abschnitte, Szenen-Cache) | `crates/cf_export` | Etappe 3 |
| Claude-Agent in der App + MCP-Server nach außen | `crates/cf_agent` | Etappe 2 |
| Effekte (Partikel, Explosionen, Wetter, Übergänge, Text) | `crates/cf_fx` | Etappe 5 |
| Audio (KI-Stimmen lokal, Musik, Beat-Sync, Untertitel) | `crates/cf_audio` | Etappe 6 |
| Server (Konten, Abo, Credits, Claude-Proxy, Sync) | `server/` | Etappe 7 |
| Zusammenarbeit (CRDT-Sync, Rechte) | `crates/cf_collab` | Etappe 8 |

## Technik-Entscheidungen

| Thema | Wahl | Begründung |
|---|---|---|
| Sprache | Rust | Geschwindigkeit, Sicherheit, ein Code für Mac + Windows |
| Engine | Bevy 0.20 (wgpu: Metal / DX12 / Vulkan) | Echtzeit-3D, PBR, Schatten, Atmosphäre, Bloom, Partikel, frei & quelloffen (MIT/Apache) |
| Szenen-Format | JSON (später zusätzlich Skripte für prozedurale Objekte) | Für Claude leicht zu lesen/schreiben, gut zu prüfen, versionierbar, CRDT-fähig |
| Claude-Anbindung | Eingebauter Agent (Claude-API über eigenen Server) + MCP-Server | In der App direkt nutzbar; zusätzlich aus Claude Code / Desktop steuerbar |
| Fernsteuerung der Engine | Werkzeug-Schnittstelle über lokalen Port (Bevy Remote Protocol als Basis) | Claude ändert Objekte live, ohne Dateien neu zu schreiben |
| Stimmen | Lokal & kostenlos (Piper TTS), später optional Premium-Anbieter | Anfangs ohne laufende Kosten |
| Encoding | ffmpeg + VideoToolbox (Mac) / NVENC, AMF, QSV (Windows) | Export ≈ Echtzeit oder schneller |
| Builds | GitHub Actions: macOS (Apple Silicon) + Windows | Kein eigener Mac/PC zum Bauen nötig |

## Geschwindigkeit bei langen Filmen (4K, 1 Stunde)

- **Szenen-Cache:** Jede Szene hat einen Fingerabdruck (Hash ihres Dokuments). Nur
  geänderte Szenen werden neu gerendert, der Rest kommt aus dem Cache.
- **Parallel in Abschnitten:** Lange Filme werden in Abschnitte zerlegt, gleichzeitig
  gerendert und verlustfrei aneinandergehängt.
- **Proxy-Vorschau:** Arbeiten in niedriger Auflösung, 4K erst beim Export.
- **Instanzen:** Tausende gleiche Objekte (Steine, Bäume) teilen sich Mesh und Material.

## Wie Claude arbeiten wird (Etappe 2+)

Werkzeuge (Auswahl):

- `scene.get` / `scene.patch` – Dokument lesen / gezielt ändern (per Objekt-id)
- `object.create`, `object.update`, `object.delete`
- `view.screenshot(camera, size)` – Bild der aktuellen Ansicht
- `view.contact_sheet(start, end, n)` – mehrere Zeitpunkte als Raster
- `scene.validate` – Fehler und Warnungen in Klartext
- `timeline.*`, `fx.*`, `audio.*`, `export.*`
