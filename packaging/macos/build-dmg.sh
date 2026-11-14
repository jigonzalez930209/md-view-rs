#!/usr/bin/env bash
set -euo pipefail
# Run on macOS with the release binary already built for both architectures.
VERSION="${VERSION:-0.11.1}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APP="$ROOT/dist/MD View.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$ROOT/target/universal-apple-darwin/release/mdview" "$APP/Contents/MacOS/mdview"
cp "$ROOT/packaging/macos/Info.plist" "$APP/Contents/Info.plist"
cp "$ROOT/assets/icons/mdview.icns" "$APP/Contents/Resources/mdview.icns"
codesign --force --deep --options runtime --entitlements "$ROOT/packaging/macos/entitlements.plist" --sign "${CODESIGN_IDENTITY:?set CODESIGN_IDENTITY}" "$APP"
hdiutil create -volname "MD View" -srcfolder "$APP" -ov -format UDZO "$ROOT/dist/MD-View-$VERSION.dmg"
if [ -n "${NOTARY_PROFILE:-}" ]; then
  xcrun notarytool submit "$ROOT/dist/MD-View-$VERSION.dmg" --keychain-profile "$NOTARY_PROFILE" --wait
  xcrun stapler staple "$ROOT/dist/MD-View-$VERSION.dmg"
fi
