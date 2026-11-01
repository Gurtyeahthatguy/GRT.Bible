//! Brown-Driver-Briggs as the Open Scriptures Hebrew Bible project transcribed it,
//! each entry found by its headword and by the Strong's numbers the lexical index gives it.

use std::collections::HashMap;

use grtb::markup::{attr, decode_entities, escape, strong_number, tokens, Token};
use grtb::module::{range_osis, EntryContent};
use grtb::passage::parse_osis;

/// Strong's numbers by BDB entry, from `LexicalIndex.xml`.
pub fn strongs_by_entry(index: &str) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for token in tokens(index) {
        if let Token::Tag { raw_name: "xref", attrs, .. } = token {
            let (Some(bdb), Some(strong)) = (attr(attrs, "bdb"), attr(attrs, "strong")) else { continue };
            if let Some(n) = strong_number(&format!("H{strong}")) {
                let list = out.entry(bdb).or_default();
                if !list.contains(&n) {
                    list.push(n);
                }
            }
        }
    }
    out
}

/// One entry's XML in the stored subset.
fn entry_markup(xml: &str) -> (String, String) {
    let mut out = String::from("<p>");
    let mut headword = String::new();
    let mut in_word = 0usize;
    let mut skip = 0usize;
    let mut open: Vec<&'static str> = Vec::new();
    for token in tokens(xml) {
        match token {
            Token::Text(t) => {
                if skip > 0 {
                    continue;
                }
                let text = decode_entities(t);
                if in_word > 0 && headword.is_empty() {
                    headword = text.trim().to_string();
                }
                let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
                if text.starts_with(char::is_whitespace) {
                    out.push(' ');
                }
                out.push_str(&escape(&words));
                if text.ends_with(char::is_whitespace) && !words.is_empty() {
                    out.push(' ');
                }
            }
            Token::Tag { raw_name, closing, empty, attrs } => match (raw_name, closing) {
                ("status" | "page", false) if !empty => skip += 1,
                ("status" | "page", true) => skip = skip.saturating_sub(1),
                ("w", false) if !empty => in_word += 1,
                ("w", true) => in_word = in_word.saturating_sub(1),
                ("def" | "foreign" | "em", false) if !empty => {
                    out.push_str("<i>");
                    open.push("i");
                }
                ("pos" | "stem" | "asp", false) if !empty => {
                    out.push_str("<b>");
                    open.push("b");
                }
                ("def" | "foreign" | "em" | "pos" | "stem" | "asp" | "ref", true) => {
                    if let Some(tag) = open.pop() {
                        out.push_str(&format!("</{tag}>"));
                    }
                }
                ("sense", false) if !empty => {
                    let n = attr(attrs, "n").map(|n| format!("{n}. ")).unwrap_or_default();
                    out.push_str(&format!("<br><b>{}</b>", escape(&n)));
                }
                ("ref", false) if !empty => {
                    let target = attr(attrs, "r").map(|r| parse_osis(&r)).and_then(|v| v.into_iter().next());
                    match target {
                        Some((a, b)) => {
                            out.push_str(&format!("<a data-ref=\"{}\">", escape(&range_osis(a, b))));
                            open.push("a");
                        }
                        None => open.push("span"),
                    }
                }
                _ => {}
            },
        }
    }
    while let Some(tag) = open.pop() {
        if tag != "span" {
            out.push_str(&format!("</{tag}>"));
        }
    }
    out = out.replace("</span>", "");
    out.push_str("</p>");
    let mut tidy = String::with_capacity(out.len());
    for c in out.chars() {
        if c == ' ' && tidy.ends_with(' ') {
            continue;
        }
        tidy.push(c);
    }
    let tidy = tidy.replace("<p> ", "<p>").replace(" </p>", "</p>").replace(" <br>", "<br>").replace("<br> ", "<br>");
    (headword, tidy)
}

pub fn parse(bdb: &str, strongs: &HashMap<String, Vec<String>>) -> Vec<EntryContent> {
    let mut entries = Vec::new();
    for piece in bdb.split("<entry ").skip(1) {
        let (head, rest) = piece.split_once('>').unwrap_or((piece, ""));
        let body = rest.split("</entry>").next().unwrap_or("");
        let id = attr(head, "id").unwrap_or_default();
        let (headword, markup) = entry_markup(body);
        if headword.is_empty() {
            continue;
        }
        let mut aliases = strongs.get(&id).cloned().unwrap_or_default();
        aliases.push(format!("bdb:{id}"));
        entries.push(EntryContent { key: headword.clone(), title: headword, body: markup, aliases, ..Default::default() });
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_with_senses_and_references() {
        let xml = r#"<section id="a.ae"><entry id="a.ae.ab"><w>אָב</w> 1101 <pos>n.m</pos>. <def>father</def>
<sense n="1"><def>father of individual</def> <ref r="Gen.2.24">Gn 2:24</ref></sense><status p="1">done</status></entry></section>"#;
        let mut strongs = HashMap::new();
        strongs.insert("a.ae.ab".to_string(), vec!["H1".to_string()]);
        let entries = parse(xml, &strongs);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "אָב");
        assert_eq!(entries[0].aliases, ["H1", "bdb:a.ae.ab"]);
        assert_eq!(
            entries[0].body,
            r#"<p>אָב 1101 <b>n.m</b>. <i>father</i><br><b>1. </b><i>father of individual</i> <a data-ref="Gen.2.24">Gn 2:24</a></p>"#
        );
    }

    #[test]
    fn the_index_gives_strongs_numbers() {
        let map = strongs_by_entry(r#"<entry id="aac"><xref bdb="a.ae.ab" strong="1" twot="4a"/></entry><entry id="b"><xref bdb="z" strong="b"/></entry>"#);
        assert_eq!(map["a.ae.ab"], ["H1"]);
        assert!(!map.contains_key("z"));
    }
}
