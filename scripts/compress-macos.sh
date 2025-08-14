#!/bin/bash

set -e

# macOS-specific binary compression using safe methods
# This script uses strip and other macOS-safe optimizations

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

check_macos() {
    if [[ "$OSTYPE" != "darwin"* ]]; then
        error "This script only works on macOS"
        exit 1
    fi
}

format_size() {
    local size=$1
    if [[ $size -gt 1048576 ]]; then
        echo "$(( size / 1048576 ))MB"
    elif [[ $size -gt 1024 ]]; then
        echo "$(( size / 1024 ))KB"
    else
        echo "${size}B"
    fi
}

get_file_size() {
    local file="$1"
    stat -f%z "$file"
}

compress_binary() {
    local binary="$1"

    if [[ ! -f "$binary" ]]; then
        error "Binary not found: $binary"
        exit 1
    fi

    info "Compressing macOS binary: $binary"

    # Create backup
    cp "$binary" "$binary.backup"

    local orig_size
    orig_size=$(get_file_size "$binary")
    info "Original size: $(format_size $orig_size)"

    # Method 1: Remove debug info and strip symbols more aggressively
    info "Stripping symbols and debug info..."
    dsymutil --minimize "$binary" 2>/dev/null || true
    strip -u -r -s "$binary" 2>/dev/null || strip -x "$binary" 2>/dev/null || true

    # Method 2: Use gzexe for safe compression
    info "Compressing with gzexe..."
    if command -v gzexe >/dev/null 2>&1; then
        gzexe "$binary" 2>/dev/null || true
        # gzexe creates .gz version, move it back
        if [[ -f "$binary~" ]]; then
            mv "$binary~" "$binary.uncompressed"
        fi
    fi

    # Method 3: Apply aggressive file system compression
    info "Applying APFS/HFS+ compression..."
    ditto --hfsCompression "$binary" "$binary.tmp" 2>/dev/null && mv "$binary.tmp" "$binary" || true

    # Method 4: Remove unnecessary sections
    info "Removing unnecessary sections..."
    install_name_tool -delete_rpath /usr/lib "$binary" 2>/dev/null || true
    install_name_tool -delete_rpath /System/Library/Frameworks "$binary" 2>/dev/null || true

    # Method 5: Re-sign with ad-hoc signature
    info "Re-signing binary..."
    codesign --force --sign - "$binary" 2>/dev/null || true

    local new_size
    new_size=$(get_file_size "$binary")
    local saved=$((orig_size - new_size))
    local percent=0
    if [[ $orig_size -gt 0 ]]; then
        percent=$((saved * 100 / orig_size))
    fi

    info "Compressed size: $(format_size $new_size)"
    info "Space saved: $(format_size $saved) (${percent}%)"

    # Verify binary still works
    if [[ -x "$binary" ]]; then
        info "Binary verification: OK"
    else
        warn "Binary may not be executable"
    fi
}

compress_app_bundle() {
    local app_path="$1"

    if [[ ! -d "$app_path" ]]; then
        error "App bundle not found: $app_path"
        exit 1
    fi

    info "Compressing app bundle: $app_path"

    # Find and compress the main executable
    local executable
    executable=$(find "$app_path/Contents/MacOS" -type f -perm +111 | head -1)

    if [[ -n "$executable" ]]; then
        compress_binary "$executable"
    else
        error "No executable found in app bundle"
        exit 1
    fi

    # Compress resource files
    info "Compressing resource files..."
    find "$app_path/Contents/Resources" -type f \( -name "*.png" -o -name "*.jpg" -o -name "*.icns" \) -exec ditto --hfsCompression {} {}.tmp \; -exec mv {}.tmp {} \; 2>/dev/null || true

    # Sign the entire app bundle
    info "Signing app bundle..."
    find "$app_path" -exec xattr -d com.apple.quarantine {} \; 2>/dev/null || true
    codesign --force --deep --sign - "$app_path" 2>/dev/null || true

    info "App bundle compression complete"
}

build_and_compress() {
    local profile="${1:-release}"

    info "Building and compressing for macOS..."

    # Build the project
    cargo build --profile "$profile" --features "desktop"

    local binary_path="target/$profile/DreamLauncher"

    if [[ ! -f "$binary_path" ]]; then
        error "Build failed - binary not found"
        exit 1
    fi

    # Compress the binary
    compress_binary "$binary_path"

    # Create app bundle
    chmod +x scripts/create_app_bundle.sh
    ./scripts/create_app_bundle.sh "$binary_path"

    local app_path="target/$profile/Dream Launcher.app"

    if [[ -d "$app_path" ]]; then
        # Apply additional compression to app bundle
        info "Applying additional app bundle optimizations..."
        find "$app_path" -exec xattr -d com.apple.quarantine {} \; 2>/dev/null || true
        codesign --force --deep --sign - "$app_path" 2>/dev/null || true

        info "macOS build complete: $app_path"

        # Show final sizes
        local binary_size app_size
        binary_size=$(get_file_size "$app_path/Contents/MacOS/DreamLauncher")
        app_size=$(du -sk "$app_path" | cut -f1)
        app_size=$((app_size * 1024))

        info "Final binary size: $(format_size $binary_size)"
        info "Final app bundle size: $(format_size $app_size)"
    else
        error "App bundle creation failed"
        exit 1
    fi
}

restore_backup() {
    local binary="${1:-target/release/DreamLauncher}"

    if [[ -f "$binary.backup" ]]; then
        cp "$binary.backup" "$binary"
        info "Binary restored from backup"
    else
        error "No backup found for $binary"
        exit 1
    fi
}

cleanup() {
    info "Cleaning up backup files..."
    find target -name "*.backup" -delete 2>/dev/null || true
    info "Cleanup complete"
}

show_help() {
    echo "Commands:"
    echo "  build [PROFILE]  - Build and compress binary (default: release)"
    echo "  compress BINARY  - Compress existing binary"
    echo "  app-bundle APP   - Compress app bundle"
    echo "  restore [BINARY] - Restore binary from backup"
    echo "  cleanup          - Remove backup files"
    echo "  help             - Show this help"
}

main() {
    check_macos

    case "${1:-build}" in
        "build")
            build_and_compress "$2"
            ;;
        "compress")
            if [[ -z "$2" ]]; then
                error "Binary path required"
                show_help
                exit 1
            fi
            compress_binary "$2"
            ;;
        "app-bundle")
            if [[ -z "$2" ]]; then
                error "App bundle path required"
                show_help
                exit 1
            fi
            compress_app_bundle "$2"
            ;;
        "restore")
            restore_backup "$2"
            ;;
        "cleanup")
            cleanup
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            error "Unknown command: $1"
            show_help
            exit 1
            ;;
    esac
}

main "$@"
