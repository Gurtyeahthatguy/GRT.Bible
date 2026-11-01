//! The Open Scriptures Hebrew Bible: the Westminster Leningrad Codex in OSIS, every
//! word with its Strong's number and morphology, and the lexical index that names them.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use grtb::canon::book_by_osis;
use grtb::markup::{attr, decode_entities, strong_number, tokens, Token};
use grtb::module::{BookContent, VerseRow, WordRow};

/// Headword, transliteration and gloss of each lexicon entry by its OSHB lemma, such as
/// `1254a`: `LexicalIndex.xml` names the entries and `AugIndex.xml` ties lemmas to them.
pub fn lexical_index(xml: &str, aug: &str) -> HashMap<String, (String, String, String)> {
    let mut by_id = HashMap::new();
    let mut by_strong: HashMap<String, (String, String, String)> = HashMap::new();
    for piece in xml.split("<entry ").skip(1) {
        let (head, entry) = piece.split_once('>').unwrap_or(("", piece));
        let id = attr(head, "id").unwrap_or_default();
        let entry = entry.split("</entry>").next().unwrap_or("");
        let (mut word, mut xlit, mut def, mut strong) = (String::new(), String::new(), String::new(), None);
        let mut inside: Option<&str> = None;
        for token in tokens(entry) {
            match token {
                Token::Tag { raw_name: "w", closing: false, attrs, .. } if word.is_empty() => {
                    xlit = attr(attrs, "xlit").unwrap_or_default();
                    inside = Some("w");
                }
                Token::Tag { raw_name: "def", closing: false, .. } if def.is_empty() => inside = Some("def"),
                Token::Tag { raw_name: "xref", attrs, .. } => {
                    strong = attr(attrs, "strong").and_then(|s| strong_number(&format!("H{s}")));
                }
                Token::Tag { closing: true, .. } => inside = None,
                Token::Text(t) => match inside {
                    Some("w") => word.push_str(&decode_entities(t)),
                    Some("def") => def.push_str(&decode_entities(t)),
                    _ => {}
                },
                _ => {}
            }
        }
        let row = (word.trim().to_string(), xlit, def.trim().to_string());
        if let Some(s) = strong {
            by_strong.entry(s.trim_start_matches('H').to_string()).or_insert(row.clone());
        }
        by_id.insert(id, row);
    }
    let mut out = by_strong;
    let mut lemma: Option<String> = None;
    for token in tokens(aug) {
        match token {
            Token::Tag { raw_name: "w", closing: false, attrs, .. } => lemma = attr(attrs, "aug"),
            Token::Text(t) => {
                if let (Some(l), Some(row)) = (lemma.take(), by_id.get(t.trim())) {
                    out.insert(l, row.clone());
                }
            }
            _ => {}
        }
    }
    out
}

/// The Strong's number of an OSHB lemma such as `c/d/776` or `1254 a`, its key in the
/// augmented index (`1254a`), and which part carries it.
fn main_lemma(lemma: &str) -> (Option<String>, String, u8) {
    for (i, part) in lemma.split('/').enumerate() {
        if part.starts_with(|c: char| c.is_ascii_digit()) {
            let key: String = part.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
            return (strong_number(&format!("H{}", part.trim())), key, i as u8);
        }
    }
    (None, String::new(), 0)
}

struct Book {
    osis: String,
    content: BookContent,
}

fn parse_book(xml: &str, index: &HashMap<String, (String, String, String)>) -> Result<Book, String> {
    let mut content = BookContent::default();
    let mut osis = String::new();
    let mut at: Option<(u16, u16)> = None;
    let mut word: Option<WordRow> = None;
    // Inside a note, words are skipped unless they are a qere, which replaces the ketiv before it.
    let mut note = 0usize;
    let mut qere = false;
    let mut ketiv = false;
    let mut segment: Option<String> = None;
    let mut next_paragraph = true;
    let mut this_paragraph = false;
    let mut words: Vec<WordRow> = Vec::new();
    let finish_verse = |content: &mut BookContent, words: &mut Vec<WordRow>, at: (u16, u16), paragraph: bool| {
        let text = words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ").replace(" \u{05BE}", "\u{05BE}").replace("\u{05BE} ", "\u{05BE}");
        if !text.is_empty() {
            content.verses.push(VerseRow { chapter: at.0, verse: at.1, text, paragraph });
        }
        content.words.append(words);
    };
    for token in tokens(xml) {
        match token {
            Token::Tag { raw_name: "verse", closing: false, attrs, .. } => {
                let id = attr(attrs, "osisID").unwrap_or_default();
                let parts: Vec<&str> = id.split('.').collect();
                if parts.len() == 3 {
                    osis = parts[0].to_string();
                    at = Some((parts[1].parse().map_err(|_| "bad chapter")?, parts[2].parse().map_err(|_| "bad verse")?));
                    this_paragraph = std::mem::take(&mut next_paragraph);
                }
            }
            Token::Tag { raw_name: "verse", closing: true, .. } => {
                if let Some(place) = at.take() {
                    finish_verse(&mut content, &mut words, place, this_paragraph || place.1 == 1);
                }
            }
            Token::Tag { raw_name: "note", closing, empty: false, .. } => {
                if closing {
                    note = note.saturating_sub(1);
                } else {
                    note += 1;
                }
            }
            Token::Tag { raw_name: "rdg", closing, attrs, .. } => qere = !closing && attr(attrs, "type").as_deref() == Some("x-qere"),
            Token::Tag { raw_name: "seg", closing: false, attrs, .. } => segment = attr(attrs, "type"),
            Token::Tag { raw_name: "seg", closing: true, .. } => segment = None,
            Token::Tag { raw_name: "w", closing: false, attrs, .. } => {
                if note > 0 && !qere {
                    continue;
                }
                ketiv = attr(attrs, "type").as_deref() == Some("x-ketiv");
                if ketiv {
                    continue;
                }
                let lemma = attr(attrs, "lemma").unwrap_or_default();
                let (strong, key, main) = main_lemma(&lemma);
                let digits: String = key.chars().filter(char::is_ascii_digit).collect();
                let (headword, xlit, gloss) = index.get(&key).or_else(|| index.get(&digits)).cloned().unwrap_or_default();
                let (chapter, verse) = at.unwrap_or((0, 0));
                word = Some(WordRow {
                    chapter,
                    verse,
                    text: String::new(),
                    lemma: headword,
                    strong: strong.unwrap_or_default(),
                    gloss,
                    xlit,
                    morph: attr(attrs, "morph").map(|m| format!("oshb:{m}")).unwrap_or_default(),
                    main,
                });
            }
            Token::Tag { raw_name: "w", closing: true, .. } => {
                if ketiv {
                    ketiv = false;
                    continue;
                }
                if let Some(w) = word.take() {
                    words.push(w);
                }
            }
            Token::Text(t) => {
                if let Some(w) = word.as_mut() {
                    w.text.push_str(&decode_entities(t).replace('/', ""));
                    continue;
                }
                match segment.as_deref() {
                    Some("x-maqqef") | Some("x-sof-pasuq") | Some("x-paseq") | Some("x-reversednun") => {
                        if let Some(last) = words.last_mut() {
                            last.text.push_str(decode_entities(t).trim());
                        }
                    }
                    Some("x-pe") | Some("x-samekh") => next_paragraph = true,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    if osis.is_empty() {
        return Err("no verse in the file".into());
    }
    content.tidy();
    Ok(Book { osis, content })
}

pub fn parse(dir: &Path, index: &HashMap<String, (String, String, String)>) -> Result<BTreeMap<u8, BookContent>, String> {
    let mut out = BTreeMap::new();
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "xml").unwrap_or(false))
        .collect();
    files.sort();
    for path in files {
        let xml = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let book = parse_book(&xml, index).map_err(|e| format!("{}: {e}", path.display()))?;
        let b = book_by_osis(&book.osis).ok_or_else(|| format!("{}: unknown book {}", path.display(), book.osis))?;
        out.insert(b.index, book.content);
    }
    if out.is_empty() {
        return Err(format!("no OSIS files in {}", dir.display()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<osis><div type="book" osisID="Gen"><chapter osisID="Gen.1">
<verse osisID="Gen.1.1"><w lemma="b/7225" n="1.0" morph="HR/Ncfsa" id="a">בְּ/רֵאשִׁ֖ית</w>
<w lemma="5921 a" morph="HR" id="b">עַל</w><seg type="x-maqqef">־</seg><w lemma="d/776" morph="HTd/Ncbsa" id="c">הָ/אָֽרֶץ</w><seg type="x-sof-pasuq">׃</seg></verse>
<verse osisID="Gen.1.2"><w type="x-ketiv" lemma="168" morph="HNcmsc/Sp3ms" id="d">אהל/ה</w><note type="variant"><catchWord>אהל/ה</catchWord><rdg type="x-qere"><w lemma="168" morph="HNcmsc/Sp3ms" id="e">אָֽהֳל/וֹ֙</w></rdg></note> <seg type="x-pe">פ</seg></verse>
<verse osisID="Gen.1.3"><w lemma="1961" morph="HVqp3ms" id="f">הָיָה</w></verse>
</chapter></div></osis>"#;

    #[test]
    fn words_maqqef_qere_and_paragraphs() {
        let mut index = HashMap::new();
        index.insert("7225".to_string(), ("רֵאשִׁית".to_string(), "rēʾšît".to_string(), "beginning".to_string()));
        let book = parse_book(SAMPLE, &index).unwrap();
        assert_eq!(book.osis, "Gen");
        let v = &book.content.verses;
        assert_eq!(v[0].text, "בְּרֵאשִׁ֖ית עַל־הָאָֽרֶץ׃");
        assert_eq!(v[1].text, "אָֽהֳלוֹ֙");
        assert!(v[2].paragraph, "a pe ends the paragraph");
        let w = &book.content.words;
        assert_eq!((w[0].strong.as_str(), w[0].main, w[0].lemma.as_str(), w[0].gloss.as_str()), ("H7225", 1, "רֵאשִׁית", "beginning"));
        assert_eq!(w[0].morph, "oshb:HR/Ncfsa");
        assert_eq!(w[1].strong, "H5921");
        assert_eq!(w.len(), 5);
    }

    #[test]
    fn the_lexical_index() {
        let index = lexical_index(
            r#"<entry id="aac"><w xlit="ʾāb">אָב</w> <pos>N</pos> <def>father</def><xref bdb="a.ae.ab" strong="1" twot="4a"/></entry><entry id="bxy"><w xlit="bārāʾ">בָּרָא</w> <def>create</def><xref strong="1254"/></entry><entry id="bxx"><w xlit="bārāʾ">בָּרָא</w> <def>be fat</def><xref strong="1254"/></entry>"#,
            r#"<w aug="1254a">bxy</w><w aug="1254b">bxx</w>"#,
        );
        assert_eq!(index["1"], ("אָב".to_string(), "ʾāb".to_string(), "father".to_string()));
        assert_eq!(index["1254a"].2, "create");
        assert_eq!(index["1254b"].2, "be fat");
    }
}
