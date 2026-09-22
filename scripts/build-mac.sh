#!/usr/bin/env bash
# Builds a release Jarvis.app + .dmg for macOS.
#
# The GUI is bundled by Tauri; the background assistant (jarvis-app) is
# packaged as a Tauri "sidecar", which expects the binary under
# crates/jarvis-gui/binaries/jarvis-app-<target-triple>.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$PWD
export PATH="$ROOT/frontend/node_modules/.bin:$HOME/.cargo/bin:$PATH"

TARGET=$(rustc -vV | sed -n 's/^host: //p')

echo "==> frontend dependencies"
[ -d frontend/node_modules ] || (cd frontend && npm install)

echo "==> jarvis-app (release, $TARGET)"
cargo build --release -p jarvis-app
mkdir -p crates/jarvis-gui/binaries
cp target/release/jarvis-app "crates/jarvis-gui/binaries/jarvis-app-$TARGET"

echo "==> Jarvis.app + dmg"
# the sidecar config is applied only here: with externalBin in tauri.conf.json,
# `tauri dev` would copy the release sidecar over target/debug/jarvis-app
tauri build --config crates/jarvis-gui/tauri.sidecar.conf.json "$@"

echo
echo "Done:"
ls -1 target/release/bundle/macos/*.app target/release/bundle/dmg/*.dmg 2>/dev/null || true
