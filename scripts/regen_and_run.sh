#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/.."

cd "$ROOT"

SHARED_ENGINE_OWNER_MARKER='-- bishop-owner: shared-engine'

has_owner_marker() {
    local file="$1"
    local marker="$2"
    head -n 3 "$file" | grep -Fqx -- "$marker"
}

validate_shared_engine_sources() {
    local src_dir="$1"
    local src_file

    shopt -s nullglob
    for src_file in "$src_dir"/*.lua; do
        if ! has_owner_marker "$src_file" "$SHARED_ENGINE_OWNER_MARKER"; then
            echo "Missing shared engine ownership marker in $src_file" >&2
            exit 1
        fi
    done
    shopt -u nullglob
}

sync_shared_engine_scripts() {
    local src_dir="$1"
    local target_dir="$2"
    local src_file
    local target_file
    local filename

    mkdir -p "$target_dir"

    shopt -s nullglob
    for src_file in "$src_dir"/*.lua; do
        rsync -a "$src_file" "$target_dir/"
    done

    for target_file in "$target_dir"/*.lua; do
        filename="$(basename "$target_file")"
        if [ ! -f "$src_dir/$filename" ] && has_owner_marker "$target_file" "$SHARED_ENGINE_OWNER_MARKER"; then
            rm -f "$target_file"
        fi
    done
    shopt -u nullglob
}

echo "==> Running lua_api_gen..."
cargo run -p lua_api_gen

echo "==> Building editor (triggers build.rs)..."
cargo build -p editor

echo "==> Syncing _engine scripts to game projects..."
SRC="editor/scripts/_engine"
validate_shared_engine_sources "$SRC"
for game_dir in games/*/; do
    target="${game_dir}Resources/scripts/_engine"
    if [ -d "$target" ]; then
        echo "    Updating $target"
        sync_shared_engine_scripts "$SRC" "$target"
    fi
done

echo "==> Refreshing exported game binaries..."
(
    cd editor
    cargo make copy-game-bins-mac
)

echo "==> Launching editor..."
cargo run -p editor
