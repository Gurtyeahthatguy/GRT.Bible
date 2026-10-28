//! The `.grtb` module: one read-only SQLite file per translation, commentary,
//! dictionary, set of cross-references or book.
//!
//! A Bible keeps its verses in its own numbering, named by `versification`.
//! Every other kind keeps its references in the internal scheme, already
//! mapped from the numbering of its source.

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::canon::{book, books};
use crate::integrity::Report;
use crate::refs::Ref;
use crate::text::fold;

pub const SCHEMA_VERSION: u32 = 2;
pub const EXTENSION: &str = "grtb";
const APPLICATION_ID: i64 = 0x4752_5442;

const SCHEMA: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE books (
    id          INTEGER PRIMARY KEY,
    osis        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    abbrev      TEXT NOT NULL,
    testament   TEXT NOT NULL CHECK (testament IN ('OT','NT')),
    deutero     INTEGER NOT NULL DEFAULT 0,
    position    INTEGER NOT NULL,
    chapters    INTEGER NOT NULL
);
CREATE TABLE verses (
    book_id     INTEGER NOT NULL,
    chapter     INTEGER NOT NULL,
    verse       INTEGER NOT NULL,
    text        TEXT NOT NULL,
    paragraph   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (book_id, chapter, verse)
);
CREATE TABLE notes (
    book_id     INTEGER NOT NULL,
    chapter     INTEGER NOT NULL,
    verse       INTEGER NOT NULL,
    marker      TEXT,
    text        TEXT NOT NULL
);
CREATE TABLE titles (
    book_id     INTEGER NOT NULL,
    chapter     INTEGER NOT NULL,
    verse       INTEGER NOT NULL,
    kind        TEXT NOT NULL DEFAULT 'section',
    text        TEXT NOT NULL
);
CREATE INDEX notes_place ON notes (book_id, chapter, verse);
CREATE INDEX titles_place ON titles (book_id, chapter, verse);
CREATE VIRTUAL TABLE verses_fts USING fts5(text, content='', tokenize='unicode61 remove_diacritics 2');
CREATE TABLE tagged (
    book_id     INTEGER NOT NULL,
    chapter     INTEGER NOT NULL,
    verse       INTEGER NOT NULL,
    markup      TEXT NOT NULL,
    PRIMARY KEY (book_id, chapter, verse)
);
";

const RESOURCE_SCHEMA: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE entries (
    id          INTEGER PRIMARY KEY,
    parent      INTEGER,
    key         TEXT NOT NULL,
    fold        TEXT NOT NULL,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL
);
CREATE TABLE spans (
    entry       INTEGER NOT NULL,
    first       INTEGER NOT NULL,
    last        INTEGER NOT NULL
);
CREATE TABLE links (
    first       INTEGER NOT NULL,
    last        INTEGER NOT NULL,
    position    INTEGER NOT NULL,
    label       TEXT NOT NULL,
    targets     TEXT NOT NULL
);
CREATE INDEX entries_fold ON entries (fold);
CREATE INDEX entries_parent ON entries (parent, id);
CREATE INDEX spans_first ON spans (first, last);
CREATE INDEX links_first ON links (first, last);
";

/// What a module holds.
pub const KINDS: [&str; 5] = ["bible", "commentary", "dictionary", "crossrefs", "book"];

/// A reference as one sortable number: book, chapter and verse in fixed places.
pub fn pack(r: Ref) -> i64 {
    r.book as i64 * 1_000_000 + r.chapter as i64 * 1_000 + r.verse as i64
}

pub fn unpack(n: i64) -> Ref {
    Ref::new((n / 1_000_000) as u8, (n / 1_000 % 1_000) as u16, (n % 1_000) as u16)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// One of `KINDS`; modules written before schema 2 are Bibles.
    #[serde(default = "bible")]
    pub kind: String,
    pub id: String,
    pub abbreviation: String,
    pub title: String,
    pub language: String,
    pub direction: String,
    pub year: Option<i32>,
    pub canon: String,
    pub versification: String,
    pub license: String,
    pub source: String,
    pub books_present: u32,
    pub source_notes: bool,
    pub schema_version: u32,
    pub generated_at: String,
    pub source_hash: String,
    #[serde(default)]
    pub missing_books: Vec<String>,
    /// SWORD features worth keeping, such as `GreekDef` for a Strong's lexicon.
    #[serde(default)]
    pub features: Vec<String>,
}

fn bible() -> String {
    "bible".into()
}

impl Manifest {
    fn pairs(&self) -> Vec<(&'static str, String)> {
        vec![
            ("kind", self.kind.clone()),
            ("id", self.id.clone()),
            ("abbreviation", self.abbreviation.clone()),
            ("title", self.title.clone()),
            ("language", self.language.clone()),
            ("direction", self.direction.clone()),
            ("year", self.year.map(|y| y.to_string()).unwrap_or_default()),
            ("canon", self.canon.clone()),
            ("versification", self.versification.clone()),
            ("license", self.license.clone()),
            ("source", self.source.clone()),
            ("books_present", self.books_present.to_string()),
            ("source_notes", self.source_notes.to_string()),
            ("schema_version", self.schema_version.to_string()),
            ("generated_at", self.generated_at.clone()),
            ("source_hash", self.source_hash.clone()),
            ("missing_books", serde_json::to_string(&self.missing_books).unwrap()),
            ("features", serde_json::to_string(&self.features).unwrap()),
        ]
    }

    fn from_pairs(map: &BTreeMap<String, String>) -> Result<Manifest, String> {
        let get = |k: &str| map.get(k).cloned().ok_or_else(|| format!("module has no '{k}' in meta"));
        let kind = map.get("kind").cloned().unwrap_or_else(bible);
        if !KINDS.contains(&kind.as_str()) {
            return Err(format!("module kind '{kind}' is not one this version reads"));
        }
        Ok(Manifest {
            kind,
            id: get("id")?,
            abbreviation: get("abbreviation")?,
            title: get("title")?,
            language: get("language")?,
            direction: map.get("direction").cloned().unwrap_or_else(|| "ltr".into()),
            year: map.get("year").and_then(|y| y.parse().ok()),
            canon: get("canon")?,
            versification: get("versification")?,
            license: get("license")?,
            source: map.get("source").cloned().unwrap_or_default(),
            books_present: get("books_present")?.parse().map_err(|_| "bad books_present")?,
            source_notes: get("source_notes")? == "true",
            schema_version: get("schema_version")?.parse().map_err(|_| "bad schema_version")?,
            generated_at: map.get("generated_at").cloned().unwrap_or_default(),
            source_hash: map.get("source_hash").cloned().unwrap_or_default(),
            missing_books: map.get("missing_books").and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default(),
            features: map.get("features").and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerseRow {
    pub chapter: u16,
    pub verse: u16,
    pub text: String,
    pub paragraph: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleRow {
    pub chapter: u16,
    pub verse: u16,
    /// `section`, `major`, `psalm` or `summary`.
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteRow {
    pub chapter: u16,
    pub verse: u16,
    pub marker: String,
    pub text: String,
}

/// A verse with its words tagged, kept for the study of the original languages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaggedRow {
    pub chapter: u16,
    pub verse: u16,
    /// The verse with `<w lemma=".." morph="..">word</w>` around tagged words.
    pub markup: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookContent {
    pub verses: Vec<VerseRow>,
    pub titles: Vec<TitleRow>,
    pub notes: Vec<NoteRow>,
    pub tagged: Vec<TaggedRow>,
}

impl BookContent {
    pub fn chapters(&self) -> u16 {
        self.verses.iter().map(|v| v.chapter).max().unwrap_or(0)
    }

    /// Orders rows and drops a repeated verse number, keeping the first.
    pub fn tidy(&mut self) {
        self.verses.sort_by_key(|v| (v.chapter, v.verse));
        self.verses.dedup_by_key(|v| (v.chapter, v.verse));
        self.titles.sort_by_key(|t| (t.chapter, t.verse));
        self.notes.sort_by_key(|n| (n.chapter, n.verse));
        self.tagged.sort_by_key(|t| (t.chapter, t.verse));
        self.tagged.dedup_by_key(|t| (t.chapter, t.verse));
    }
}

/// One article of a commentary, dictionary or book.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryContent {
    /// Position of the parent entry in the list, for the sections of a book.
    pub parent: Option<u32>,
    /// Headword of a dictionary, empty elsewhere.
    pub key: String,
    pub title: String,
    /// Markup in the subset described in `markup.rs`.
    pub body: String,
    /// Internal ranges a commentary entry speaks about.
    pub spans: Vec<(Ref, Ref)>,
}

/// The references one verse sends the reader to, under one heading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkGroup {
    pub first: Ref,
    pub last: Ref,
    pub label: String,
    pub targets: Vec<(Ref, Ref)>,
}

/// Writes a range the way `refs::parse_range` reads it back.
pub fn range_osis(a: Ref, b: Ref) -> String {
    if a == b {
        a.osis()
    } else {
        format!("{}-{}", a.osis(), b.osis())
    }
}

/// Everything needed to write a module, books keyed by canon index.
#[derive(Debug, Clone)]
pub struct ModuleContent {
    pub manifest: Manifest,
    pub books: BTreeMap<u8, BookContent>,
    pub integrity: Option<Report>,
    pub entries: Vec<EntryContent>,
    pub links: Vec<LinkGroup>,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Writes a module. The same content always gives the same bytes.
pub fn write(path: &Path, content: &ModuleContent) -> Result<(), String> {
    let staging = path.with_extension("grtb-building");
    let _ = std::fs::remove_file(&staging);
    let _ = std::fs::remove_file(path);
    {
        let db = Connection::open(&staging).map_err(err)?;
        db.execute_batch("PRAGMA page_size = 4096; PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;").map_err(err)?;
        let bible = content.manifest.kind == "bible";
        db.execute_batch(if bible { SCHEMA } else { RESOURCE_SCHEMA }).map_err(err)?;
        db.pragma_update(None, "user_version", SCHEMA_VERSION).map_err(err)?;
        db.pragma_update(None, "application_id", APPLICATION_ID).map_err(err)?;
        let tx = db.unchecked_transaction().map_err(err)?;

        let mut meta: Vec<(String, String)> =
            content.manifest.pairs().into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        if let Some(report) = &content.integrity {
            meta.push(("integrity".into(), serde_json::to_string(report).map_err(err)?));
        }
        meta.sort();
        for (k, v) in &meta {
            tx.execute("INSERT INTO meta (key, value) VALUES (?1, ?2)", params![k, v]).map_err(err)?;
        }

        if !bible {
            write_resources(&tx, content)?;
        }
        let language = content.manifest.language.as_str();
        for (index, body) in content.books.iter().filter(|_| bible) {
            let b = book(*index).ok_or_else(|| format!("no canon book {index}"))?;
            let (name, abbrev) = b.name_in(language);
            tx.execute(
                "INSERT INTO books (id, osis, name, abbrev, testament, deutero, position, chapters)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![b.index, b.osis, name, abbrev, b.testament, b.deutero as i32, b.index, body.chapters()],
            )
            .map_err(err)?;
            for v in &body.verses {
                tx.execute(
                    "INSERT INTO verses (book_id, chapter, verse, text, paragraph) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![b.index, v.chapter, v.verse, v.text, v.paragraph as i32],
                )
                .map_err(err)?;
                let rowid = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO verses_fts (rowid, text) VALUES (?1, ?2)",
                    params![rowid, fold(&v.text, language)],
                )
                .map_err(err)?;
            }
            for t in &body.titles {
                tx.execute(
                    "INSERT INTO titles (book_id, chapter, verse, kind, text) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![b.index, t.chapter, t.verse, t.kind, t.text],
                )
                .map_err(err)?;
            }
            for n in &body.notes {
                tx.execute(
                    "INSERT INTO notes (book_id, chapter, verse, marker, text) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![b.index, n.chapter, n.verse, n.marker, n.text],
                )
                .map_err(err)?;
            }
            for t in &body.tagged {
                tx.execute(
                    "INSERT INTO tagged (book_id, chapter, verse, markup) VALUES (?1, ?2, ?3, ?4)",
                    params![b.index, t.chapter, t.verse, t.markup],
                )
                .map_err(err)?;
            }
        }
        if bible {
            tx.execute("INSERT INTO verses_fts (verses_fts) VALUES ('optimize')", []).map_err(err)?;
        }
        tx.commit().map_err(err)?;
        let target = path.to_str().ok_or("module path is not valid UTF-8")?;
        db.execute("VACUUM INTO ?1", params![target]).map_err(err)?;
    }
    std::fs::remove_file(&staging).map_err(err)?;
    Ok(())
}

fn write_resources(tx: &rusqlite::Transaction, content: &ModuleContent) -> Result<(), String> {
    let language = content.manifest.language.as_str();
    for (i, e) in content.entries.iter().enumerate() {
        let id = i as i64 + 1;
        let parent = e.parent.map(|p| p as i64 + 1);
        tx.execute(
            "INSERT INTO entries (id, parent, key, fold, title, body) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, parent, e.key, fold(e.key.trim(), language), e.title, e.body],
        )
        .map_err(err)?;
        for (a, b) in &e.spans {
            tx.execute("INSERT INTO spans (entry, first, last) VALUES (?1, ?2, ?3)", params![id, pack(*a), pack(*b)])
                .map_err(err)?;
        }
    }
    for (i, l) in content.links.iter().enumerate() {
        let targets = l.targets.iter().map(|(a, b)| range_osis(*a, *b)).collect::<Vec<_>>().join(" ");
        tx.execute(
            "INSERT INTO links (first, last, position, label, targets) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![pack(l.first), pack(l.last), i as i64, l.label, targets],
        )
        .map_err(err)?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookInfo {
    pub index: u8,
    pub osis: String,
    pub name: String,
    pub abbrev: String,
    pub chapters: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chapter {
    pub verses: Vec<VerseRow>,
    pub titles: Vec<TitleRow>,
    pub notes: Vec<NoteRow>,
}

/// An entry as the interface reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryRow {
    pub id: i64,
    pub parent: Option<i64>,
    pub key: String,
    pub title: String,
    pub body: String,
    /// Internal ranges, as `Gen.1.1` or `Gen.1.1-Gen.1.3`; verse 0 is a chapter introduction.
    pub spans: Vec<String>,
}

/// A line in a list of keys or sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    pub id: i64,
    pub key: String,
    pub title: String,
    pub children: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hit {
    pub book: u8,
    pub chapter: u16,
    pub verse: u16,
    pub text: String,
}

/// An open module.
pub struct Module {
    db: Connection,
    pub manifest: Manifest,
    pub integrity: Option<Report>,
}

impl Module {
    pub fn open(path: &Path) -> Result<Module, String> {
        let uri = format!("file:{}?immutable=1", path.to_str().ok_or("module path is not valid UTF-8")?);
        let db = Connection::open_with_flags(
            uri,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
        let id: i64 = db
            .query_row("PRAGMA application_id", [], |r| r.get(0))
            .map_err(|_| format!("{} is not a GRT Bible module", path.display()))?;
        if id != APPLICATION_ID {
            return Err(format!("{} is not a GRT Bible module", path.display()));
        }
        let mut stmt = db.prepare("SELECT key, value FROM meta").map_err(err)?;
        let map: BTreeMap<String, String> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        drop(stmt);
        let manifest = Manifest::from_pairs(&map)?;
        if manifest.schema_version > SCHEMA_VERSION {
            return Err(format!("{} needs a newer GRT Bible (schema {})", path.display(), manifest.schema_version));
        }
        let integrity = map.get("integrity").and_then(|s| serde_json::from_str(s).ok());
        let module = Module { db, manifest, integrity };
        module.check_tables()?;
        Ok(module)
    }

    /// Refuses a file whose tables do not match its kind, before anything is read from it.
    fn check_tables(&self) -> Result<(), String> {
        let needed: &[&str] = if self.manifest.kind == "bible" {
            &["books", "verses", "notes", "titles", "verses_fts"]
        } else {
            &["entries", "spans", "links"]
        };
        for table in needed {
            let found = self
                .db
                .query_row("SELECT 1 FROM sqlite_master WHERE name = ?1", params![table], |_| Ok(()))
                .optional()
                .map_err(err)?;
            if found.is_none() {
                return Err(format!("the module has no '{table}' table"));
            }
        }
        Ok(())
    }

    pub fn kind(&self) -> &str {
        &self.manifest.kind
    }

    /// Whether SQLite finds the file sound.
    pub fn quick_check(&self) -> Result<(), String> {
        let answer: String = self.db.query_row("PRAGMA quick_check", [], |r| r.get(0)).map_err(err)?;
        if answer == "ok" {
            Ok(())
        } else {
            Err(format!("the module file is damaged: {answer}"))
        }
    }

    /// The tagged form of a verse, for modules that keep one.
    pub fn tagged(&self, book: u8, chapter: u16, verse: u16) -> Result<Option<String>, String> {
        let has = self
            .db
            .query_row("SELECT 1 FROM sqlite_master WHERE name = 'tagged'", [], |_| Ok(()))
            .optional()
            .map_err(err)?;
        if has.is_none() {
            return Ok(None);
        }
        self.db
            .query_row(
                "SELECT markup FROM tagged WHERE book_id = ?1 AND chapter = ?2 AND verse = ?3",
                params![book, chapter, verse],
                |r| r.get(0),
            )
            .optional()
            .map_err(err)
    }

    pub fn entry_count(&self) -> Result<u32, String> {
        if self.kind() == "bible" {
            return Ok(0);
        }
        let table = if self.kind() == "crossrefs" { "links" } else { "entries" };
        self.db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).map_err(err)
    }

    fn spans_of(&self, id: i64) -> Result<Vec<String>, String> {
        let mut stmt = self.db.prepare_cached("SELECT first, last FROM spans WHERE entry = ?1 ORDER BY first").map_err(err)?;
        let rows = stmt
            .query_map(params![id], |r| Ok(range_osis(unpack(r.get(0)?), unpack(r.get(1)?))))
            .map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    fn entry_rows(&self, sql: &str, args: impl rusqlite::Params) -> Result<Vec<EntryRow>, String> {
        let mut stmt = self.db.prepare_cached(sql).map_err(err)?;
        let rows: Vec<(i64, Option<i64>, String, String, String)> = stmt
            .query_map(args, |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        rows.into_iter()
            .map(|(id, parent, key, title, body)| Ok(EntryRow { id, parent, key, title, body, spans: self.spans_of(id)? }))
            .collect()
    }

    pub fn entry(&self, id: i64) -> Result<Option<EntryRow>, String> {
        Ok(self.entry_rows("SELECT id, parent, key, title, body FROM entries WHERE id = ?1", params![id])?.pop())
    }

    /// Commentary entries about a verse, with the introductions of its chapter and book.
    pub fn entries_at(&self, r: Ref) -> Result<Vec<EntryRow>, String> {
        let v = pack(r);
        let chapter = pack(Ref::new(r.book, r.chapter, 0));
        let whole = pack(Ref::new(r.book, 0, 0));
        self.entry_rows(
            "SELECT id, parent, key, title, body FROM entries WHERE id IN (
                 SELECT entry FROM spans WHERE (first BETWEEN ?1 AND ?2 AND last >= ?2) OR first = ?3 OR first = ?4)
             ORDER BY id",
            params![whole, v, chapter, whole],
        )
    }

    /// The first entry about a verse at or after the given one, to step through a commentary.
    pub fn next_span(&self, after: Ref, forward: bool) -> Result<Option<String>, String> {
        let sql = if forward {
            "SELECT first FROM spans WHERE first > ?1 AND first % 1000 > 0 ORDER BY first LIMIT 1"
        } else {
            "SELECT first FROM spans WHERE first < ?1 AND first % 1000 > 0 ORDER BY first DESC LIMIT 1"
        };
        let found: Option<i64> = self.db.query_row(sql, params![pack(after)], |r| r.get(0)).optional().map_err(err)?;
        Ok(found.map(|n| unpack(n).osis()))
    }

    /// Headwords starting with the folded text, in the order of the module.
    pub fn lookup(&self, text: &str, limit: u32) -> Result<Vec<Heading>, String> {
        let folded = fold(text.trim(), &self.manifest.language);
        let mut stmt = self
            .db
            .prepare_cached(
                "SELECT id, key, title FROM entries WHERE fold >= ?1 AND fold < ?1 || char(1114111) ORDER BY fold, id LIMIT ?2",
            )
            .map_err(err)?;
        let rows = stmt
            .query_map(params![folded, limit], |r| Ok(Heading { id: r.get(0)?, key: r.get(1)?, title: r.get(2)?, children: 0 }))
            .map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    /// The entry whose headword is exactly this, ignoring case and accents.
    pub fn entry_by_key(&self, key: &str) -> Result<Option<EntryRow>, String> {
        let folded = fold(key.trim(), &self.manifest.language);
        Ok(self.entry_rows("SELECT id, parent, key, title, body FROM entries WHERE fold = ?1 ORDER BY id LIMIT 1", params![folded])?.pop())
    }

    /// Sections of a book under a parent, or at the top with `None`.
    pub fn children(&self, parent: Option<i64>) -> Result<Vec<Heading>, String> {
        let mut stmt = self
            .db
            .prepare_cached(
                "SELECT e.id, e.key, e.title, (SELECT COUNT(*) FROM entries c WHERE c.parent = e.id)
                 FROM entries e WHERE e.parent IS ?1 ORDER BY e.id",
            )
            .map_err(err)?;
        let rows = stmt
            .query_map(params![parent], |r| Ok(Heading { id: r.get(0)?, key: r.get(1)?, title: r.get(2)?, children: r.get(3)? }))
            .map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    /// Cross-references whose source includes a verse, in the order of the module.
    pub fn links_at(&self, r: Ref) -> Result<Vec<LinkGroup>, String> {
        let v = pack(r);
        let mut stmt = self
            .db
            .prepare_cached(
                "SELECT first, last, label, targets FROM links WHERE first BETWEEN ?1 AND ?2 AND last >= ?2 ORDER BY position",
            )
            .map_err(err)?;
        let rows: Vec<(i64, i64, String, String)> = stmt
            .query_map(params![pack(Ref::new(r.book, 0, 0)), v], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        Ok(rows
            .into_iter()
            .map(|(first, last, label, targets)| LinkGroup {
                first: unpack(first),
                last: unpack(last),
                label,
                targets: targets.split_whitespace().filter_map(crate::refs::parse_range).collect(),
            })
            .collect())
    }

    pub fn books(&self) -> Result<Vec<BookInfo>, String> {
        let mut stmt = self.db.prepare("SELECT id, osis, name, abbrev, chapters FROM books ORDER BY position").map_err(err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(BookInfo { index: r.get(0)?, osis: r.get(1)?, name: r.get(2)?, abbrev: r.get(3)?, chapters: r.get(4)? })
            })
            .map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    pub fn chapter(&self, book: u8, chapter: u16) -> Result<Chapter, String> {
        let mut out = Chapter::default();
        let mut stmt = self
            .db
            .prepare_cached("SELECT chapter, verse, text, paragraph FROM verses WHERE book_id = ?1 AND chapter = ?2 ORDER BY verse")
            .map_err(err)?;
        out.verses = stmt
            .query_map(params![book, chapter], |r| {
                Ok(VerseRow { chapter: r.get(0)?, verse: r.get(1)?, text: r.get(2)?, paragraph: r.get::<_, i32>(3)? != 0 })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        let mut stmt = self
            .db
            .prepare_cached("SELECT chapter, verse, kind, text FROM titles WHERE book_id = ?1 AND chapter = ?2 ORDER BY verse, rowid")
            .map_err(err)?;
        out.titles = stmt
            .query_map(params![book, chapter], |r| {
                Ok(TitleRow { chapter: r.get(0)?, verse: r.get(1)?, kind: r.get(2)?, text: r.get(3)? })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        let mut stmt = self
            .db
            .prepare_cached("SELECT chapter, verse, COALESCE(marker, ''), text FROM notes WHERE book_id = ?1 AND chapter = ?2 ORDER BY verse, rowid")
            .map_err(err)?;
        out.notes = stmt
            .query_map(params![book, chapter], |r| {
                Ok(NoteRow { chapter: r.get(0)?, verse: r.get(1)?, marker: r.get(2)?, text: r.get(3)? })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        Ok(out)
    }

    pub fn verse_numbers(&self, book: u8, chapter: u16) -> Result<Vec<u16>, String> {
        let mut stmt = self
            .db
            .prepare_cached("SELECT verse FROM verses WHERE book_id = ?1 AND chapter = ?2 ORDER BY verse")
            .map_err(err)?;
        let rows = stmt.query_map(params![book, chapter], |r| r.get(0)).map_err(err)?;
        rows.collect::<Result<_, _>>().map_err(err)
    }

    pub fn verse(&self, book: u8, chapter: u16, verse: u16) -> Result<Option<String>, String> {
        self.db
            .query_row(
                "SELECT text FROM verses WHERE book_id = ?1 AND chapter = ?2 AND verse = ?3",
                params![book, chapter, verse],
                |r| r.get(0),
            )
            .optional()
            .map_err(err)
    }

    /// Verses matching a folded FTS5 query, in canon order.
    pub fn search(&self, fts: &str, books_filter: &[u8], limit: u32, offset: u32) -> Result<(u32, Vec<Hit>), String> {
        let filter = if books_filter.is_empty() {
            String::new()
        } else {
            format!(
                " AND v.book_id IN ({})",
                books_filter.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",")
            )
        };
        let count_sql = format!(
            "SELECT COUNT(*) FROM verses_fts f JOIN verses v ON v.rowid = f.rowid WHERE verses_fts MATCH ?1{filter}"
        );
        let total: u32 = self.db.query_row(&count_sql, params![fts], |r| r.get(0)).map_err(err)?;
        let sql = format!(
            "SELECT v.book_id, v.chapter, v.verse, v.text FROM verses_fts f JOIN verses v ON v.rowid = f.rowid
             WHERE verses_fts MATCH ?1{filter} ORDER BY v.book_id, v.chapter, v.verse LIMIT ?2 OFFSET ?3"
        );
        let mut stmt = self.db.prepare(&sql).map_err(err)?;
        let hits = stmt
            .query_map(params![fts, limit, offset], |r| {
                Ok(Hit { book: r.get(0)?, chapter: r.get(1)?, verse: r.get(2)?, text: r.get(3)? })
            })
            .map_err(err)?
            .collect::<Result<_, _>>()
            .map_err(err)?;
        Ok((total, hits))
    }

    /// Whether a book is in the module, by canon index.
    pub fn has_book(&self, index: u8) -> bool {
        self.db
            .query_row("SELECT 1 FROM books WHERE id = ?1", params![index], |_| Ok(()))
            .optional()
            .ok()
            .flatten()
            .is_some()
    }
}

/// Canon books a module does not contain, as OSIS names.
pub fn missing_books(content: &BTreeMap<u8, BookContent>) -> Vec<String> {
    books().iter().filter(|b| !content.contains_key(&b.index)).map(|b| b.osis.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assemble::{assemble_resource, Description};

    fn r(text: &str) -> Ref {
        Ref::parse(text).unwrap()
    }

    fn description() -> Description {
        Description {
            id: "test".into(),
            abbreviation: "TST".into(),
            title: "Test".into(),
            language: "en".into(),
            versification: "english".into(),
            license: "Public domain".into(),
            ..Default::default()
        }
    }

    fn written(kind: &str, entries: Vec<EntryContent>, links: Vec<LinkGroup>) -> (tempfile::TempDir, Module) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("TST.grtb");
        let content = assemble_resource(&description(), kind, entries, links, vec!["GreekDef".into()], "2026-10-27", "");
        write(&path, &content).unwrap();
        let again = dir.path().join("again.grtb");
        write(&again, &content).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), std::fs::read(&again).unwrap(), "same content, same bytes");
        let module = Module::open(&path).unwrap();
        module.quick_check().unwrap();
        (dir, module)
    }

    #[test]
    fn packed_references_keep_their_order() {
        assert_eq!(unpack(pack(r("Ps.119.176"))), r("Ps.119.176"));
        assert!(pack(r("Gen.50.26")) < pack(r("Exod.1.1")));
        assert!(pack(r("Ps.9.21")) < pack(r("Ps.10.1")));
    }

    #[test]
    fn a_commentary_answers_by_verse() {
        let entry = |spans: Vec<(Ref, Ref)>, body: &str| EntryContent { body: body.into(), spans, ..Default::default() };
        let intro = Ref::new(r("John.1.1").book, 1, 0);
        let (_dir, m) = written("commentary", vec![
            entry(vec![(intro, intro)], "<p>Chapter.</p>"),
            entry(vec![(r("John.1.1"), r("John.1.5"))], "<p>Word.</p>"),
            entry(vec![(r("John.1.6"), r("John.1.8"))], "<p>John.</p>"),
        ], vec![]);
        assert_eq!(m.kind(), "commentary");
        assert_eq!(m.manifest.features, ["GreekDef"]);
        let found: Vec<String> = m.entries_at(r("John.1.3")).unwrap().into_iter().map(|e| e.body).collect();
        assert_eq!(found, ["<p>Chapter.</p>", "<p>Word.</p>"]);
        assert_eq!(m.entries_at(r("John.1.3")).unwrap()[1].spans, ["John.1.1-John.1.5"]);
        assert_eq!(m.next_span(r("John.1.3"), true).unwrap().as_deref(), Some("John.1.6"));
        assert_eq!(m.next_span(r("John.1.6"), false).unwrap().as_deref(), Some("John.1.1"));
        assert!(m.entries_at(r("Gen.1.1")).unwrap().is_empty());
        assert_eq!(m.entry_count().unwrap(), 3);
    }

    #[test]
    fn a_dictionary_answers_by_headword() {
        let entry = |key: &str| EntryContent { key: key.into(), title: key.into(), body: format!("<p>{key}</p>"), ..Default::default() };
        let (_dir, m) = written("dictionary", vec![entry("Aaron"), entry("Aaronites"), entry("Abba"), entry("Élie")], vec![]);
        let keys = |q: &str| m.lookup(q, 10).unwrap().into_iter().map(|h| h.key).collect::<Vec<_>>();
        assert_eq!(keys("aar"), ["Aaron", "Aaronites"]);
        assert_eq!(keys("elie"), ["Élie"]);
        assert_eq!(keys("").len(), 4);
        assert_eq!(m.entry_by_key("AARON").unwrap().unwrap().body, "<p>Aaron</p>");
    }

    #[test]
    fn a_book_answers_by_section() {
        let entry = |parent: Option<u32>, title: &str| EntryContent { parent, title: title.into(), body: String::new(), ..Default::default() };
        let (_dir, m) = written("book", vec![entry(None, "Part"), entry(Some(0), "One"), entry(Some(0), "Two"), entry(None, "End")], vec![]);
        let top = m.children(None).unwrap();
        assert_eq!(top.iter().map(|h| (h.title.as_str(), h.children)).collect::<Vec<_>>(), [("Part", 2), ("End", 0)]);
        let inner = m.children(Some(top[0].id)).unwrap();
        assert_eq!(inner.iter().map(|h| h.title.as_str()).collect::<Vec<_>>(), ["One", "Two"]);
    }

    #[test]
    fn cross_references_answer_by_verse() {
        let group = |at: &str, label: &str, to: &[&str]| LinkGroup {
            first: r(at),
            last: r(at),
            label: label.into(),
            targets: to.iter().map(|t| crate::refs::parse_range(t).unwrap()).collect(),
        };
        let (_dir, m) = written("crossrefs", vec![], vec![
            group("Gen.1.1", "beginning", &["Prov.8.22-Prov.8.24", "John.1.1"]),
            group("Gen.1.1", "God", &["Exod.20.11"]),
            group("Gen.1.2", "without", &["Job.26.7"]),
        ]);
        let at = m.links_at(r("Gen.1.1")).unwrap();
        assert_eq!(at.iter().map(|g| g.label.as_str()).collect::<Vec<_>>(), ["beginning", "God"]);
        assert_eq!(at[0].targets[0], (r("Prov.8.22"), r("Prov.8.24")));
        assert_eq!(m.entry_count().unwrap(), 3);
        assert_eq!(m.manifest.books_present, 1);
    }

    #[test]
    fn an_unknown_kind_is_refused() {
        let mut map = BTreeMap::new();
        for (k, v) in [("kind", "atlas"), ("id", "x"), ("abbreviation", "X"), ("title", "X"), ("language", "en"), ("canon", ""), ("versification", ""), ("license", ""), ("books_present", "0"), ("source_notes", "false"), ("schema_version", "2")] {
            map.insert(k.to_string(), v.to_string());
        }
        assert!(Manifest::from_pairs(&map).unwrap_err().contains("atlas"));
        map.remove("kind");
        assert_eq!(Manifest::from_pairs(&map).unwrap().kind, "bible");
    }
}
