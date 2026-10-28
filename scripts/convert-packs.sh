#!/usr/bin/env bash
# Rebuilds the study packs in packs/ from the SWORD modules in sources/packs/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${1:-$ROOT/sources/packs}"
OUT="$ROOT/packs"
BIN="$ROOT/target/release/grtb-convert"

cargo build --release -p grtb-convert --manifest-path "$ROOT/Cargo.toml"
mkdir -p "$OUT"

"$BIN" sword "$SRC/TSK.zip" --as crossrefs \
  --id eng-treasury-scripture-knowledge --abbreviation TSK --title "Treasury of Scripture Knowledge" \
  --language en --year 1836 --license "Public domain" --date 2026-10-27 \
  --source "CrossWire SWORD module TSK, version 1.4" -o "$OUT/TSK.grtb"

ls -la "$OUT"
