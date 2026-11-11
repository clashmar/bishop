#!/usr/bin/env bash
set -euo pipefail

echo "==> Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# shellcheck source=/dev/null
source "$HOME/.cargo/env"

echo "==> Installing Rust WASM targets..."
rustup target add wasm32-unknown-unknown wasm32-unknown-emscripten

echo "==> Installing build tools..."
brew install cargo-make emscripten

echo "==> Setup complete!"