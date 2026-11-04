# Fahrplan

Jede Etappe endet mit etwas, das man ausprobieren kann.

| # | Etappe | Ergebnis | Status |
|---|---|---|---|
| 1 | Fundament | App öffnet eine 3D-Welt aus einer Szenen-Datei, Flug-Kamera, Live-Reload, Screenshots, Mac/Windows-Builds | ✅ |
| 2 | Claude verbunden | Claude Desktop / Claude Code steuern die App über MCP: Szene bauen, Objekte ändern, Kontrollbilder aus beliebigem Blickwinkel, Rückgängig, Ein-Tasten-Einrichtung (F2) | ✅ |
| 2b | Chat in der App | Eingebautes Chat-Fenster (Claude-API, später über eigenen Abo-Server) | später (Etappe 7) |
| 2c | Detaillierte Objekte | Extrusion, Drehkörper, Röhren, 3D-Text, abgerundete Kanten, Wiederholung, Muster mit Struktur, Glas/Klarlack, Objekt-Bibliothek | ✅ inkl. glTF/GLB-Import |
| 2d | Fortsetzungs-System | Projekte mit Logbuch, `resume`, „Weitermachen“ nach Nutzungslimit | ✅ |
| 3 | Zeit & Export | Zeitleiste, Keyframes, Kamerafahrten, Export als MP4 bis 4K mit Hardware-Encoder | ✅ |
| 4 | Klicken & sagen | Objekt anklicken → hervorgehoben → Claude ändert genau dieses; Klartext-Ablauf („Was passiert“) | ✅ |
| 5 | Effekte | Partikel (Feuer, Rauch, Funken, Schnee, Regen, Explosion, Magie, Staub, Fontäne), Licht-Flackern/Blitz, Blenden, Kamerawackeln | ✅ |
| 5b | Film-Export | Alle Storyboard-Shots zu einem Video, Schnitte oder Blenden | ✅ |
| 6 | Ton | Musik, Geräusche, Sprecher (Systemstimmen) im Export ✅ · eigene Klangsynthese + Musik-Generator ✅ · Vorschau in der App, Beat-Sync, bessere KI-Stimmen | teilweise |
| 6b | Lange Filme | Shot-Cache, Fortsetzen, Video-Clips, Bilder, Einblendungen, ffmpeg mitgeliefert | ✅ |
| 6c | 2D & Cartoons | SVG-Grafiken, 2D-Kamera, Cartoon-Farbstil ✅ · Figuren mit Gelenken, Lippen-Sync | teilweise |
| 6d | Oberfläche | Übersichtliche Oberfläche wie CapCut (Spuren, Medien, Eigenschaften), inkrementelle Updates | ✅ (Ausbau folgt) |
| 7 | Produkt | Konten, Abo, Credits, Claude über eigenen Server, Auto-Update, signierte Installer | |
| 8 | Teams | Gemeinsames Arbeiten in Echtzeit, Vorlagen-Bibliothek / Marktplatz | |
| 9 | Lange Filme | Regie-Agent schreibt Storyboard, mehrere Claude-Instanzen bauen Szenen parallel, 1-h-Filme | |

## Grundsätze

- **Anfangs kostenlos:** nur freie Bausteine (Bevy, Piper-Stimmen, ffmpeg). Bezahldienste
  erst, wenn das Produkt verkauft wird.
- **Schnelles Rendern ist Pflicht:** Ziel ist ein Export in Echtzeit oder schneller.
- **Alles hat eine id:** damit Claude und Menschen dasselbe Objekt meinen.
