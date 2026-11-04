#!/usr/bin/env bash
# Signiert mit dem Developer-ID-Zertifikat und lässt die App von Apple notarisieren,
# damit sie auf jedem Mac ohne Warnung startet. Benötigt die GitHub-Secrets aus docs/RELEASE.md.
set -euo pipefail
APP="$1"
KEYCHAIN=build.keychain
echo "$APPLE_CERT_P12" | base64 --decode > cert.p12
security create-keychain -p ci "$KEYCHAIN"
security default-keychain -s "$KEYCHAIN"
security unlock-keychain -p ci "$KEYCHAIN"
security import cert.p12 -k "$KEYCHAIN" -P "$APPLE_CERT_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple: -s -k ci "$KEYCHAIN" >/dev/null
rm cert.p12
codesign --force --deep --options runtime --timestamp --sign "$APPLE_SIGN_IDENTITY" "$APP"
ditto -c -k --keepParent "$APP" notarize.zip
xcrun notarytool submit notarize.zip --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_APP_PASSWORD" --wait
xcrun stapler staple "$APP"
rm notarize.zip
