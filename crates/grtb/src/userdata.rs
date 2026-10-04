//! The reader's own file, with every reference in the internal scheme.

use grt_container::{read_archive, write_archive, Entry, Manifest, Part, README_NAME, README_TEXT};
use serde::{Deserialize, Serialize};

use crate::refs::Ref;

pub const KIND: &str = "bible";
pub const FORMAT_VERSION: u32 = 1;
pub const SCHEME: &str = "hebrew";
const HISTORY_LIMIT: usize = 100;

const PARTS: [&str; 5] = [
    "content/reading.json",
    "content/notes.json",
    "content/highlights.json",
    "content/bookmarks.json",
    "content/videos.json",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub module: String,
    #[serde(rename = "ref")]
    pub reference: Ref,
    /// How far into that verse the top of the window was, from 0 to 1.
    #[serde(default)]
    pub offset: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    #[serde(default)]
    pub current: Option<Position>,
    #[serde(default)]
    pub history: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub start: Ref,
    pub end: Ref,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Highlight {
    pub start: Ref,
    pub end: Ref,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,
    #[serde(rename = "ref")]
    pub reference: Ref,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UserData {
    pub reading: Reading,
    pub notes: Vec<Note>,
    pub highlights: Vec<Highlight>,
    pub bookmarks: Vec<Bookmark>,
    pub saved_videos: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Wrapped<T> {
    version: u32,
    scheme: String,
    #[serde(flatten)]
    body: T,
}

#[derive(Serialize, Deserialize)]
struct NotesBody {
    notes: Vec<Note>,
}

#[derive(Serialize, Deserialize)]
struct HighlightsBody {
    highlights: Vec<Highlight>,
}

#[derive(Serialize, Deserialize)]
struct BookmarksBody {
    bookmarks: Vec<Bookmark>,
}

#[derive(Serialize, Deserialize)]
struct VideosBody {
    saved: Vec<String>,
}

fn json<T: Serialize>(body: T) -> Result<Vec<u8>, String> {
    let wrapped = Wrapped { version: FORMAT_VERSION, scheme: SCHEME.to_string(), body };
    let mut text = serde_json::to_string_pretty(&wrapped).map_err(|e| e.to_string())?;
    text.push('\n');
    Ok(text.into_bytes())
}

impl UserData {
    /// Records a reading position and keeps the history short and free of repeats.
    pub fn visit(&mut self, position: Position) {
        let same_chapter = |a: &Position, b: &Position| {
            a.reference.book == b.reference.book && a.reference.chapter == b.reference.chapter
        };
        self.reading.history.retain(|p| !same_chapter(p, &position));
        self.reading.history.insert(0, position.clone());
        self.reading.history.truncate(HISTORY_LIMIT);
        self.reading.current = Some(position);
    }

    pub fn to_archive(&self) -> Result<Vec<u8>, String> {
        let mut manifest = Manifest::new(KIND);
        for path in PARTS {
            manifest.parts.push(Part { path: path.into(), media_type: "application/json".into() });
        }
        let entries = vec![
            Entry::new(README_NAME, README_TEXT.as_bytes().to_vec()),
            Entry::new("manifest.json", manifest.to_json().map_err(|e| e.to_string())?.into_bytes()),
            Entry::new(PARTS[0], json(&self.reading)?),
            Entry::new(PARTS[1], json(NotesBody { notes: self.notes.clone() })?),
            Entry::new(PARTS[2], json(HighlightsBody { highlights: self.highlights.clone() })?),
            Entry::new(PARTS[3], json(BookmarksBody { bookmarks: self.bookmarks.clone() })?),
            Entry::new(PARTS[4], json(VideosBody { saved: self.saved_videos.clone() })?),
        ];
        write_archive(entries).map_err(|e| e.to_string())
    }

    pub fn from_archive(bytes: &[u8]) -> Result<UserData, String> {
        let entries = read_archive(bytes).map_err(|e| format!("not a readable .grt file: {e}"))?;
        let find = |name: &str| entries.iter().find(|e| e.name == name).map(|e| e.data.as_slice());
        let manifest = find("manifest.json")
            .ok_or("the file has no manifest")
            .and_then(|d| Manifest::from_json(&String::from_utf8_lossy(d)).map_err(|_| "unreadable manifest"))?;
        if manifest.kind != KIND {
            return Err(format!("this is a '{}' file, not GRT Bible data", manifest.kind));
        }
        if manifest.format_version > FORMAT_VERSION {
            return Err("this file was written by a newer GRT Bible".into());
        }
        fn part<T: for<'de> Deserialize<'de>>(data: Option<&[u8]>) -> Result<Option<T>, String> {
            match data {
                None => Ok(None),
                Some(d) => {
                    let w: Wrapped<T> = serde_json::from_slice(d).map_err(|e| e.to_string())?;
                    if w.scheme != SCHEME {
                        return Err(format!("references use the '{}' scheme", w.scheme));
                    }
                    Ok(Some(w.body))
                }
            }
        }
        Ok(UserData {
            reading: part::<Reading>(find(PARTS[0]))?.unwrap_or_default(),
            notes: part::<NotesBody>(find(PARTS[1]))?.map(|b| b.notes).unwrap_or_default(),
            highlights: part::<HighlightsBody>(find(PARTS[2]))?.map(|b| b.highlights).unwrap_or_default(),
            bookmarks: part::<BookmarksBody>(find(PARTS[3]))?.map(|b| b.bookmarks).unwrap_or_default(),
            saved_videos: part::<VideosBody>(find(PARTS[4]))?.map(|b| b.saved).unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> UserData {
        let mut data = UserData::default();
        data.visit(Position { module: "VUL".into(), reference: Ref::parse("Ps.23.1").unwrap(), offset: 0.0 });
        data.notes.push(Note {
            id: "n1".into(),
            start: Ref::parse("Sir.24.3").unwrap(),
            end: Ref::parse("Sir.24.4").unwrap(),
            text: "Wisdom speaks.".into(),
        });
        data.highlights.push(Highlight {
            start: Ref::parse("Dan.13.45").unwrap(),
            end: Ref::parse("Dan.13.45").unwrap(),
            color: "yellow".into(),
        });
        data.bookmarks.push(Bookmark { id: "b1".into(), reference: Ref::parse("John.3.16").unwrap(), label: String::new() });
        data.saved_videos.push("ps23".into());
        data
    }

    #[test]
    fn round_trip() {
        let bytes = sample().to_archive().unwrap();
        assert_eq!(UserData::from_archive(&bytes).unwrap(), sample());
    }

    #[test]
    fn identical_bytes_for_identical_data() {
        assert_eq!(sample().to_archive().unwrap(), sample().to_archive().unwrap());
    }

    #[test]
    fn references_are_osis_strings() {
        let bytes = sample().to_archive().unwrap();
        let entries = read_archive(&bytes).unwrap();
        let notes = entries.iter().find(|e| e.name == "content/notes.json").unwrap();
        let text = String::from_utf8(notes.data.clone()).unwrap();
        assert!(text.contains("\"start\": \"Sir.24.3\""));
        assert!(text.contains("\"scheme\": \"hebrew\""));
    }

    #[test]
    fn history_has_no_repeated_chapters() {
        let mut data = sample();
        data.visit(Position { module: "MAR".into(), reference: Ref::parse("Ps.23.4").unwrap(), offset: 0.5 });
        assert_eq!(data.reading.history.len(), 1);
        assert_eq!(data.reading.current.as_ref().unwrap().module, "MAR");
    }
}
