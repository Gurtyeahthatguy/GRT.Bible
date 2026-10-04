//! The installed modules and what can be read from them.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use grtb::canon::{book, book_by_osis};
use grtb::module::{BookInfo, Module, NoteRow, TitleRow};
use grtb::refs::Ref;
use grtb::versification::{scheme, Scheme};
use serde::Serialize;

/// Bundled modules in the order they are offered.
const PREFERRED: [&str; 8] = ["VUL", "MAR", "WEBC", "DRA", "RIV", "LXX", "N1904", "WLC"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    pub abbreviation: String,
    pub id: String,
    pub title: String,
    pub language: String,
    pub direction: String,
    pub year: Option<i32>,
    pub versification: String,
    pub license: String,
    pub source: String,
    pub generated_at: String,
    pub bundled: bool,
    pub has_notes: bool,
    pub books: Vec<BookInfo>,
    pub missing_books: Vec<String>,
    pub summary: String,
}

pub struct Entry {
    pub info: ModuleInfo,
    pub path: PathBuf,
    module: Mutex<Module>,
}

impl Entry {
    pub fn with<T>(&self, f: impl FnOnce(&Module) -> T) -> T {
        let guard = self.module.lock().unwrap_or_else(|e| e.into_inner());
        f(&guard)
    }

    pub fn scheme(&self) -> &'static Scheme {
        scheme(&self.info.versification)
    }

    pub fn book_info(&self, index: u8) -> Option<&BookInfo> {
        self.info.books.iter().find(|b| b.index == index)
    }
}

#[derive(Default)]
pub struct Library {
    entries: Vec<Arc<Entry>>,
}

fn open_entry(path: &Path, bundled: bool) -> Result<Entry, String> {
    let module = Module::open(path)?;
    let books = module.books()?;
    let m = &module.manifest;
    let info = ModuleInfo {
        abbreviation: m.abbreviation.clone(),
        id: m.id.clone(),
        title: m.title.clone(),
        language: m.language.clone(),
        direction: m.direction.clone(),
        year: m.year,
        versification: m.versification.clone(),
        license: m.license.clone(),
        source: m.source.clone(),
        generated_at: m.generated_at.clone(),
        bundled,
        has_notes: m.source_notes,
        books,
        missing_books: m.missing_books.clone(),
        summary: module.integrity.as_ref().map(grtb::integrity::summary).unwrap_or_default(),
    };
    Ok(Entry { info, path: path.to_path_buf(), module: Mutex::new(module) })
}

impl Library {
    /// Opens every module in the given folders; problems are returned, not fatal.
    pub fn load(folders: &[(PathBuf, bool)]) -> (Library, Vec<String>) {
        let mut library = Library::default();
        let mut problems = Vec::new();
        for (folder, bundled) in folders {
            let Ok(read) = std::fs::read_dir(folder) else { continue };
            let mut paths: Vec<PathBuf> = read
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == grtb::module::EXTENSION).unwrap_or(false))
                .collect();
            paths.sort();
            for path in paths {
                match open_entry(&path, *bundled) {
                    Ok(entry) if library.entries.iter().any(|e| e.info.abbreviation == entry.info.abbreviation) => {
                        problems.push(format!("{} skipped: another module is already called {}", path.display(), entry.info.abbreviation));
                    }
                    Ok(entry) => library.entries.push(Arc::new(entry)),
                    Err(e) => problems.push(e),
                }
            }
        }
        library.sort();
        (library, problems)
    }

    fn sort(&mut self) {
        let rank = |e: &Arc<Entry>| PREFERRED.iter().position(|p| *p == e.info.abbreviation).unwrap_or(PREFERRED.len());
        self.entries.sort_by(|a, b| {
            (!a.info.bundled, rank(a), a.info.abbreviation.clone()).cmp(&(!b.info.bundled, rank(b), b.info.abbreviation.clone()))
        });
    }

    pub fn infos(&self) -> Vec<ModuleInfo> {
        self.entries.iter().map(|e| e.info.clone()).collect()
    }

    pub fn get(&self, abbreviation: &str) -> Result<Arc<Entry>, String> {
        self.entries
            .iter()
            .find(|e| e.info.abbreviation == abbreviation)
            .cloned()
            .ok_or_else(|| format!("no module called {abbreviation}"))
    }

    pub fn contains(&self, abbreviation: &str) -> bool {
        self.entries.iter().any(|e| e.info.abbreviation.eq_ignore_ascii_case(abbreviation))
    }

    pub fn add(&mut self, path: &Path) -> Result<ModuleInfo, String> {
        let entry = open_entry(path, false)?;
        let info = entry.info.clone();
        self.entries.push(Arc::new(entry));
        self.sort();
        Ok(info)
    }

    pub fn remove(&mut self, abbreviation: &str) -> Result<PathBuf, String> {
        let entry = self.get(abbreviation)?;
        if entry.info.bundled {
            return Err("modules that come with the program cannot be removed".into());
        }
        self.entries.retain(|e| e.info.abbreviation != abbreviation);
        Ok(entry.path.clone())
    }

    pub fn all(&self) -> Vec<Arc<Entry>> {
        self.entries.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Place {
    pub book: &'static str,
    pub chapter: u16,
    pub verse: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerseView {
    pub verse: u16,
    pub text: String,
    pub paragraph: bool,
    pub canon: String,
    pub canon_end: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterView {
    pub module: String,
    pub book: &'static str,
    pub chapter: u16,
    pub verses: Vec<VerseView>,
    pub titles: Vec<TitleRow>,
    pub notes: Vec<NoteRow>,
    pub prev: Option<Place>,
    pub next: Option<Place>,
    pub canon_first: Option<String>,
    pub canon_last: Option<String>,
}

pub fn book_index(osis: &str) -> Result<u8, String> {
    book_by_osis(osis).map(|b| b.index).ok_or_else(|| format!("unknown book {osis}"))
}

fn osis_of(index: u8) -> &'static str {
    book(index).map(|b| b.osis).unwrap_or("Gen")
}

/// Chapter text with the internal reference of every verse.
pub fn chapter(entry: &Entry, osis: &str, chapter: u16) -> Result<ChapterView, String> {
    let index = book_index(osis)?;
    let info = entry.book_info(index).ok_or_else(|| format!("{} does not contain {osis}", entry.info.abbreviation))?;
    let chapter = chapter.clamp(1, info.chapters.max(1));
    let raw = entry.with(|m| m.chapter(index, chapter))?;
    let s = entry.scheme();
    let verses: Vec<VerseView> = raw
        .verses
        .into_iter()
        .map(|v| {
            let (a, b) = s.to_canon(Ref::new(index, v.chapter, v.verse));
            VerseView { verse: v.verse, text: v.text, paragraph: v.paragraph, canon: a.osis(), canon_end: b.osis() }
        })
        .collect();
    let canon_refs: Vec<Ref> = verses.iter().filter_map(|v| Ref::parse(&v.canon)).filter(Ref::in_canon).collect();
    let books = &entry.info.books;
    let position = books.iter().position(|b| b.index == index).unwrap();
    let prev = if chapter > 1 {
        Some(Place { book: osis_of(index), chapter: chapter - 1, verse: 1 })
    } else if position > 0 {
        let b = &books[position - 1];
        Some(Place { book: osis_of(b.index), chapter: b.chapters, verse: 1 })
    } else {
        None
    };
    let next = if chapter < info.chapters {
        Some(Place { book: osis_of(index), chapter: chapter + 1, verse: 1 })
    } else {
        books.get(position + 1).map(|b| Place { book: osis_of(b.index), chapter: 1, verse: 1 })
    };
    Ok(ChapterView {
        module: entry.info.abbreviation.clone(),
        book: osis_of(index),
        chapter,
        canon_first: canon_refs.iter().min().map(Ref::osis),
        canon_last: canon_refs.iter().max().map(Ref::osis),
        verses,
        titles: raw.titles,
        notes: raw.notes,
        prev,
        next,
    })
}

/// Where an internal reference is shown in a module.
pub fn resolve(entry: &Entry, canon: Ref) -> Option<Place> {
    let info = entry.book_info(canon.book)?;
    let exists = |r: Ref| entry.with(|m| m.verse(r.book, r.chapter, r.verse).ok().flatten().is_some());
    if let Some(r) = entry.scheme().from_canon(canon, &exists) {
        return Some(Place { book: osis_of(r.book), chapter: r.chapter, verse: r.verse });
    }
    Some(Place { book: osis_of(canon.book), chapter: canon.chapter.min(info.chapters), verse: 1 })
}

#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    pub verse: u16,
    pub chapter: u16,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub canon: String,
    pub cells: Vec<Vec<Cell>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParallelView {
    pub modules: Vec<String>,
    pub primary: ChapterView,
    pub rows: Vec<Row>,
}

/// The primary module's chapter, with what each other module has for the same verses.
pub fn parallel(primary: &Entry, others: &[Arc<Entry>], osis: &str, chapter_number: u16) -> Result<ParallelView, String> {
    let view = chapter(primary, osis, chapter_number)?;
    let ranges: Vec<(Ref, Ref)> = view
        .verses
        .iter()
        .map(|v| (Ref::parse(&v.canon).unwrap_or(Ref::new(0, 0, 0)), Ref::parse(&v.canon_end).unwrap_or(Ref::new(0, 0, 0))))
        .collect();
    let mut rows: Vec<Row> = view
        .verses
        .iter()
        .map(|v| Row { canon: v.canon.clone(), cells: vec![vec![Cell { verse: v.verse, chapter: view.chapter, text: v.text.clone() }]] })
        .collect();
    for other in others {
        let column = rows.first().map(|r| r.cells.len()).unwrap_or(1);
        for row in rows.iter_mut() {
            row.cells.push(Vec::new());
        }
        let (Some(first), Some(last)) = (
            view.canon_first.as_deref().and_then(Ref::parse),
            view.canon_last.as_deref().and_then(Ref::parse),
        ) else {
            continue;
        };
        let (Some(from), Some(to)) = (resolve(other, first), resolve(other, last)) else { continue };
        let index = book_index(from.book)?;
        if book_index(to.book)? != index {
            continue;
        }
        let s = other.scheme();
        for c in from.chapter..=to.chapter.max(from.chapter) {
            let raw = other.with(|m| m.chapter(index, c))?;
            for v in raw.verses {
                let (canon, _) = s.to_canon(Ref::new(index, v.chapter, v.verse));
                if canon < first || canon > last {
                    continue;
                }
                let slot = ranges
                    .iter()
                    .position(|(a, b)| canon >= *a && canon <= *b)
                    .or_else(|| ranges.iter().rposition(|(a, _)| *a <= canon))
                    .unwrap_or(0);
                if let Some(row) = rows.get_mut(slot) {
                    row.cells[column].push(Cell { verse: v.verse, chapter: v.chapter, text: v.text });
                }
            }
        }
    }
    let mut modules = vec![primary.info.abbreviation.clone()];
    modules.extend(others.iter().map(|o| o.info.abbreviation.clone()));
    Ok(ParallelView { modules, primary: view, rows })
}

#[derive(Debug, Clone, Serialize)]
pub struct HitView {
    pub module: String,
    pub book: &'static str,
    pub chapter: u16,
    pub verse: u16,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchView {
    pub total: u32,
    pub hits: Vec<HitView>,
    pub terms: Vec<String>,
}

pub fn search(entries: &[Arc<Entry>], query: &str, books: &[u8], offset: u32, limit: u32) -> Result<SearchView, String> {
    let limit = limit.clamp(1, 200);
    let mut total = 0u32;
    let mut hits = Vec::new();
    let mut skip = offset;
    let mut terms: Vec<String> = Vec::new();
    for entry in entries {
        let language = entry.info.language.clone();
        let Some(fts) = grtb::text::fts_query(query, &language) else { continue };
        for word in grtb::text::fold(query, &language).split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
            if !terms.iter().any(|t| t == word) {
                terms.push(word.to_string());
            }
        }
        let want = limit.saturating_sub(hits.len() as u32);
        let (count, found) = entry.with(|m| m.search(&fts, books, want.max(1), skip))?;
        total += count;
        if skip >= count {
            skip -= count;
            continue;
        }
        skip = 0;
        if want == 0 {
            continue;
        }
        for h in found {
            hits.push(HitView { module: entry.info.abbreviation.clone(), book: osis_of(h.book), chapter: h.chapter, verse: h.verse, text: h.text });
        }
    }
    Ok(SearchView { total, hits, terms })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modules_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../modules")
    }

    fn library() -> Library {
        let (library, problems) = Library::load(&[(modules_dir(), true)]);
        assert!(problems.is_empty(), "{problems:?}");
        library
    }

    #[test]
    fn bundled_modules_open_in_order() {
        let names: Vec<String> = library().infos().into_iter().map(|i| i.abbreviation).collect();
        assert_eq!(names, PREFERRED.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    }

    #[test]
    fn psalm_numbering_meets_in_the_internal_scheme() {
        let lib = library();
        let vul = chapter(&lib.get("VUL").unwrap(), "Ps", 22).unwrap();
        assert_eq!(vul.verses[0].canon, "Ps.23.1");
        let webc = lib.get("WEBC").unwrap();
        let place = resolve(&webc, Ref::parse("Ps.51.3").unwrap()).unwrap();
        assert_eq!((place.chapter, place.verse), (51, 1));
        let vul_place = resolve(&lib.get("VUL").unwrap(), Ref::parse("Ps.23.1").unwrap()).unwrap();
        assert_eq!((vul_place.chapter, vul_place.verse), (22, 1));
    }

    #[test]
    fn navigation_skips_missing_books() {
        let lib = library();
        let riv = chapter(&lib.get("RIV").unwrap(), "Neh", 13).unwrap();
        assert_eq!(riv.next.unwrap().book, "Esth");
        let mar = chapter(&lib.get("MAR").unwrap(), "Neh", 13).unwrap();
        assert_eq!(mar.next.unwrap().book, "Tob");
        assert!(chapter(&lib.get("MAR").unwrap(), "Gen", 1).unwrap().prev.is_none());
    }

    #[test]
    fn parallel_aligns_by_internal_reference() {
        let lib = library();
        let view = parallel(&lib.get("WEBC").unwrap(), &[lib.get("VUL").unwrap()], "Ps", 51).unwrap();
        let first = &view.rows[0];
        assert_eq!(first.canon, "Ps.51.3");
        assert!(first.cells[1].iter().any(|c| c.chapter == 50 && c.verse == 3), "{:?}", first.cells[1]);
    }

    #[test]
    fn search_folds_accents() {
        let lib = library();
        let result = search(&[lib.get("MAR").unwrap()], "perche iddio", &[], 0, 10).unwrap();
        assert!(result.total > 0);
        assert!(result.hits[0].text.to_lowercase().contains("perch"));
    }
}
