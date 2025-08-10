#!/bin/bash

set -e

APP_NAME="Dream Launcher"
BUNDLE_ID="com.frogdream.dreamlauncher"
EXECUTABLE_NAME="DreamLauncher"
BUILD_TYPE="release"

[[ "$1" == "--debug" || "$1" == "-d" ]] && BUILD_TYPE="debug"

EXECUTABLE_PATH="target/$BUILD_TYPE/$EXECUTABLE_NAME"
APP_PATH="target/$BUILD_TYPE/$APP_NAME.app"

[[ ! -f "$EXECUTABLE_PATH" ]] && { echo "Executable not found: $EXECUTABLE_PATH"; exit 1; }

echo "Creating app bundle: $APP_PATH"

rm -rf "$APP_PATH"
mkdir -p "$APP_PATH/Contents/MacOS" "$APP_PATH/Contents/Resources"

cp "$EXECUTABLE_PATH" "$APP_PATH/Contents/MacOS/$EXECUTABLE_NAME"

cat > "$APP_PATH/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleExecutable</key>
    <string>$EXECUTABLE_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

if [[ -f "assets/icons/app_icon.icns" ]]; then
    cp "assets/icons/app_icon.icns" "$APP_PATH/Contents/Resources/"
    plutil -insert CFBundleIconFile -string "app_icon.icns" "$APP_PATH/Contents/Info.plist"
fi

echo "App bundle created: $APP_PATH"
