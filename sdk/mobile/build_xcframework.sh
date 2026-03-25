#!/usr/bin/env bash
set -euo pipefail

echo "Building Rift iOS XCFramework..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

IOS_DEV=aarch64-apple-ios
IOS_SIM_ARM=aarch64-apple-ios-sim
MAC_ARM=aarch64-apple-darwin

# Clean previous build artifacts.
rm -rf dist/ios/headers dist/ios/Sources/RiftSDK dist/ios/RiftSDK.xcframework
mkdir -p dist/ios/Sources/RiftSDK dist/ios/headers

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
cp "$SWIFT_GEN_DIR"/*.swift dist/ios/Sources/RiftSDK/
cp "$SWIFT_GEN_DIR"/*.h dist/ios/headers/

cat > dist/ios/headers/module.modulemap <<EOF
module rift_ffiFFI {
    header "rift_ffiFFI.h"
    export *
}
EOF

# 3. Build XCFramework.
echo "Creating XCFramework..."
xcodebuild -create-xcframework \
  -library "target/${IOS_DEV}/release/librift_mobile.a"     -headers dist/ios/headers \
  -library "target/${IOS_SIM_ARM}/release/librift_mobile.a"  -headers dist/ios/headers \
  -library "target/${MAC_ARM}/release/librift_mobile.a"      -headers dist/ios/headers \
  -output dist/ios/RiftSDK.xcframework

# 4. Post-process: move modulemap from Headers/ to Modules/ in each slice.
# This prevents "Multiple commands produce module.modulemap" collisions
# when the consumer app has multiple XCFramework binary packages.
# See: https://github.com/software-mansion/starknet.swift/pull/249
echo "Relocating modulemap to Modules/ for collision avoidance..."
for slice in dist/ios/RiftSDK.xcframework/*/; do
  if [ -d "$slice/Headers" ] && [ -f "$slice/Headers/module.modulemap" ]; then
    mkdir -p "$slice/Modules"
    mv "$slice/Headers/module.modulemap" "$slice/Modules/module.modulemap"
    # Update header path to be relative from Modules/ to Headers/.
    sed -i '' 's|header "rift_ffiFFI.h"|header "../Headers/rift_ffiFFI.h"|' "$slice/Modules/module.modulemap"
  fi
done

echo "Done: dist/ios/RiftSDK.xcframework"
