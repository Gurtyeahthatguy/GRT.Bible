//! USFM to verses, headings and notes.

use crate::module::{BookContent, NoteRow, TitleRow, VerseRow};

#[derive(Debug, Clone, Default)]
pub struct UsfmBook {
    pub code: String,
    pub name: String,
    pub content: BookContent,
}

const NOTE_OPEN: char = '\u{E000}';
const NOTE_CLOSE: char = '\u{E001}';

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Paragraph,
    Title(&'static str),
    Skip,
    Character,
    Chapter,
    Verse,
    Id,
    Header,
}

fn classify(name: &str) -> Kind {
    let base = name.trim_end_matches(|c: char| c.is_ascii_digit());
    match base {
        "id" => Kind::Id,
        "c" => Kind::Chapter,
        "v" => Kind::Verse,
        "h" => Kind::Header,
        "p" | "m" | "pi" | "mi" | "pc" | "pr" | "pm" | "pmo" | "pmc" | "pmr" | "ph" | "li" | "lim" | "lh" | "lf"
        | "q" | "qr" | "qc" | "qm" | "b" | "cls" | "tr" | "po" | "nb" => Kind::Paragraph,
        "s" | "sd" => Kind::Title("section"),
        "ms" => Kind::Title("major"),
        "d" => Kind::Title("psalm"),
        "sp" | "qa" => Kind::Title("section"),
        "toc" | "toca" | "ide" | "rem" | "sts" | "usfm" | "cl" | "cd" | "ca" | "cp" | "va" | "vp" | "periph"
        | "restore" | "ip" | "ipi" | "im" | "imi" | "ipq" | "imq" | "ipr" | "iq" | "ib" | "ili" | "iot" | "io"
        | "ior" | "iex" | "ie" | "imt" | "is" | "imte" | "mt" | "mte" | "lit" | "fig" | "r" | "mr" | "sr" | "rq"
        | "esb" | "esbe" | "cat" => Kind::Skip,
        _ => Kind::Character,
    }
}

/// Pulls footnotes and cross references out, leaving a numbered placeholder.
fn extract_notes(text: &str) -> (String, Vec<String>) {
    let mut out = String::with_capacity(text.len());
    let mut notes = Vec::new();
    let mut rest = text;
    loop {
        let next = ["\\f ", "\\fe ", "\\x ", "\\ef ", "\\ex "]
            .iter()
            .filter_map(|open| rest.find(open).map(|i| (i, *open)))
            .min_by_key(|(i, _)| *i);
        let Some((start, open)) = next else {
            out.push_str(rest);
            break;
        };
        let close = format!("{}*", open.trim_end());
        out.push_str(&rest[..start]);
        let body_start = start + open.len();
        let end = rest[body_start..].find(&close).map(|i| body_start + i).unwrap_or(rest.len());
        let body = &rest[body_start..end];
        if open == "\\f " || open == "\\fe " {
            out.push(NOTE_OPEN);
            out.push_str(&notes.len().to_string());
            out.push(NOTE_CLOSE);
            notes.push(body.to_string());
        }
        rest = rest.get(end + close.len()..).unwrap_or("");
    }
    (out, notes)
}

fn strip_attributes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    for ch in text.chars() {
        match ch {
            '|' => skipping = true,
            '\\' => {
                skipping = false;
                out.push(ch);
            }
            _ if skipping => {}
            _ => out.push(ch),
        }
    }
    out
}

fn tidy_space(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split('\n') {
        let collapsed = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&collapsed);
    }
    out
}

/// A footnote body: the caller and the reference go, the rest stays as plain text.
fn note_text(body: &str) -> (String, String) {
    let body = strip_attributes(body);
    let mut chars = body.trim_start().splitn(2, char::is_whitespace);
    let caller = chars.next().unwrap_or("").to_string();
    let rest = chars.next().unwrap_or("");
    let mut out = String::new();
    let mut skip = false;
    for piece in split_markers(rest) {
        match piece {
            Piece::Marker { name, .. } => {
                skip = matches!(name.as_str(), "fr" | "xo" | "fv");
            }
            Piece::Text(t) if !skip => out.push_str(t),
            Piece::Text(t) => {
                // A reference ends at the first space after its numbers.
                if let Some(i) = t.find(|c: char| c.is_alphabetic()) {
                    out.push_str(&t[i..]);
                    skip = false;
                }
            }
        }
    }
    let marker = if caller == "+" || caller == "-" { String::new() } else { caller };
    (marker, tidy_space(&out.replace('~', "\u{a0}")))
}

enum Piece<'a> {
    Marker { name: String, closing: bool },
    Text(&'a str),
}

fn split_markers(text: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut last = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            if i > last {
                out.push(Piece::Text(&text[last..i]));
            }
            let mut j = i + 1;
            if j < bytes.len() && bytes[j] == b'+' {
                j += 1;
            }
            let name_start = j;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'-') {
                j += 1;
            }
            let name = text[name_start..j].to_string();
            let closing = j < bytes.len() && bytes[j] == b'*';
            if closing {
                j += 1;
            } else if j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            out.push(Piece::Marker { name, closing });
            i = j;
            last = j;
        } else {
            i += 1;
        }
    }
    if last < bytes.len() {
        out.push(Piece::Text(&text[last..]));
    }
    out
}

fn flush_title(title: &mut Option<(&'static str, String)>, pending: &mut Vec<(String, String)>) {
    if let Some((kind, text)) = title.take() {
        let text = tidy_space(&text.replace('\n', " "));
        if !text.is_empty() {
            pending.push((kind.to_string(), text));
        }
    }
}

/// Parses one book.
pub fn parse(source: &str) -> Result<UsfmBook, String> {
    let source = source.trim_start_matches('\u{feff}').replace("\r\n", "\n").replace("//", " ");
    let (body, raw_notes) = extract_notes(&source);
    let body = strip_attributes(&body);

    let mut book = UsfmBook::default();
    let mut chapter: u16 = 0;
    let mut current: Option<usize> = None;
    let mut pending_paragraph = false;
    let mut break_pending = false;
    let mut pending_titles: Vec<(String, String)> = Vec::new();
    let mut title: Option<(&'static str, String)> = None;
    let mut mode = Kind::Character;
    let mut expect: Option<Kind> = None;
    let mut pending_notes: Vec<usize> = Vec::new();

    for piece in split_markers(&body) {
        match piece {
            Piece::Marker { name, closing } => {
                if name.is_empty() {
                    continue;
                }
                let kind = classify(&name);
                if closing {
                    if matches!(kind, Kind::Skip) {
                        mode = Kind::Character;
                    }
                    continue;
                }
                match kind {
                    Kind::Character => {}
                    Kind::Id | Kind::Chapter | Kind::Verse => {
                        flush_title(&mut title, &mut pending_titles);
                        expect = Some(kind);
                        mode = Kind::Character;
                    }
                    Kind::Header => {
                        flush_title(&mut title, &mut pending_titles);
                        mode = Kind::Header;
                    }
                    Kind::Paragraph => {
                        if title.is_some() {
                            flush_title(&mut title, &mut pending_titles);
                            pending_paragraph = true;
                        }
                        if name != "nb" {
                            break_pending = true;
                        }
                        mode = Kind::Character;
                    }
                    Kind::Title(k) => {
                        flush_title(&mut title, &mut pending_titles);
                        if chapter > 0 {
                            title = Some((k, String::new()));
                        }
                        mode = Kind::Title(k);
                    }
                    Kind::Skip => {
                        flush_title(&mut title, &mut pending_titles);
                        mode = Kind::Skip;
                    }
                }
            }
            Piece::Text(text) => {
                let mut text = text;
                if let Some(kind) = expect.take() {
                    let trimmed = text.trim_start();
                    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
                    let token = &trimmed[..end];
                    text = &trimmed[end..];
                    match kind {
                        Kind::Id => {
                            book.code = token.to_uppercase();
                            text = "";
                            mode = Kind::Skip;
                        }
                        Kind::Chapter => {
                            chapter = token.parse().map_err(|_| format!("{}: bad chapter '{token}'", book.code))?;
                            current = None;
                        }
                        Kind::Verse => {
                            let number: String = token.chars().take_while(|c| c.is_ascii_digit()).collect();
                            let verse: u16 = number
                                .parse()
                                .map_err(|_| format!("{} {chapter}: bad verse '{token}'", book.code))?;
                            if chapter == 0 {
                                chapter = 1;
                            }
                            let paragraph = pending_paragraph || break_pending;
                            book.content.verses.push(VerseRow { chapter, verse, text: String::new(), paragraph });
                            current = Some(book.content.verses.len() - 1);
                            for (kind, t) in pending_titles.drain(..) {
                                book.content.titles.push(TitleRow { chapter, verse, kind, text: t });
                            }
                            for n in pending_notes.drain(..) {
                                let (marker, t) = note_text(&raw_notes[n]);
                                book.content.notes.push(NoteRow { chapter, verse, marker, text: t });
                            }
                            pending_paragraph = false;
                            break_pending = false;
                        }
                        _ => {}
                    }
                }
                // Note placeholders attach to the verse they sit in.
                let mut plain = String::with_capacity(text.len());
                let mut rest = text;
                while let Some(open) = rest.find(NOTE_OPEN) {
                    plain.push_str(&rest[..open]);
                    let after = &rest[open + NOTE_OPEN.len_utf8()..];
                    let close = after.find(NOTE_CLOSE).unwrap_or(after.len());
                    if let Ok(n) = after[..close].parse::<usize>() {
                        match (current, mode) {
                            (Some(i), m) if !matches!(m, Kind::Skip) && title.is_none() => {
                                let (marker, t) = note_text(&raw_notes[n]);
                                let v = &book.content.verses[i];
                                book.content.notes.push(NoteRow { chapter: v.chapter, verse: v.verse, marker, text: t });
                            }
                            _ => pending_notes.push(n),
                        }
                    }
                    rest = after.get(close + NOTE_CLOSE.len_utf8()..).unwrap_or("");
                }
                plain.push_str(rest);
                let plain = plain.replace('~', "\u{a0}").replace('\n', " ");
                match mode {
                    Kind::Skip => {}
                    Kind::Header => {
                        if book.name.is_empty() {
                            book.name = plain.trim().to_string();
                        }
                        mode = Kind::Skip;
                    }
                    Kind::Title(_) if title.is_some() => {
                        if let Some((_, t)) = title.as_mut() {
                            t.push_str(&plain);
                        }
                    }
                    _ => {
                        if let Some(i) = current {
                            if plain.trim().is_empty() {
                                // The space between two character markers still separates words.
                                let verse = &mut book.content.verses[i];
                                if !break_pending && !plain.is_empty() && !verse.text.is_empty() && !verse.text.ends_with(' ') {
                                    verse.text.push(' ');
                                }
                                continue;
                            }
                            let verse = &mut book.content.verses[i];
                            if break_pending && !verse.text.trim().is_empty() {
                                verse.text.push('\n');
                            }
                            break_pending = false;
                            verse.text.push_str(&plain);
                        }
                    }
                }
            }
        }
    }
    flush_title(&mut title, &mut pending_titles);
    for verse in &mut book.content.verses {
        verse.text = tidy_space(&verse.text);
    }
    if book.code.is_empty() {
        return Err("no \\id line".into());
    }
    book.content.tidy();
    Ok(book)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PSALM: &str = r#"\id PSA World English Bible
\h Psalms
\mt1 The Psalms
\c 3
\d A Psalm by David, when he fled from Absalom his son.
\q1
\v 1 \w Yahweh|strong="H3068"\w*, how my adversaries have increased!
\q2 Many are those who rise up against me.
\q1
\v 2 Many there are who say of my soul,\f + \fr 3:2 \ft Or, life\f*
\q2 “There is no help for him in God.”
\qs Selah.\qs*
\s1 A second heading
\p
\v 3 But you, Yahweh, are a shield around me.
"#;

    #[test]
    fn verses_lines_and_headings() {
        let book = parse(PSALM).unwrap();
        assert_eq!(book.code, "PSA");
        assert_eq!(book.name, "Psalms");
        let v = &book.content.verses;
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].text, "Yahweh, how my adversaries have increased!\nMany are those who rise up against me.");
        assert!(v[0].paragraph && v[1].paragraph && v[2].paragraph);
        assert_eq!(v[1].text, "Many there are who say of my soul,\n“There is no help for him in God.” Selah.");
        let t = &book.content.titles;
        assert_eq!(t[0].kind, "psalm");
        assert_eq!((t[0].verse, t[0].text.as_str()), (1, "A Psalm by David, when he fled from Absalom his son."));
        assert_eq!((t[1].verse, t[1].kind.as_str()), (3, "section"));
        let n = &book.content.notes;
        assert_eq!(n.len(), 1);
        assert_eq!((n[0].verse, n[0].text.as_str()), (2, "Or, life"));
    }

    #[test]
    fn words_in_character_markers_keep_their_spaces() {
        let book = parse("\\id PSA\n\\c 24\n\\q1\n\\v 1 \\w The|strong=\"H1\"\\w* \\w earth|strong=\"H2\"\\w* \\+w is|x\\+w* the \\w Lord’s|strong=\"H3\"\\w*.\n").unwrap();
        assert_eq!(book.content.verses[0].text, "The earth is the Lord’s.");
    }

    #[test]
    fn continuous_prose_is_one_paragraph() {
        let book = parse("\\id GEN\n\\c 1\n\\p\n\\v 1 In principio.\n\\v 2 Terra autem.\n\\p\n\\v 3 Dixitque.\n").unwrap();
        let v = &book.content.verses;
        assert_eq!((v[0].paragraph, v[1].paragraph, v[2].paragraph), (true, false, true));
        assert_eq!(v[1].text, "Terra autem.");
    }

    #[test]
    fn a_superscription_holding_a_verse_is_text() {
        let book = parse("\\id PSA\n\\c 9\n\\d\n\\v 1 Εἰς τὸ τέλος.\n\\p\n\\v 2 Ἐξομολογήσομαί.\n").unwrap();
        assert!(book.content.titles.is_empty());
        assert_eq!(book.content.verses[0].text, "Εἰς τὸ τέλος.");
    }

    #[test]
    fn verse_ranges_keep_the_first_number() {
        let book = parse("\\id ACT\n\\c 1\n\\p\n\\v 1-2 Both verses.\n\\v 3 Next.\n").unwrap();
        assert_eq!(book.content.verses[0].verse, 1);
        assert_eq!(book.content.verses[1].verse, 3);
    }

    #[test]
    fn glossa_keywords_stay() {
        let (marker, text) = note_text("+ \\fr 1.1 \\fk In principio creavit, \\ft etc. Non dicit.");
        assert_eq!(marker, "");
        assert_eq!(text, "In principio creavit, etc. Non dicit.");
    }
}
