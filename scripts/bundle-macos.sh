#!/usr/bin/env bash
# Build a release binary and wrap it in a minimal macOS app bundle.
# Usage: scripts/bundle-macos.sh [install-dir]   (default: ~/Applications)
set -euo pipefail

cd "$(dirname "$0")/.."
install_dir="${1:-$HOME/Applications}"
app="$install_dir/Quick X.app"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"

cargo build --release
target_dir="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$target_dir/release/quick-x" "$app/Contents/MacOS/quick-x"
cp assets/AppIcon.icns "$app/Contents/Resources/AppIcon.icns"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Quick X</string>
  <key>CFBundleDisplayName</key><string>Quick X</string>
  <key>CFBundleIdentifier</key><string>com.cesarferreira.quick-x</string>
  <key>CFBundleExecutable</key><string>quick-x</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Make Finder and the Dock pick up the new icon instead of a cached one.
touch "$app"
echo "Installed $app"
