#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT_DIR/target/release/bundle/macos/WAPC.app"
MACOS_DIR="$APP_DIR/Contents/MacOS"
RESOURCES_DIR="$APP_DIR/Contents/Resources"
VERSION="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')"

cargo build --release --bin wapc-desktop

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"
cp "$ROOT_DIR/target/release/wapc-desktop" "$MACOS_DIR/WAPC"

cat > "$APP_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>WAPC</string>
  <key>CFBundleIdentifier</key>
  <string>com.wapc.desktop</string>
  <key>CFBundleName</key>
  <string>WAPC</string>
  <key>CFBundleDisplayName</key>
  <string>WAPC</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>__WAPC_VERSION__</string>
  <key>CFBundleVersion</key>
  <string>__WAPC_VERSION__</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST
python3 - "$APP_DIR/Contents/Info.plist" "$VERSION" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
version = sys.argv[2]
path.write_text(path.read_text().replace("__WAPC_VERSION__", version))
PY

if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "$APP_DIR" >/dev/null
fi

echo "$APP_DIR"
