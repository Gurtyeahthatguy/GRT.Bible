//! Study packs of 0.5: the SBLGNT apparatus, the Eusebian canons and the atlas of places and events.

use std::collections::BTreeMap;
use std::path::Path;

use grtb::markup::escape;
use grtb::module::EntryContent;
use grtb::refs::Ref;
use grtb::versification::scheme;

/// Editions and manuscripts the SBLGNT apparatus names, as it abbreviates them.
pub const SIGLA: [(&str, &str); 12] = [
    ("WH", "Westcott and Hort, 1881"),
    ("WHmarg", "a reading in the margin of Westcott and Hort"),
    ("WHapp", "a reading in the appendix of Westcott and Hort"),
    ("Treg", "Tregelles, 1857-1879"),
    ("Tregmarg", "a reading in the margin of Tregelles"),
    ("NIV", "the Greek text behind the New International Version, 2011"),
    ("RP", "Robinson and Pierpont, Byzantine text, 2005"),
    ("NA28", "Nestle-Aland, 28th edition"),
    ("NA27", "Nestle-Aland, 27th edition"),
    ("TR", "the Textus Receptus"),
    ("Holmes", "Michael Holmes, editor of the SBLGNT"),
    ("Greeven", "Greeven's synopsis"),
];

fn is_siglum(word: &str) -> bool {
    SIGLA.iter().any(|(s, _)| *s == word)
}

fn book_by_osis(osis: &str) -> Option<u8> {
    grtb::canon::books().iter().find(|b| b.osis == osis).map(|b| b.index)
}

/// A reading and the editions behind it: `ὃν εἶπον Treg NA28 RP` into its words and sigla.
fn reading(text: &str) -> (String, Vec<String>) {
    let mut words: Vec<&str> = text.split_whitespace().collect();
    let mut sigla = Vec::new();
    while let Some(last) = words.last() {
        if !is_siglum(last) {
            break;
        }
        sigla.insert(0, last.to_string());
        words.pop();
    }
    (words.join(" "), sigla)
}

/// One unit of variation as markup: the text printed, a bracket, then the other readings.
fn unit(line: &str) -> Option<String> {
    let (printed, others) = line.split_once(" ] ").or_else(|| line.split_once(']'))?;
    let (text, sigla) = reading(printed.trim());
    let mut out = format!("<p><b>{}</b>", escape(&text));
    if !sigla.is_empty() {
        out.push_str(&format!(" <i>{}</i>", escape(&sigla.join(" "))));
    }
    out.push_str(" ]");
    for (i, other) in others.split(';').enumerate() {
        let (text, sigla) = reading(other.trim());
        out.push_str(if i == 0 { " " } else { "; " });
        out.push_str(&escape(&text));
        if !sigla.is_empty() {
            out.push_str(&format!(" <i>{}</i>", escape(&sigla.join(" "))));
        }
    }
    out.push_str("</p>");
    Some(out)
}

/// The apparatus of one book: a heading `John 1:18`, then one line per unit of variation.
pub fn apparatus_book(osis: &str, text: &str) -> Result<Vec<(Ref, Vec<String>)>, String> {
    let book = book_by_osis(osis).ok_or_else(|| format!("unknown book {osis}"))?;
    let mut out: Vec<(Ref, Vec<String>)> = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let heading = line.rsplit_once(' ').and_then(|(name, place)| {
            let (c, v) = place.split_once(':')?;
            (!name.contains(']') && name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == ' '))
                .then_some(())
                .and_then(|_| Some((c.parse::<u16>().ok()?, v.parse::<u16>().ok()?)))
        });
        if let Some((chapter, verse)) = heading {
            out.push((Ref::new(book, chapter, verse), Vec::new()));
            continue;
        }
        let Some((_, units)) = out.last_mut() else { continue };
        // The first unit opens with the verse number, the next ones with a bullet.
        let body = line.trim_start_matches('•').trim();
        let body = match body.split_once(' ') {
            Some((number, rest)) if number.split(':').all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) => rest,
            _ => body,
        };
        if let Some(markup) = unit(body) {
            units.push(markup);
        }
    }
    Ok(out)
}

/// Every book of the apparatus, one entry per verse, in the internal numbering.
pub fn apparatus(dir: &Path) -> Result<(Vec<EntryContent>, Vec<u8>), String> {
    let english = scheme("english");
    let mut files: Vec<_> = std::fs::read_dir(dir).map_err(|e| e.to_string())?.filter_map(Result::ok).map(|e| e.path()).collect();
    files.retain(|p| p.extension().is_some_and(|x| x == "txt"));
    files.sort();
    let mut all = Vec::new();
    let mut bytes = Vec::new();
    for path in &files {
        let raw = std::fs::read(path).map_err(|e| e.to_string())?;
        let osis = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        all.extend(apparatus_book(&osis, &String::from_utf8_lossy(&raw))?);
        bytes.extend(raw);
    }
    all.sort_by_key(|(r, _)| *r);
    let mut legend = String::from("<p>Each line gives the words of the SBLGNT, with the editions that print them, then after the bracket the other readings and the editions behind each. A dash marks words left out, a plus words added.</p>");
    for (s, name) in SIGLA {
        legend.push_str(&format!("<p><i>{s}</i> {}</p>", escape(name)));
    }
    let mut entries = vec![EntryContent { key: "sigla".into(), title: "Sigla".into(), body: legend, ..Default::default() }];
    for (at, units) in all.into_iter().filter(|(_, u)| !u.is_empty()) {
        let (first, last) = english.to_canon(at);
        entries.push(EntryContent { title: at.osis(), body: units.concat(), spans: vec![(first, last)], ..Default::default() });
    }
    Ok((entries, bytes))
}

const GOSPELS: [(&str, &str, &str); 4] = [("MAT", "Matt", "Mt"), ("MRK", "Mark", "Mk"), ("LUK", "Luke", "Lk"), ("JHN", "John", "Jn")];
const ROMAN: [&str; 10] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"];

fn place(text: &str) -> Option<(u16, u16)> {
    let (_, cv) = text.trim().split_once(' ')?;
    let (c, v) = cv.split_once(':')?;
    Some((c.parse().ok()?, v.parse().ok()?))
}

/// The Eusebian canons: one entry per row of a table, spanning the sections it sets side by side.
pub fn harmony(dir: &Path) -> Result<(Vec<EntryContent>, Vec<u8>), String> {
    let english = scheme("english");
    let mut bytes = Vec::new();
    let mut sections: BTreeMap<(&str, u32), (Ref, Ref)> = BTreeMap::new();
    for (file, osis, short) in GOSPELS {
        let raw = std::fs::read(dir.join(format!("{file}-sections-aligned.txt"))).map_err(|e| e.to_string())?;
        let book = book_by_osis(osis).ok_or("no gospel")?;
        for (n, line) in String::from_utf8_lossy(&raw).lines().enumerate() {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 5 {
                continue;
            }
            let number: u32 = fields[0].trim().parse().map_err(|_| format!("{file} line {}: no section number", n + 1))?;
            let (Some(a), Some(b)) = (place(fields[3]), place(fields[4])) else {
                return Err(format!("{file} line {}: bad range", n + 1));
            };
            let first = english.to_canon(Ref::new(book, a.0, a.1)).0;
            let last = english.to_canon(Ref::new(book, b.0, b.1)).1;
            sections.insert((short, number), (first, last));
        }
        bytes.extend(raw);
    }
    let mut entries = Vec::new();
    for table in 1..=13u32 {
        let raw = std::fs::read(dir.join(format!("canon-{table}.txt"))).map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&raw).to_string();
        bytes.extend(raw);
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().unwrap_or_default().split('\t').map(str::trim).collect();
        // Tables 10 to 13 are the tenth canon, what one Gospel alone tells.
        let name = ROMAN[(table.min(10) - 1) as usize];
        for (row, line) in lines.enumerate().filter(|(_, l)| !l.trim().is_empty()) {
            let mut spans = Vec::new();
            let mut parts = Vec::new();
            for (column, cell) in line.split('\t').enumerate() {
                let Some(short) = header.get(column).copied() else { continue };
                for number in cell.split(|c: char| !c.is_ascii_digit()).filter(|n| !n.is_empty()) {
                    let n: u32 = number.parse().unwrap_or(0);
                    let Some(&(a, b)) = sections.get(&(short, n)) else {
                        return Err(format!("canon {table} row {}: no section {short} {n}", row + 1));
                    };
                    spans.push((a, b));
                    parts.push(format!("<a data-ref=\"{}\">{short} §{n}</a>", grtb::module::range_osis(a, b)));
                }
            }
            if spans.is_empty() {
                continue;
            }
            let title = format!("Canon {name} · {}", header.join(", "));
            entries.push(EntryContent { key: format!("{table}.{}", row + 1), title, body: format!("<p>{}</p>", parts.join(" · ")), spans, ..Default::default() });
        }
    }
    Ok((entries, bytes))
}

#[derive(serde::Deserialize)]
struct AtlasFile {
    places: Vec<AtlasItem>,
    events: Vec<AtlasItem>,
    shapes: Vec<(String, String)>,
}

#[derive(serde::Deserialize)]
struct AtlasItem {
    key: String,
    title: String,
    body: String,
    verses: Vec<String>,
    #[serde(default)]
    point: Option<(f64, f64)>,
    #[serde(default)]
    years: Option<(i32, i32)>,
    #[serde(default)]
    aliases: Vec<String>,
}

/// Verses as ranges of neighbours, in the internal numbering.
fn ranges(verses: &[String]) -> Vec<(Ref, Ref)> {
    let english = scheme("english");
    let mut refs: Vec<Ref> = verses.iter().filter_map(|v| Ref::parse(v)).map(|r| english.to_canon(r).0).filter(|r| r.in_canon()).collect();
    refs.sort();
    refs.dedup();
    let mut out: Vec<(Ref, Ref)> = Vec::new();
    for r in refs {
        match out.last_mut() {
            Some((_, last)) if last.book == r.book && last.chapter == r.chapter && last.verse + 1 == r.verse => *last = r,
            _ => out.push((r, r)),
        }
    }
    out
}

/// The atlas written by scripts/build-atlas.py: places with a point, events with years, map outlines.
pub fn atlas(path: &Path) -> Result<(Vec<EntryContent>, Vec<(String, String)>, Vec<u8>), String> {
    let raw = std::fs::read(path).map_err(|e| e.to_string())?;
    let file: AtlasFile = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for p in file.places {
        let point = p.point.map(|(lat, lon)| ((lat * 1e5).round() as i32, (lon * 1e5).round() as i32));
        entries.push(EntryContent { key: p.key, title: p.title, body: p.body, spans: ranges(&p.verses), aliases: p.aliases, point, ..Default::default() });
    }
    for e in file.events {
        entries.push(EntryContent { key: e.key, title: e.title, body: e.body, spans: ranges(&e.verses), years: e.years, ..Default::default() });
    }
    Ok((entries, file.shapes, raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_apparatus_keeps_each_reading_with_its_editions() {
        let text = "ΚΑΤΑ ΙΩΑΝΝΗΝ\n\nJohn 1:18\n18 μονογενὴς θεὸς WH Treg NA28 ] ὁ μονογενὴς υἱός RP\n\nJohn 1:27\n27 ὁ Treg NA28 ] – WH; αὐτός ἐστιν ὁ RP \n• ἐρχόμενος WH Treg NA28 ] + ὃς ἔμπροσθέν μου γέγονεν RP\n";
        let verses = apparatus_book("John", text).unwrap();
        assert_eq!(verses.len(), 2);
        assert_eq!(verses[0].0, Ref::parse("John.1.18").unwrap());
        assert_eq!(verses[0].1, ["<p><b>μονογενὴς θεὸς</b> <i>WH Treg NA28</i> ] ὁ μονογενὴς υἱός <i>RP</i></p>"]);
        assert_eq!(verses[1].1.len(), 2);
        assert_eq!(verses[1].1[0], "<p><b>ὁ</b> <i>Treg NA28</i> ] – <i>WH</i>; αὐτός ἐστιν ὁ <i>RP</i></p>");
        assert!(verses[1].1[1].starts_with("<p><b>ἐρχόμενος</b>"));
    }

    #[test]
    fn neighbouring_verses_make_one_range() {
        let got = ranges(&["Gen.12.2".into(), "Gen.12.1".into(), "Gen.12.3".into(), "Gen.13.1".into(), "Gen.12.3".into()]);
        assert_eq!(got, vec![(Ref::parse("Gen.12.1").unwrap(), Ref::parse("Gen.12.3").unwrap()), (Ref::parse("Gen.13.1").unwrap(), Ref::parse("Gen.13.1").unwrap())]);
    }
}
