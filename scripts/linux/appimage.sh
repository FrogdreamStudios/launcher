#!/bin/bash

# Script to create AppImage for Dream Launcher.

set -e

# Check if FUSE is available.
if ! command -v fusermount &> /dev/null; then
    echo "Warning: fusermount not found"
fi

# Function to check and install Python.
check_python() {
    echo "Checking Python installation..."
    
    # Check if Python 3 is available
    if command -v python3 >/dev/null 2>&1; then
        PYTHON_VERSION=$(python3 --version 2>&1 | cut -d' ' -f2)
        echo "Python $PYTHON_VERSION found"
        return 0
    elif command -v python >/dev/null 2>&1; then
        PYTHON_VERSION=$(python --version 2>&1 | cut -d' ' -f2)
        if [[ "$PYTHON_VERSION" == 3.* ]]; then
            echo "Python $PYTHON_VERSION found"
            return 0
        fi
    fi
    
    echo "Python 3.x not found. Attempting to install..."
    
    # Try different package managers
    if command -v apt-get >/dev/null 2>&1; then
        echo "Installing Python via apt-get..."
        sudo apt-get update && sudo apt-get install -y python3 python3-pip
    elif command -v yum >/dev/null 2>&1; then
        echo "Installing Python via yum..."
        sudo yum install -y python3 python3-pip
    elif command -v dnf >/dev/null 2>&1; then
        echo "Installing Python via dnf..."
        sudo dnf install -y python3 python3-pip
    elif command -v pacman >/dev/null 2>&1; then
        echo "Installing Python via pacman..."
        sudo pacman -S --noconfirm python python-pip
    elif command -v zypper >/dev/null 2>&1; then
        echo "Installing Python via zypper..."
        sudo zypper install -y python3 python3-pip
    else
        echo "No supported package manager found. Please install Python manually: https://www.python.org/downloads/"
        return 1
    fi
    
    # Verify installation
    if command -v python3 >/dev/null 2>&1; then
        PYTHON_VERSION=$(python3 --version 2>&1 | cut -d' ' -f2)
        echo "Python $PYTHON_VERSION installed successfully"
        return 0
    else
        echo "Python installation failed"
        return 1
    fi
}

# Check Python before creating AppImage.
check_python

APP_NAME="Dream Launcher"
EXECUTABLE_NAME="DreamLauncher"
APP_DIR="DreamLauncher.AppDir"
DESKTOP_FILE="$APP_DIR/DreamLauncher.desktop"
ICON_FILE="$APP_DIR/DreamLauncher.png"

echo "Creating AppImage for $APP_NAME..."

# Clean up any existing AppDir.
rm -rf "$APP_DIR"

# Create AppDir structure.
mkdir -p "$APP_DIR/usr/bin"
mkdir -p "$APP_DIR/usr/share/applications"
mkdir -p "$APP_DIR/usr/share/icons/hicolor/256x256/apps"

# Copy the executable.
cp "target/release/$EXECUTABLE_NAME" "$APP_DIR/usr/bin/"

# Create desktop file.
cat > "$DESKTOP_FILE" << EOF
[Desktop Entry]
Type=Application
Name=$APP_NAME
Exec=$EXECUTABLE_NAME
Icon=DreamLauncher
Comment=A powerful and lightweight Minecraft launcher
Categories=Game;
Terminal=false
StartupWMClass=DreamLauncher
EOF

# Copy PNG icon from iconset.
if [[ -f "assets/icons/app_icon.iconset/icon_256x256.png" ]]; then
    cp "assets/icons/app_icon.iconset/icon_256x256.png" "$ICON_FILE"
    cp "assets/icons/app_icon.iconset/icon_256x256.png" "$APP_DIR/usr/share/icons/hicolor/256x256/apps/DreamLauncher.png"
    echo "Using 256x256 PNG icon from iconset"
elif [[ -f "assets/icons/app_icon.iconset/icon_512x512.png" ]]; then
    cp "assets/icons/app_icon.iconset/icon_512x512.png" "$ICON_FILE"
    cp "assets/icons/app_icon.iconset/icon_512x512.png" "$APP_DIR/usr/share/icons/hicolor/256x256/apps/DreamLauncher.png"
    echo "Using 512x512 PNG icon from iconset"
else
    echo "Warning: No suitable PNG icon found in iconset"
fi

# Create AppRun script.
cat > "$APP_DIR/AppRun" << 'EOF'
#!/bin/bash

# Get the directory where this AppImage is located
HERE="$(dirname "$(readlink -f "${0}")")" 

# Export library path
export LD_LIBRARY_PATH="$HERE/usr/lib:$LD_LIBRARY_PATH"

# Run the application
exec "$HERE/usr/bin/DreamLauncher" "$@"
EOF

chmod +x "$APP_DIR/AppRun"

# Note: desktop file is already created in the correct location.
# Icon is already copied to the correct locations.

# Download appimagetool if not available
if ! command -v appimagetool &> /dev/null; then
    echo "Downloading appimagetool..."
    if command -v wget &> /dev/null; then
        wget -O appimagetool "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
    elif command -v curl &> /dev/null; then
        curl -L -o appimagetool "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
    else
        echo "Error: Neither wget nor curl found. Cannot download appimagetool"
        exit 1
    fi
    chmod +x appimagetool
    APPIMAGETOOL="./appimagetool"
else
    APPIMAGETOOL="appimagetool"
fi

# Create AppImage.
echo "Creating AppImage..."
"$APPIMAGETOOL" "$APP_DIR" "Dream Launcher.AppImage"

# Clean up.
rm -rf "$APP_DIR"
if [[ -f "./appimagetool" ]]; then
    rm ./appimagetool
fi

echo "AppImage created successfully: Dream Launcher.AppImage"
