#!/bin/bash

# Check JSON integrity for Minecraft versions

if [ $# -eq 0 ]; then
    echo "Usage: $0 <minecraft_directory>"
    echo "Example: $0 ~/.minecraft"
    exit 1
fi

MINECRAFT_DIR="$1"
VERSIONS_DIR="$MINECRAFT_DIR/versions"

if [ ! -d "$VERSIONS_DIR" ]; then
    echo "Error: versions directory not found: $VERSIONS_DIR"
    exit 1
fi

total_checked=0
corrupted_count=0
corrupted_versions=()

# Check each version directory
for version_dir in "$VERSIONS_DIR"/*; do
    if [ -d "$version_dir" ]; then
        version_name=$(basename "$version_dir")
        json_file="$version_dir/$version_name.json"

        if [ -f "$json_file" ]; then
            echo "Checking version: $version_name"
            total_checked=$((total_checked + 1))

            # Check if file is empty
            if [ ! -s "$json_file" ]; then
                echo "JSON file is empty"
                corrupted_count=$((corrupted_count + 1))
                corrupted_versions+=("$version_name (empty file)")
                continue
            fi

            # Check JSON syntax using Python
            if command -v python3 >/dev/null 2>&1; then
                if python3 -c "import json; json.load(open('$json_file'))" 2>/dev/null; then
                    echo "JSON file is valid"
                else
                    echo "JSON file is corrupted (syntax error)"
                    corrupted_count=$((corrupted_count + 1))
                    corrupted_versions+=("$version_name (syntax error)")

                    # Show first few characters for diagnosis
                    echo "First 50 characters:"
                    head -c 50 "$json_file" | od -c
                fi
            elif command -v jq >/dev/null 2>&1; then
                if jq empty "$json_file" >/dev/null 2>&1; then
                    echo "JSON file is valid"
                else
                    echo "JSON file is corrupted"
                    corrupted_count=$((corrupted_count + 1))
                    corrupted_versions+=("$version_name (jq validation failed)")
                fi
            else
                echo "Cannot validate JSON (no python3 or jq available)"

                first_char=$(head -c 1 "$json_file")
                last_char=$(tail -c 2 "$json_file" | head -c 1)

                if [ "$first_char" = "{" ] && [ "$last_char" = "}" ]; then
                    echo "Basic structure check passed"
                else
                    echo "Basic structure check failed (first: '$first_char', last: '$last_char')"
                    corrupted_count=$((corrupted_count + 1))
                    corrupted_versions+=("$version_name (structure check failed)")
                fi
            fi
        else
            echo "Skipping $version_name: JSON file not found"
        fi
    fi
done

echo "Done!"
