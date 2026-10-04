//! Nestle 1904 from the word list published by biblicalhumanities.org.

use std::collections::BTreeMap;

use grtb::canon::book_by_osis;
use grtb::module::{BookContent, NoteRow, VerseRow};

pub fn parse(text: &str) -> Result<BTreeMap<u8, BookContent>, String> {
    let mut out: BTreeMap<u8, BookContent> = BTreeMap::new();
    let mut shorter_ending = String::new();
    for (n, line) in text.trim_start_matches('\u{feff}').lines().enumerate().skip(1) {
        let mut cols = line.split('\t');
        let (Some(bcv), Some(word)) = (cols.next(), cols.next()) else { continue };
        let (osis, cv) = bcv.split_once(' ').ok_or_else(|| format!("line {}: bad reference {bcv}", n + 1))?;
        let (c, v) = cv.split_once(':').ok_or_else(|| format!("line {}: bad reference {bcv}", n + 1))?;
        let chapter: u16 = c.parse().map_err(|_| format!("line {}: bad chapter", n + 1))?;
        let verse: u16 = v.parse().map_err(|_| format!("line {}: bad verse", n + 1))?;
        let book = book_by_osis(osis).ok_or_else(|| format!("line {}: unknown book {osis}", n + 1))?;
        if osis == "Mark" && verse == 99 {
            if !shorter_ending.is_empty() {
                shorter_ending.push(' ');
            }
            shorter_ending.push_str(word);
            continue;
        }
        let body = out.entry(book.index).or_default();
        match body.verses.last_mut() {
            Some(last) if last.chapter == chapter && last.verse == verse => {
                last.text.push(' ');
                last.text.push_str(word);
            }
            _ => body.verses.push(VerseRow { chapter, verse, text: word.to_string(), paragraph: verse == 1 }),
        }
    }
    if !shorter_ending.is_empty() {
        let mark = out.get_mut(&book_by_osis("Mark").unwrap().index).ok_or("no Mark")?;
        mark.notes.push(NoteRow { chapter: 16, verse: 8, marker: String::new(), text: shorter_ending });
    }
    for body in out.values_mut() {
        body.tidy();
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_become_verses() {
        let csv = "BCV\ttext\tfunc_morph\n\
                   Matt 1:1\tΒίβλος\tN-NSF\n\
                   Matt 1:1\tγενέσεως\tN-GSF\n\
                   Matt 1:2\tἈβραὰμ\tN-PRI\n\
                   Mark 16:8\tἐφοβοῦντο\tV\n\
                   Mark 16:99\tΠάντα\tA\n";
        let out = parse(csv).unwrap();
        let matt = &out[&book_by_osis("Matt").unwrap().index];
        assert_eq!(matt.verses[0].text, "Βίβλος γενέσεως");
        assert_eq!(matt.verses.len(), 2);
        let mark = &out[&book_by_osis("Mark").unwrap().index];
        assert_eq!(mark.notes[0].text, "Πάντα");
        assert_eq!(mark.verses.len(), 1);
    }
}
