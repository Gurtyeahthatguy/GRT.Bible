//! Passages written by people and by other programs: "Pr 8:22-24; 16:4",
//! "Gen.1.1-Gen.1.3". The result is in the numbering of the source; the
//! caller maps it into the internal scheme.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::canon::{book, book_by_osis, books};
use crate::refs::Ref;

/// Short forms used by SWORD modules and printed references, beyond the names in `books.tsv`.
const ALIASES: &str = "
Gen ge gn
Exod ex exo
Lev le lv
Num nu nm nb
Deut de dt
Josh jos
Judg jdg jud jg
Ruth ru rth
1Sam 1sa 1s 1sm
2Sam 2sa 2s 2sm
1Kgs 1ki 1kg 1k
2Kgs 2ki 2kg 2k
1Chr 1ch
2Chr 2ch
Ezra ezr
Neh ne
Tob tb
Jdt jth jdth
Esth es est
1Macc 1ma 1mac
2Macc 2ma 2mac
Job jb
Ps psa psalm pss
Prov pr pro prv
Eccl ec ecc qoh
Song so sos ss cant canticles songofsolomon canticleofcanticles
Wis ws
Sir ecclus ecclesiasticus
Isa is
Jer je
Lam la
Bar ba
Ezek eze ezk
Dan da dn
Hos ho
Joel joe jl
Amos am
Obad ob
Jonah jon jnh
Mic mi
Nah na
Hab hb
Zeph zep zp
Hag hg
Zech zec zc
Mal ml
Matt mt mat
Mark mr mk mar
Luke lu lk luk
John joh jn jhn
Acts ac
Rom ro rm
1Cor 1co
2Cor 2co
Gal ga
Eph ep
Phil php phi pp
Col co
1Thess 1th 1thes
2Thess 2th 2thes
1Tim 1ti
2Tim 2ti
Titus ti tt
Phlm phm philem
Heb he
Jas jam jm
1Pet 1pe 1pt
2Pet 2pe 2pt
1John 1jo 1jn
2John 2jo 2jn
3John 3jo 3jn
Jude jd
Rev re rv apoc apocalypse revelationofjohn
";

/// Books outside the canon's list that live inside a canon book, with the chapter they become.
fn inside_canon(osis: &str) -> Option<(&'static str, u16)> {
    Some(match osis {
        "EpJer" => ("Bar", 6),
        "Sus" => ("Dan", 13),
        "Bel" => ("Dan", 14),
        _ => return None,
    })
}

fn normal(name: &str) -> String {
    let trimmed = name.trim().trim_end_matches('.');
    let mut words = trimmed.split_whitespace();
    let first = words.next().unwrap_or("");
    let lead = match first.to_ascii_lowercase().as_str() {
        "i" if trimmed.len() > 2 => "1".to_string(),
        "ii" => "2".to_string(),
        "iii" => "3".to_string(),
        other => other.to_string(),
    };
    let mut out = lead;
    for w in words {
        out.push_str(&w.to_lowercase());
    }
    out.chars().filter(|c| c.is_alphanumeric()).collect()
}

struct Names {
    exact: HashMap<String, u8>,
    /// Full names in canon order, for a prefix of one.
    full: Vec<(String, u8)>,
}

fn names() -> &'static Names {
    static N: OnceLock<Names> = OnceLock::new();
    N.get_or_init(|| {
        let mut exact = HashMap::new();
        let mut full = Vec::new();
        for b in books() {
            exact.insert(b.osis.to_lowercase(), b.index);
            for (name, abbrev) in b.names {
                exact.entry(normal(name)).or_insert(b.index);
                exact.entry(normal(abbrev)).or_insert(b.index);
                full.push((normal(name), b.index));
            }
        }
        for line in ALIASES.lines() {
            let mut parts = line.split_whitespace();
            let Some(osis) = parts.next() else { continue };
            let index = book_by_osis(osis).expect("aliases name canon books").index;
            for alias in parts {
                exact.insert(alias.to_string(), index);
            }
        }
        Names { exact, full }
    })
}

/// The canon book a name or abbreviation stands for.
pub fn book_from_name(name: &str) -> Option<u8> {
    let key = normal(name);
    if key.is_empty() {
        return None;
    }
    if let Some(i) = names().exact.get(&key) {
        return Some(*i);
    }
    if key.len() < 3 {
        return None;
    }
    names().full.iter().find(|(n, _)| n.starts_with(&key)).map(|(_, i)| *i)
}

fn whole_chapter(b: u8, chapter: u16) -> (Ref, Ref) {
    let last = book(b).map(|k| k.verse_count(chapter)).filter(|&n| n > 0).unwrap_or(1);
    (Ref::new(b, chapter, 1), Ref::new(b, chapter, last))
}

fn whole_book(b: u8) -> (Ref, Ref) {
    let k = book(b).expect("canon book");
    let chapters = k.chapters();
    (Ref::new(b, 1, 1), Ref::new(b, chapters, k.verse_count(chapters).max(1)))
}

/// Where the reference before the next one left off.
#[derive(Debug, Clone, Copy, Default)]
pub struct Context {
    pub book: Option<u8>,
    pub chapter: Option<u16>,
}

fn number(text: &str) -> Option<u16> {
    let digits: String = text.trim().chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok().filter(|&n| n > 0)
}

/// `c:v`, `c.v` or `n`, as chapter and optional verse.
fn point(text: &str) -> Option<(u16, Option<u16>)> {
    let text = text.trim();
    match text.split_once([':', '.']) {
        Some((c, v)) => Some((number(c)?, Some(number(v)?))),
        None => Some((number(text)?, None)),
    }
}

/// Splits "1 Co 8:6" into the book words and the numbers after them.
fn split_book(segment: &str) -> (&str, &str) {
    let s = segment.trim_start();
    let mut chars = s.char_indices().peekable();
    let mut start_of_numbers = s.len();
    // A leading 1, 2 or 3 belongs to the book when letters follow it.
    if let Some(&(_, c)) = chars.peek() {
        if matches!(c, '1' | '2' | '3') {
            let rest = s[1..].trim_start();
            if rest.starts_with(|ch: char| ch.is_alphabetic()) {
                chars.next();
            }
        }
    }
    let mut seen_letter = false;
    for (i, c) in chars {
        if c.is_alphabetic() {
            seen_letter = true;
        } else if c.is_ascii_digit() && seen_letter {
            start_of_numbers = i;
            break;
        } else if c.is_ascii_digit() {
            return ("", s);
        }
    }
    if !seen_letter {
        return ("", s);
    }
    (&s[..start_of_numbers], &s[start_of_numbers..])
}

/// References written in words, as SWORD modules and printed books write them.
pub fn parse_words(text: &str, mut ctx: Context) -> Vec<(Ref, Ref)> {
    let mut out = Vec::new();
    let text = text.replace(['\u{2013}', '\u{2014}'], "-");
    for segment in text.split(';') {
        let (name, numbers) = split_book(segment);
        let mut explicit_book = false;
        if !name.trim().is_empty() {
            match book_from_name(name) {
                Some(b) => {
                    ctx.book = Some(b);
                    ctx.chapter = None;
                    explicit_book = true;
                }
                None => continue,
            }
        }
        let Some(b) = ctx.book else { continue };
        if numbers.trim().is_empty() {
            if explicit_book {
                out.push(whole_book(b));
            }
            continue;
        }
        let mut verse_mode = !explicit_book && ctx.chapter.is_some();
        for piece in numbers.split(',') {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            let (head, tail) = match piece.split_once('-') {
                Some((h, t)) => (h, Some(t)),
                None => (piece, None),
            };
            let Some((first, first_verse)) = point(head) else { continue };
            let start = match first_verse {
                Some(v) => {
                    verse_mode = true;
                    ctx.chapter = Some(first);
                    Ref::new(b, first, v)
                }
                None if verse_mode => match ctx.chapter {
                    Some(c) => Ref::new(b, c, first),
                    None => continue,
                },
                None => {
                    // A chapter, or a run of chapters.
                    ctx.chapter = Some(first);
                    let last = tail.and_then(number).filter(|&n| n >= first).unwrap_or(first);
                    out.push((whole_chapter(b, first).0, whole_chapter(b, last).1));
                    ctx.chapter = Some(last);
                    continue;
                }
            };
            let end = match tail.map(point) {
                None => start,
                Some(Some((c, Some(v)))) => {
                    ctx.chapter = Some(c);
                    Ref::new(b, c, v)
                }
                Some(Some((v, None))) => Ref::new(b, start.chapter, v),
                Some(None) => start,
            };
            if end >= start {
                out.push((start, end));
            }
        }
    }
    out
}

fn osis_point(text: &str, default_book: Option<u8>) -> Option<(u8, Option<u16>, Option<u16>)> {
    let text = text.split('!').next().unwrap_or("");
    let parts: Vec<&str> = text.split('.').collect();
    let starts_with_book = parts.first().map(|p| p.chars().any(|c| c.is_alphabetic())).unwrap_or(false);
    let (b, numbers) = if starts_with_book {
        let name = parts[0];
        if let Some((host, chapter)) = inside_canon(name) {
            let b = book_by_osis(host)?.index;
            let verse = parts.get(2).and_then(|v| v.parse().ok());
            return Some((b, Some(chapter), verse));
        }
        (book_by_osis(name).map(|k| k.index).or_else(|| book_from_name(name))?, &parts[1..])
    } else {
        (default_book?, &parts[..])
    };
    let chapter = numbers.first().and_then(|c| c.parse().ok()).filter(|&c: &u16| c > 0);
    let verse = numbers.get(1).and_then(|v| v.parse().ok()).filter(|&v: &u16| v > 0);
    Some((b, chapter, verse))
}

/// An `osisRef` attribute: one or more ranges such as `Gen.1.1-Gen.1.3`, maybe with a work prefix.
pub fn parse_osis(text: &str) -> Vec<(Ref, Ref)> {
    let mut out = Vec::new();
    for item in text.split_whitespace() {
        let item = match item.split_once(':') {
            Some((work, rest)) if !work.contains('.') => rest,
            _ => item,
        };
        let (head, tail) = match item.split_once('-') {
            Some((h, t)) => (h, Some(t)),
            None => (item, None),
        };
        let Some((b, chapter, verse)) = osis_point(head, None) else { continue };
        let (start, natural_end) = match (chapter, verse) {
            (None, _) => whole_book(b),
            (Some(c), None) => whole_chapter(b, c),
            (Some(c), Some(v)) => (Ref::new(b, c, v), Ref::new(b, c, v)),
        };
        let end = match tail {
            None => natural_end,
            Some(t) if !t.chars().any(|c| c.is_alphabetic()) && !t.contains('.') => match verse {
                Some(_) => t.parse().ok().map(|v| Ref::new(b, start.chapter, v)).unwrap_or(natural_end),
                None => t.parse().ok().map(|c| whole_chapter(b, c).1).unwrap_or(natural_end),
            },
            Some(t) => match osis_point(t, Some(b)) {
                Some((eb, Some(c), Some(v))) => Ref::new(eb, c, v),
                Some((eb, Some(c), None)) => whole_chapter(eb, c).1,
                Some((eb, None, _)) => whole_book(eb).1,
                None => natural_end,
            },
        };
        if end >= start {
            out.push((start, end));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(ranges: &[(Ref, Ref)]) -> Vec<String> {
        ranges.iter().map(|(a, b)| crate::module::range_osis(*a, *b)).collect()
    }

    #[test]
    fn sword_abbreviations() {
        for (name, osis) in [
            ("Ge", "Gen"), ("Jud", "Judg"), ("Jude", "Jude"), ("Mr", "Mark"), ("Joh", "John"), ("Php", "Phil"),
            ("1Jo", "1John"), ("Re", "Rev"), ("Song of Solomon", "Song"), ("II Kings", "2Kgs"), ("I Samuel", "1Sam"),
            ("Ecclus", "Sir"), ("Sirach", "Sir"), ("Giovanni", "John"), ("Ioannes", "John"), ("Revelation of John", "Rev"),
            ("Deuteronomy", "Deut"), ("Lamentations", "Lam"), ("Hebr", "Heb"),
        ] {
            assert_eq!(book_from_name(name).and_then(book).map(|b| b.osis), Some(osis), "{name}");
        }
        assert!(book_from_name("See").is_none());
        assert!(book_from_name("x").is_none());
    }

    #[test]
    fn treasury_lists() {
        let ctx = Context { book: book_by_osis("Gen").map(|b| b.index), chapter: Some(1) };
        assert_eq!(
            show(&parse_words("Pr 8:22-24; 16:4; Mr 13:19; Joh 1:1-3; Heb 1:10; 1Jo 1:1", ctx)),
            ["Prov.8.22-Prov.8.24", "Prov.16.4", "Mark.13.19", "John.1.1-John.1.3", "Heb.1.10", "1John.1.1"]
        );
        assert_eq!(show(&parse_words("Ps 33:6,9", ctx)), ["Ps.33.6", "Ps.33.9"]);
        assert_eq!(show(&parse_words("33:15-17,26", Context { book: book_by_osis("Jer").map(|b| b.index), chapter: None })), ["Jer.33.15-Jer.33.17", "Jer.33.26"]);
        assert_eq!(show(&parse_words("26", ctx)), ["Gen.1.26"]);
        assert_eq!(show(&parse_words("2:21-25; 5:2", ctx)), ["Gen.2.21-Gen.2.25", "Gen.5.2"]);
        assert_eq!(show(&parse_words("Lu 3:23-4:2", ctx)), ["Luke.3.23-Luke.4.2"]);
        assert_eq!(show(&parse_words("Ps 23", ctx)), ["Ps.23.1-Ps.23.6"]);
        assert_eq!(show(&parse_words("Jude 1:3", ctx)), ["Jude.1.3"]);
        assert!(parse_words("See below", ctx).is_empty());
    }

    #[test]
    fn osis_attributes() {
        assert_eq!(show(&parse_osis("Bible:Rev.1.8")), ["Rev.1.8"]);
        assert_eq!(show(&parse_osis("Exod.6.16-Exod.6.20 Josh.21.4")), ["Exod.6.16-Exod.6.20", "Josh.21.4"]);
        assert_eq!(show(&parse_osis("Gen.1.1-3")), ["Gen.1.1-Gen.1.3"]);
        assert_eq!(show(&parse_osis("Ps.23")), ["Ps.23.1-Ps.23.6"]);
        assert_eq!(show(&parse_osis("Sus.1.45")), ["Dan.13.45"]);
        assert_eq!(show(&parse_osis("John.3.16!a")), ["John.3.16"]);
        assert!(parse_osis("Nowhere.1.1").is_empty());
    }
}
