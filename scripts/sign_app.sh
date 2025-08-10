#!/bin/bash

set -e

detect_app() {
    for path in "target/release/Dream Launcher.app" "target/release/DreamLauncher" "target/debug/Dream Launcher.app" "target/debug/DreamLauncher"; do
        [[ -e "$path" ]] && echo "$path" && return
    done
}

SIGN_TARGET="${1:-$(detect_app)}"

[[ -z "$SIGN_TARGET" ]] && { echo "No app found. Build first."; exit 1; }

echo "Signing: $SIGN_TARGET"

if [[ -d "$SIGN_TARGET" ]]; then
    find "$SIGN_TARGET" -exec xattr -d com.apple.quarantine {} \; 2>/dev/null || true
else
    xattr -d com.apple.quarantine "$SIGN_TARGET" 2>/dev/null || true
fi

if security find-certificate -c "Dream Launcher Certificate" >/dev/null 2>&1; then
    codesign --force --deep --sign "Dream Launcher Certificate" "$SIGN_TARGET" 2>/dev/null || \
    codesign --force --deep --sign - "$SIGN_TARGET"
else
    codesign --force --deep --sign - "$SIGN_TARGET"
fi

echo "Signed successfully"
