#!/bin/bash

# Create a macOS app bundle for the Dream Launcher application.

set -e

APP_NAME="Dream Launcher"
BUNDLE_ID="com.frogdream.dreamlauncher"
EXECUTABLE_NAME="DreamLauncher"
BUILD_TYPE="release"

# Check if custom binary path is provided.
if [[ -n "$1" && "$1" != "--debug" && "$1" != "-d" ]]; then
    EXECUTABLE_PATH="$1"
    # Extract build type from path for app bundle location
    if [[ "$EXECUTABLE_PATH" == *"/debug/"* ]]; then
        BUILD_TYPE="debug"
    elif [[ "$EXECUTABLE_PATH" == *"/release/"* ]]; then
        BUILD_TYPE="release"
    else
        # For cross-compilation targets, use the target directory
        BUILD_TYPE=$(dirname "$EXECUTABLE_PATH" | sed 's|target/||' | sed 's|/[^/]*$||')
        if [[ -z "$BUILD_TYPE" ]]; then
            BUILD_TYPE="release"
        fi
    fi
else
    [[ "$1" == "--debug" || "$1" == "-d" ]] && BUILD_TYPE="debug"
    EXECUTABLE_PATH="target/$BUILD_TYPE/$EXECUTABLE_NAME"
fi

# Set app bundle path based on executable path.
if [[ "$EXECUTABLE_PATH" == target/*/release/* || "$EXECUTABLE_PATH" == target/*/debug/* ]]; then
    # Cross-compilation target
    TARGET_DIR=$(dirname "$EXECUTABLE_PATH")
    APP_PATH="$TARGET_DIR/$APP_NAME.app"
else
    APP_PATH="target/$BUILD_TYPE/$APP_NAME.app"
fi

[[ ! -f "$EXECUTABLE_PATH" ]] && exit 1

# Function to check and install Python
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
    
    echo "Python 3.x not found. Checking if Homebrew is available..."
    
    # Check if Homebrew is installed
    if command -v brew >/dev/null 2>&1; then
        echo "Installing Python via Homebrew..."
        brew install python3
        if [ $? -eq 0 ]; then
            echo "Python installation completed"
            return 0
        else
            echo "Failed to install Python via Homebrew"
        fi
    else
        echo "Homebrew not found. Please install Python manually:"
        echo "1. Install Homebrew: /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
        echo "2. Install Python: brew install python3"
        echo "Or download Python from: https://www.python.org/downloads/macos/"
    fi
    
    return 1
}

# Check Python before creating bundle
check_python

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

# Note: App icon is now embedded in the executable, no need to copy external files
