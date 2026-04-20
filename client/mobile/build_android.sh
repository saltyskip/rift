#!/usr/bin/env bash
set -euo pipefail

echo "Building Rift Android SDK..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

DIST="$SCRIPT_DIR/dist/android"

# Install cargo-ndk if missing.
if ! command -v cargo-ndk &>/dev/null; then
    echo "Installing cargo-ndk..."
    cargo install cargo-ndk
fi

# Add Android targets.
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  i686-linux-android \
  x86_64-linux-android

# Build for all Android ABIs.
echo "Building Rust for Android targets..."
for abi in armeabi-v7a arm64-v8a x86 x86_64; do
    echo "  -> $abi"
    cargo ndk --manifest-path mobile/Cargo.toml -t "$abi" build --release
done

# Clean and prepare dist.
rm -rf "$DIST/jniLibs" "$DIST/kotlin"
mkdir -p "$DIST"

# Generate Kotlin bindings.
echo "Generating Kotlin bindings..."
KOTLIN_GEN_DIR=target/uniffi/kotlin
mkdir -p "$KOTLIN_GEN_DIR"

cargo run -p uniffi-bindgen-cli --release -- \
  generate \
  --language kotlin \
  --library target/aarch64-linux-android/release/librift_mobile.so \
  --config uniffi.toml \
  --out-dir "$KOTLIN_GEN_DIR"

# Copy Kotlin bindings.
mkdir -p "$DIST/kotlin/src"
cp -r "$KOTLIN_GEN_DIR"/ink "$DIST/kotlin/src/" 2>/dev/null || \
  cp "$KOTLIN_GEN_DIR"/*.kt "$DIST/kotlin/src/"

# Copy hand-written wrappers alongside the generated bindings. These
# implement the UniFFI foreign traits with platform-specific primitives
# (e.g. SharedPreferences-backed storage). They live in a committed
# source directory and are merged into the generated tree at build time.
echo "Copying hand-written Kotlin wrappers..."
cp -r "$SCRIPT_DIR/android-wrappers/"* "$DIST/kotlin/src/"

# Copy native libraries.
copy_lib() {
    local target=$1 abi=$2
    mkdir -p "$DIST/jniLibs/$abi"
    cp "target/$target/release/librift_mobile.so" "$DIST/jniLibs/$abi/"
}

copy_lib aarch64-linux-android arm64-v8a
copy_lib armv7-linux-androideabi armeabi-v7a
copy_lib x86_64-linux-android x86_64
copy_lib i686-linux-android x86

echo "Done: $DIST"
