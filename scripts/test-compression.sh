#!/bin/bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { echo -e "${GREEN}$1${NC}"; }
warn()    { echo -e "${YELLOW}$1${NC}"; }
error()   { echo -e "${RED}$1${NC}"; }

format_size() {
    local size=$1
    awk -v s="$size" '{
        if (s > 1024*1024*1024) printf "%.2fGB", s/1024/1024/1024
        else if (s > 1024*1024) printf "%.2fMB", s/1024/1024
        else if (s > 1024) printf "%.2fKB", s/1024
        else printf "%dB", s
    }'
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
        upx $method "$testb" &>/dev/null && {
            local comp
            [[ "$OSTYPE" == "darwin"* ]] && comp=$(stat -f%z "$testb") || comp=$(stat -c%s "$testb")
            local ratio=$(echo "scale=1; $comp*100/$orig" | bc)
            local saved=$(echo "scale=1; ($orig-$comp)*100/$orig" | bc)
            printf "%-15s %-10s %-10s %-7s%% %-7s%%\n" "$desc" "$(format_size $orig)" "$(format_size $comp)" "$ratio" "$saved"
        } || printf "%-15s %-10s %-10s %-7s %-7s\n" "$desc" "$(format_size $orig)" "FAIL" "-" "-"
    done
}

verify_binary() {
    local bin="$1"
    [[ ! -f "$bin" ]] && error "Binary not found: $bin" && return 1
    [[ ! -x "$bin" ]] && chmod +x "$bin"
    "$bin" --help &>/dev/null && info "OK" && return 0
    return 1
}

apply_best_compression() {
    local bin="$1"
    [[ ! -f "$bin" ]] && error "Binary not found: $bin" && exit 1
    cp "$bin" "$bin.backup"
    upx --best --lzma "$bin" && info "Compressed: $bin" || { error "Failed"; cp "$bin.backup" "$bin"; exit 1; }
    verify_binary "$bin" || { warn "Verify failed, restoring"; cp "$bin.backup" "$bin"; }
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

command -v bc &>/dev/null || warn "bc not found, calculations may fail"
main "$@"
