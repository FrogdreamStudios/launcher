#!/bin/bash

set -e

APP_BUNDLE="target/release/Dream Launcher.app"
DMG_NAME="DreamLauncher-macOS.dmg"
BACKGROUND_IMG="assets/images/other/drop_and_go.png"

# Remove existing DMG if it exists
rm -f "$DMG_NAME"

create-dmg \
  --volname "Dream Launcher" \
  --window-pos 200 200 \
  --window-size 800 400 \
  --icon-size 128 \
  --background "$BACKGROUND_IMG" \
  --background-color "#141414" \
  --icon "Dream Launcher.app" 158 278 \
  --app-drop-link 578 278 \
  "$DMG_NAME" \
  "$APP_BUNDLE"
