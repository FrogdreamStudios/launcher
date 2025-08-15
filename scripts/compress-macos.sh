#!/bin/bash

# Compress macOS binaries and app bundles using APFS/HFS+ compression.

set -e

RED='\033[0;31m'
NC='\033[0m'

error() { echo -e "${RED}[ERROR]${NC} $1"; }

[[ "$OSTYPE" != "darwin"* ]] && error "This script only works on macOS" && exit 1

compress_binary() {
    local binary="$1"
    [[ ! -f "$binary" ]] && error "Binary not found: $binary" && exit 1
    cp "$binary" "$binary.backup"
    ditto --hfsCompression "$binary" "$binary.tmp" 2>/dev/null && mv "$binary.tmp" "$binary"
    [[ -x "$binary" ]] || error "Binary may not be executable"
}

compress_app_bundle() {
    local app_path="$1"
    [[ ! -d "$app_path" ]] && error "App bundle not found: $app_path" && exit 1
    local executable
    executable=$(find "$app_path/Contents/MacOS" -type f -perm +111 | head -1)
    [[ -n "$executable" ]] && compress_binary "$executable" || { error "No executable found"; exit 1; }
    find "$app_path/Contents/Resources" -type f \( -name "*.png" -o -name "*.jpg" -o -name "*.icns" \) \
        -exec ditto --hfsCompression {} {}.tmp \; -exec mv {}.tmp {} \; 2>/dev/null
}

restore_backup() {
    local binary="${1:-target/release/DreamLauncher}"
    [[ -f "$binary.backup" ]] && cp "$binary.backup" "$binary" || { error "No backup found"; exit 1; }
}

cleanup() {
    find . -name "*.backup" -delete 2>/dev/null
}

case "$1" in
    --restore) restore_backup "$2" ;;
    --cleanup) cleanup ;;
    *.app) compress_app_bundle "$1" ;;
    *) compress_binary "$1" ;;
esac
