#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT_DIR/target/release/bundle/macos/WAPC.app"

"$ROOT_DIR/scripts/package-macos-app.sh" >/dev/null
rm -rf /Applications/WAPC.app
cp -R "$APP_DIR" /Applications/WAPC.app
echo "/Applications/WAPC.app"
