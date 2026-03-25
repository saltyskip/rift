#!/usr/bin/env bash
set -euo pipefail

echo "Building Rift iOS XCFramework..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

IOS_DEV=aarch64-apple-ios
IOS_SIM_ARM=aarch64-apple-ios-sim
MAC_ARM=aarch64-apple-darwin

# Clean previous build artifacts.
rm -rf dist/ios/headers dist/ios/Sources/RiftSDK dist/ios/rift_ffiFFI.xcframework dist/ios/RiftSDK.xcframework
mkdir -p dist/ios/Sources/RiftSDK dist/ios/headers/rift_ffiFFI

# 1. Build for all Apple targets.
echo "Building Rust for Apple targets..."
cargo build --release \
  --target "$IOS_DEV" \
  --target "$IOS_SIM_ARM" \
  --target "$MAC_ARM" \
  -p rift_mobile

# 2. Generate Swift bindings.
echo "Generating Swift bindings..."
SWIFT_GEN_DIR=target/uniffi/swift
mkdir -p "$SWIFT_GEN_DIR"

cargo run -p uniffi-bindgen-cli --release -- \
  generate \
  --language swift \
  --library "target/${IOS_DEV}/release/librift_mobile.a" \
  --out-dir "$SWIFT_GEN_DIR"

# Copy generated Swift source and headers.
# Headers go into a rift_ffiFFI/ subdirectory so that when Xcode
# flattens XCFramework headers into include/, they end up at
# include/rift_ffiFFI/module.modulemap instead of include/module.modulemap.
# This prevents collisions with other XCFramework packages.
cp "$SWIFT_GEN_DIR"/*.swift dist/ios/Sources/RiftSDK/
cp "$SWIFT_GEN_DIR"/*.h dist/ios/headers/rift_ffiFFI/

cat > dist/ios/headers/rift_ffiFFI/module.modulemap <<EOF
module rift_ffiFFI {
    header "rift_ffiFFI.h"
    export *
}
EOF

# 3. Build XCFramework.
# The parent directory (dist/ios/headers) is passed to -headers so the
# rift_ffiFFI/ subdirectory is preserved inside the XCFramework.
# The XCFramework is named rift_ffiFFI to match the module name and
# the binary target name in Package.swift (SPM requires all three to match).
echo "Creating XCFramework..."
xcodebuild -create-xcframework \
  -library "target/${IOS_DEV}/release/librift_mobile.a"     -headers dist/ios/headers \
  -library "target/${IOS_SIM_ARM}/release/librift_mobile.a"  -headers dist/ios/headers \
  -library "target/${MAC_ARM}/release/librift_mobile.a"      -headers dist/ios/headers \
  -output dist/ios/rift_ffiFFI.xcframework

echo "Done: dist/ios/rift_ffiFFI.xcframework"
