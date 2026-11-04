# Mac-App signieren & notarisieren

Ohne Signatur warnt macOS beim ersten Start. Mit deinem Apple-Developer-Account
kann GitHub die App automatisch signieren und von Apple prüfen lassen. Dann startet sie
auf jedem Mac ohne Warnung.

## Einmalig einrichten

1. **Zertifikat erstellen:** In Xcode → Settings → Accounts → dein Team → *Manage
   Certificates* → `+` → **Developer ID Application**.
2. **Exportieren:** Programm *Schlüsselbundverwaltung* → „Meine Zertifikate“ →
   „Developer ID Application: …“ → Rechtsklick → *Exportieren* als `.p12` mit Passwort.
3. **Als Text umwandeln:** Terminal: `base64 -i Zertifikat.p12 | pbcopy`
   (jetzt ist es in der Zwischenablage).
4. **App-spezifisches Passwort:** appleid.apple.com → Anmelden & Sicherheit →
   App-spezifische Passwörter → neu, z. B. „CineForge CI“.
5. **In GitHub hinterlegen:** Repository → Settings → Secrets and variables → Actions →
   *New repository secret*:

| Name | Inhalt |
|---|---|
| `APPLE_CERT_P12` | der base64-Text aus Schritt 3 |
| `APPLE_CERT_PASSWORD` | Passwort der .p12-Datei |
| `APPLE_SIGN_IDENTITY` | z. B. `Developer ID Application: Max Muster (AB12CD34EF)` |
| `APPLE_ID` | deine Apple-ID (E-Mail) |
| `APPLE_TEAM_ID` | die 10-stellige Team-ID (developer.apple.com → Membership) |
| `APPLE_APP_PASSWORD` | das Passwort aus Schritt 4 |

Ab dem nächsten Build wird die App automatisch signiert und notarisiert.
Diese Daten bitte **nie** in den Chat oder in Dateien kopieren, nur in GitHub-Secrets.
