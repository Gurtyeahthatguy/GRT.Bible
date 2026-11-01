//! The SBL Greek New Testament with MorphGNT's parsing: one text file per book,
//! one word per line, and a lexeme list that gives each lemma its Strong's number.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use grtb::canon::book_by_osis;
use grtb::module::{BookContent, VerseRow, WordRow};

const BOOKS: [&str; 27] = [
    "Matt", "Mark", "Luke", "John", "Acts", "Rom", "1Cor", "2Cor", "Gal", "Eph", "Phil", "Col", "1Thess", "2Thess",
    "1Tim", "2Tim", "Titus", "Phlm", "Heb", "Jas", "1Pet", "2Pet", "1John", "2John", "3John", "Jude", "Rev",
];

/// Strong's number and gloss of each lemma, from `lexemes.yaml`.
pub fn lexemes(text: &str) -> HashMap<String, (String, String)> {
    let mut out = HashMap::new();
    let mut lemma: Option<String> = None;
    let mut strong = String::new();
    let mut gloss = String::new();
    let mut flush = |lemma: &mut Option<String>, strong: &mut String, gloss: &mut String| {
        if let Some(l) = lemma.take() {
            out.insert(l, (std::mem::take(strong), std::mem::take(gloss)));
        }
    };
    for line in text.lines() {
        if !line.starts_with(' ') && line.ends_with(':') {
            flush(&mut lemma, &mut strong, &mut gloss);
            lemma = Some(line.trim_end_matches(':').to_string());
        } else if let Some(v) = line.trim().strip_prefix("strongs:") {
            let first = v.trim().trim_start_matches('[').split([',', ']']).next().unwrap_or("").trim();
            if let Ok(n) = first.parse::<u32>() {
                strong = format!("G{n}");
            }
        } else if let Some(v) = line.trim().strip_prefix("gloss:") {
            gloss = v.trim().trim_matches('"').to_string();
        }
    }
    flush(&mut lemma, &mut strong, &mut gloss);
    out
}

pub fn parse(dir: &Path, lexemes: &HashMap<String, (String, String)>) -> Result<BTreeMap<u8, BookContent>, String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with("-morphgnt.txt"))
        .collect();
    files.sort();
    let mut out = BTreeMap::new();
    for path in files {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut body = BookContent::default();
        let mut book = None;
        for (number, line) in text.lines().enumerate() {
            let f: Vec<&str> = line.split(' ').collect();
            if f.len() != 7 || f[0].len() != 6 {
                return Err(format!("{}:{}: expected seven fields", path.display(), number + 1));
            }
            let index: usize = f[0][..2].parse().map_err(|_| format!("{}:{}: bad book", path.display(), number + 1))?;
            let osis = BOOKS.get(index.wrapping_sub(1)).ok_or_else(|| format!("{}: book {index}", path.display()))?;
            book = Some(book_by_osis(osis).expect("New Testament books are in the canon").index);
            let chapter: u16 = f[0][2..4].parse().map_err(|_| "bad chapter")?;
            let verse: u16 = f[0][4..6].parse().map_err(|_| "bad verse")?;
            let (strong, gloss) = lexemes.get(f[6]).cloned().unwrap_or_default();
            // The signs of SBLGNT's apparatus mean nothing without it.
            let printed: String = f[3].chars().filter(|c| !('\u{2E00}'..='\u{2E0F}').contains(c)).collect();
            body.words.push(WordRow {
                chapter,
                verse,
                text: printed.clone(),
                lemma: f[6].to_string(),
                strong,
                gloss,
                xlit: String::new(),
                morph: format!("mgnt:{}{}", f[1], f[2]),
                main: 0,
            });
            match body.verses.last_mut() {
                Some(v) if (v.chapter, v.verse) == (chapter, verse) => {
                    v.text.push(' ');
                    v.text.push_str(&printed);
                }
                _ => body.verses.push(VerseRow { chapter, verse, text: printed, paragraph: verse == 1 }),
            }
        }
        if let Some(b) = book {
            body.tidy();
            out.insert(b, body);
        }
    }
    if out.is_empty() {
        return Err(format!("no *-morphgnt.txt files in {}", dir.display()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_book_with_its_lexemes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("64-Jn-morphgnt.txt"),
            "040101 P- -------- Ἐν Ἐν ἐν ἐν\n040101 N- ----DSF- ἀρχῇ ἀρχῇ ἀρχῇ ἀρχή\n040101 V- 3IAI-S-- ἦν, ἦν ἦν εἰμί\n040102 RD ----NSM- οὗτος οὗτος οὗτος οὗτος\n",
        )
        .unwrap();
        let lex = lexemes("ἀρχή:\n    pos: N\n    strongs: 746\n    gloss: beginning\nεἰμί:\n    strongs: [1510, 1511]\n    gloss: \"to be\"\n");
        let books = parse(dir.path(), &lex).unwrap();
        let john = &books[&book_by_osis("John").unwrap().index];
        assert_eq!(john.verses[0].text, "Ἐν ἀρχῇ ἦν,");
        assert_eq!(john.words.len(), 4);
        assert_eq!((john.words[1].strong.as_str(), john.words[1].gloss.as_str()), ("G746", "beginning"));
        assert_eq!(john.words[2].strong, "G1510");
        assert_eq!(john.words[2].morph, "mgnt:V-3IAI-S--");
        assert_eq!(john.words[2].gloss, "to be");
    }
}
