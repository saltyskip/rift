#!/usr/bin/env bash
# Build the Rift Flutter SDK:
#   - Generates Dart bindings via flutter_rust_bridge_codegen
#   - Builds native static libs for iOS targets → XCFramework
#   - Builds native shared libs for Android targets
#   - Packages everything into dist/flutter/
set -euo pipefail
cd "$(dirname "$0")"

CRATE="rift_flutter_ffi"
DIST="dist/flutter"
HEADERS_DIR="$DIST/headers"
IOS_DIST="$DIST/ios"
ANDROID_DIST="$DIST/android"

rm -rf "$DIST"
mkdir -p "$HEADERS_DIR" "$IOS_DIST" "$ANDROID_DIST"

# ── Step 1: Generate Dart bindings ──────────────────────────────────────────

if ! command -v flutter_rust_bridge_codegen &>/dev/null; then
    echo "[Rift] Installing flutter_rust_bridge_codegen..."
    cargo install flutter_rust_bridge_codegen
fi

# frb codegen reads the Rust source and emits Dart + C header.
flutter_rust_bridge_codegen generate \
    --rust-input "crate::api" \
    --rust-root "flutter_ffi/" \
    --dart-output "$DIST/lib/src/rust/frb_generated.dart" \
    --c-output "$HEADERS_DIR/${CRATE}.h" \
    --no-web

echo "[Rift] Dart bindings generated → $DIST/lib/src/rust/"

# ── Step 2: Build iOS targets ────────────────────────────────────────────────

APPLE_TARGETS=(aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin)
for target in "${APPLE_TARGETS[@]}"; do
    echo "[Rift] Building $target..."
    cargo build --release --target "$target" -p "$CRATE"
done

# Bundle into XCFramework (device + simulator + macOS).
xcodebuild -create-xcframework \
    -library "target/aarch64-apple-ios/release/lib${CRATE}.a" \
        -headers "$HEADERS_DIR" \
    -library "target/aarch64-apple-ios-sim/release/lib${CRATE}.a" \
        -headers "$HEADERS_DIR" \
    -library "target/aarch64-apple-darwin/release/lib${CRATE}.a" \
        -headers "$HEADERS_DIR" \
    -output "$IOS_DIST/${CRATE}.xcframework"

echo "[Rift] XCFramework → $IOS_DIST/${CRATE}.xcframework"

# ── Step 3: Build Android targets ────────────────────────────────────────────

if ! command -v cargo-ndk &>/dev/null; then
    echo "[Rift] Installing cargo-ndk..."
    cargo install cargo-ndk
fi

declare -A ABI_MAP=(
    [aarch64-linux-android]="arm64-v8a"
    [armv7-linux-androideabi]="armeabi-v7a"
    [i686-linux-android]="x86"
    [x86_64-linux-android]="x86_64"
)

for target in "${!ABI_MAP[@]}"; do
    abi="${ABI_MAP[$target]}"
    echo "[Rift] Building $target ($abi)..."
    cargo ndk --target "$target" --platform 21 build --release -p "$CRATE"
    mkdir -p "$ANDROID_DIST/jniLibs/$abi"
    cp "target/$target/release/lib${CRATE}.so" "$ANDROID_DIST/jniLibs/$abi/"
done

echo "[Rift] Android libraries → $ANDROID_DIST/jniLibs/"

# ── Step 4: Assemble the plugin package ──────────────────────────────────────

# Copy hand-written Dart wrapper and plugin boilerplate.
# Preserve the frb-generated lib/src/rust/ directory.
cp -r flutter_dart_wrappers/lib/. "$DIST/lib/"
cp flutter_dart_wrappers/pubspec.yaml "$DIST/"
cp -r flutter_dart_wrappers/ios/Classes "$IOS_DIST/"
cp flutter_dart_wrappers/ios/rift_flutter_ffi.podspec "$IOS_DIST/"
# Place the XCFramework next to the podspec so CocoaPods can find it.
# (Already built above into $IOS_DIST.)

cp -r flutter_dart_wrappers/android/. "$ANDROID_DIST/"

echo "[Rift] Flutter SDK assembled → $DIST"
echo "       To use as a path dependency in pubspec.yaml:"
echo "         rift_flutter_ffi:"
echo "           path: ./path/to/$DIST"
