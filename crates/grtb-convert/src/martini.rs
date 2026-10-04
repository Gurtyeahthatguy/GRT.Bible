//! Martini's translation from the digital edition's RTF export.

use std::collections::BTreeMap;

use grtb::canon::books;
use grtb::module::{BookContent, TitleRow, VerseRow};

use crate::rtf::{events, Event};

pub struct Parsed {
    pub books: BTreeMap<u8, BookContent>,
    pub warnings: Vec<String>,
}

fn marker(text: &str) -> Vec<(u8, u16, u16)> {
    let mut out = Vec::new();
    for part in text.split('_').skip(1) {
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() == 8 {
            let b: u8 = digits[..2].parse().unwrap();
            let c: u16 = digits[2..5].parse().unwrap();
            let v: u16 = digits[5..].parse().unwrap();
            out.push((b, c, v));
        }
    }
    out
}

/// Slips in the digital edition: book, chapter, verse, found, meant.
const CORRECTIONS: &[(&str, u16, u16, &str, &str)] = &[
    ("Gen", 5, 19, "800 anni", "ottocento anni"),
    ("Exod", 32, 23, "noi noi sappiamo", "noi nol sappiamo"),
    ("2Chr", 18, 15, "vo]te", "volte"),
    ("Tob", 2, 22, "la la moglie", "la moglie"),
    ("Jdt", 9, 16, "ti ti compiaci", "ti compiaci"),
    ("1Macc", 5, 46, "a a destra", "a destra"),
    ("Ps", 23, 5, "su« Salvatore", "suo Salvatore"),
    ("Ps", 65, 7, "lo lo date", "lo date"),
    ("Ps", 117, 1, "Date lode al perché egli Signore,è buono", "Date lode al Signore, perché egli è buono"),
    ("Wis", 10, 1, "fu fu formato", "fu formato"),
    ("Sir", 11, 11, "si da da fare", "si dà da fare"),
    ("Sir", 29, 32, "e da da mangiare", "e dà da mangiare"),
    ("Sir", 29, 33, "da da mangiare", "dà da mangiare"),
    ("Sir", 41, 17, "i i documenti", "i documenti"),
    ("Isa", 16, 8, "deserti,è", "deserti, è"),
    ("John", 7, 35, "noi noi troveremo", "noi nol troveremo"),
    ("John", 9, 21, "noi noi sappiamo", "noi nol sappiamo"),
    ("Acts", 26, 16, "e di di quelle", "e di quelle"),
    ("2Cor", 5, 8, "di di partirci", "di partirci"),
];

pub fn correct(books: &mut BTreeMap<u8, BookContent>, warnings: &mut Vec<String>) {
    for (osis, chapter, verse, found, meant) in CORRECTIONS {
        let index = grtb::canon::book_by_osis(osis).map(|b| b.index).unwrap_or(0);
        let hit = books
            .get_mut(&index)
            .and_then(|body| body.verses.iter_mut().find(|v| v.chapter == *chapter && v.verse == *verse))
            .filter(|v| v.text.contains(found));
        match hit {
            Some(v) => v.text = v.text.replacen(found, meant, 1),
            None => warnings.push(format!("correction for {osis}.{chapter}.{verse} no longer applies")),
        }
    }
}

/// Tidies spacing and quotes the way the printed edition has them.
pub fn clean(text: &str) -> String {
    let mut s: String = text
        .replace('\u{a0}', " ")
        .replace(['’', '‘'], "'")
        .split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    for (from, to) in [(" ,", ","), (" ;", ";"), (" :", ":"), (" .", "."), (" ?", "?"), (" !", "!"), ("( ", "("), (" )", ")"), (",,", ",")] {
        while s.contains(from) {
            s = s.replace(from, to);
        }
    }
    s
}

pub fn parse(bytes: &[u8]) -> Result<Parsed, String> {
    let canon = books();
    let mut out: BTreeMap<u8, BookContent> = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut current: Option<(u8, usize)> = None;
    let mut summary = String::new();
    let mut paragraph_next = true;
    let mut pending_break = false;
    let mut label_open = false;

    let flush_summary = |out: &mut BTreeMap<u8, BookContent>, current: Option<(u8, usize)>, summary: &mut String| {
        let text = clean(summary);
        summary.clear();
        if let (Some((b, i)), false) = (current, text.is_empty()) {
            let body = out.get_mut(&b).unwrap();
            let chapter = body.verses[i].chapter;
            body.titles.push(TitleRow { chapter, verse: body.verses[i].verse, kind: "summary".into(), text });
        }
    };

    for event in events(bytes) {
        match event {
            Event::Paragraph => {
                if let Some((b, i)) = current {
                    if !out[&b].verses[i].text.trim().is_empty() {
                        pending_break = true;
                    }
                }
                if !summary.is_empty() {
                    summary.push(' ');
                }
            }
            Event::Text { text, hidden: true, .. } => {
                for (b, c, v) in marker(&text) {
                    let Some(book) = canon.get((b as usize).wrapping_sub(1)) else {
                        warnings.push(format!("bookmark for book {b} outside the canon"));
                        continue;
                    };
                    flush_summary(&mut out, current, &mut summary);
                    if pending_break || v == 1 {
                        paragraph_next = true;
                    }
                    let body = out.entry(book.index).or_default();
                    body.verses.push(VerseRow { chapter: c, verse: v, text: String::new(), paragraph: paragraph_next });
                    current = Some((book.index, body.verses.len() - 1));
                    paragraph_next = false;
                    pending_break = false;
                    label_open = true;
                }
            }
            Event::Text { text, italic: true, bold: false, .. } if label_open => {
                summary.push_str(&text);
            }
            Event::Text { text, bold: true, .. } if label_open => {
                // The printed verse number, or the "Gen 1:1" label on a first verse.
                flush_summary(&mut out, current, &mut summary);
                let rest = text.trim_start();
                let label_end = rest
                    .char_indices()
                    .find(|(_, ch)| !(ch.is_alphanumeric() || *ch == ':' || *ch == ' '))
                    .map(|(i, _)| i)
                    .unwrap_or(rest.len());
                let tail = &rest[label_end..];
                label_open = false;
                if let (Some((b, i)), false) = (current, tail.trim().is_empty()) {
                    out.get_mut(&b).unwrap().verses[i].text.push_str(tail);
                }
            }
            Event::Text { text, .. } => {
                if label_open && text.trim().is_empty() {
                    continue;
                }
                flush_summary(&mut out, current, &mut summary);
                label_open = false;
                if let Some((b, i)) = current {
                    let verse = &mut out.get_mut(&b).unwrap().verses[i];
                    if pending_break && !text.trim().is_empty() {
                        verse.text.push('\n');
                        pending_break = false;
                    }
                    verse.text.push_str(&text);
                }
            }
        }
    }

    for (index, body) in out.iter_mut() {
        for v in &mut body.verses {
            v.text = clean(&v.text);
            if v.text.is_empty() {
                let osis = canon[*index as usize - 1].osis;
                warnings.push(format!("{osis}.{}.{} has no text", v.chapter, v.verse));
            }
        }
        body.verses.retain(|v| !v.text.is_empty());
        let before = body.verses.len();
        body.tidy();
        if body.verses.len() != before {
            warnings.push(format!("{} repeated verse bookmarks dropped in {}", before - body.verses.len(), canon[*index as usize - 1].osis));
        }
    }
    if out.len() != 73 {
        return Err(format!("expected 73 books, found {}", out.len()));
    }
    Ok(Parsed { books: out, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fragment of Psalms, with one bookmark for every other book so the count check passes.
    fn sample() -> Vec<u8> {
        let mut rtf = br"{\rtf1{\v _23050001}{\i Piange l'adulterio.}\par{\b Sal 50:1} Salmo di Davidde.\par Abbi misericordia di me. {\v _23050002}{\b 2} E secondo le molte operazioni.\par{\v _23051001}{\b Sal 51:1} Perch\'e9 ti glorii ?".to_vec();
        for b in (1..=73u8).filter(|&b| b != 23) {
            rtf.extend_from_slice(format!("{{\\v _{b:02}001001}}{{\\b X 1:1}} x\\par ").as_bytes());
        }
        rtf.push(b'}');
        rtf
    }

    #[test]
    fn verse_labels_summaries_and_psalm_titles() {
        let parsed = parse(&sample()).unwrap();
        let ps = &parsed.books[&23];
        assert_eq!(ps.verses[0].text, "Salmo di Davidde.\nAbbi misericordia di me.");
        assert!(ps.verses[0].paragraph);
        assert_eq!(ps.verses[1].text, "E secondo le molte operazioni.");
        assert!(!ps.verses[1].paragraph);
        assert_eq!(ps.verses[2].text, "Perché ti glorii?");
        assert_eq!(ps.titles[0].text, "Piange l'adulterio.");
        assert_eq!(ps.titles[0].kind, "summary");
    }

    #[test]
    fn spacing_is_tidied() {
        assert_eq!(clean("Signore ,  ( dice ) l\u{2019}uomo ;"), "Signore, (dice) l'uomo;");
    }
}
