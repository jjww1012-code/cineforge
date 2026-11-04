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
| Bearbeitungs-Operationen (Merge-Patch, Hinzufügen, Löschen, Übersicht) | `crates/cf_scene/src/ops.rs` | ✅ Etappe 2 |
| MCP-Server (HTTP, nur lokal) + Werkzeuge + stdio-Brücke + Verlauf/Rückgängig | `apps/studio/src/{control,tools,bridge,claude_setup}.rs` | ✅ Etappe 2 |
| Chat-Fenster in der App (Claude-API) | `crates/cf_agent` | Etappe 7 |
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

## Wie Claude arbeitet

```
Claude Desktop ──stdio──► cineforge mcp (Brücke) ──HTTP──┐
Claude Code ─────────────────────HTTP────────────────────┤
                                                         ▼
                                   127.0.0.1:47800/mcp  (Server-Thread)
                                                         │  Kanal
                                                         ▼
                              Bevy-Welt: Werkzeug ausführen ─► Szene prüfen ─► speichern ─► neu aufbauen
                                                         │
                                         Screenshot: unsichtbare Kamera ─► JPEG ─► zurück an Claude
```

- Jede Änderung wird **vorher** vollständig geprüft; ungültige Befehle ändern nichts.
- Änderungen landen sofort in der Szenen-Datei (atomar geschrieben) und im Verlauf.
- Kontrollbilder rendert eine unsichtbare Zusatz-Kamera; das Fenster bleibt unberührt.
  Dieselbe Technik wird für den Video-Export genutzt.
- Später folgen: `contact_sheet` (mehrere Zeitpunkte), `timeline.*`, `fx.*`, `audio.*`, `export.*`.
