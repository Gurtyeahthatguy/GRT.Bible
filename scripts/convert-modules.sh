#!/usr/bin/env bash
# Rebuilds the bundled modules in modules/ from the sources in sources/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${1:-$ROOT/sources}"
OUT="$ROOT/modules"
BIN="$ROOT/target/release/grtb-convert"

cargo build --release -p grtb-convert --manifest-path "$ROOT/Cargo.toml"
mkdir -p "$OUT"

"$BIN" usfm "$SRC/latVUC_usfm.zip" --strip-brackets \
  --id lat-vulgate-clementine --abbreviation VUL --title "Biblia Sacra Vulgata (Clementina)" \
  --language la --year 1592 --canon catholic --versification vulgate --license "Public domain" \
  --source "eBible.org latVUC, with the Glossa Ordinaria from Migne's edition" -o "$OUT/VUL.grtb"

"$BIN" usfm "$SRC/eng-web-c_usfm.zip" \
  --id eng-web-catholic --abbreviation WEBC --title "World English Bible, Catholic Edition" \
  --language en --year 2020 --canon catholic --versification webc --license "Public domain" \
  --source "eBible.org eng-web-c" -o "$OUT/WEBC.grtb"

"$BIN" usfm "$SRC/engDRA_usfm.zip" \
  --id eng-douay-rheims-1899 --abbreviation DRA --title "Douay-Rheims Bible, Challoner revision" \
  --language en --year 1899 --canon catholic --versification douay --license "Public domain" \
  --source "eBible.org engDRA" -o "$OUT/DRA.grtb"

"$BIN" martini "$SRC/martini/martini.rtf" \
  --id ita-martini-1781 --abbreviation MAR --title "La Sacra Bibbia, tradotta da Antonio Martini" \
  --language it --year 1781 --canon catholic --versification martini --license "Public domain" \
  --source "archive.org bibbia-martini, digital edition" -o "$OUT/MAR.grtb"

"$BIN" usfm "$SRC/ita1927_usfm.zip" \
  --id ita-riveduta-1927 --abbreviation RIV --title "La Sacra Bibbia, Riveduta 1927" \
  --language it --year 1927 --canon protestant --versification english --license "Public domain" \
  --source "eBible.org ita1927" -o "$OUT/RIV.grtb"

"$BIN" usfm "$SRC/grcbrent_usfm.zip" \
  --id grc-septuagint-brenton --abbreviation LXX --title "Septuaginta, Brenton edition" \
  --language grc --year 1851 --canon catholic --versification lxx --license "Public domain" \
  --source "eBible.org grcbrent" -o "$OUT/LXX.grtb"

"$BIN" n1904 "$SRC/n1904/Nestle1904.csv" \
  --id grc-nestle-1904 --abbreviation N1904 --title "Novum Testamentum Graece, Nestle 1904" \
  --language grc --year 1904 --canon partial --versification hebrew --license "Public domain" \
  --source "biblicalhumanities.org Nestle1904, CC0 word list" -o "$OUT/N1904.grtb"

"$BIN" usfm "$SRC/hebwlc_usfm.zip" --hebrew-paragraphs \
  --id hbo-westminster-leningrad --abbreviation WLC --title "Westminster Leningrad Codex" \
  --language hbo --direction rtl --canon partial --versification hebrew --license "Public domain" \
  --source "eBible.org hebwlc" -o "$OUT/WLC.grtb"

ls -la "$OUT"
