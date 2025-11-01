#!/bin/bash

# Music Player Release Script
# Builds optimized binaries for multiple platforms

set -e

PROJECT_NAME="musicplayer"
VERSION=$(grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

echo "Building $PROJECT_NAME v$VERSION for multiple platforms..."

# Clean previous builds
cargo clean

# Build for current platform (Linux x86_64)
echo "Building for Linux x86_64..."
cargo build --release
mkdir -p releases
cp target/release/$PROJECT_NAME releases/${PROJECT_NAME}-linux-x86_64

# Build for Windows (requires cross-compilation setup)
# Note: This requires rustup target add x86_64-pc-windows-gnu
# and mingw-w64 installed on Linux
if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    echo "Building for Windows x86_64..."
    cargo build --release --target x86_64-pc-windows-gnu
    cp target/x86_64-pc-windows-gnu/release/${PROJECT_NAME}.exe releases/${PROJECT_NAME}-windows-x86_64.exe
else
    echo "Skipping Windows build (mingw not found)"
fi

# Build for macOS (requires cross-compilation setup)
# Note: This requires rustup target add x86_64-apple-darwin
# and osxcross or similar setup
if rustup target list | grep -q "x86_64-apple-darwin.*installed"; then
    echo "Building for macOS x86_64..."
    cargo build --release --target x86_64-apple-darwin
    cp target/x86_64-apple-darwin/release/$PROJECT_NAME releases/${PROJECT_NAME}-macos-x86_64
else
    echo "Skipping macOS build (target not installed)"
fi

echo "Release builds completed!"
echo "Binaries available in releases/ directory:"
ls -la releases/