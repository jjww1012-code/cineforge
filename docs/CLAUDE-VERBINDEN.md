# Claude mit CineForge verbinden

CineForge spricht **MCP** (Model Context Protocol), den offenen Standard, über den
Claude Werkzeuge in anderen Programmen benutzt. Damit arbeitet dein normales Claude
(mit deinem bestehenden Claude-Abo) direkt in der App. Es fallen keine zusätzlichen
Kosten an.

## Claude Desktop (empfohlen)

1. CineForge in den **Programme-Ordner** ziehen (Mac) und starten.
2. In CineForge **F2** drücken. Unten erscheint „Claude Desktop verbunden ✓“.
3. Claude Desktop **komplett beenden** (Mac: Cmd+Q) und neu öffnen.
4. Im Chat erscheint unter den Werkzeugen (Schieberegler-Symbol) **cineforge**.
5. Loslegen, zum Beispiel:
   > Öffne CineForge und bau mir eine Unterwasserstadt mit leuchtenden Kuppeln.
   > Schau dir das Ergebnis an und verbessere es, bis es gut aussieht.

Läuft CineForge noch nicht, startet Claude die App automatisch.

<details>
<summary>Von Hand einrichten (falls F2 nicht klappt)</summary>

Datei öffnen:
- Mac: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

Eintragen (Pfad anpassen):

```json
{
  "mcpServers": {
    "cineforge": {
      "command": "/Applications/CineForge.app/Contents/MacOS/cineforge",
      "args": ["mcp"]
    }
  }
}
```

Windows: `"command": "C:\\Programme\\CineForge\\CineForge.exe"`
</details>

## Claude Code

App starten, dann im Terminal:

```bash
claude mcp add --transport http cineforge http://127.0.0.1:47800/mcp
```

## Was Claude damit kann

| Werkzeug | Zweck |
|---|---|
| `get_guide` | Szenen-Format lernen |
| `list_objects`, `get_scene` | Aktuellen Stand lesen |
| `add_objects`, `update_object`, `remove_objects` | Objekte bauen und ändern |
| `update_scene_settings` | Himmel, Sonne, Nebel, Kamera, Name |
| `set_scene` | Komplett neue Szene |
| `screenshot` | Ergebnis ansehen, auch aus beliebigem Blickwinkel |
| `undo` | Letzte Änderung zurücknehmen |
| `list_scenes`, `open_scene`, `new_scene` | Szenen-Dateien verwalten |

Jede Änderung wird geprüft, bevor sie übernommen wird. Fehlerhafte Befehle werden mit
Begründung abgelehnt, die Szene bleibt dann unverändert. Alles landet sofort in der
Szenen-Datei und ist mit **Cmd/Strg+Z** rückgängig zu machen.

## Sicherheit

- Der Server ist nur auf diesem Rechner erreichbar (127.0.0.1), nicht im Netzwerk.
- Anfragen von Webseiten werden abgelehnt.
- Claude kann nur Szenen im Ordner `Dokumente/CineForge/Szenen` öffnen und anlegen.
