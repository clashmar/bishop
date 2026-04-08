#!/usr/bin/env bash
set -euo pipefail

echo "==> Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

echo "==> Installing cargo-make..."
brew install cargo-make

echo "==> Setup complete!"