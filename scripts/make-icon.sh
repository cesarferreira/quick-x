#!/usr/bin/env bash
# Render assets/icon.svg into assets/AppIcon.icns. Needs rsvg-convert (brew install librsvg).
set -euo pipefail

cd "$(dirname "$0")/.."
iconset="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    rsvg-convert -w "$size" -h "$size" assets/icon.svg -o "$iconset/icon_${size}x${size}.png"
    rsvg-convert -w "$((size * 2))" -h "$((size * 2))" assets/icon.svg -o "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o assets/AppIcon.icns
echo "Wrote assets/AppIcon.icns"
