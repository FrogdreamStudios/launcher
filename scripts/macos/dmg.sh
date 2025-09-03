#!/bin/bash

set -e

APP_BUNDLE=$(find target/dx/DreamLauncher/bundle/macos/ -name "*.app" -type d | head -1)
if [ -z "$APP_BUNDLE" ]; then
    echo "Error: No .app bundle found"
    exit 1
fi

APP_NAME=$(basename "$APP_BUNDLE")
DMG_NAME="Dream Launcher.dmg"
BACKGROUND_IMG="assets/images/other/drop_and_go.png"

# Remove existing DMG if it exists
rm -f "$DMG_NAME"

create-dmg \
  --volname "Dream Launcher" \
  --window-pos 200 200 \
  --window-size 800 400 \
  --icon-size 64 \
  --background "$BACKGROUND_IMG" \
  --icon "$APP_NAME" 190 278 \
  --app-drop-link 610 278 \
  "$DMG_NAME" \
  "$APP_BUNDLE"
