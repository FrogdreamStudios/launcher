#!/bin/bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { echo "$1"; }
warn()    { echo "$1"; }
error()   { echo "$1"; }

format_size() {
    local size=$1
    if [[ $size -gt 1073741824 ]]; then
        echo "$(( size / 1073741824 ))GB"
    elif [[ $size -gt 1048576 ]]; then
        echo "$(( size / 1048576 ))MB"
    elif [[ $size -gt 1024 ]]; then
        echo "$(( size / 1024 ))KB"
    else
        echo "${size}B"
    fi
}

check_upx() {
    if ! command -v upx &>/dev/null; then
        error "UPX not installed"
        exit 1
    fi
}

build_binary() {
    local profile="${1:-release}"
    local target="$2"
    local path
    if [[ -n "$target" ]]; then
        cargo build --profile "$profile" --target "$target" --features "desktop" &>/dev/null
        [[ "$target" == *"windows"* ]] && path="target/$target/$profile/DreamLauncher.exe" || path="target/$target/$profile/DreamLauncher"
    else
        cargo build --profile "$profile" --features "desktop" &>/dev/null
        [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]] && path="target/$profile/DreamLauncher.exe" || path="target/$profile/DreamLauncher"
    fi
    echo "$path"
}

test_compression() {
    local bin="$1"
    local out="compression_test"
    [[ ! -f "$bin" ]] && error "Binary not found: $bin" && exit 1
    mkdir -p "$out" && rm -f "$out"/*
    local orig
    [[ "$OSTYPE" == "darwin"* ]] && orig=$(stat -f%z "$bin") || orig=$(stat -c%s "$bin")
    local methods=(
        "--fast:Fast"
        "--best:Best"
        "--ultra-brute:UltraBrute"
        "--best --lzma:BestLZMA"
        "--ultra-brute --lzma:UltraBruteLZMA"
    )
    printf "%-15s %-10s %-10s %-7s %-7s\n" "Method" "Orig" "Comp" "Ratio" "Saved"
    for m in "${methods[@]}"; do
        IFS=':' read -r method desc <<< "$m"
        local testb="$out/$(echo $desc | tr ' ' '_').bin"
        cp "$bin" "$testb"

        # Add platform-specific flags
        local upx_args=($method)
        if [[ "$OSTYPE" == "darwin"* ]]; then
            upx_args+=(--force-macos)
        fi

        upx "${upx_args[@]}" "$testb" >/dev/null 2>&1
        if [[ -f "$testb" ]]; then
            local comp
            [[ "$OSTYPE" == "darwin"* ]] && comp=$(stat -f%z "$testb") || comp=$(stat -c%s "$testb")
            if [[ $comp -lt $orig ]]; then
                local ratio=$(($comp * 100 / $orig))
                local saved=$(( ($orig - $comp) * 100 / $orig ))
                printf "%-15s %-10s %-10s %-7s%% %-7s%%\n" "$desc" "$(format_size $orig)" "$(format_size $comp)" "$ratio" "$saved"
            else
                printf "%-15s %-10s %-10s %-7s %-7s\n" "$desc" "$(format_size $orig)" "FAIL" "-" "-"
            fi
        else
            printf "%-15s %-10s %-10s %-7s %-7s\n" "$desc" "$(format_size $orig)" "FAIL" "-" "-"
        fi
    done
}

verify_binary() {
    local bin="$1"
    [[ ! -f "$bin" ]] && error "Binary not found: $bin" && return 1
    [[ ! -x "$bin" ]] && chmod +x "$bin"

    if [[ "$OSTYPE" == "darwin"* ]]; then
        local size
        size=$(stat -f%z "$bin")
        if [[ $size -gt 100000 ]]; then
            return 0
        else
            return 1
        fi
    else
        "$bin" --help &>/dev/null && return 0
        return 1
    fi
}

apply_best_compression() {
    local bin="$1"
    [[ ! -f "$bin" ]] && error "Binary not found: $bin" && exit 1
    cp "$bin" "$bin.backup"

    # Add platform-specific flags
    local upx_args=(--best --lzma)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        upx_args+=(--force-macos)
    fi

    upx "${upx_args[@]}" "$bin" >/dev/null 2>&1
    local new_size orig_size
    [[ "$OSTYPE" == "darwin"* ]] && new_size=$(stat -f%z "$bin") || new_size=$(stat -c%s "$bin")
    [[ "$OSTYPE" == "darwin"* ]] && orig_size=$(stat -f%z "$bin.backup") || orig_size=$(stat -c%s "$bin.backup")

    if [[ $new_size -lt $orig_size ]]; then
        info "Compression successful"
    else
        error "Compression failed"
        cp "$bin.backup" "$bin"
        exit 1
    fi
    verify_binary "$bin" || { warn "Verification failed, restoring backup"; cp "$bin.backup" "$bin"; }
}

cleanup() {
    rm -rf compression_test
    find target -name "*.backup" -delete 2>/dev/null || true
}

show_help() {
    echo "Usage: $0 [test [PROFILE] [TARGET] | compress BIN | verify BIN | cleanup | help]"
}

main() {
    check_upx
    case "${1:-test}" in
        test)
            local profile="${2:-release}" target="$3"
            bin=$(build_binary "$profile" "$target")
            test_compression "$bin"
            ;;
        compress)
            [[ -z "$2" ]] && error "Binary path required" && show_help && exit 1
            apply_best_compression "$2"
            ;;
        verify)
            [[ -z "$2" ]] && error "Binary path required" && show_help && exit 1
            verify_binary "$2"
            ;;
        cleanup) cleanup ;;
        help|--help|-h) show_help ;;
        *) error "Unknown command: $1"; show_help; exit 1 ;;
    esac
}

main "$@"
