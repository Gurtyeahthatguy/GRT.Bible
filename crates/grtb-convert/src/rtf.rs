//! Just enough RTF to read a word processor export: text runs and their formatting.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Text { text: String, hidden: bool, italic: bool, bold: bool },
    Paragraph,
}

#[derive(Clone, Copy, Default)]
struct State {
    hidden: bool,
    italic: bool,
    bold: bool,
    skip: bool,
    unicode_skip: usize,
}

const DESTINATIONS: &[&str] = &[
    "fonttbl", "colortbl", "stylesheet", "info", "listtable", "listoverridetable", "rsidtbl", "generator",
    "xmlnstbl", "mmathPr", "themedata", "colorschememapping", "latentstyles", "datastore", "pgdsctbl", "fldinst",
    "pict", "bkmkstart", "bkmkend", "header", "headerl", "headerr", "footer", "footerl", "footerr",
    "wgrffmtfilter", "panose", "falt", "ftnsep", "ftnsepc", "aftnsep", "aftnsepc", "pnseclvl", "shppict",
    "nonshppict", "listpicture", "objdata", "defchp", "defpap", "footnote", "annotation", "field",
];

fn cp1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘',
        '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    if (0x80..0xA0).contains(&byte) {
        HIGH[(byte - 0x80) as usize]
    } else {
        byte as char
    }
}

pub fn events(bytes: &[u8]) -> Vec<Event> {
    let mut out: Vec<Event> = Vec::new();
    let mut stack: Vec<State> = Vec::new();
    let mut state = State::default();
    let mut uc = 1usize;
    let mut i = 0;
    let push = |out: &mut Vec<Event>, state: &State, ch: char| {
        if state.skip {
            return;
        }
        if let Some(Event::Text { text, hidden, italic, bold }) = out.last_mut() {
            if *hidden == state.hidden && *italic == state.italic && *bold == state.bold {
                text.push(ch);
                return;
            }
        }
        out.push(Event::Text { text: ch.to_string(), hidden: state.hidden, italic: state.italic, bold: state.bold });
    };

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'{' => {
                stack.push(state);
                state.unicode_skip = 0;
                i += 1;
            }
            b'}' => {
                state = stack.pop().unwrap_or_default();
                i += 1;
            }
            b'\r' | b'\n' => i += 1,
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    break;
                }
                let c = bytes[i];
                if c.is_ascii_alphabetic() {
                    let start = i;
                    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
                    let num_start = i;
                    if i < bytes.len() && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                        i += 1;
                        while i < bytes.len() && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    let param: Option<i32> = std::str::from_utf8(&bytes[num_start..i]).ok().and_then(|s| s.parse().ok());
                    if i < bytes.len() && bytes[i] == b' ' {
                        i += 1;
                    }
                    let on = param.map(|p| p != 0).unwrap_or(true);
                    match word {
                        w if DESTINATIONS.contains(&w) => state.skip = true,
                        "v" => state.hidden = on,
                        "i" => state.italic = on,
                        "b" => state.bold = on,
                        "plain" => {
                            state.hidden = false;
                            state.italic = false;
                            state.bold = false;
                        }
                        "par" | "line" | "page" | "sect" => {
                            if !state.skip {
                                out.push(Event::Paragraph);
                            }
                        }
                        "tab" => push(&mut out, &state, ' '),
                        "uc" => uc = param.unwrap_or(1).max(0) as usize,
                        "u" => {
                            let mut n = param.unwrap_or(0);
                            if n < 0 {
                                n += 65536;
                            }
                            if let Some(ch) = char::from_u32(n as u32) {
                                push(&mut out, &state, ch);
                            }
                            state.unicode_skip = uc;
                        }
                        "emdash" => push(&mut out, &state, '—'),
                        "endash" => push(&mut out, &state, '–'),
                        "lquote" => push(&mut out, &state, '‘'),
                        "rquote" => push(&mut out, &state, '’'),
                        "ldblquote" => push(&mut out, &state, '“'),
                        "rdblquote" => push(&mut out, &state, '”'),
                        _ => {}
                    }
                } else {
                    i += 1;
                    match c {
                        b'*' => state.skip = true,
                        b'\'' => {
                            if i + 2 <= bytes.len() {
                                let hex = std::str::from_utf8(&bytes[i..i + 2]).unwrap_or("00");
                                i += 2;
                                if state.unicode_skip > 0 {
                                    state.unicode_skip -= 1;
                                } else if let Ok(v) = u8::from_str_radix(hex, 16) {
                                    push(&mut out, &state, cp1252(v));
                                }
                            }
                        }
                        b'~' => push(&mut out, &state, '\u{a0}'),
                        b'_' => push(&mut out, &state, '-'),
                        b'-' => {}
                        b'\\' | b'{' | b'}' => push(&mut out, &state, c as char),
                        _ => {}
                    }
                }
            }
            _ => {
                if state.unicode_skip > 0 {
                    state.unicode_skip -= 1;
                } else {
                    push(&mut out, &state, cp1252(b));
                }
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_and_formatting() {
        let rtf = br"{\rtf1{\fonttbl{\f0 Times;}}{\v _01001001}{\i Summary.}\par{\b Gen 1:1} Al principio cre\'f2 Dio.\u8217? ok}";
        let ev = events(rtf);
        assert_eq!(ev[0], Event::Text { text: "_01001001".into(), hidden: true, italic: false, bold: false });
        assert_eq!(ev[1], Event::Text { text: "Summary.".into(), hidden: false, italic: true, bold: false });
        assert_eq!(ev[2], Event::Paragraph);
        assert_eq!(ev[3], Event::Text { text: "Gen 1:1".into(), hidden: false, italic: false, bold: true });
        assert_eq!(ev[4], Event::Text { text: " Al principio creò Dio.’ ok".into(), hidden: false, italic: false, bold: false });
    }
}
