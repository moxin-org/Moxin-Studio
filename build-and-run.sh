#!/bin/bash
# Build Moxin Studio from source and update the .app bundle
set -e

cd "$(dirname "$0")"

echo "[build] Compiling Moxin Studio (release)..."
cargo build -p moly-shell --release

# Copy the freshly built binary into the .app bundle
APP_BINARY="Moxin Studio.app/Contents/MacOS/moxin-studio"
CARGO_BINARY="target/release/moxin-studio"

if [ -f "$CARGO_BINARY" ]; then
    cp "$CARGO_BINARY" "$APP_BINARY"
    echo "[build] Updated .app bundle with new binary"
else
    echo "[build] ERROR: $CARGO_BINARY not found"
    exit 1
fi

echo "[build] Launching Moxin Studio..."
open "Moxin Studio.app"
