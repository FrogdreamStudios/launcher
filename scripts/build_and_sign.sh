#!/bin/bash

set -e

BUILD_TYPE="release"
CREATE_DMG=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--debug)
            BUILD_TYPE="debug"
            shift
            ;;
        --dmg)
            CREATE_DMG=true
            shift
            ;;
        *)
            echo "Usage: $0 [-d|--debug] [--dmg]"
            exit 1
            ;;
    esac
done

echo "Building DreamLauncher ($BUILD_TYPE mode)..."

if [[ "$BUILD_TYPE" == "release" ]]; then
    cargo build --release
else
    cargo build
fi

echo "Creating app bundle..."
./scripts/create_app_bundle.sh $([[ "$BUILD_TYPE" == "debug" ]] && echo "--debug")

if [[ "$(uname)" != "Darwin" ]]; then
    echo "Build complete"
    exit 0
fi

SIGN_TARGET="target/$BUILD_TYPE/Dream Launcher.app"

echo "Signing: $SIGN_TARGET"
find "$SIGN_TARGET" -exec xattr -d com.apple.quarantine {} \; 2>/dev/null || true
codesign --force --deep --sign - "$SIGN_TARGET"

if [[ "$CREATE_DMG" == true ]] && [[ "$BUILD_TYPE" == "release" ]]; then
    echo "Creating DMG..."
    rm -f DreamLauncher-macOS.dmg

    create-dmg \
        --volname "Dream Launcher" \
        --window-pos 200 120 \
        --window-size 800 400 \
        --icon-size 100 \
        --icon "Dream Launcher.app" 200 190 \
        --hide-extension "Dream Launcher.app" \
        --app-drop-link 600 185 \
        "DreamLauncher-macOS.dmg" \
        "$SIGN_TARGET" 2>/dev/null || {
            echo "create-dmg failed, using hdiutil..."
            hdiutil create -volname "Dream Launcher" -srcfolder "$SIGN_TARGET" -ov -format UDZO "DreamLauncher-macOS.dmg"
        }

    codesign --force --sign - "DreamLauncher-macOS.dmg"
    echo "DMG created: DreamLauncher-macOS.dmg"
fi

echo "Build complete: $SIGN_TARGET"
