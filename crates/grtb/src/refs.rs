//! Verse references, written in the OSIS style: `Ps.23.1`.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::canon::{book, book_by_osis};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ref {
    pub book: u8,
    pub chapter: u16,
    pub verse: u16,
}

impl Ref {
    pub const fn new(book: u8, chapter: u16, verse: u16) -> Self {
        Ref { book, chapter, verse }
    }

    pub fn parse(text: &str) -> Option<Ref> {
        let mut parts = text.trim().split('.');
        let osis = parts.next()?;
        let chapter = parts.next()?.parse().ok()?;
        let verse = parts.next().map(|v| v.parse().ok()).unwrap_or(Some(1))?;
        if parts.next().is_some() || chapter == 0 {
            return None;
        }
        Some(Ref { book: book_by_osis(osis)?.index, chapter, verse })
    }

    pub fn osis(&self) -> String {
        let name = book(self.book).map(|b| b.osis).unwrap_or("?");
        format!("{}.{}.{}", name, self.chapter, self.verse)
    }

    /// Whether the verse exists in the internal scheme.
    pub fn in_canon(&self) -> bool {
        book(self.book).map(|b| self.verse >= 1 && self.verse <= b.verse_count(self.chapter)).unwrap_or(false)
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.osis())
    }
}

impl Serialize for Ref {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.osis())
    }
}

impl<'de> Deserialize<'de> for Ref {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Ref::parse(&text).ok_or_else(|| serde::de::Error::custom(format!("not a reference: {text}")))
    }
}

/// `Ps.23.1-6`, or a single verse, within one chapter.
pub fn parse_range(text: &str) -> Option<(Ref, Ref)> {
    let (head, tail) = match text.split_once('-') {
        Some((h, t)) => (h, Some(t)),
        None => (text, None),
    };
    let start = Ref::parse(head)?;
    let end = match tail {
        None => start,
        Some(t) if t.contains('.') => Ref::parse(t)?,
        Some(t) => Ref { verse: t.trim().parse().ok()?, ..start },
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let r = Ref::parse("Sir.24.3").unwrap();
        assert_eq!(r.osis(), "Sir.24.3");
        assert!(r.in_canon());
        assert_eq!(Ref::parse("Dan.13.45").unwrap().osis(), "Dan.13.45");
    }

    #[test]
    fn rejects_nonsense() {
        assert!(Ref::parse("Xyz.1.1").is_none());
        assert!(Ref::parse("Ps.0.1").is_none());
        assert!(Ref::parse("Ps.1.x").is_none());
        assert!(!Ref::parse("Ps.151.1").unwrap().in_canon());
    }

    #[test]
    fn ranges() {
        let (a, b) = parse_range("Ps.23.1-6").unwrap();
        assert_eq!((a.verse, b.verse), (1, 6));
        let (a, b) = parse_range("John.3.16-John.4.2").unwrap();
        assert_eq!((a.chapter, b.chapter), (3, 4));
        assert!(parse_range("Ps.23.6-1").is_none());
    }
}
