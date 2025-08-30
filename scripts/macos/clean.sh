#!/bin/bash

# Clean macOS app bundle by removing extended attributes and setting permissions.

set -e

APP_PATH="$1"

if [[ -z "$APP_PATH" || ! -d "$APP_PATH" ]]; then
  echo "App bundle not found: $APP_PATH"
  exit 1
fi

find "$APP_PATH" -type f -exec xattr -c {} \; 2>/dev/null || true
find "$APP_PATH" -type f -exec xattr -d com.apple.quarantine {} \; 2>/dev/null || true

chmod -R 755 "$APP_PATH"
chmod +x "$APP_PATH/Contents/MacOS/Dream Launcher"
