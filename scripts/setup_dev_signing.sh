#!/bin/bash

set -e

CERT_NAME="Dream Launcher Certificate"

if security find-certificate -c "$CERT_NAME" >/dev/null 2>&1; then
    echo "Certificate already exists"
    exit 0
fi

echo "Creating development certificate..."

security create-certificate \
    -c "$CERT_NAME" \
    -e 3650 \
    -k ~/Library/Keychains/login.keychain-db \
    -p codesigning \
    -P \
    -S "/CN=$CERT_NAME" 2>/dev/null || {
    echo "Using ad-hoc signing instead"
    exit 0
}

security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "" ~/Library/Keychains/login.keychain-db 2>/dev/null || true

echo "Certificate created successfully"
