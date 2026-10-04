//! The 73 books of the Catholic canon, in Nova Vulgata order.

use std::sync::OnceLock;

const BOOKS_TSV: &str = include_str!("../data/books.tsv");
const CANON_TXT: &str = include_str!("../data/canon.txt");

/// Languages with their own book names.
pub const NAME_LANGUAGES: [&str; 3] = ["en", "it", "la"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    /// Position in the canon, from 1.
    pub index: u8,
    pub osis: &'static str,
    pub testament: &'static str,
    pub deutero: bool,
    /// Name and abbreviation for each of `NAME_LANGUAGES`.
    pub names: [(&'static str, &'static str); 3],
    /// Verse count of each chapter in the internal scheme.
    pub verses: Vec<u16>,
}

impl Book {
    pub fn chapters(&self) -> u16 {
        self.verses.len() as u16
    }

    pub fn verse_count(&self, chapter: u16) -> u16 {
        if chapter == 0 {
            return 0;
        }
        self.verses.get(chapter as usize - 1).copied().unwrap_or(0)
    }

    /// Name and abbreviation in a language, Latin for the languages that have none.
    pub fn name_in(&self, language: &str) -> (&'static str, &'static str) {
        let key = match language {
            "en" => 0,
            "it" => 1,
            _ => 2,
        };
        self.names[key]
    }
}

fn load() -> Vec<Book> {
    let mut books = Vec::with_capacity(73);
    let mut counts = CANON_TXT.lines().filter(|l| !l.trim().is_empty());
    for (i, line) in BOOKS_TSV.lines().skip(1).filter(|l| !l.trim().is_empty()).enumerate() {
        let f: Vec<&'static str> = line.split('\t').collect();
        let count_line = counts.next().expect("canon.txt has fewer lines than books.tsv");
        let mut parts = count_line.split_whitespace();
        let osis = parts.next().unwrap();
        assert_eq!(osis, f[0], "canon.txt and books.tsv disagree on order");
        books.push(Book {
            index: (i + 1) as u8,
            osis: f[0],
            testament: f[1],
            deutero: f[2] == "1",
            names: [(f[3], f[4]), (f[5], f[6]), (f[7], f[8])],
            verses: parts.map(|n| n.parse().unwrap()).collect(),
        });
    }
    books
}

pub fn books() -> &'static [Book] {
    static BOOKS: OnceLock<Vec<Book>> = OnceLock::new();
    BOOKS.get_or_init(load)
}

pub fn book(index: u8) -> Option<&'static Book> {
    if index == 0 {
        return None;
    }
    books().get(index as usize - 1)
}

pub fn book_by_osis(osis: &str) -> Option<&'static Book> {
    books().iter().find(|b| b.osis == osis)
}

/// The canon position of a USFM book code, for books that live in the canon.
pub fn osis_for_usfm(code: &str) -> Option<&'static str> {
    Some(match code {
        "GEN" => "Gen", "EXO" => "Exod", "LEV" => "Lev", "NUM" => "Num", "DEU" => "Deut",
        "JOS" => "Josh", "JDG" => "Judg", "RUT" => "Ruth", "1SA" => "1Sam", "2SA" => "2Sam",
        "1KI" => "1Kgs", "2KI" => "2Kgs", "1CH" => "1Chr", "2CH" => "2Chr", "EZR" => "Ezra",
        "NEH" => "Neh", "TOB" => "Tob", "JDT" => "Jdt", "EST" | "ESG" => "Esth",
        "1MA" => "1Macc", "2MA" => "2Macc", "JOB" => "Job", "PSA" => "Ps", "PRO" => "Prov",
        "ECC" => "Eccl", "SNG" => "Song", "WIS" => "Wis", "SIR" => "Sir", "ISA" => "Isa",
        "JER" => "Jer", "LAM" => "Lam", "BAR" => "Bar", "EZK" => "Ezek", "DAN" | "DAG" => "Dan",
        "HOS" => "Hos", "JOL" => "Joel", "AMO" => "Amos", "OBA" => "Obad", "JON" => "Jonah",
        "MIC" => "Mic", "NAM" => "Nah", "HAB" => "Hab", "ZEP" => "Zeph", "HAG" => "Hag",
        "ZEC" => "Zech", "MAL" => "Mal", "MAT" => "Matt", "MRK" => "Mark", "LUK" => "Luke",
        "JHN" => "John", "ACT" => "Acts", "ROM" => "Rom", "1CO" => "1Cor", "2CO" => "2Cor",
        "GAL" => "Gal", "EPH" => "Eph", "PHP" => "Phil", "COL" => "Col", "1TH" => "1Thess",
        "2TH" => "2Thess", "1TI" => "1Tim", "2TI" => "2Tim", "TIT" => "Titus", "PHM" => "Phlm",
        "HEB" => "Heb", "JAS" => "Jas", "1PE" => "1Pet", "2PE" => "2Pet", "1JN" => "1John",
        "2JN" => "2John", "3JN" => "3John", "JUD" => "Jude", "REV" => "Rev",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seventy_three_books() {
        let all = books();
        assert_eq!(all.len(), 73);
        assert_eq!(all.iter().filter(|b| b.testament == "OT").count(), 46);
        assert_eq!(all.iter().filter(|b| b.deutero).count(), 7);
    }

    #[test]
    fn catholic_structure() {
        assert_eq!(book_by_osis("Dan").unwrap().chapters(), 14);
        assert_eq!(book_by_osis("Dan").unwrap().verse_count(3), 100);
        assert_eq!(book_by_osis("Esth").unwrap().chapters(), 16);
        assert_eq!(book_by_osis("Bar").unwrap().chapters(), 6);
        assert_eq!(book_by_osis("Ps").unwrap().chapters(), 150);
        assert_eq!(book_by_osis("Mal").unwrap().chapters(), 3);
    }

    #[test]
    fn names_by_language() {
        let john = book_by_osis("John").unwrap();
        assert_eq!(john.name_in("it"), ("Giovanni", "Gv"));
        assert_eq!(john.name_in("la"), ("Ioannes", "Io"));
        assert_eq!(john.name_in("grc"), ("Ioannes", "Io"));
        assert_eq!(john.name_in("en").1, "John");
    }
}
