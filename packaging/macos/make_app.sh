#!/usr/bin/env bash
# Baut aus der fertigen Binärdatei ein doppelklickbares macOS-App-Paket.
set -euo pipefail
BIN="$1"; APP="$2"
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/cineforge"
sed "s/__VERSION__/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"
# Ad-hoc-Signatur (Pflicht auf Apple Silicon); echte Signatur folgt ggf. separat.
codesign --force --deep --sign - "$APP"
echo "Fertig: $APP"
