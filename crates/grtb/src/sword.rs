//! Reading CrossWire SWORD Bible modules from a local file.

use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::module::{BookContent, NoteRow, TitleRow, VerseRow};
use crate::source::{decode, sha256_hex};
use crate::usfm::UsfmBook;

const TABLES: &str = include_str!("../data/sword.txt");

/// Book list and verse counts of one SWORD versification.
#[derive(Debug, Clone)]
pub struct V11n {
    pub ot: Vec<(String, Vec<u16>)>,
    pub nt: Vec<(String, Vec<u16>)>,
}

fn tables() -> &'static HashMap<String, V11n> {
    static T: OnceLock<HashMap<String, V11n>> = OnceLock::new();
    T.get_or_init(|| {
        let mut out = HashMap::new();
        let mut name = String::new();
        let mut cur = V11n { ot: vec![], nt: vec![] };
        for line in TABLES.lines() {
            if let Some(n) = line.strip_prefix("# ") {
                if !name.is_empty() {
                    out.insert(name.clone(), cur);
                }
                name = n.to_string();
                cur = V11n { ot: vec![], nt: vec![] };
                continue;
            }
            let mut parts = line.split_whitespace();
            let (Some(testament), Some(book)) = (parts.next(), parts.next()) else { continue };
            let counts: Vec<u16> = parts.map(|n| n.parse().unwrap()).collect();
            if testament == "OT" {
                cur.ot.push((book.to_string(), counts));
            } else {
                cur.nt.push((book.to_string(), counts));
            }
        }
        out.insert(name, cur);
        out
    })
}

pub fn v11n(name: &str) -> Option<&'static V11n> {
    tables().get(name).or_else(|| tables().iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v))
}

/// The internal scheme closest to a SWORD versification.
pub fn scheme_for(name: &str) -> &'static str {
    match name.to_ascii_lowercase().as_str() {
        "vulg" => "vulgate",
        "lxx" | "orthodox" | "synodal" | "synodalprot" => "lxx",
        "mt" | "leningrad" | "german" | "luther" => "hebrew",
        _ => "english",
    }
}

fn usfm_code(osis: &str) -> Option<&'static str> {
    if let Some(b) = crate::canon::book_by_osis(osis) {
        return Some(match b.osis {
            "Gen" => "GEN", "Exod" => "EXO", "Lev" => "LEV", "Num" => "NUM", "Deut" => "DEU", "Josh" => "JOS",
            "Judg" => "JDG", "Ruth" => "RUT", "1Sam" => "1SA", "2Sam" => "2SA", "1Kgs" => "1KI", "2Kgs" => "2KI",
            "1Chr" => "1CH", "2Chr" => "2CH", "Ezra" => "EZR", "Neh" => "NEH", "Tob" => "TOB", "Jdt" => "JDT",
            "Esth" => "EST", "1Macc" => "1MA", "2Macc" => "2MA", "Job" => "JOB", "Ps" => "PSA", "Prov" => "PRO",
            "Eccl" => "ECC", "Song" => "SNG", "Wis" => "WIS", "Sir" => "SIR", "Isa" => "ISA", "Jer" => "JER",
            "Lam" => "LAM", "Bar" => "BAR", "Ezek" => "EZK", "Dan" => "DAN", "Hos" => "HOS", "Joel" => "JOL",
            "Amos" => "AMO", "Obad" => "OBA", "Jonah" => "JON", "Mic" => "MIC", "Nah" => "NAM", "Hab" => "HAB",
            "Zeph" => "ZEP", "Hag" => "HAG", "Zech" => "ZEC", "Mal" => "MAL", "Matt" => "MAT", "Mark" => "MRK",
            "Luke" => "LUK", "John" => "JHN", "Acts" => "ACT", "Rom" => "ROM", "1Cor" => "1CO", "2Cor" => "2CO",
            "Gal" => "GAL", "Eph" => "EPH", "Phil" => "PHP", "Col" => "COL", "1Thess" => "1TH", "2Thess" => "2TH",
            "1Tim" => "1TI", "2Tim" => "2TI", "Titus" => "TIT", "Phlm" => "PHM", "Heb" => "HEB", "Jas" => "JAS",
            "1Pet" => "1PE", "2Pet" => "2PE", "1John" => "1JN", "2John" => "2JN", "3John" => "3JN", "Jude" => "JUD",
            _ => "REV",
        });
    }
    Some(match osis {
        "EpJer" => "LJE",
        "Sus" => "SUS",
        "Bel" => "BEL",
        "PrAzar" => "S3Y",
        _ => return None,
    })
}

/// A parsed `.conf` file.
#[derive(Debug, Clone, Default)]
pub struct Conf {
    pub name: String,
    pub values: BTreeMap<String, String>,
}

impl Conf {
    pub fn parse(text: &str) -> Conf {
        let mut conf = Conf::default();
        let mut last_key: Option<String> = None;
        let mut continuing = false;
        for raw in text.lines() {
            let line = raw.trim_end_matches('\r');
            if continuing {
                if let Some(k) = &last_key {
                    let v = conf.values.entry(k.clone()).or_default();
                    v.push('\n');
                    v.push_str(line.trim_end_matches('\\'));
                }
                continuing = line.ends_with('\\');
                continue;
            }
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                conf.name = trimmed[1..trimmed.len() - 1].to_string();
            } else if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim().to_string();
                let value = v.trim().trim_end_matches('\\').to_string();
                continuing = v.trim_end().ends_with('\\');
                conf.values.entry(key.clone()).or_insert(value);
                last_key = Some(key);
            }
        }
        conf
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str).filter(|v| !v.is_empty())
    }
}

/// Files of a module, found either in a folder or inside a ZIP.
trait Files {
    fn read(&mut self, path: &str) -> Option<Vec<u8>>;
    fn list(&self) -> Vec<String>;
}

struct DirFiles(PathBuf);

impl Files for DirFiles {
    fn read(&mut self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(path)).ok()
    }
    fn list(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![self.0.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(rel) = p.strip_prefix(&self.0) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        out.sort();
        out
    }
}

struct ZipFiles(zip::ZipArchive<std::io::Cursor<Vec<u8>>>);

impl Files for ZipFiles {
    fn read(&mut self, path: &str) -> Option<Vec<u8>> {
        let name = self.0.file_names().find(|n| n.eq_ignore_ascii_case(path))?.to_string();
        let mut entry = self.0.by_name(&name).ok()?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data).ok()?;
        Some(data)
    }
    fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = self.0.file_names().map(str::to_string).collect();
        names.sort();
        names
    }
}

pub struct SwordModule {
    pub conf: Conf,
    pub books: Vec<UsfmBook>,
    pub hash: String,
    pub warnings: Vec<String>,
}

fn u16le(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32le(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// Okumura LZSS as SWORD writes it.
pub fn lzss_decompress(input: &[u8]) -> Vec<u8> {
    const N: usize = 4096;
    const F: usize = 18;
    const THRESHOLD: usize = 3;
    let mut ring = [b' '; N];
    let mut r = N - F;
    let mut out = Vec::with_capacity(input.len() * 3);
    let mut i = 0;
    while i < input.len() {
        let flags = input[i];
        i += 1;
        for bit in 0..8 {
            if i >= input.len() {
                break;
            }
            if flags & (1 << bit) != 0 {
                let c = input[i];
                i += 1;
                out.push(c);
                ring[r] = c;
                r = (r + 1) & (N - 1);
            } else {
                if i + 1 >= input.len() {
                    i = input.len();
                    break;
                }
                let lo = input[i] as usize;
                let hi = input[i + 1] as usize;
                i += 2;
                let position = lo | ((hi & 0xF0) << 4);
                let length = (hi & 0x0F) + THRESHOLD;
                for k in 0..length {
                    let c = ring[(position + k) & (N - 1)];
                    out.push(c);
                    ring[r] = c;
                    r = (r + 1) & (N - 1);
                }
            }
        }
    }
    out
}

fn decompress(kind: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    match kind.to_ascii_uppercase().as_str() {
        "ZIP" | "" => {
            flate2::read::ZlibDecoder::new(data).read_to_end(&mut out).map_err(|e| format!("zlib: {e}"))?;
        }
        "LZSS" => out = lzss_decompress(data),
        "BZIP2" => {
            bzip2_rs::DecoderReader::new(data).read_to_end(&mut out).map_err(|e| format!("bzip2: {e}"))?;
        }
        "XZ" => {
            lzma_rs::xz_decompress(&mut std::io::BufReader::new(data), &mut out).map_err(|e| format!("xz: {e:?}"))?;
        }
        other => return Err(format!("compression {other} is not supported")),
    }
    Ok(out)
}

/// Entry text by verse index, for one testament.
struct Testament {
    entries: Vec<Option<Vec<u8>>>,
}

fn read_testament(files: &mut dyn Files, dir: &str, prefix: &str, conf: &Conf) -> Result<Option<Testament>, String> {
    let driver = conf.get("ModDrv").unwrap_or("zText").to_ascii_lowercase();
    let path = |ext: &str| format!("{dir}{prefix}{ext}");
    match driver.as_str() {
        "ztext" | "ztext4" => {
            let wide = driver == "ztext4";
            let (Some(bzs), Some(bzv), Some(bzz)) = (files.read(&path(".bzs")), files.read(&path(".bzv")), files.read(&path(".bzz"))) else {
                return Ok(None);
            };
            let compress = conf.get("CompressType").unwrap_or("ZIP");
            let mut blocks: HashMap<u32, Vec<u8>> = HashMap::new();
            let step = if wide { 12 } else { 10 };
            let mut entries = Vec::with_capacity(bzv.len() / step);
            for i in (0..bzv.len().saturating_sub(step - 1)).step_by(step) {
                let block = u32le(&bzv, i);
                let offset = u32le(&bzv, i + 4) as usize;
                let size = if wide { u32le(&bzv, i + 8) as usize } else { u16le(&bzv, i + 8) as usize };
                if size == 0 {
                    entries.push(None);
                    continue;
                }
                if !blocks.contains_key(&block) {
                    let at = block as usize * 12;
                    if at + 12 > bzs.len() {
                        entries.push(None);
                        continue;
                    }
                    let start = u32le(&bzs, at) as usize;
                    let len = u32le(&bzs, at + 4) as usize;
                    let raw = bzz.get(start..start + len).ok_or("block runs past the end of the data file")?;
                    blocks.insert(block, decompress(compress, raw)?);
                }
                let data = &blocks[&block];
                entries.push(data.get(offset..offset + size).map(<[u8]>::to_vec));
            }
            Ok(Some(Testament { entries }))
        }
        "rawtext" | "rawtext4" => {
            let wide = driver == "rawtext4";
            let (Some(vss), Some(text)) = (files.read(&path(".vss")), files.read(&format!("{dir}{prefix}"))) else {
                return Ok(None);
            };
            let step = if wide { 8 } else { 6 };
            let mut entries = Vec::with_capacity(vss.len() / step);
            for i in (0..vss.len().saturating_sub(step - 1)).step_by(step) {
                let start = u32le(&vss, i) as usize;
                let size = if wide { u32le(&vss, i + 4) as usize } else { u16le(&vss, i + 4) as usize };
                entries.push(if size == 0 { None } else { text.get(start..start + size).map(<[u8]>::to_vec) });
            }
            Ok(Some(Testament { entries }))
        }
        other => Err(format!("module driver {other} is not a Bible text driver GRT Bible can read")),
    }
}

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
            let code = match n.strip_prefix('x') {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => n.parse().ok()?,
            };
            char::from_u32(code)?
        }
    })
}

fn decode_entities(text: &str) -> String {
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

fn remove_between(text: &str, open: &str, close: &str, found: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let lower_open = open.to_ascii_lowercase();
    loop {
        let hay = rest.to_ascii_lowercase();
        let Some(start) = hay.find(&lower_open) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let tag_end = hay[start..].find('>').map(|i| start + i + 1).unwrap_or(hay.len());
        let self_closing = rest[start..tag_end].ends_with("/>");
        if self_closing {
            rest = &rest[tag_end..];
            continue;
        }
        let end = hay[tag_end..].find(&close.to_ascii_lowercase()).map(|i| tag_end + i).unwrap_or(hay.len());
        found.push(rest[start..end].to_string());
        rest = rest.get(end + close.len()..).unwrap_or("");
    }
    out
}

fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = false;
    for ch in text.chars() {
        match ch {
            '<' => depth = true,
            '>' if depth => depth = false,
            _ if !depth => out.push(ch),
            _ => {}
        }
    }
    out
}

struct Entry {
    text: String,
    titles: Vec<(String, String)>,
    notes: Vec<String>,
    paragraph_before: bool,
    paragraph_after: bool,
}

/// OSIS, ThML or GBF markup to plain text with line breaks.
fn plain_entry(raw: &str) -> Entry {
    let mut notes = Vec::new();
    let mut titles = Vec::new();
    let mut text = remove_between(raw, "<note", "</note>", &mut notes);
    text = remove_between(&text, "<RF", "<Rf>", &mut notes);
    let mut title_bodies = Vec::new();
    text = remove_between(&text, "<title", "</title>", &mut title_bodies);
    text = remove_between(&text, "<TS", "<Ts>", &mut title_bodies);
    for t in title_bodies {
        let open = t.split('>').next().unwrap_or("").replace('\'', "\"");
        if open.contains("type=\"chapter\"") || open.contains("type=\"x-gen\"") {
            continue;
        }
        let kind = if open.contains("type=\"psalm\"") { "psalm" } else { "section" };
        let clean = decode_entities(&strip_tags(&t)).split_whitespace().collect::<Vec<_>>().join(" ");
        if !clean.is_empty() {
            titles.push((kind.to_string(), clean));
        }
    }
    let breaks = [
        "<milestone type=\"x-p\"", "<p>", "<p ", "</p>", "<lg", "</lg>", "<CM>", "<div type=\"paragraph\"", "<div sID",
    ];
    let lines = ["<lb", "<l ", "<l>", "</l>", "<br", "<CL>"];
    let mut marked = text.clone();
    for b in breaks {
        marked = marked.replace(b, &format!("\u{E010}{b}"));
    }
    for l in lines {
        marked = marked.replace(l, &format!("\u{E011}{l}"));
    }
    let flat = decode_entities(&strip_tags(&marked));
    let first_text = flat.find(|c: char| !c.is_whitespace() && c != '\u{E010}' && c != '\u{E011}');
    let paragraph_before = first_text.map(|i| flat[..i].contains('\u{E010}')).unwrap_or(false);
    let last_text = flat.rfind(|c: char| !c.is_whitespace() && c != '\u{E010}' && c != '\u{E011}');
    let paragraph_after = last_text.map(|i| flat[i..].contains('\u{E010}')).unwrap_or(false);
    let body = match (first_text, last_text) {
        (Some(a), Some(z)) => {
            let end = z + flat[z..].chars().next().map(char::len_utf8).unwrap_or(1);
            flat[a..end].replace(['\u{E010}', '\u{E011}'], "\n")
        }
        _ => String::new(),
    };
    let text = body
        .split('\n')
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let notes = notes
        .into_iter()
        .map(|n| decode_entities(&strip_tags(&n)).split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|n| !n.is_empty())
        .collect();
    Entry { text, titles, notes, paragraph_before, paragraph_after }
}

fn find_conf(files: &dyn Files) -> Vec<String> {
    files.list().into_iter().filter(|n| n.to_lowercase().starts_with("mods.d/") && n.to_lowercase().ends_with(".conf")).collect()
}

/// Opens a module from a folder holding `mods.d` and `modules`, or a ZIP of one.
pub fn open(path: &Path) -> Result<SwordModule, String> {
    let (mut files, hash): (Box<dyn Files>, String) = if path.is_dir() {
        (Box::new(DirFiles(path.to_path_buf())), String::new())
    } else {
        let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let hash = sha256_hex(&bytes);
        let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("not a SWORD ZIP: {e}"))?;
        (Box::new(ZipFiles(archive)), hash)
    };
    let confs = find_conf(files.as_ref());
    let conf_name = confs.first().ok_or("no mods.d/*.conf file: this does not look like a SWORD module")?.clone();
    let conf = Conf::parse(&decode(&files.read(&conf_name).ok_or("cannot read the .conf file")?));
    if conf.values.contains_key("CipherKey") {
        return Err("this module is locked with a cipher key and cannot be imported".into());
    }
    let driver = conf.get("ModDrv").unwrap_or("").to_ascii_lowercase();
    if !driver.starts_with("ztext") && !driver.starts_with("rawtext") {
        return Err(format!("'{}' is not a Bible text module (driver {driver})", conf.name));
    }
    let versification = conf.get("Versification").unwrap_or("KJV");
    let table = v11n(versification).ok_or_else(|| format!("unknown versification {versification}"))?;
    let mut dir = conf.get("DataPath").ok_or("the .conf has no DataPath")?.trim_start_matches("./").replace('\\', "/");
    if !dir.ends_with('/') {
        dir.push('/');
    }
    let mut warnings = Vec::new();
    let mut books = Vec::new();
    for (prefix, list) in [("ot", &table.ot), ("nt", &table.nt)] {
        let Some(testament) = read_testament(files.as_mut(), &dir, prefix, &conf)? else { continue };
        let mut index = 2usize;
        for (osis, chapters) in list.iter() {
            index += 1;
            let code = usfm_code(osis);
            let mut content = BookContent::default();
            let mut next_paragraph = false;
            for (c, &count) in chapters.iter().enumerate() {
                let chapter = c as u16 + 1;
                let heading = testament.entries.get(index).cloned().flatten();
                index += 1;
                let mut pending_titles: Vec<(String, String)> = Vec::new();
                if let Some(h) = heading {
                    let e = plain_entry(&decode(&h));
                    pending_titles.extend(e.titles);
                }
                for v in 1..=count {
                    let raw = testament.entries.get(index).cloned().flatten();
                    index += 1;
                    let Some(raw) = raw else { continue };
                    let e = plain_entry(&decode(&raw));
                    for (kind, t) in pending_titles.drain(..).chain(e.titles.into_iter()) {
                        content.titles.push(TitleRow { chapter, verse: v, kind, text: t });
                    }
                    for n in e.notes {
                        content.notes.push(NoteRow { chapter, verse: v, marker: String::new(), text: n });
                    }
                    if e.text.is_empty() {
                        continue;
                    }
                    content.verses.push(VerseRow {
                        chapter,
                        verse: v,
                        text: e.text,
                        paragraph: next_paragraph || e.paragraph_before || v == 1,
                    });
                    next_paragraph = e.paragraph_after;
                }
            }
            if content.verses.is_empty() {
                continue;
            }
            match code {
                Some(code) => books.push(UsfmBook { code: code.to_string(), name: osis.clone(), content }),
                None => warnings.push(format!("{osis} is not in the Catholic canon and was left out")),
            }
        }
    }
    if books.is_empty() {
        return Err("the module holds no readable verses".into());
    }
    Ok(SwordModule { conf, books, hash, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_match_their_totals() {
        let kjv = v11n("KJV").unwrap();
        assert_eq!(kjv.ot.len(), 39);
        assert_eq!(kjv.nt.len(), 27);
        let verses: usize = kjv.ot.iter().chain(kjv.nt.iter()).map(|(_, c)| c.iter().map(|&n| n as usize).sum::<usize>()).sum();
        assert_eq!(verses, 31102);
        assert_eq!(v11n("Vulg").unwrap().ot.iter().find(|(b, _)| b == "Dan").unwrap().1.len(), 14);
    }

    #[test]
    fn conf_values_and_continuations() {
        let conf = Conf::parse("[KJV]\nDataPath=./modules/texts/ztext/kjv/\nModDrv=zText\nAbout=Line one\\\nline two\nVersification=KJV\n");
        assert_eq!(conf.name, "KJV");
        assert_eq!(conf.get("DataPath"), Some("./modules/texts/ztext/kjv/"));
        assert_eq!(conf.get("About"), Some("Line one\nline two"));
        assert_eq!(conf.get("Versification"), Some("KJV"));
    }

    #[test]
    fn osis_markup_becomes_text() {
        let e = plain_entry(r#"<title type="psalm" canonical="true">A Psalm of David.</title><w lemma="strong:H3068">The <divineName>Lord</divineName></w> is my shepherd;<note type="translation">Or, feeds</note> I shall not want.<milestone type="x-p"/>"#);
        assert_eq!(e.text, "The Lord is my shepherd; I shall not want.");
        assert_eq!(e.titles[0], ("psalm".to_string(), "A Psalm of David.".to_string()));
        assert!(plain_entry(r#"<title type="chapter">CHAPTER 1.</title>In the beginning"#).titles.is_empty());
        assert_eq!(e.notes, vec!["Or, feeds".to_string()]);
        assert!(e.paragraph_after);
        assert!(!e.paragraph_before);
    }

    #[test]
    fn gbf_and_entities() {
        let e = plain_entry("<CM>In the beginning<RF>A note<Rf> God &amp; the heaven<CL>and the earth.");
        assert_eq!(e.text, "In the beginning God & the heaven\nand the earth.");
        assert!(e.paragraph_before);
        assert_eq!(e.notes, vec!["A note".to_string()]);
    }

    #[test]
    fn lzss_literals_and_copies() {
        // "abcabcabc": three literals, then a copy of six from the ring buffer start.
        let r = 4096 - 18;
        let lo = (r & 0xFF) as u8;
        let hi = (((r >> 4) & 0xF0) as u8) | (6 - 3);
        let input = [0b0000_0111u8, b'a', b'b', b'c', lo, hi];
        assert_eq!(lzss_decompress(&input), b"abcabcabc");
    }

    #[test]
    fn schemes_for_versifications() {
        assert_eq!(scheme_for("Vulg"), "vulgate");
        assert_eq!(scheme_for("KJV"), "english");
        assert_eq!(scheme_for("Leningrad"), "hebrew");
        assert_eq!(scheme_for("Synodal"), "lxx");
    }
}
