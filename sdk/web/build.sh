#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "Building web SDK..."
npm run build

echo "Copying IIFE build to server..."
cp dist/index.global.js ../../server/src/sdk/rift.js

echo "Done."
