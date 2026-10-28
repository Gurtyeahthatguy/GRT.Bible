# GRT Bible

A Bible reader for the 73-book Catholic canon that runs on your own computer.

- **Offline.** Reading, search, parallel reading, notes and highlights need no
  network.
- **Nothing social.** No account, no sharing, no reading plans, no streaks.
- **Public domain texts only.** Other translations can be imported from files
  you already have.
- **Study material from files.** Commentaries, cross-references, dictionaries
  and books install from a file, and appear beside the verse you are reading.
- **Your data is one file.** Position, history, notes, highlights, bookmarks,
  saved videos and video lists live in a single `.grt` file you can copy
  anywhere.

Ready-made programs for Linux, Windows and Android are attached to each release.
Building and installing: [INSTALL.md](INSTALL.md).

It belongs to the GRT family, beside [GRT
Office](https://github.com/Gurtyeahthatguy/GRT.Office) and [GRT
Sentry](https://github.com/Gurtyeahthatguy/GRT.Sentry). Office promises it never
touches the network; this program has one section that does, described below.

## What leaves the machine

| What | Where | When |
|---|---|---|
| A video link | Your web browser opens it | Only when you press **Open in browser**, with the video section switched on |
| Nothing else | | |

The program itself opens no connection. The video section is off until you say
yes to it, and a link opens only if its site is on the list in
[crates/grtb/src/catalog.rs](crates/grtb/src/catalog.rs). No reading position,
search or note is ever sent anywhere. See [Checking the claims](#checking-the-claims).

## Translations

| Abbreviation | Text | Language | Books | Source |
|---|---|---|---|---|
| VUL | Clementine Vulgate, with the Glossa Ordinaria as notes | Latin | 73 | eBible.org `latVUC` |
| MAR | Antonio Martini, 1781 | Italian | 73 | archive.org `bibbia-martini` |
| WEBC | World English Bible, Catholic Edition | English | 73 | eBible.org `eng-web-c` |
| DRA | Douay-Rheims, Challoner revision | English | 73 | eBible.org `engDRA` |
| RIV | Riveduta, 1927 | Italian | 66 | eBible.org `ita1927` |
| LXX | Septuagint, Brenton's Greek text | Greek | 46 | eBible.org `grcbrent` |
| N1904 | Nestle 1904 | Greek | 27 | biblicalhumanities.org, CC0 |
| WLC | Westminster Leningrad Codex | Hebrew | 39 | eBible.org `hebwlc` |

The Septuagint is Brenton's Greek rather than Swete's: no complete digital
Swete is in the public domain. Martini comes from a digital edition whose RTF
marks every verse; a short list of slips in it is corrected in
[crates/grtb-convert/src/martini.rs](crates/grtb-convert/src/martini.rs).

Books a translation lacks are listed, greyed out, not hidden.

## Study packs

Attached to each release as separate files, installed from the library:

| File | Kind | Source |
|---|---|---|
| `TSK.grtb` | Cross-references: the Treasury of Scripture Knowledge, 66,726 groups | CrossWire SWORD module `TSK` 1.4, public domain |

[MODULES.md](MODULES.md) explains, step by step, how to install these and any
SWORD or USFM module on Linux, Windows and Android.

## Verse numbers

Translations number verses differently: the Psalms of the Vulgate run one
behind the Hebrew, English Bibles leave psalm titles unnumbered, Daniel and
Esther carry Greek additions in different places. Every note, highlight,
bookmark and history entry is stored in one internal scheme, Hebrew numbering
with Esther in sixteen chapters, Daniel in fourteen and the Letter of Jeremiah
as Baruch 6, and converted when shown. A note taken in the Vulgate appears on
the same verse in any other translation.

The conversion tables are in [crates/grtb/data/versification](crates/grtb/data/versification),
one line per difference:

```
Ps.22.1-6 = Ps.23.1-6
```

They start from the Paratext versification files published by the United Bible
Societies under the MIT licence, corrected where a text departs from them.

## Using it

| Key | Action |
|---|---|
| `Left` `Right` | previous or next chapter |
| `[` `]` | previous or next book |
| `Ctrl+K` | go to a passage |
| `Ctrl+F` | search |
| `Ctrl+C` | copy the selection with its reference |
| `Ctrl+P` | print |
| `Ctrl +` `Ctrl -` | text size |
| `Esc` | close, or clear the selection |

**Go to a passage** accepts `Gv 3,16`, `Gv 3,16-18`, `sir 24`, `1mac 2,15-20`,
`Ps 23`, `Giovanni 3 16` and `John 3:16`, in Italian, Latin or English, with any
separator. Numbers are read in the translation you are reading. `Gn` is Genesis
in Latin and Jonah in Italian, so it asks.

**Click a verse** to select it, click its neighbours to extend, Shift-click for
a range. The bar that appears copies, highlights, adds a note or a bookmark,
shows the references and commentary on it, or prints it. Copied text ends with
the reference, `Gv 3,16-17 (MAR)`; where the reference goes, and whether the
abbreviation and verse numbers come with it, is in Settings. The copy also
carries the quotation as HTML, which GRT Paper pastes as a quote with its
reference.

**Compare** puts up to four translations side by side, aligned verse by verse.

**Search** matches words without regard to case, accents, Greek breathings or
Hebrew points. Quotes keep a phrase together.

**Study** holds the verse of the day, your notes, highlights, bookmarks and
history. The file is `user.grt` in the program's data folder; Settings exports
and imports it. Notes also export as one Markdown file, with front matter, that
GRT Notes and other Markdown notebooks open.

**Library** shows what the installed material says about the selected verse,
or the first one in view: its cross-references, each opening a preview of the
passage in your translation, and the chosen commentary, which steps from one
comment to the next. Dictionaries are looked up by headword, books read by
section, and the **Installed** tab lists every module with its kind, language,
licence, source and the file it came from.

**Print** lays the chapter, the selected verses, the comparison or all your
notes on pages of a fixed size, A4, A5 or Letter, with a header of our own and
page numbers, and shows them before anything is printed. On Linux the pages go
to GTK's print dialog with the paper already set; on Windows to the system's.

## On a phone

The Android build is the same program on a smaller screen: the tools sit in a bar
along the bottom, a sideways swipe turns the chapter, and the back button closes
whatever is open before it leaves the app. The first start takes a few seconds,
while the texts are unpacked from the package into the app's own storage.

It asks Android for no permission at all, the internet included, so the system
itself will not let it open a connection. A video link is handed to your browser
or video app, which has its own. Nothing is copied to a Google account either:
backup is off, and your `.grt` file is exported from Settings as anywhere else.

Two things work differently from the desktop. A SWORD or USFM folder cannot be
chosen, only a file, because Android hands over single documents. And the file
picker offers every file, since it filters by media type and `.grtb` and `.grt`
have none. Printing goes to Android's print service, which also saves a PDF.

## Installing modules

Library, then **Installed**: **Install a .grtb file…** takes a module as it is;
**Convert a SWORD or USFM module…** reads a SWORD module of any of the five
kinds (the ZIP CrossWire distributes, or its folder) or USFM files (a ZIP, a
folder or one file). The program suggests the verse numbering that fits, and
offers a commentary made mostly of references as cross-references. Strong's
numbers and morphology in an imported Bible are kept in the module for the
study of the original languages. Nothing is downloaded: you choose a file
already on your computer, and you are responsible for the right to use it.
Every step, on every system, is in [MODULES.md](MODULES.md).

## The video section

The catalog lists 4,675 videos from five Catholic channels, sorted into three
categories:

| Channel | Videos |
|---|---|
| [The Counsel of Trent](https://www.youtube.com/@TheCounselofTrent) | 613 |
| [Jesus and Whatnot](https://www.youtube.com/@Jesus.andwhatnot) | 146 |
| [Nicholas Bowling](https://www.youtube.com/@NicholasBowling) | 335 |
| [Distinguo](https://www.youtube.com/@Distinguo) | 25 |
| [Matt Fradd, Pints With Aquinas](https://www.youtube.com/@pintswithaquinas) | 3,556 |

**Debates** holds debates, rebuttals and replies to critics; **Doctrine** the
teaching of the Church, from the sacraments and Mary to the papacy and moral
questions; **Theology** everything else. The sorting reads titles, so a video
can land in the wrong place: change its `category` in `catalog/catalog.json`
and the correction survives the next refresh. A video whose title names a
passage, such as "James 2", also appears on that chapter.

Any video can be linked to the verses you have selected, and then shows as a
small mark beside them; videos can also be kept in lists of your own. Both are
saved in your `.grt` file.

GRT Bible is not affiliated with any of these channels. The list is one
reader's selection, made from what the channels publish in the open; none of
them has been asked, told, or has agreed to anything, and none of them has any
say in what is listed here.

The catalog is a `.grt` package holding a JSON list, and optionally
thumbnails, so the screen never fetches anything. Shorts are left out, and so
are thumbnails and descriptions, which belong to the channels.

To refresh it from the channels in `catalog/channels.json`, then package it:

```bash
node scripts/collect-videos.mjs
cargo run --release -p grtb-convert -- catalog catalog -o catalog/videos.grt
```

The collector reads the channels' public pages, as a browser would. It is a
tool for whoever keeps the catalog and is not part of the program; so is
`node scripts/check-videos.mjs`, which lists the links that no longer answer.

## Where things are

| What | Linux | Windows | Android |
|---|---|---|---|
| Your file, `user.grt`, and installed modules | `~/.local/share/org.grt.bible/` | `%APPDATA%\org.grt.bible\` | inside the application |
| Settings | `~/.config/org.grt.bible/settings.json` | `%APPDATA%\org.grt.bible\settings.json` | inside the application |
| The translations that come with it | inside the package | inside the package | unpacked out of the package at the first start |

Export in Settings writes that same file wherever you choose, and importing one
replaces what is there. Building the program from source, and what the `.grtb`
files are made from, are in [INSTALL.md](INSTALL.md).

## Checking the claims

```bash
./scripts/check-build.sh target/release/grt-bible
./scripts/check-network.sh target/release/grt-bible
```

The first reads the release binary for build paths, addresses outside
[scripts/allowed-strings.txt](scripts/allowed-strings.txt), telemetry libraries
and debug symbols. The second runs the program under `strace` and lists every
address it connects to.

```bash
./scripts/check-apk.sh
```

reads the package a phone would get: the permissions it asks for, which should be
none, and its library the same way.

```bash
cargo test --workspace
npm test
./scripts/preview.sh
```

150 tests cover the reference parser, the versification tables, the module and
user file formats, the SWORD readers for every kind of module, markup that
tries to smuggle in scripts, and the interface running against a stubbed
backend. The preview serves the real interface in a browser, with a few
chapters of every module and of the study packs, so it can be looked at
without building.

## Limits

- The Windows build has never been run by a person. It compiles, the workflow
  produces it, and what it shares with Linux is covered by the tests. Treat it as
  one to try rather than one to rely on, and say so in an issue if it misbehaves.
- The phone build is Android only. An iPhone one needs a Mac to compile.
- The Septuagint stops where Brenton's text stops, and Nestle 1904 is the New
  Testament alone. What a translation lacks is listed and greyed out rather than
  hidden.
- A note, a highlight and a bookmark belong to a verse, not to a phrase inside
  it.
- Search covers the translations, not commentaries, dictionaries or books.
- Strong's numbers kept from an imported SWORD Bible are not shown yet.
- The video catalog is sorted by reading titles, so a video sits in the wrong
  category until someone corrects it.

## Licence

MIT. See [LICENSE](LICENSE). The texts are in the public domain. The
versification data derived from Paratext is MIT, United Bible Societies; the
chapter and verse counts the SWORD importer needs were read from CrossWire's
canon tables. The Treasury of Scripture Knowledge is in the public domain.
