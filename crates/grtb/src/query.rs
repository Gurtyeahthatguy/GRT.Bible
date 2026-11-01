//! Searches beyond words of the text: Strong's numbers, lemmas and morphology,
//! joined by AND, OR, NOT and NEAR, with parentheses.
//!
//! `strong:G25`, or just `G25`, finds a Strong's number; `lemma:ἀγάπη` a lemma;
//! `morph:"aorist imperative"` a form. `+` joins conditions on one word:
//! `lemma:ἀγαπάω+morph:imperative`. Anything else is text, as plain search reads it.

use std::collections::BTreeMap;

use crate::module::{Module, WordFilter};
use crate::text::fts_query;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WordTerm {
    pub strong: Option<String>,
    pub lemma: Option<String>,
    pub morph: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Text(String),
    Word(WordTerm),
    And(Box<Node>, Box<Node>),
    Or(Box<Node>, Box<Node>),
    Not(Box<Node>, Box<Node>),
    Near(Box<Node>, Box<Node>, u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Open,
    Close,
    And,
    Or,
    Not,
    Near(u32),
    Term(String),
}

fn lex(input: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '(' || c == ')' {
            out.push(if c == '(' { Tok::Open } else { Tok::Close });
            i += 1;
            continue;
        }
        // A term runs to whitespace or a parenthesis, and a quote keeps spaces inside it.
        let mut term = String::new();
        let mut quoted = false;
        while i < chars.len() {
            let ch = chars[i];
            if ch == '"' {
                quoted = !quoted;
                term.push(ch);
            } else if !quoted && (ch.is_whitespace() || ch == '(' || ch == ')') {
                break;
            } else {
                term.push(ch);
            }
            i += 1;
        }
        if quoted {
            return Err("a quotation mark is not closed".into());
        }
        out.push(match term.as_str() {
            "AND" => Tok::And,
            "OR" => Tok::Or,
            "NOT" => Tok::Not,
            t if t == "NEAR" || t.starts_with("NEAR/") => {
                let n = t.strip_prefix("NEAR/").map(|n| n.parse().map_err(|_| format!("{t}: NEAR/ takes a number of words"))).transpose()?;
                Tok::Near(n.unwrap_or(10).min(100))
            }
            _ => Tok::Term(term),
        });
    }
    Ok(out)
}

fn is_strong(text: &str) -> bool {
    let mut c = text.chars();
    matches!(c.next(), Some('G' | 'H' | 'g' | 'h')) && c.clone().next().is_some() && c.all(|d| d.is_ascii_digit())
}

fn unquote(text: &str) -> String {
    text.trim_matches('"').to_string()
}

fn term(text: &str) -> Result<Node, String> {
    let has_field = text.split('+').any(|p| p.starts_with("strong:") || p.starts_with("lemma:") || p.starts_with("morph:"));
    if !has_field && !is_strong(text) {
        return Ok(Node::Text(text.to_string()));
    }
    let mut word = WordTerm::default();
    for part in text.split('+') {
        if let Some(v) = part.strip_prefix("strong:") {
            word.strong = crate::markup::strong_number(&unquote(v));
            if word.strong.is_none() {
                return Err(format!("{v} is not a Strong's number, such as G25 or H7225"));
            }
        } else if let Some(v) = part.strip_prefix("lemma:") {
            word.lemma = Some(unquote(v));
        } else if let Some(v) = part.strip_prefix("morph:") {
            word.morph.extend(unquote(v).split([' ', ',']).filter(|w| !w.is_empty()).map(str::to_string));
        } else if is_strong(part) {
            word.strong = crate::markup::strong_number(part);
        } else {
            return Err(format!("'{part}' cannot be joined to a word with +"));
        }
    }
    Ok(Node::Word(word))
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }

    fn or(&mut self) -> Result<Node, String> {
        let mut left = self.and()?;
        while self.peek() == Some(&Tok::Or) {
            self.at += 1;
            left = Node::Or(Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Node, String> {
        let mut left = self.near()?;
        loop {
            match self.peek() {
                Some(Tok::And) => {
                    self.at += 1;
                    left = Node::And(Box::new(left), Box::new(self.near()?));
                }
                Some(Tok::Not) => {
                    self.at += 1;
                    left = Node::Not(Box::new(left), Box::new(self.near()?));
                }
                Some(Tok::Term(_)) | Some(Tok::Open) => left = Node::And(Box::new(left), Box::new(self.near()?)),
                _ => return Ok(left),
            }
        }
    }

    fn near(&mut self) -> Result<Node, String> {
        let mut left = self.atom()?;
        while let Some(Tok::Near(n)) = self.peek().cloned() {
            self.at += 1;
            left = Node::Near(Box::new(left), Box::new(self.atom()?), n);
        }
        Ok(left)
    }

    fn atom(&mut self) -> Result<Node, String> {
        match self.peek().cloned() {
            Some(Tok::Open) => {
                self.at += 1;
                let inner = self.or()?;
                if self.peek() != Some(&Tok::Close) {
                    return Err("a parenthesis is not closed".into());
                }
                self.at += 1;
                Ok(inner)
            }
            Some(Tok::Term(t)) => {
                self.at += 1;
                term(&t)
            }
            Some(other) => Err(format!("{} needs something to search for", match other {
                Tok::And => "AND",
                Tok::Or => "OR",
                Tok::Not => "NOT",
                Tok::Near(_) => "NEAR",
                _ => "A parenthesis",
            })),
            None => Err("the search ends where it expects a term".into()),
        }
    }
}

pub fn parse(input: &str) -> Result<Node, String> {
    let toks = lex(input)?;
    if toks.is_empty() {
        return Err("there is nothing to search for".into());
    }
    let mut p = Parser { toks, at: 0 };
    let node = p.or()?;
    if p.at != p.toks.len() {
        return Err("a closing parenthesis has no opening one".into());
    }
    Ok(node)
}

/// Whether a query needs more than plain search.
pub fn is_advanced(input: &str) -> bool {
    lex(input)
        .map(|toks| toks.iter().any(|t| !matches!(t, Tok::Term(x) if !x.contains(':') && !is_strong(x))))
        .unwrap_or(false)
}

/// The verses that match, by book, chapter and verse, with the positions of the words that made them.
pub type Matches = BTreeMap<(u8, u16, u16), Vec<u32>>;

fn word(module: &Module, term: &WordTerm) -> Result<Matches, String> {
    let hebrew = matches!(module.manifest.language.as_str(), "hbo" | "he" | "heb" | "arc");
    let mut filter = WordFilter::default();
    if term.strong.is_some() || term.lemma.is_some() {
        let lemmas = module.find_lemmas(term.strong.as_deref(), if term.strong.is_some() { None } else { term.lemma.as_deref() })?;
        let lemmas: Vec<_> = match (&term.strong, &term.lemma) {
            (Some(_), Some(l)) => {
                let key = crate::text::fold(l, &module.manifest.language);
                lemmas.into_iter().filter(|r| crate::text::fold(&r.lemma, &module.manifest.language) == key).collect()
            }
            _ => lemmas,
        };
        if lemmas.is_empty() {
            return Ok(Matches::new());
        }
        filter.lemma_ids = lemmas.iter().map(|l| l.id).collect();
    }
    if !term.morph.is_empty() {
        let words: Vec<&str> = term.morph.iter().map(String::as_str).collect();
        let (pattern, unknown) = crate::morph::pattern(&words, hebrew);
        if let Some(u) = unknown.first() {
            return Err(format!("'{u}' is not a term of morphology, such as aorist, imperative, qal or plural"));
        }
        filter.features = Some(pattern);
    }
    Ok(module.word_matches(&filter)?.into_iter().map(|(b, c, v, p)| ((b, c, v), p)).collect())
}

fn text(module: &Module, text: &str) -> Result<Matches, String> {
    let Some(fts) = fts_query(text, &module.manifest.language) else { return Ok(Matches::new()) };
    Ok(module.text_matches(&fts)?.into_iter().map(|k| (k, Vec::new())).collect())
}

/// Runs a query against one module.
pub fn run(module: &Module, node: &Node) -> Result<Matches, String> {
    Ok(match node {
        Node::Text(t) => text(module, t)?,
        Node::Word(w) => word(module, w)?,
        Node::And(a, b) => {
            let (a, b) = (run(module, a)?, run(module, b)?);
            a.into_iter()
                .filter_map(|(k, mut p)| {
                    let q = b.get(&k)?;
                    p.extend(q);
                    p.sort_unstable();
                    p.dedup();
                    Some((k, p))
                })
                .collect()
        }
        Node::Or(a, b) => {
            let mut a = run(module, a)?;
            for (k, p) in run(module, b)? {
                let e = a.entry(k).or_default();
                e.extend(p);
                e.sort_unstable();
                e.dedup();
            }
            a
        }
        Node::Not(a, b) => {
            let b = run(module, b)?;
            run(module, a)?.into_iter().filter(|(k, _)| !b.contains_key(k)).collect()
        }
        Node::Near(a, b, n) => match (a.as_ref(), b.as_ref()) {
            (Node::Text(x), Node::Text(y)) => {
                let (Some(fx), Some(fy)) = (fts_query(x, &module.manifest.language), fts_query(y, &module.manifest.language)) else {
                    return Ok(Matches::new());
                };
                let fts = format!("NEAR({fx} {fy}, {n})");
                module.text_matches(&fts)?.into_iter().map(|k| (k, Vec::new())).collect()
            }
            (Node::Text(_), _) | (_, Node::Text(_)) => {
                return Err("NEAR joins two words of the text, or two of Strong's numbers, lemmas and forms".into());
            }
            _ => {
                let (a, b) = (run(module, a)?, run(module, b)?);
                a.into_iter()
                    .filter_map(|(k, pa)| {
                        let pb = b.get(&k)?;
                        let close: Vec<u32> = pa
                            .iter()
                            .chain(pb.iter())
                            .copied()
                            .filter(|p| pa.iter().any(|x| x.abs_diff(*p) <= *n) && pb.iter().any(|y| y.abs_diff(*p) <= *n))
                            .collect();
                        let near = pa.iter().any(|x| pb.iter().any(|y| x != y && x.abs_diff(*y) <= *n));
                        near.then(|| {
                            let mut c = close;
                            c.sort_unstable();
                            c.dedup();
                            (k, c)
                        })
                    })
                    .collect()
            }
        },
    })
}

/// The plain words of a query, folded, for marking them in the results.
pub fn text_terms(node: &Node, language: &str, out: &mut Vec<String>) {
    match node {
        Node::Text(t) => {
            for w in crate::text::fold(t, language).split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
                if !out.iter().any(|x| x == w) {
                    out.push(w.to_string());
                }
            }
        }
        Node::Word(_) => {}
        Node::And(a, b) | Node::Or(a, b) | Node::Near(a, b, _) => {
            text_terms(a, language, out);
            text_terms(b, language, out);
        }
        Node::Not(a, _) => text_terms(a, language, out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(strong: Option<&str>, lemma: Option<&str>, morph: &[&str]) -> Node {
        Node::Word(WordTerm { strong: strong.map(str::to_string), lemma: lemma.map(str::to_string), morph: morph.iter().map(|s| s.to_string()).collect() })
    }

    #[test]
    fn the_grammar() {
        assert_eq!(parse("G25").unwrap(), w(Some("G25"), None, &[]));
        assert_eq!(parse("strong:H07225").unwrap(), w(Some("H7225"), None, &[]));
        assert_eq!(parse(r#"lemma:ἀγαπάω+morph:"aorist imperative""#).unwrap(), w(None, Some("ἀγαπάω"), &["aorist", "imperative"]));
        assert_eq!(
            parse("love NOT hate").unwrap(),
            Node::Not(Box::new(Node::Text("love".into())), Box::new(Node::Text("hate".into())))
        );
        assert_eq!(
            parse("(G25 OR G26) NEAR/3 G2316").unwrap(),
            Node::Near(Box::new(Node::Or(Box::new(w(Some("G25"), None, &[])), Box::new(w(Some("G26"), None, &[])))), Box::new(w(Some("G2316"), None, &[])), 3)
        );
        assert_eq!(
            parse("\"in principio\" verbum").unwrap(),
            Node::And(Box::new(Node::Text("\"in principio\"".into())), Box::new(Node::Text("verbum".into())))
        );
        assert!(parse("love AND").is_err());
        assert!(parse("(love").is_err());
        assert!(parse("strong:love").is_err());
        assert!(is_advanced("G25"));
        assert!(is_advanced("love OR hate"));
        assert!(!is_advanced("in principio"));
    }

    fn greek() -> (tempfile::TempDir, Module) {
        use crate::assemble::{assemble, Description};
        use crate::module::{BookContent, VerseRow, WordRow};
        let words: [(u16, &str, &str, &str, &str); 7] = [
            (1, "ἠγάπησεν", "ἀγαπάω", "G25", "mgnt:V-3AAI-S--"),
            (1, "ὁ", "ὁ", "G3588", "mgnt:RA----NSM-"),
            (1, "θεὸς", "θεός", "G2316", "mgnt:N-----NSM-"),
            (2, "ἀγαπᾶτε", "ἀγαπάω", "G25", "mgnt:V-2PAD-P--"),
            (2, "τὸν", "ὁ", "G3588", "mgnt:RA----ASM-"),
            (2, "θεόν", "θεός", "G2316", "mgnt:N-----ASM-"),
            (3, "λόγος", "λόγος", "G3056", "mgnt:N-----NSM-"),
        ];
        let mut body = BookContent::default();
        for v in 1..=3u16 {
            let text: Vec<&str> = words.iter().filter(|w| w.0 == v).map(|w| w.1).collect();
            body.verses.push(VerseRow { chapter: 3, verse: v, text: text.join(" "), paragraph: v == 1 });
        }
        for (v, text, lemma, strong, morph) in words {
            body.words.push(WordRow { chapter: 3, verse: v, text: text.into(), lemma: lemma.into(), strong: strong.into(), morph: morph.into(), ..Default::default() });
        }
        let mut books = std::collections::BTreeMap::new();
        books.insert(crate::canon::book_by_osis("John").unwrap().index, body);
        let description = Description { id: "t".into(), abbreviation: "T".into(), title: "T".into(), language: "grc".into(), versification: "hebrew".into(), license: "x".into(), ..Default::default() };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("T.grtb");
        crate::module::write(&path, &assemble(&description, books, "2026-10-30", "")).unwrap();
        let module = Module::open(&path).unwrap();
        (dir, module)
    }

    fn verses(m: &Module, q: &str) -> Vec<u16> {
        run(m, &parse(q).unwrap()).unwrap().keys().map(|k| k.2).collect()
    }

    #[test]
    fn queries_over_words_and_text() {
        let (_dir, m) = greek();
        assert!(m.has_words().unwrap());
        assert_eq!(verses(&m, "G25"), [1, 2]);
        assert_eq!(verses(&m, "lemma:αγαπαω+morph:imperative"), [2]);
        assert_eq!(verses(&m, "morph:aorist"), [1]);
        assert_eq!(verses(&m, "G25 NOT morph:imperative"), [1]);
        assert_eq!(verses(&m, "G3056 OR morph:imperative"), [2, 3]);
        assert_eq!(verses(&m, "θεο"), [1, 2]);
        assert_eq!(verses(&m, "G25 NEAR/1 G3588"), [1, 2]);
        assert_eq!(verses(&m, "G25 NEAR/1 G2316"), Vec::<u16>::new());
        assert_eq!(run(&m, &parse("G2316").unwrap()).unwrap()[&(50, 3, 2)], [3]);
        assert!(run(&m, &parse("morph:blue").unwrap()).unwrap_err().contains("blue"));
        assert!(run(&m, &parse("θεος NEAR G25").unwrap()).is_err());
    }
}
