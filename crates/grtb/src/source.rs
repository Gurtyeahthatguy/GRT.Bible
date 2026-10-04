//! Reading source files and fitting their books into the canon.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::canon::{book_by_osis, osis_for_usfm};
use crate::module::BookContent;
use crate::usfm::{self, UsfmBook};

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Text from bytes that are normally UTF-8 and occasionally Latin-1.
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.trim_start_matches('\u{feff}').to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

fn is_usfm_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    !lower.contains("__macosx") && (lower.ends_with(".usfm") || lower.ends_with(".sfm"))
}

#[derive(Debug, Default)]
pub struct Loaded {
    pub books: Vec<UsfmBook>,
    pub hash: String,
    pub warnings: Vec<String>,
}

/// USFM from a single file, a folder of files, or a ZIP archive of them.
pub fn read_usfm(path: &Path) -> Result<Loaded, String> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let hash;
    if path.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_usfm_name(&p.to_string_lossy()))
            .collect();
        entries.sort();
        let mut hasher = Sha256::new();
        for p in entries {
            let bytes = std::fs::read(&p).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            hasher.update(name.as_bytes());
            hasher.update([0]);
            hasher.update(&bytes);
            files.push((name, bytes));
        }
        hash = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    } else {
        let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        hash = sha256_hex(&bytes);
        if bytes.starts_with(b"PK") {
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
            let mut names: Vec<String> = archive.file_names().filter(|n| is_usfm_name(n)).map(str::to_string).collect();
            names.sort();
            for name in names {
                let mut entry = archive.by_name(&name).map_err(|e| e.to_string())?;
                let mut data = Vec::new();
                entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
                files.push((name, data));
            }
        } else {
            files.push((path.file_name().unwrap_or_default().to_string_lossy().to_string(), bytes));
        }
    }
    if files.is_empty() {
        return Err(format!("no USFM files in {}", path.display()));
    }
    let mut loaded = Loaded { hash, ..Default::default() };
    for (name, bytes) in files {
        match usfm::parse(&decode(&bytes)) {
            Ok(book) => loaded.books.push(book),
            Err(e) => loaded.warnings.push(format!("{name}: {e}")),
        }
    }
    Ok(loaded)
}

fn shift_chapters(mut body: BookContent, chapter: Option<u16>, offset: i32) -> BookContent {
    let fix = |c: u16| match chapter {
        Some(fixed) => fixed,
        None => (c as i32 + offset) as u16,
    };
    for v in &mut body.verses {
        v.chapter = fix(v.chapter);
    }
    for t in &mut body.titles {
        t.chapter = fix(t.chapter);
    }
    for n in &mut body.notes {
        n.chapter = fix(n.chapter);
    }
    body
}

fn take_chapters(body: &BookContent, keep: impl Fn(u16) -> bool) -> BookContent {
    BookContent {
        verses: body.verses.iter().filter(|v| keep(v.chapter)).cloned().collect(),
        titles: body.titles.iter().filter(|t| keep(t.chapter)).cloned().collect(),
        notes: body.notes.iter().filter(|n| keep(n.chapter)).cloned().collect(),
    }
}

fn append(target: &mut BookContent, extra: BookContent) {
    target.verses.extend(extra.verses);
    target.titles.extend(extra.titles);
    target.notes.extend(extra.notes);
}

/// Fits source books into the 73-book canon and its chapter layout.
pub fn arrange(books: Vec<UsfmBook>) -> (BTreeMap<u8, BookContent>, Vec<String>) {
    let mut warnings = Vec::new();
    let by_code: BTreeMap<String, BookContent> = books.into_iter().map(|b| (b.code, b.content)).collect();
    let has = |code: &str| by_code.contains_key(code);
    let mut out: BTreeMap<u8, BookContent> = BTreeMap::new();

    for (code, body) in &by_code {
        let code = code.as_str();
        let skip_short = (code == "EST" && has("ESG")) || (code == "DAN" && has("DAG"));
        if skip_short {
            continue;
        }
        let Some(osis) = osis_for_usfm(code) else {
            if !matches!(code, "SUS" | "BEL" | "LJE" | "FRT" | "BAK" | "OTH" | "INT" | "CNC" | "GLO" | "TDX" | "NDX" | "XXA" | "XXB" | "XXC" | "XXD" | "XXE" | "XXF" | "XXG") {
                warnings.push(format!("{code} is not in the Catholic canon and was left out"));
            }
            continue;
        };
        let index = book_by_osis(osis).unwrap().index;
        if code == "EZR" && !has("NEH") && body.chapters() > 10 {
            out.insert(index, take_chapters(body, |c| c <= 10));
            let neh = take_chapters(body, |c| c > 10);
            out.insert(book_by_osis("Neh").unwrap().index, shift_chapters(neh, None, -10));
            continue;
        }
        out.insert(index, body.clone());
    }

    let dan = book_by_osis("Dan").unwrap().index;
    for (code, chapter) in [("SUS", 13u16), ("BEL", 14u16)] {
        if let Some(body) = by_code.get(code) {
            let target = out.entry(dan).or_default();
            if target.verses.iter().any(|v| v.chapter == chapter) {
                warnings.push(format!("{code} left out: Daniel already has chapter {chapter}"));
            } else {
                append(target, shift_chapters(body.clone(), Some(chapter), 0));
            }
        }
    }
    if let Some(body) = by_code.get("LJE") {
        let bar = book_by_osis("Bar").unwrap().index;
        let target = out.entry(bar).or_default();
        if target.verses.iter().any(|v| v.chapter == 6) {
            warnings.push("LJE left out: Baruch already has chapter 6".into());
        } else {
            append(target, shift_chapters(body.clone(), Some(6), 0));
        }
    }
    if by_code.contains_key("S3Y") {
        warnings.push("S3Y left out: the Song of the Three is expected inside Daniel 3".into());
    }
    for body in out.values_mut() {
        body.tidy();
    }
    out.retain(|_, body| !body.verses.is_empty());
    (out, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(text: &str) -> UsfmBook {
        usfm::parse(text).unwrap()
    }

    #[test]
    fn greek_additions_move_into_place() {
        let books = vec![
            book("\\id DAG\n\\c 12\n\\p\n\\v 1 Michael.\n"),
            book("\\id SUS\n\\c 1\n\\p\n\\v 1 Susanna.\n"),
            book("\\id BEL\n\\c 1\n\\p\n\\v 1 Bel.\n"),
            book("\\id BAR\n\\c 5\n\\p\n\\v 1 Baruch.\n"),
            book("\\id LJE\n\\c 1\n\\p\n\\v 1 Letter.\n"),
            book("\\id 1ES\n\\c 1\n\\p\n\\v 1 Esdras A.\n"),
        ];
        let (out, warnings) = arrange(books);
        let dan = &out[&book_by_osis("Dan").unwrap().index];
        let chapters: Vec<u16> = dan.verses.iter().map(|v| v.chapter).collect();
        assert_eq!(chapters, vec![12, 13, 14]);
        let bar = &out[&book_by_osis("Bar").unwrap().index];
        assert_eq!(bar.verses.last().unwrap().chapter, 6);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn esdras_b_is_split() {
        let mut text = String::from("\\id EZR\n");
        for c in 1..=23 {
            text.push_str(&format!("\\c {c}\n\\p\n\\v 1 Chapter {c}.\n"));
        }
        let (out, _) = arrange(vec![book(&text)]);
        assert_eq!(out[&book_by_osis("Ezra").unwrap().index].chapters(), 10);
        let neh = &out[&book_by_osis("Neh").unwrap().index];
        assert_eq!(neh.chapters(), 13);
        assert_eq!(neh.verses[0].text, "Chapter 11.");
    }
}
