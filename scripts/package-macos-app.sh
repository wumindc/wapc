#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

(
  cd "$ROOT_DIR/src-tauri"
  cargo tauri build "$@"
)

APP_DIR="$ROOT_DIR/target/release/bundle/macos/WAPC.app"
if [[ ! -d "$APP_DIR" ]]; then
  echo "Expected Tauri to produce $APP_DIR, but it was not found." >&2
  exit 1
fi

echo "$APP_DIR"
