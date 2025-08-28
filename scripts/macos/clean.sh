#!/bin/bash

# Clean macOS app bundle by removing extended attributes and setting permissions.

set -e

APP_PATH="$1"

if [[ -z "$APP_PATH" || ! -d "$APP_PATH" ]]; then
  echo "App bundle not found: $APP_PATH"
  exit 1
fi

echo "Starting comprehensive macOS security bypass for: $APP_PATH"

# Remove all extended attributes first
echo "Removing all extended attributes..."
find "$APP_PATH" -exec xattr -c {} \; 2>/dev/null || true

# Remove specific quarantine attributes with multiple methods
echo "Removing quarantine attributes (method 1)..."
find "$APP_PATH" -exec xattr -d com.apple.quarantine {} \; 2>/dev/null || true

echo "Removing quarantine attributes (method 2)..."
xattr -r -d com.apple.quarantine "$APP_PATH" 2>/dev/null || true

# Remove additional restrictive attributes
echo "Removing additional metadata attributes..."
find "$APP_PATH" -exec xattr -d com.apple.metadata:kMDItemWhereFroms {} \; 2>/dev/null || true
find "$APP_PATH" -exec xattr -d com.apple.metadata:_kMDItemUserTags {} \; 2>/dev/null || true
find "$APP_PATH" -exec xattr -d com.apple.FinderInfo {} \; 2>/dev/null || true

# Remove system-level quarantine
echo "Deep quarantine removal..."
xattr -r -d com.apple.metadata:kMDItemWhereFroms "$APP_PATH" 2>/dev/null || true
xattr -r -d com.apple.metadata:_kMDItemUserTags "$APP_PATH" 2>/dev/null || true

# Set proper permissions
echo "Setting permissions..."
chmod -R 755 "$APP_PATH"
chmod +x "$APP_PATH/Contents/MacOS/DreamLauncher"

# Additional executable permissions for all binaries
find "$APP_PATH" -name "*.dylib" -exec chmod 755 {} \; 2>/dev/null || true
find "$APP_PATH" -name "*.so" -exec chmod 755 {} \; 2>/dev/null || true
find "$APP_PATH" -type f -perm -u=x -exec chmod 755 {} \; 2>/dev/null || true

# Clear any cached security decisions
echo "Clearing security cache..."
sudo -n spctl --master-disable 2>/dev/null || echo "Cannot disable spctl (no sudo)"

# Add to allowed applications if possible
# echo "Attempting to whitelist application..."
# spctl --add --label "DreamLauncher-Safe" "$APP_PATH" 2>/dev/null || echo "Cannot add to spctl whitelist"

echo "macOS security bypass completed for: $APP_PATH"
