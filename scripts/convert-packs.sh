#!/usr/bin/env bash
# Rebuilds the study packs in packs/. The sources live in sources/, which the
# repository leaves out; they were taken at these commits:
#   morphgnt/sblgnt                  aaed91e57c8e4a8dc9a2383e129ca5e75fe6393d  -> sources/sblgnt/*.txt
#   morphgnt/morphological-lexicon   0dca2af89f413cbb24f617ddbdc347e9d798ddf3  -> sources/morph-lexicon/lexemes.yaml
#   openscriptures/morphhb           3d15126fb1ef74867fc1434be1942e837932691f  -> sources/morphhb/*.xml (wlc/, without VerseMap.xml)
#   openscriptures/HebrewLexicon     21c9add13bc727d3a951361778e97e3ff7afd1ce  -> sources/hebrew-lexicon/
#   CodeBlue519/drb                  09d0e9dd810fb305eda529561be964dea0b3f42a  -> sources/haydock/haydock.tsv (commentary/haydock.tsv)
#   HistoricalChristianFaith/Commentaries-Database, release "latest" of 6 Nov 2026 -> sources/hcf/commentaries.sqlite
# and the CrossWire modules TSK, StrongsGreek, StrongsHebrew and AbbottSmith in sources/packs/,
# Easton, Smith and Nave in sources/dicts/.
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

DICTS="$SOURCES/dicts"

"$BIN" haydock "$SOURCES/haydock/haydock.tsv" \
  --id eng-haydock --abbreviation HAYDOCK --title "Haydock's Catholic Bible Commentary" \
  --language en --year 1859 --versification hebrew --license "Public domain" --date 2026-11-06 \
  --source "github.com/CodeBlue519/drb, commentary/haydock.tsv" -o "$OUT/HAYDOCK.grtb"

"$BIN" fathers "$SOURCES/hcf/commentaries.sqlite" --excerpts catena \
  --id eng-catena-aurea --abbreviation CATENA --title "Catena Aurea, Commentary on the Four Gospels" \
  --language en --year 1841 --versification hebrew --license "Public domain" --date 2026-11-06 \
  --source "github.com/HistoricalChristianFaith/Commentaries-Database, Newman's translation of 1841" -o "$OUT/CATENA.grtb"

"$BIN" fathers "$SOURCES/hcf/commentaries.sqlite" --excerpts fathers \
  --id eng-church-fathers --abbreviation FATHERS --title "The Fathers on Scripture" \
  --language en --versification hebrew --license "Public domain" --date 2026-11-06 \
  --source "github.com/HistoricalChristianFaith/Commentaries-Database, excerpts from the Ante-Nicene and Nicene and Post-Nicene Fathers" -o "$OUT/FATHERS.grtb"

"$BIN" sword "$DICTS/Easton.zip" \
  --id eng-easton --abbreviation EASTON --title "Easton's Bible Dictionary" \
  --language en --year 1897 --license "Public domain" --date 2026-11-06 \
  --source "CrossWire SWORD module Easton, version 2.0.1" -o "$OUT/EASTON.grtb"

"$BIN" sword "$DICTS/Smith.zip" \
  --id eng-smith --abbreviation SMITH --title "Smith's Bible Dictionary" \
  --language en --year 1884 --license "Public domain" --date 2026-11-06 \
  --source "CrossWire SWORD module Smith, version 1.3" -o "$OUT/SMITH.grtb"

"$BIN" sword "$DICTS/Nave.zip" \
  --id eng-nave --abbreviation NAVE --title "Nave's Topical Bible" \
  --language en --year 1896 --license "Public domain" --date 2026-11-06 \
  --source "CrossWire SWORD module Nave, version 3.0" -o "$OUT/NAVE.grtb"

ls -la "$OUT"
