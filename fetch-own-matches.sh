#!/bin/sh
# Recover salts for your own Deadlock matches via the Steam Game Coordinator.
# Usage: curl -fsSL https://raw.githubusercontent.com/DTV-GG/dtv-ingest/master/fetch-own-matches.sh | sh
set -eu

[ "$(uname -s)" = Linux ] || { echo "Only Linux is supported; on Windows use fetch-own-matches.ps1" >&2; exit 1; }

bin=$(mktemp)
trap 'rm -f "$bin"' EXIT
curl -fsSL -o "$bin" https://github.com/DTV-GG/dtv-ingest/releases/latest/download/dtv-ingest-ubuntu-latest
chmod +x "$bin"
RUST_LOG=info "$bin" --own-matches
