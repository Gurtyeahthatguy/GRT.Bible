//! Source markup (OSIS, ThML, TEI, GBF or plain text) turned into the small
//! subset a module stores for commentaries, dictionaries and books.
//!
//! The subset is `<p>`, `<h4>`, `<b>`, `<i>`, `<sup>`, `<br>`, `<a data-ref="Gen.1.1-Gen.1.3">`
//! for a passage in the internal scheme and `<a data-key="KEY">` for another entry of
//! the same module. Nothing else is written, and every attribute value is escaped,
//! so the interface can rebuild it element by element without trusting it.

use crate::refs::Ref;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Osis,
    Thml,
    Tei,
    Gbf,
    Plain,
}

impl Source {
    /// The markup a SWORD `.conf` declares with `SourceType`.
    pub fn from_conf(value: Option<&str>) -> Source {
        match value.unwrap_or("").to_ascii_lowercase().as_str() {
            "osis" => Source::Osis,
            "thml" => Source::Thml,
            "tei" => Source::Tei,
            "gbf" => Source::Gbf,
            _ => Source::Plain,
        }
    }
}

/// Where a reference in the source points: an OSIS attribute or words such as "Ge 1:1".
pub enum Target<'a> {
    Osis(&'a str),
    Words(&'a str),
}

/// Turns a reference found in the source into internal ranges.
pub type Resolver<'r> = &'r dyn Fn(Target) -> Vec<(Ref, Ref)>;

fn entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        _ => {
            let n = name.strip_prefix('#')?;
            let code = match n.strip_prefix('x').or_else(|| n.strip_prefix('X')) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => n.parse().ok()?,
            };
            char::from_u32(code)?
        }
    })
}

pub fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find(';').filter(|&j| j <= 8).and_then(|j| entity(&after[..j]).map(|c| (j, c))) {
            Some((j, c)) => {
                out.push(c);
                rest = &after[j + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// A piece of markup: text, or a tag with its name and attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    Text(&'a str),
    Tag { raw_name: &'a str, closing: bool, empty: bool, attrs: &'a str },
}

/// Splits markup into text and tags; a `<` that opens nothing is text.
pub fn tokens(input: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut text_start = 0;
    while let Some(found) = input[pos..].find('<') {
        let open = pos + found;
        let Some(length) = input[open + 1..].find('>') else { break };
        let close = open + 1 + length;
        let inner = &input[open + 1..close];
        let skipped = inner.starts_with('!') || inner.starts_with('?');
        let body = inner.trim_start_matches('/');
        if !skipped && !body.starts_with(|c: char| c.is_ascii_alphabetic()) {
            pos = open + 1;
            continue;
        }
        if open > text_start {
            out.push(Token::Text(&input[text_start..open]));
        }
        text_start = close + 1;
        pos = close + 1;
        if skipped {
            continue;
        }
        let closing = inner.starts_with('/');
        let empty = body.ends_with('/');
        let body = body.trim_end_matches('/').trim();
        let name_end = body.find(|c: char| c.is_whitespace()).unwrap_or(body.len());
        out.push(Token::Tag { raw_name: &body[..name_end], closing, empty, attrs: &body[name_end..] });
    }
    if text_start < input.len() {
        out.push(Token::Text(&input[text_start..]));
    }
    out
}

/// Elements that never hold anything, written with or without the closing slash.
const VOID: [&str; 10] = ["br", "hr", "img", "lb", "sync", "milestone", "meta", "link", "input", "wbr"];

/// The value of an attribute, with entities decoded.
pub fn attr(attrs: &str, name: &str) -> Option<String> {
    let bytes = attrs.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let key = &attrs[start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            if key.is_empty() {
                i += 1;
            }
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let value = if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
            let quote = bytes[i];
            i += 1;
            let begin = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            let v = &attrs[begin..i.min(bytes.len())];
            i += 1;
            v
        } else {
            let begin = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            &attrs[begin..i]
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(decode_entities(value));
        }
    }
    None
}

/// Writes the subset, opening paragraphs only when text arrives.
#[derive(Default)]
struct Builder {
    out: String,
    /// Inline elements open in the current block, outermost first.
    stack: Vec<(String, String)>,
    in_block: bool,
    heading: bool,
    space: bool,
    line_start: bool,
    /// Set right after an opening, so the space that follows it is not written inside.
    glue: bool,
}

impl Builder {
    fn ensure_block(&mut self) {
        if self.in_block {
            return;
        }
        self.out.push_str(if self.heading { "<h4>" } else { "<p>" });
        self.in_block = true;
        self.line_start = true;
        self.space = false;
        for (name, attrs) in &self.stack {
            self.out.push_str(&format!("<{name}{attrs}>"));
        }
    }

    fn close_block(&mut self) {
        if !self.in_block {
            return;
        }
        if self.out.ends_with("<br>") {
            self.out.truncate(self.out.len() - 4);
        }
        for (name, _) in self.stack.iter().rev() {
            self.out.push_str(&format!("</{name}>"));
        }
        self.out.push_str(if self.heading { "</h4>" } else { "</p>" });
        self.in_block = false;
    }

    fn text(&mut self, raw: &str) {
        let decoded = decode_entities(raw);
        for (i, word) in decoded.split(char::is_whitespace).enumerate() {
            if i > 0 {
                self.space = true;
            }
            if word.is_empty() {
                continue;
            }
            self.ensure_block();
            if self.space && !self.line_start && !self.glue {
                self.out.push(' ');
            }
            self.out.push_str(&escape(word));
            self.space = false;
            self.line_start = false;
            self.glue = false;
        }
    }

    fn literal(&mut self, text: &str) {
        self.ensure_block();
        if self.space && !self.line_start {
            self.out.push(' ');
        }
        self.out.push_str(&escape(text));
        self.space = false;
        self.line_start = false;
        self.glue = false;
    }

    fn br(&mut self) {
        if self.in_block && !self.line_start {
            self.out.push_str("<br>");
            self.line_start = true;
            self.space = false;
        }
    }

    fn paragraph(&mut self) {
        self.close_block();
    }

    fn heading(&mut self, on: bool) {
        self.close_block();
        self.heading = on;
    }

    fn open(&mut self, name: &str, attrs: String) {
        if self.in_block {
            if self.space && !self.line_start {
                self.out.push(' ');
                self.space = false;
            }
            self.out.push_str(&format!("<{name}{attrs}>"));
            self.glue = true;
        }
        self.stack.push((name.to_string(), attrs));
    }

    fn close(&mut self, name: &str) {
        let Some(at) = self.stack.iter().rposition(|(n, _)| n == name) else { return };
        let closed: Vec<(String, String)> = self.stack.drain(at..).collect();
        if self.in_block {
            for (n, _) in closed.iter().rev() {
                self.out.push_str(&format!("</{n}>"));
            }
            for (n, a) in closed.iter().skip(1) {
                self.out.push_str(&format!("<{n}{a}>"));
            }
        }
        self.stack.extend(closed.into_iter().skip(1));
    }

    fn finish(mut self) -> String {
        self.close_block();
        self.out.replace("<p></p>", "").replace("<h4></h4>", "")
    }
}

/// What an element becomes in the subset.
enum Role {
    Transparent,
    Skip,
    Paragraph,
    Line,
    Heading,
    Inline(&'static str),
    Passage,
    EntryLink,
    Note,
}

fn gbf_role(name: &str) -> Role {
    match &name.to_ascii_uppercase()[..] {
        "CM" | "CP" => Role::Paragraph,
        "CL" => Role::Line,
        "FB" => Role::Inline("b"),
        "FI" => Role::Inline("i"),
        "FS" => Role::Inline("sup"),
        "RF" => Role::Note,
        "RX" => Role::Passage,
        "TS" | "TH" | "TT" => Role::Heading,
        n if n.starts_with('W') => Role::Skip,
        _ => Role::Transparent,
    }
}

fn role(source: Source, name: &str, attrs: &str) -> Role {
    if source == Source::Gbf && name.len() == 2 && name.chars().all(|c| c.is_ascii_alphabetic()) {
        return gbf_role(name);
    }
    let lower = name.to_ascii_lowercase();
    let kind = attr(attrs, "type").unwrap_or_default().to_ascii_lowercase();
    let rend = attr(attrs, "rend").unwrap_or_default().to_ascii_lowercase();
    match lower.as_str() {
        "p" | "lg" | "list" | "def" | "ul" | "ol" | "blockquote" => Role::Paragraph,
        "div" if kind == "x-milestone" || kind == "book" || kind == "chapter" => Role::Transparent,
        "div" => Role::Paragraph,
        "milestone" if kind == "x-p" || kind == "pb" => Role::Paragraph,
        "milestone" if kind == "line" || kind == "x-line" => Role::Line,
        "lb" | "br" | "l" | "item" | "li" | "row" | "sense" => Role::Line,
        "title" if source == Source::Tei => Role::Skip,
        "title" | "head" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Role::Heading,
        "b" | "strong" | "orth" => Role::Inline("b"),
        "i" | "em" | "emph" | "pron" | "foreign" => Role::Inline("i"),
        "sup" => Role::Inline("sup"),
        "hi" if kind == "bold" || rend == "bold" => Role::Inline("b"),
        "hi" if ["italic", "emphasis", "x-italic", "illuminated"].contains(&kind.as_str()) || rend == "italic" => Role::Inline("i"),
        "hi" if kind == "super" || rend == "sup" || rend == "super" => Role::Inline("sup"),
        "reference" | "scripref" => Role::Passage,
        "ref" if attr(attrs, "osisRef").is_some() => Role::Passage,
        "ref" if attr(attrs, "target").is_some() => Role::EntryLink,
        "note" => Role::Note,
        "img" | "figure" | "script" | "style" | "rdg" => Role::Skip,
        _ => Role::Transparent,
    }
}

fn is_closing(source: Source, name: &str, closing: bool) -> bool {
    if source == Source::Gbf && name.len() == 2 && name.chars().all(|c| c.is_ascii_alphabetic()) {
        return name.chars().nth(1).unwrap().is_ascii_lowercase();
    }
    closing
}

/// Elements whose start and end are written as two empty milestones.
fn milestone_end(attrs: &str) -> bool {
    attr(attrs, "eID").is_some()
}

struct PassageOpen {
    at: usize,
    target: Option<String>,
    words: String,
    stack_len: usize,
}

/// Converts one entry of the source into the stored subset.
pub fn to_markup(raw: &str, source: Source, resolve: Resolver) -> String {
    if source == Source::Plain {
        return plain(raw);
    }
    let mut b = Builder::default();
    let mut skip_depth: Vec<String> = Vec::new();
    let mut passages: Vec<PassageOpen> = Vec::new();
    let mut open_roles: Vec<(String, Role)> = Vec::new();
    for token in tokens(raw) {
        match token {
            Token::Text(t) => {
                if !skip_depth.is_empty() {
                    continue;
                }
                if let Some(p) = passages.last_mut() {
                    p.words.push_str(t);
                }
                b.text(t);
            }
            Token::Tag { raw_name, closing, empty, attrs } => {
                let closing = is_closing(source, raw_name, closing);
                let key = raw_name.to_ascii_lowercase();
                // GBF word tags such as <WH0430> stand alone.
                let empty = empty || VOID.contains(&key.as_str()) || (source == Source::Gbf && raw_name.len() != 2);
                if let Some(top) = skip_depth.last() {
                    if closing && *top == key {
                        skip_depth.pop();
                    } else if !closing && !empty && *top == key {
                        skip_depth.push(key);
                    }
                    continue;
                }
                if closing {
                    let Some(at) = open_roles.iter().rposition(|(n, _)| *n == key) else { continue };
                    let (_, role) = open_roles.remove(at);
                    end_role(&mut b, role, &mut passages, resolve);
                    continue;
                }
                let r = role(source, raw_name, attrs);
                if empty || milestone_end(attrs) {
                    match r {
                        Role::Paragraph | Role::Heading => b.paragraph(),
                        Role::Line => b.br(),
                        _ => {}
                    }
                    continue;
                }
                match r {
                    Role::Skip => skip_depth.push(key),
                    Role::Paragraph => {
                        b.paragraph();
                        open_roles.push((key, Role::Paragraph));
                    }
                    Role::Line => {
                        b.br();
                        open_roles.push((key, Role::Line));
                    }
                    Role::Heading => {
                        b.heading(true);
                        open_roles.push((key, Role::Heading));
                    }
                    Role::Inline(name) => {
                        b.open(name, String::new());
                        open_roles.push((key, Role::Inline(name)));
                    }
                    Role::Passage => {
                        b.ensure_block();
                        let target = attr(attrs, "osisRef").or_else(|| attr(attrs, "passage"));
                        passages.push(PassageOpen { at: b.out.len(), target, words: String::new(), stack_len: b.stack.len() });
                        open_roles.push((key, Role::Passage));
                    }
                    Role::EntryLink => {
                        let target = attr(attrs, "target").unwrap_or_default();
                        let target = target.rsplit(':').next().unwrap_or("").to_string();
                        b.open("a", format!(" data-key=\"{}\"", escape(&target)));
                        open_roles.push((key, Role::EntryLink));
                    }
                    Role::Note => {
                        b.space = true;
                        b.literal("[");
                        b.open("i", String::new());
                        open_roles.push((key, Role::Note));
                    }
                    Role::Transparent => open_roles.push((key, Role::Transparent)),
                }
            }
        }
    }
    while let Some((_, role)) = open_roles.pop() {
        end_role(&mut b, role, &mut passages, resolve);
    }
    b.finish()
}

fn end_role(b: &mut Builder, role: Role, passages: &mut Vec<PassageOpen>, resolve: Resolver) {
    match role {
        Role::Paragraph => b.paragraph(),
        Role::Line => b.br(),
        Role::Heading => b.heading(false),
        Role::Inline(name) => b.close(name),
        Role::EntryLink => b.close("a"),
        Role::Note => {
            b.close("i");
            b.out.push(']');
        }
        Role::Passage => {
            let Some(p) = passages.pop() else { return };
            // A reference that spans a paragraph break, or nests in a way the
            // subset cannot hold, keeps its words without the link.
            if !b.in_block || b.stack.len() != p.stack_len || p.at > b.out.len() {
                return;
            }
            let ranges = match &p.target {
                Some(t) if t.contains('.') => resolve(Target::Osis(t)),
                Some(t) => resolve(Target::Words(t)),
                None => resolve(Target::Words(&decode_entities(&p.words))),
            };
            if let Some((first, last)) = ranges.first() {
                let link = crate::module::range_osis(*first, *last);
                let lead = b.out[p.at..].len() - b.out[p.at..].trim_start().len();
                b.out.insert_str(p.at + lead, &format!("<a data-ref=\"{}\">", escape(&link)));
                b.out.push_str("</a>");
            }
        }
        Role::Transparent | Role::Skip => {}
    }
}

/// Plain text: blank lines part paragraphs, single line ends become breaks.
fn plain(raw: &str) -> String {
    let text = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::new();
    for para in text.split("\n\n") {
        let lines: Vec<String> = para
            .lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|l| !l.is_empty())
            .map(|l| escape(&l))
            .collect();
        if !lines.is_empty() {
            out.push_str("<p>");
            out.push_str(&lines.join("<br>"));
            out.push_str("</p>");
        }
    }
    out
}

/// The words of the markup without any tags, for titles and plain previews.
pub fn plain_text(markup: &str) -> String {
    let mut out = String::new();
    for token in tokens(markup) {
        match token {
            Token::Text(t) => out.push_str(&decode_entities(t)),
            Token::Tag { raw_name: "p" | "h4" | "br", .. } => out.push(' '),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A word of a verse with what the source says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text(String),
    Word { text: String, lemma: Vec<String>, morph: Vec<String> },
}

fn push_tag(pieces: &mut Vec<Piece>, lemma: Option<String>, morph: Option<String>) {
    let lemma: Vec<String> = lemma.into_iter().collect();
    let morph: Vec<String> = morph.into_iter().collect();
    if let Some(Piece::Word { lemma: l, morph: m, .. }) = pieces.last_mut() {
        l.extend(lemma);
        m.extend(morph);
        return;
    }
    // The tag belongs to the last word of the text before it.
    let Some(Piece::Text(text)) = pieces.last_mut() else { return };
    let trimmed = text.trim_end_matches(|c: char| !c.is_alphanumeric());
    let start = trimmed.rfind(|c: char| !c.is_alphanumeric() && c != '\'' && c != '\u{2019}').map(|i| i + 1).unwrap_or(0);
    if start >= trimmed.len() {
        return;
    }
    let word = trimmed[start..].to_string();
    let after = text[trimmed.len()..].to_string();
    text.truncate(start);
    let before_empty = text.is_empty();
    if before_empty {
        pieces.pop();
    }
    pieces.push(Piece::Word { text: word, lemma, morph });
    if !after.is_empty() {
        pieces.push(Piece::Text(after));
    }
}

/// The verse with its tagged words marked, when the source tags any.
pub fn tag_words(raw: &str, source: Source) -> Option<String> {
    let mut pieces: Vec<Piece> = Vec::new();
    let mut skip: Vec<String> = Vec::new();
    let mut word: Option<(String, Vec<String>, Vec<String>)> = None;
    let mut any = false;
    let skipped = ["note", "title", "rf", "ts"];
    for token in tokens(raw) {
        match token {
            Token::Text(t) => {
                if !skip.is_empty() {
                    continue;
                }
                let t = decode_entities(t);
                match &mut word {
                    Some((text, _, _)) => text.push_str(&t),
                    None => match pieces.last_mut() {
                        Some(Piece::Text(last)) => last.push_str(&t),
                        _ => pieces.push(Piece::Text(t)),
                    },
                }
            }
            Token::Tag { raw_name, closing, empty, attrs } => {
                let closing = is_closing(source, raw_name, closing);
                let key = raw_name.to_ascii_lowercase();
                if let Some(top) = skip.last() {
                    if closing && *top == key {
                        skip.pop();
                    } else if !closing && !empty && *top == key {
                        skip.push(key);
                    }
                    continue;
                }
                let empty = empty || VOID.contains(&key.as_str());
                if !closing && !empty && skipped.contains(&key.as_str()) {
                    skip.push(key);
                    continue;
                }
                if source == Source::Gbf && raw_name.len() > 2 && raw_name.starts_with('W') {
                    let rest = &raw_name[1..];
                    match rest.chars().next() {
                        Some('H') | Some('G') => push_tag(&mut pieces, Some(format!("strong:{rest}")), None),
                        Some('T') => push_tag(&mut pieces, None, Some(format!("strongMorph:{rest}"))),
                        _ => continue,
                    }
                    any = true;
                    continue;
                }
                if key == "sync" {
                    let value = attr(attrs, "value").unwrap_or_default();
                    match attr(attrs, "type").unwrap_or_default().to_ascii_lowercase().as_str() {
                        "strongs" if !value.is_empty() => push_tag(&mut pieces, Some(format!("strong:{value}")), None),
                        "morph" if !value.is_empty() => {
                            let class = attr(attrs, "class").unwrap_or_else(|| "morph".into());
                            push_tag(&mut pieces, None, Some(format!("{class}:{value}")));
                        }
                        _ => continue,
                    }
                    any = true;
                    continue;
                }
                if key == "w" && !closing && !empty {
                    let split = |v: Option<String>| v.map(|s| s.split_whitespace().map(str::to_string).collect()).unwrap_or_default();
                    word = Some((String::new(), split(attr(attrs, "lemma")), split(attr(attrs, "morph"))));
                    continue;
                }
                if key == "w" && closing {
                    if let Some((text, lemma, morph)) = word.take() {
                        if lemma.is_empty() && morph.is_empty() {
                            match pieces.last_mut() {
                                Some(Piece::Text(last)) => last.push_str(&text),
                                _ => pieces.push(Piece::Text(text)),
                            }
                        } else {
                            any = true;
                            pieces.push(Piece::Word { text, lemma, morph });
                        }
                    }
                }
            }
        }
    }
    if !any {
        return None;
    }
    let mut out = String::new();
    for piece in pieces {
        match piece {
            Piece::Text(t) => out.push_str(&escape(&t)),
            Piece::Word { text, lemma, morph } => {
                out.push_str("<w");
                if !lemma.is_empty() {
                    out.push_str(&format!(" lemma=\"{}\"", escape(&lemma.join(" "))));
                }
                if !morph.is_empty() {
                    out.push_str(&format!(" morph=\"{}\"", escape(&morph.join(" "))));
                }
                out.push('>');
                out.push_str(&escape(&text));
                out.push_str("</w>");
            }
        }
    }
    Some(out.split_whitespace().collect::<Vec<_>>().join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: Target) -> Vec<(Ref, Ref)> {
        Vec::new()
    }

    fn gen1(t: Target) -> Vec<(Ref, Ref)> {
        let r = Ref::parse("Gen.1.1").unwrap();
        match t {
            Target::Osis(_) | Target::Words(_) => vec![(r, r)],
        }
    }

    #[test]
    fn osis_commentary() {
        let raw = r#"<div sID="gen3" type="x-p"/><hi type="bold">Ver.</hi> 1. The <reference osisRef="Gen.1.1">book</reference> &amp; more.<div eID="gen3" type="x-p"/><div sID="g4" type="x-p"/>Next<note>a note</note> part.<div eID="g4" type="x-p"/>"#;
        assert_eq!(
            to_markup(raw, Source::Osis, &gen1),
            r#"<p><b>Ver.</b> 1. The <a data-ref="Gen.1.1">book</a> &amp; more.</p><p>Next [<i>a note</i>] part.</p>"#
        );
    }

    #[test]
    fn thml_cross_references() {
        let raw = "<br />beginning.<br /><scripRef>Pr 8:22-24</scripRef><br />";
        assert_eq!(to_markup(raw, Source::Thml, &gen1), r#"<p>beginning.<br><a data-ref="Gen.1.1">Pr 8:22-24</a></p>"#);
        assert_eq!(to_markup(raw, Source::Thml, &none), "<p>beginning.<br>Pr 8:22-24</p>");
    }

    #[test]
    fn tei_dictionary() {
        let raw = "<entryFree n=\"25\">\n<title>X</title><orth>ἀγαπάω</orth><lb/>\n<pron rend=\"italic\">{ag-ap-ah'-o}</pron>\n<def>to love. Compare <ref target=\"StrongsGreek:5368\">5368</ref>.</def></entryFree>";
        assert_eq!(
            to_markup(raw, Source::Tei, &none),
            r#"<p><b>ἀγαπάω</b><br><i>{ag-ap-ah'-o}</i></p><p>to love. Compare <a data-key="5368">5368</a>.</p>"#
        );
    }

    #[test]
    fn hostile_markup_stays_inert() {
        let raw = r#"<p onclick="x()">Hi <script>alert(1)</script><a href="javascript:x">there</a> <img src=x onerror=y> "q" <b>bold"#;
        let out = to_markup(raw, Source::Thml, &none);
        assert_eq!(out, "<p>Hi there &quot;q&quot; <b>bold</b></p>");
        assert!(!out.contains("script") && !out.contains("href") && !out.contains("onerror"));
    }

    #[test]
    fn plain_text_paragraphs() {
        assert_eq!(plain(" 1  'ab  awb\r\n\r\n a primitive word;\r\n father\r\n"), "<p>1 'ab awb</p><p>a primitive word;<br>father</p>");
        assert_eq!(plain_text("<h4>Title</h4><p>One<br>two &amp; three</p>"), "Title One two & three");
    }

    #[test]
    fn strongs_numbers_are_kept() {
        let osis = r#"<w lemma="strong:H07225" morph="x">In the beginning</w> <w lemma="strong:H0430">God</w><note>n</note> created."#;
        assert_eq!(
            tag_words(osis, Source::Osis).unwrap(),
            r#"<w lemma="strong:H07225" morph="x">In the beginning</w> <w lemma="strong:H0430">God</w> created."#
        );
        let gbf = "In the beginning<WH07225> God<WH0430><WTH8804> created.";
        assert_eq!(
            tag_words(gbf, Source::Gbf).unwrap(),
            r#"In the <w lemma="strong:H07225">beginning</w> <w lemma="strong:H0430" morph="strongMorph:TH8804">God</w> created."#
        );
        let thml = r#"Jesus<sync type="Strongs" value="G2424"/> wept."#;
        assert_eq!(tag_words(thml, Source::Thml).unwrap(), r#"<w lemma="strong:G2424">Jesus</w> wept."#);
        assert!(tag_words("Plain <i>text</i>.", Source::Osis).is_none());
    }

    #[test]
    fn attributes() {
        assert_eq!(attr(r#" type="x-p" sID='a b' n=3"#, "sid").as_deref(), Some("a b"));
        assert_eq!(attr(r#" n=3 passage="Ge 1:1""#, "passage").as_deref(), Some("Ge 1:1"));
        assert_eq!(attr(r#" x="&amp;""#, "x").as_deref(), Some("&"));
        assert!(attr(" x", "x").is_none());
    }
}
