#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Building znt (release)..."
cargo install --path "$SCRIPT_DIR/crates/zen-cli" --locked 2>/dev/null \
  || cargo install --path "$SCRIPT_DIR/crates/zen-cli"

echo ""
echo "Installed: $(which znt)"
echo "Version:   $(znt --version 2>/dev/null || echo '(no --version flag yet)')"
