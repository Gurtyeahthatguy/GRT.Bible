//! The `.grtb` module: one read-only SQLite file per translation.

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::canon::{book, books};
use crate::integrity::Report;
use crate::text::fold;

pub const SCHEMA_VERSION: u32 = 1;
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
";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
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
}

impl Manifest {
    fn pairs(&self) -> Vec<(&'static str, String)> {
        vec![
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
        ]
    }

    fn from_pairs(map: &BTreeMap<String, String>) -> Result<Manifest, String> {
        let get = |k: &str| map.get(k).cloned().ok_or_else(|| format!("module has no '{k}' in meta"));
        Ok(Manifest {
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookContent {
    pub verses: Vec<VerseRow>,
    pub titles: Vec<TitleRow>,
    pub notes: Vec<NoteRow>,
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
    }
}

/// Everything needed to write a module, books keyed by canon index.
#[derive(Debug, Clone)]
pub struct ModuleContent {
    pub manifest: Manifest,
    pub books: BTreeMap<u8, BookContent>,
    pub integrity: Option<Report>,
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
        db.execute_batch(SCHEMA).map_err(err)?;
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

        let language = content.manifest.language.as_str();
        for (index, body) in &content.books {
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
        }
        tx.execute("INSERT INTO verses_fts (verses_fts) VALUES ('optimize')", []).map_err(err)?;
        tx.commit().map_err(err)?;
        let target = path.to_str().ok_or("module path is not valid UTF-8")?;
        db.execute("VACUUM INTO ?1", params![target]).map_err(err)?;
    }
    std::fs::remove_file(&staging).map_err(err)?;
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
        Ok(Module { db, manifest, integrity })
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
