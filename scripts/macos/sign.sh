#!/bin/bash

# Sign macOS app bundle with ad-hoc signature and necessary entitlements.

set -e

APP_PATH="$1"
ENTITLEMENTS="entitlements.plist"

echo "Creating enhanced entitlements..."
cat > "$ENTITLEMENTS" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
    <key>com.apple.security.cs.allow-dyld-environment-variables</key>
    <true/>
    <key>com.apple.security.cs.disable-executable-page-protection</key>
    <true/>
    <key>com.apple.security.get-task-allow</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.network.server</key>
    <true/>
    <key>com.apple.security.files.user-selected.read-write</key>
    <true/>
    <key>com.apple.security.files.downloads.read-write</key>
    <true/>
</dict>
</plist>
EOF

echo "Signing application with enhanced options..."

# Try multiple signing approaches for maximum compatibility
echo "Attempt 1: Full signing with runtime and entitlements"
if codesign --force --deep --sign - --entitlements "$ENTITLEMENTS" --options runtime --timestamp=none "$APP_PATH" 2>/dev/null; then
    echo "Signing successful with runtime and entitlements..."
elif codesign --force --deep --sign - --entitlements "$ENTITLEMENTS" "$APP_PATH" 2>/dev/null; then
    echo "Signing successful with entitlements only..."
elif codesign --force --deep --sign - --options runtime "$APP_PATH" 2>/dev/null; then
    echo "Signing successful with runtime only..."
elif codesign --force --deep --sign - "$APP_PATH" 2>/dev/null; then
    echo "Basic signing successful..."
else
    echo "All signing attempts failed, continuing without signature"
fi

# Verify the signature
echo "Verifying signature..."
if codesign --verify --verbose "$APP_PATH" 2>/dev/null; then
    echo "Signature verification successful"
else
    echo "Signature verification failed"
fi

# Display signature info
echo "Signature information:"
codesign --display --verbose "$APP_PATH" 2>&1 || echo "Cannot display signature info"

echo "Code signing completed"
