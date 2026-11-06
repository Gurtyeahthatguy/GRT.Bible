//! Commentaries kept as tables: Haydock's notes as tab-separated text, and the
//! excerpts of the Fathers in the Historical Christian Faith database.

use std::collections::BTreeMap;
use std::path::Path;

use grtb::markup::escape;
use grtb::module::EntryContent;
use grtb::refs::Ref;
use grtb::versification::scheme;
use rusqlite::{Connection, OpenFlags};

/// The book index of an English book name, ignoring case and spaces.
fn book_index(name: &str) -> Option<u8> {
    let wanted: String = name.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
    let wanted = match wanted.as_str() {
        "songofsolomon" => "songofsongs",
        "prayerofazariah" => "daniel",
        other => other,
    };
    grtb::canon::books().iter().find(|b| b.name_in("en").0.replace(' ', "").to_lowercase() == wanted).map(|b| b.index)
}

/// Paragraphs of plain text as markup.
fn paragraphs<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    parts.map(str::trim).filter(|p| !p.is_empty()).map(|p| format!("<p>{}</p>", escape(p))).collect()
}

/// Haydock's notes: one line per verse, `Book<TAB>chapter:verse<TAB>text`, in Douay numbering.
pub fn haydock(tsv: &str) -> Result<Vec<EntryContent>, String> {
    let douay = scheme("douay");
    let mut by_verse: BTreeMap<Ref, Vec<String>> = BTreeMap::new();
    for (n, line) in tsv.lines().enumerate() {
        let mut fields = line.splitn(3, '\t');
        let (Some(book), Some(place), Some(text)) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        let b = book_index(book).ok_or_else(|| format!("line {}: unknown book {book}", n + 1))?;
        let (chapter, verse) = place.split_once(':').ok_or_else(|| format!("line {}: no verse in {place}", n + 1))?;
        let (Ok(chapter), Ok(verse)) = (chapter.trim().parse::<u16>(), verse.trim().parse::<u16>()) else {
            return Err(format!("line {}: bad place {place}", n + 1));
        };
        let notes = by_verse.entry(Ref::new(b, chapter, verse)).or_default();
        // The source repeats many notes word for word.
        if !notes.iter().any(|t| t == text) {
            notes.push(text.to_string());
        }
    }
    Ok(by_verse
        .into_iter()
        .filter_map(|(at, notes)| {
            let (first, last) = douay.to_canon(at);
            if !first.in_canon() {
                return None;
            }
            let body = paragraphs(notes.iter().flat_map(|t| t.split(" --- ")));
            Some(EntryContent { body, spans: vec![(first, last)], ..Default::default() })
        })
        .collect())
}

/// Which excerpts of the database to keep.
pub enum Excerpts {
    /// The Catena Aurea in Newman's translation of 1841.
    Catena,
    /// The Ante-Nicene and Nicene and Post-Nicene Fathers, as New Advent and CCEL publish them.
    Fathers,
}

/// Excerpts grouped by the verses they explain, each under the name of its author, oldest first.
pub fn fathers(path: &Path, which: Excerpts) -> Result<Vec<EntryContent>, String> {
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
    let filter = match which {
        Excerpts::Catena => "source_title = 'Catena Aurea by Aquinas'",
        Excerpts::Fathers => {
            "(source_url LIKE 'https://www.newadvent.org/fathers/%' OR source_url LIKE 'http://www.newadvent.org/fathers/%'
              OR source_url LIKE 'https://ccel.org/fathers2/ANF%' OR source_url LIKE 'http://www.ccel.org/fathers2/ANF%'
              OR source_url LIKE 'https://www.ccel.org/fathers2/ANF%') AND source_title <> 'Catena Aurea by Aquinas'"
        }
    };
    let sql = format!(
        "SELECT book, location_start, location_end, father_name, COALESCE(append_to_author_name, ''), COALESCE(ts, 9999), txt, COALESCE(source_title, '')
         FROM commentary WHERE {filter} ORDER BY book, location_start, location_end, ts, father_name, id"
    );
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    type Row = (String, i64, i64, String, String, i64, String, String);
    let rows: Vec<Row> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let english = scheme("english");
    let mut grouped: BTreeMap<(Ref, Ref), Vec<(i64, String)>> = BTreeMap::new();
    for (book, start, end, father, appended, year, text, title) in rows {
        let Some(b) = book_index(&book) else { continue };
        // Locations are chapter * 1,000,000 + verse.
        let place = |n: i64| Ref::new(b, (n / 1_000_000) as u16, (n % 1_000_000) as u16);
        let (first, last) = (english.to_canon(place(start)).0, english.to_canon(place(end.max(start))).1);
        if !first.in_canon() || !last.in_canon() || first > last {
            continue;
        }
        let by = if appended.trim().is_empty() { father.clone() } else { format!("{father}{appended}") };
        let source = match which {
            Excerpts::Fathers if !title.is_empty() => format!(", <i>{}</i>", escape(&title)),
            _ => String::new(),
        };
        let mut html = format!("<h4>{}{source}</h4>", escape(&by));
        html.push_str(&paragraphs(text.split("\n\n")));
        grouped.entry((first, last)).or_default().push((year, html));
    }
    Ok(grouped
        .into_iter()
        .map(|(span, mut parts)| {
            parts.sort_by_key(|(year, _)| *year);
            EntryContent { body: parts.into_iter().map(|(_, html)| html).collect(), spans: vec![span], ..Default::default() }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haydock_lands_on_the_internal_verse_and_drops_repeats() {
        let tsv = "Psalms\t22:1\tThe Lord ruleth me. --- A psalm of trust.\nPsalms\t22:1\tThe Lord ruleth me. --- A psalm of trust.\nSong of Solomon\t1:1\tThe kiss.\n";
        let entries = haydock(tsv).unwrap();
        assert_eq!(entries.len(), 2);
        let ps = &entries.iter().find(|e| e.spans[0].0.book == book_index("Psalms").unwrap()).unwrap();
        assert_eq!(ps.spans[0].0.osis(), "Ps.23.1");
        assert_eq!(ps.body, "<p>The Lord ruleth me.</p><p>A psalm of trust.</p>");
        assert!(haydock("Nowhere\t1:1\tx").is_err());
    }

    #[test]
    fn database_books_are_found_by_name() {
        assert_eq!(book_index("1corinthians"), book_index("1 Corinthians"));
        assert_eq!(book_index("songofsolomon"), book_index("Song of Songs"));
        assert!(book_index("matthew").is_some());
        assert_eq!(book_index("nothing"), None);
    }
}
