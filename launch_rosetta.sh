#!/bin/bash

# Rosetta 2 Minecraft Launcher for Apple Silicon Macs

# Check if we're on Apple Silicon
if [ "$(uname -m)" != "arm64" ]; then
    echo "This script is only for Apple Silicon Macs. Your architecture: $(uname -m)"
    exit 1
fi

# Check if Rosetta 2 is installed
if ! /usr/bin/pgrep oahd >/dev/null 2>&1; then
    echo "Rosetta 2 is not installed or not running"
    echo "Install Rosetta 2 with: softwareupdate --install-rosetta --agree-to-license"
    exit 1
fi

echo "Rosetta 2 is available"

# Build the launcher if needed
if [ ! -f "target/release/launcher" ]; then
    echo "Building launcher..."
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "Build failed"
        exit 1
    fi
fi

# Check for offline versions
MINECRAFT_DIR="$HOME/Library/Application Support/minecraft"
VERSIONS_DIR="$MINECRAFT_DIR/versions"

if [ ! -d "$VERSIONS_DIR" ]; then
    echo "Minecraft directory not found: $VERSIONS_DIR"
    echo "Please run the official Minecraft Launcher first to download versions."
    exit 1
fi

# List of available versions
echo "Scanning for versions..."
AVAILABLE_VERSIONS=()
for version_dir in "$VERSIONS_DIR"/*; do
    if [ -d "$version_dir" ]; then
        VERSION_NAME=$(basename "$version_dir")
        JAR_FILE="$version_dir/$VERSION_NAME.jar"
        JSON_FILE="$version_dir/$VERSION_NAME.json"

        if [ -f "$JAR_FILE" ] && [ -f "$JSON_FILE" ]; then
            AVAILABLE_VERSIONS+=("$VERSION_NAME")
        fi
    fi
done

if [ ${#AVAILABLE_VERSIONS[@]} -eq 0 ]; then
    echo "No versions found"
    echo "Run the official Minecraft Launcher first to download versions"
    exit 1
fi

echo "Found ${#AVAILABLE_VERSIONS[@]} version(s):"
for i in "${!AVAILABLE_VERSIONS[@]}"; do
    echo "  $((i + 1)). ${AVAILABLE_VERSIONS[i]}"
done
echo

# If version specified as argument, use it
if [ $# -gt 0 ]; then
    SELECTED_VERSION="$1"
    echo "Launching specified version: $SELECTED_VERSION"
else
    # Interactive selection
    echo "Select version to launch under Rosetta 2:"
    read -p "Enter number (1-${#AVAILABLE_VERSIONS[@]}): " SELECTION

    if ! [[ "$SELECTION" =~ ^[0-9]+$ ]] || [ "$SELECTION" -lt 1 ] || [ "$SELECTION" -gt ${#AVAILABLE_VERSIONS[@]} ]; then
        echo "Invalid selection"
        exit 1
    fi

    SELECTED_VERSION="${AVAILABLE_VERSIONS[$((SELECTION - 1))]}"
    echo "Launching: $SELECTED_VERSION"
fi

# Check if selected version exists
VERSION_EXISTS=false
for version in "${AVAILABLE_VERSIONS[@]}"; do
    if [ "$version" = "$SELECTED_VERSION" ]; then
        VERSION_EXISTS=true
        break
    fi
done

if [ "$VERSION_EXISTS" = false ]; then
    echo "Version $SELECTED_VERSION not found"
    echo "Available versions: ${AVAILABLE_VERSIONS[*]}"
    exit 1
fi

# Check if this version needs Intel Java
NEEDS_INTEL_JAVA=false
if [[ "$SELECTED_VERSION" < "1.17" ]]; then
    NEEDS_INTEL_JAVA=true
    echo "Warning! Version $SELECTED_VERSION requires Intel Java for native library compatibility"
fi

# Set environment for Rosetta 2
export ARCHPREFERENCE="x86_64"

# Check for Intel Java
INTEL_JAVA_PATH=""
POTENTIAL_JAVA_PATHS=(
    "/usr/libexec/java_home -a x86_64"
    "/Library/Java/JavaVirtualMachines/*/Contents/Home/bin/java"
    "/System/Library/Java/JavaVirtualMachines/*/Contents/Home/bin/java"
)

# Try to find Intel Java
for path_cmd in "${POTENTIAL_JAVA_PATHS[@]}"; do
    if [[ "$path_cmd" == *"java_home"* ]]; then
        JAVA_HOME_PATH=$(eval "$path_cmd" 2>/dev/null)
        if [ -n "$JAVA_HOME_PATH" ] && [ -f "$JAVA_HOME_PATH/bin/java" ]; then
            INTEL_JAVA_PATH="$JAVA_HOME_PATH/bin/java"
            break
        fi
    else
        for java_path in $path_cmd; do
            if [ -f "$java_path" ]; then
                # Check if it's Intel architecture
                if file "$java_path" | grep -q "x86_64\|i386"; then
                    INTEL_JAVA_PATH="$java_path"
                    break 2
                fi
            fi
        done
    fi
done

if [ -n "$INTEL_JAVA_PATH" ]; then
    echo "Found Intel Java: $INTEL_JAVA_PATH"
    export JAVA_HOME=$(dirname $(dirname "$INTEL_JAVA_PATH"))
    export PATH="$JAVA_HOME/bin:$PATH"
elif [ "$NEEDS_INTEL_JAVA" = true ]; then
    echo "Intel Java not found, trying with system Java under Rosetta 2"
fi

# Create a wrapper script that forces x86_64 architecture
WRAPPER_SCRIPT="/tmp/minecraft_rosetta_launcher.sh"
cat > "$WRAPPER_SCRIPT" << 'EOF'
#!/bin/bash
export ARCHPREFERENCE="x86_64"
exec arch -x86_64 "$@"
EOF
chmod +x "$WRAPPER_SCRIPT"

# Launch with our launcher under Rosetta 2
echo "Starting Minecraft under Rosetta 2..."
arch -x86_64 /bin/bash -c "
    export ARCHPREFERENCE=x86_64
    cd '$(pwd)'
    RUST_LOG=info ./target/release/launcher launch --version '$SELECTED_VERSION' --offline
"
LAUNCH_RESULT=$?

# Clean up
rm -f "$WRAPPER_SCRIPT"

# Check exit status
if [ $LAUNCH_RESULT -eq 0 ]; then
    echo "Minecraft launched successfully under Rosetta 2"
else
    echo "Minecraft launch failed under Rosetta 2"
fi