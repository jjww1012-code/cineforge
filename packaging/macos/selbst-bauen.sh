#!/usr/bin/env bash
# CineForge auf dem eigenen Mac bauen und installieren – ohne GitHub-Builds.
#
# Aufruf (im Terminal, im entpackten Projektordner):
#   bash packaging/macos/selbst-bauen.sh
#
# Was passiert:
#   1. Apples Entwickler-Werkzeuge prüfen (werden bei Bedarf angeboten)
#   2. Rust installieren bzw. aktualisieren (einmalig, ca. 2 Minuten)
#   3. CineForge bauen (beim ersten Mal ca. 10–20 Minuten, danach schneller)
#   4. App-Paket mit ffmpeg erstellen und nach /Programme kopieren, dann starten
set -euo pipefail

cd "$(dirname "$0")/../.."
ROOT="$(pwd)"

schritt() { printf '\n\033[1;36m▶ %s\033[0m\n' "$1"; }
fehler() { printf '\n\033[1;31m✗ %s\033[0m\n' "$1" >&2; exit 1; }

[ "$(uname -s)" = "Darwin" ] || fehler "Dieses Skript ist für macOS."
[ -f Cargo.toml ] && [ -d apps/studio ] || fehler "Bitte im CineForge-Projektordner ausführen."

# Eigener Zwischenspeicher (auch für temporäre Dateien – der System-Temp-Ordner machte Probleme)
CACHE="$HOME/Library/Caches/CineForge-Bau"
mkdir -p "$CACHE/tmp"
export TMPDIR="$CACHE/tmp"

# Genug Platz? Rust + Bau brauchen zusammen etwa 8 GB.
FREI_GB=$(df -g "$HOME" | awk 'NR==2 {print $4}')
echo "Freier Speicher: ${FREI_GB} GB"
if [ "${FREI_GB:-0}" -lt 8 ]; then
  fehler "Zu wenig Speicherplatz (${FREI_GB} GB frei, gebraucht werden etwa 8 GB). Bitte Platz schaffen – z. B. Papierkorb leeren, große Downloads löschen (Systemeinstellungen → Allgemein → Speicher) – und dann die Zeile noch einmal ausführen."
fi

schritt "1/4 Apples Entwickler-Werkzeuge"
if ! xcode-select -p >/dev/null 2>&1; then
  echo "Es öffnet sich gleich ein Fenster von Apple – bitte auf „Installieren“ klicken."
  echo "Wenn die Installation fertig ist, dieses Skript einfach noch einmal starten."
  xcode-select --install || true
  exit 0
fi
echo "✓ vorhanden"

schritt "2/4 Rust (Programmiersprache von CineForge)"
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
if ! command -v rustup >/dev/null 2>&1; then
  echo "Rust wird installiert (einmalig, etwa 1–2 Minuten) …"
  # Installer direkt in unseren Zwischenspeicher laden (nicht in den System-Temp-Ordner)
  curl --proto '=https' --tlsv1.2 -fL --retry 3 -o "$CACHE/rustup-init" \
    "https://static.rust-lang.org/rustup/dist/$(uname -m | sed 's/arm64/aarch64/')-apple-darwin/rustup-init" \
    || fehler "Rust konnte nicht heruntergeladen werden. Internetverbindung und freien Speicher prüfen, dann erneut versuchen."
  chmod +x "$CACHE/rustup-init"
  "$CACHE/rustup-init" -y --profile minimal --no-modify-path >/dev/null || fehler "Rust-Installation fehlgeschlagen."
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
rustup update stable --no-self-update >/dev/null
rustup default stable >/dev/null
echo "✓ $(rustc --version)"

schritt "3/4 CineForge bauen (beim ersten Mal 10–20 Minuten – Zeit für einen Kaffee)"
# Fester Zwischenspeicher: nach einem Update (neue ZIP) wird nur Geändertes neu gebaut
export CARGO_TARGET_DIR="$CACHE/target"
cargo build --release -p cineforge-studio

schritt "4/4 App-Paket erstellen und installieren"
mkdir -p dist
bash packaging/macos/make_app.sh "$CARGO_TARGET_DIR/release/cineforge" dist/CineForge.app >/dev/null
# ffmpeg für Video-Export (wird nur einmal geladen und dann wiederverwendet)
FF="$CACHE/ffmpeg"
if [ ! -x "$FF" ]; then
  echo "Lade ffmpeg (für den Video-Export) …"
  if curl -fL --retry 3 -o "$CACHE/ffmpeg.zip" "https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffmpeg.zip"; then
    rm -rf "$CACHE/ffbin" && unzip -q -o "$CACHE/ffmpeg.zip" -d "$CACHE/ffbin" && cp "$CACHE/ffbin/ffmpeg" "$FF" && chmod +x "$FF"
  else
    echo "⚠ ffmpeg konnte nicht geladen werden – CineForge läuft trotzdem, Video-Export später nachrüstbar."
  fi
fi
if [ -x "$FF" ]; then
  cp "$FF" dist/CineForge.app/Contents/Resources/ffmpeg
fi
codesign --force --deep --sign - dist/CineForge.app >/dev/null 2>&1 || true

# Laufende Version beenden und ersetzen
pkill -x cineforge >/dev/null 2>&1 || true
sleep 1
ZIEL="/Applications/CineForge.app"
if ! { rm -rf "$ZIEL" && cp -R dist/CineForge.app "$ZIEL"; } 2>/dev/null; then
  echo "Für den Ordner „Programme“ wird dein Mac-Passwort gebraucht (die Eingabe bleibt unsichtbar):"
  sudo rm -rf "$ZIEL"
  sudo cp -R dist/CineForge.app "$ZIEL"
fi
xattr -cr "$ZIEL" 2>/dev/null || true

VERSION=$(git -C "$ROOT" log -1 --format='%h vom %cd' --date=format:'%d.%m.%Y %H:%M' 2>/dev/null || grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
printf '\n\033[1;32m✓ Fertig! CineForge (%s) ist in „Programme“ installiert und startet jetzt.\033[0m\n' "$VERSION"
echo "Claude Desktop: in CineForge F2 drücken (falls noch nicht geschehen), dann Claude Desktop neu starten."
open "$ZIEL"
