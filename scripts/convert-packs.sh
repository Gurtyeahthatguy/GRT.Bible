#!/usr/bin/env bash
# Rebuilds the study packs in packs/. The sources live in sources/, which the
# repository leaves out; they were taken at these commits:
#   morphgnt/sblgnt                  aaed91e57c8e4a8dc9a2383e129ca5e75fe6393d  -> sources/sblgnt/*.txt
#   morphgnt/morphological-lexicon   0dca2af89f413cbb24f617ddbdc347e9d798ddf3  -> sources/morph-lexicon/lexemes.yaml
#   openscriptures/morphhb           3d15126fb1ef74867fc1434be1942e837932691f  -> sources/morphhb/*.xml (wlc/, without VerseMap.xml)
#   openscriptures/HebrewLexicon     21c9add13bc727d3a951361778e97e3ff7afd1ce  -> sources/hebrew-lexicon/
# and the CrossWire modules TSK, StrongsGreek, StrongsHebrew and AbbottSmith in sources/packs/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCES="${1:-$ROOT/sources}"
SRC="$SOURCES/packs"
OUT="$ROOT/packs"
BIN="$ROOT/target/release/grtb-convert"

cargo build --release -p grtb-convert --manifest-path "$ROOT/Cargo.toml"
mkdir -p "$OUT"

"$BIN" sword "$SRC/TSK.zip" --as crossrefs \
  --id eng-treasury-scripture-knowledge --abbreviation TSK --title "Treasury of Scripture Knowledge" \
  --language en --year 1836 --license "Public domain" --date 2026-10-27 \
  --source "CrossWire SWORD module TSK, version 1.4" -o "$OUT/TSK.grtb"

"$BIN" morphgnt "$SOURCES/sblgnt" --lexemes "$SOURCES/morph-lexicon/lexemes.yaml" \
  --id grc-sblgnt-morphgnt --abbreviation SBLGNT --title "SBL Greek New Testament, with MorphGNT" \
  --language grc --year 2010 --canon partial --versification hebrew --date 2026-10-30 \
  --license "Text CC BY 4.0, Society of Biblical Literature and Logos Bible Software; parsing and lemmas CC BY-SA 3.0, MorphGNT" \
  --source "github.com/morphgnt/sblgnt and morphgnt/morphological-lexicon" -o "$OUT/SBLGNT.grtb"

"$BIN" oshb "$SOURCES/morphhb" --index "$SOURCES/hebrew-lexicon/LexicalIndex.xml" --aug "$SOURCES/hebrew-lexicon/AugIndex.xml" \
  --id hbo-oshb --abbreviation OSHB --title "Open Scriptures Hebrew Bible" \
  --language hbo --direction rtl --canon partial --versification hebrew --date 2026-10-30 \
  --license "Text public domain (Westminster Leningrad Codex); lemmas and morphology CC BY 4.0, Open Scriptures Hebrew Bible Project" \
  --source "github.com/openscriptures/morphhb" -o "$OUT/OSHB.grtb"

"$BIN" bdb "$SOURCES/hebrew-lexicon/BrownDriverBriggs.xml" --index "$SOURCES/hebrew-lexicon/LexicalIndex.xml" \
  --id eng-bdb --abbreviation BDB --title "Brown-Driver-Briggs Hebrew and English Lexicon" \
  --language en --year 1906 --versification hebrew --date 2026-10-30 \
  --license "Text public domain; transcription CC BY 4.0, Open Scriptures Hebrew Bible Project" \
  --source "github.com/openscriptures/HebrewLexicon" -o "$OUT/BDB.grtb"

"$BIN" sword "$SRC/StrongsGreek.zip" \
  --id eng-strongs-greek --abbreviation STRONGG --title "Strong's Greek Dictionary" \
  --language en --year 1890 --license "Public domain" --date 2026-10-30 \
  --source "CrossWire SWORD module StrongsGreek, version 2.0" -o "$OUT/STRONGG.grtb"

"$BIN" sword "$SRC/StrongsHebrew.zip" \
  --id eng-strongs-hebrew --abbreviation STRONGH --title "Strong's Hebrew Dictionary" \
  --language en --year 1890 --license "Public domain" --date 2026-10-30 \
  --source "CrossWire SWORD module StrongsHebrew, version 1.2" -o "$OUT/STRONGH.grtb"

"$BIN" sword "$SRC/AbbottSmith.zip" \
  --id eng-abbott-smith --abbreviation ABBOTT --title "Abbott-Smith, Manual Greek Lexicon of the New Testament" \
  --language en --year 1922 --license "Public domain" --date 2026-10-30 \
  --source "CrossWire SWORD module AbbottSmith, version 4.1" -o "$OUT/ABBOTT.grtb"

ls -la "$OUT"
