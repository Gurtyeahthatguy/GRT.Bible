//! What a module is missing compared with the canon.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::canon::books;
use crate::module::BookContent;
use crate::refs::Ref;
use crate::versification::Scheme;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub books_present: u32,
    pub missing_books: Vec<String>,
    /// Canon chapters no verse of the module reaches, per book.
    pub unreached_chapters: BTreeMap<String, Vec<u16>>,
    /// Verse numbers skipped inside a chapter, as `chapter.verse`.
    pub verse_gaps: BTreeMap<String, Vec<String>>,
    /// Verses whose mapping falls outside the canon.
    pub unmapped: Vec<String>,
}

impl Report {
    pub fn is_complete(&self) -> bool {
        self.missing_books.is_empty() && self.unreached_chapters.is_empty() && self.verse_gaps.is_empty()
    }
}

pub fn check(content: &BTreeMap<u8, BookContent>, scheme: &Scheme) -> Report {
    let mut report = Report::default();
    for b in books() {
        let Some(body) = content.get(&b.index) else {
            report.missing_books.push(b.osis.to_string());
            continue;
        };
        report.books_present += 1;

        let mut reached = BTreeSet::new();
        let mut by_chapter: BTreeMap<u16, Vec<u16>> = BTreeMap::new();
        for v in &body.verses {
            by_chapter.entry(v.chapter).or_default().push(v.verse);
            let (start, _) = scheme.to_canon(Ref::new(b.index, v.chapter, v.verse));
            if start.in_canon() && start.book == b.index {
                reached.insert(start.chapter);
            } else if !start.in_canon() {
                report.unmapped.push(Ref::new(b.index, v.chapter, v.verse).osis());
            }
        }
        let unreached: Vec<u16> = (1..=b.chapters()).filter(|c| !reached.contains(c)).collect();
        if !unreached.is_empty() {
            report.unreached_chapters.insert(b.osis.to_string(), unreached);
        }

        let mut gaps = Vec::new();
        let last_chapter = by_chapter.keys().max().copied().unwrap_or(0);
        for c in 1..=last_chapter {
            match by_chapter.get(&c) {
                None => gaps.push(format!("{c}")),
                Some(verses) => {
                    let present: BTreeSet<u16> = verses.iter().copied().collect();
                    let top = present.iter().max().copied().unwrap_or(0);
                    gaps.extend((1..top).filter(|v| !present.contains(v)).map(|v| format!("{c}.{v}")));
                }
            }
        }
        if !gaps.is_empty() {
            report.verse_gaps.insert(b.osis.to_string(), gaps);
        }
    }
    report
}

/// How badly a scheme fits a module, lower being better.
pub fn misfit(content: &BTreeMap<u8, BookContent>, scheme: &Scheme) -> usize {
    let mut score = 0;
    for (index, body) in content {
        let mut targets: BTreeMap<u16, BTreeSet<u16>> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let mut previous: Option<Ref> = None;
        for v in &body.verses {
            let (start, end) = scheme.to_canon(Ref::new(*index, v.chapter, v.verse));
            if !start.in_canon() || start.book != *index {
                score += 1;
                continue;
            }
            if !seen.insert(start) {
                score += 1;
            }
            if previous.map(|p| start < p).unwrap_or(false) {
                score += 1;
            }
            previous = Some(start);
            for verse in start.verse..=end.verse {
                targets.entry(start.chapter).or_default().insert(verse);
            }
        }
        for verses in targets.into_values() {
            let (Some(&low), Some(&top)) = (verses.iter().min(), verses.iter().max()) else { continue };
            score += (low..top).filter(|v| !verses.contains(v)).count();
        }
    }
    score
}

/// General schemes ordered from best to worst fit.
pub fn rank_schemes(content: &BTreeMap<u8, BookContent>) -> Vec<(&'static str, usize)> {
    let mut ranked: Vec<(&'static str, usize)> = crate::versification::GENERAL_SCHEMES
        .iter()
        .map(|name| (*name, misfit(content, crate::versification::scheme(name))))
        .collect();
    ranked.sort_by_key(|(_, score)| *score);
    ranked
}

/// A readable summary for a person deciding whether to keep a module.
pub fn summary(report: &Report) -> String {
    let mut lines = vec![format!("{} of 73 books.", report.books_present)];
    if !report.missing_books.is_empty() {
        lines.push(format!("Missing: {}.", report.missing_books.join(", ")));
    }
    if !report.unreached_chapters.is_empty() {
        let parts: Vec<String> = report
            .unreached_chapters
            .iter()
            .map(|(osis, chapters)| {
                let name = crate::canon::book_by_osis(osis).map(|b| b.names[0].0).unwrap_or(osis);
                format!("{name} {}", chapters.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","))
            })
            .collect();
        lines.push(format!("Chapters without text: {}.", parts.join("; ")));
    }
    let gaps: usize = report.verse_gaps.values().map(Vec::len).sum();
    if gaps > 0 {
        lines.push(format!("{gaps} verse numbers skipped."));
    }
    lines.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::VerseRow;

    fn verse(chapter: u16, verse: u16) -> VerseRow {
        VerseRow { chapter, verse, text: "x".into(), paragraph: false }
    }

    #[test]
    fn missing_books_and_gaps() {
        let mut content = BTreeMap::new();
        let jonah = crate::canon::book_by_osis("Jonah").unwrap();
        let mut body = BookContent::default();
        for c in 1..=4u16 {
            for v in 1..=jonah.verse_count(c) {
                if !(c == 2 && v == 3) {
                    body.verses.push(verse(c, v));
                }
            }
        }
        content.insert(jonah.index, body);
        let report = check(&content, crate::versification::scheme("hebrew"));
        assert_eq!(report.books_present, 1);
        assert_eq!(report.missing_books.len(), 72);
        assert_eq!(report.verse_gaps["Jonah"], vec!["2.3".to_string()]);
        assert!(!report.unreached_chapters.contains_key("Jonah"));
    }

    #[test]
    fn english_numbering_is_recognised() {
        let mut content = BTreeMap::new();
        let mal = crate::canon::book_by_osis("Mal").unwrap();
        let mut body = BookContent::default();
        for (c, n) in [(1u16, 14u16), (2, 17), (3, 18), (4, 6)] {
            for v in 1..=n {
                body.verses.push(verse(c, v));
            }
        }
        content.insert(mal.index, body);
        let psalms = crate::canon::book_by_osis("Ps").unwrap();
        let mut body = BookContent::default();
        for v in 1..=19 {
            body.verses.push(verse(51, v));
        }
        content.insert(psalms.index, body);
        assert_eq!(rank_schemes(&content)[0].0, "english");
    }
}
