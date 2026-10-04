//! Text folding for search: case, accents, vowel points and a few spellings.

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Folds text so that a query typed without accents or points still matches.
pub fn fold(text: &str, language: &str) -> String {
    let latin = language == "la";
    let mut out = String::with_capacity(text.len());
    for ch in text.nfd() {
        if is_combining_mark(ch) {
            continue;
        }
        match ch {
            // Hebrew maqaf and paseq join or separate words.
            '\u{05BE}' | '\u{05C0}' => out.push(' '),
            'ς' => out.push('σ'),
            'æ' | 'Æ' => out.push_str("ae"),
            'œ' | 'Œ' => out.push_str("oe"),
            '\u{2019}' | '\u{2018}' | '\u{02BC}' => out.push('\''),
            _ => {
                for lower in ch.to_lowercase() {
                    let folded = match lower {
                        'j' if latin => 'i',
                        'v' if latin => 'u',
                        other => other,
                    };
                    out.push(folded);
                }
            }
        }
    }
    out
}

/// An FTS5 query from what a person typed: words become prefixes, quotes keep phrases.
pub fn fts_query(input: &str, language: &str) -> Option<String> {
    let folded = fold(input, language);
    let mut parts = Vec::new();
    let mut rest = folded.as_str();
    while !rest.is_empty() {
        rest = rest.trim_start();
        if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"').unwrap_or(stripped.len());
            let words = tokens(&stripped[..end]);
            if !words.is_empty() {
                parts.push(format!("\"{}\"", words.join(" ")));
            }
            rest = stripped.get(end + 1..).unwrap_or("");
        } else {
            let end = rest.find(|c: char| c.is_whitespace() || c == '"').unwrap_or(rest.len());
            for word in tokens(&rest[..end]) {
                parts.push(format!("\"{word}\"*"));
            }
            rest = &rest[end..];
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_and_case() {
        assert_eq!(fold("Perché Iddio È", "it"), "perche iddio e");
        assert_eq!(fold("Ἰησοῦς ὁ λόγος", "grc"), "ιησουσ ο λογοσ");
    }

    #[test]
    fn hebrew_points_go() {
        assert_eq!(fold("בְּרֵאשִׁ֖ית בָּרָ֣א", "hbo"), "בראשית ברא");
        assert_eq!(fold("עַל־פְּנֵ֣י", "hbo"), "על פני");
    }

    #[test]
    fn latin_spellings() {
        assert_eq!(fold("Cælum ejus", "la"), fold("caelum eius", "la"));
        assert_eq!(fold("Vivit", "la"), "uiuit");
    }

    #[test]
    fn queries() {
        assert_eq!(fts_query("luce Dio", "it").unwrap(), "\"luce\"* \"dio\"*");
        assert_eq!(fts_query("\"in principio\" verbum", "la").unwrap(), "\"in principio\" \"uerbum\"*");
        assert!(fts_query("  ,, ", "it").is_none());
    }
}
