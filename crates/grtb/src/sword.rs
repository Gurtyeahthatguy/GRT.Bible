//! Reading CrossWire SWORD modules from a local file: Bibles, commentaries,
//! dictionaries and books.

use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::markup::{attr, decode_entities, escape, plain_text, tag_words, to_markup, tokens, Source, Target, Token};
use crate::module::{BookContent, EntryContent, LinkGroup, NoteRow, TaggedRow, TitleRow, VerseRow};
use crate::passage::{parse_osis, parse_words, Context};
use crate::refs::Ref;
use crate::source::{decode, sha256_hex};
use crate::usfm::UsfmBook;
use crate::versification::Scheme;

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

/// No single file of a module is read past this size.
const FILE_LIMIT: u64 = 512 * 1024 * 1024;
/// Nor does one compressed block expand past this.
const BLOCK_LIMIT: u64 = 64 * 1024 * 1024;

struct DirFiles(PathBuf);

impl Files for DirFiles {
    fn read(&mut self, path: &str) -> Option<Vec<u8>> {
        let target = self.0.join(path);
        if std::fs::metadata(&target).ok()?.len() > FILE_LIMIT {
            return None;
        }
        std::fs::read(target).ok()
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
        let entry = self.0.by_name(&name).ok()?;
        if entry.size() > FILE_LIMIT {
            return None;
        }
        let mut data = Vec::new();
        entry.take(FILE_LIMIT).read_to_end(&mut data).ok()?;
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

fn u16le(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32le(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
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
    while i < input.len() && (out.len() as u64) < BLOCK_LIMIT {
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
            flate2::read::ZlibDecoder::new(data).take(BLOCK_LIMIT).read_to_end(&mut out).map_err(|e| format!("zlib: {e}"))?;
        }
        "LZSS" => out = lzss_decompress(data),
        "BZIP2" => {
            bzip2_rs::DecoderReader::new(data).take(BLOCK_LIMIT).read_to_end(&mut out).map_err(|e| format!("bzip2: {e}"))?;
        }
        "XZ" => {
            let mut reader = std::io::BufReader::new(data);
            lzma_rs::xz_decompress(&mut reader, &mut out).map_err(|e| format!("xz: {e:?}"))?;
            out.truncate(BLOCK_LIMIT as usize);
        }
        other => return Err(format!("compression {other} is not supported")),
    }
    Ok(out)
}

/// Where each verse's text is stored in one testament. Verses that share a
/// place share one entry, which tells them apart from verses that repeat a text.
struct Testament {
    /// Decompressed blocks, or the whole data file of an uncompressed module as block 0.
    blocks: HashMap<u32, Vec<u8>>,
    entries: Vec<Option<(u32, usize, usize)>>,
}

impl Testament {
    fn get(&self, index: usize) -> Option<(u64, &[u8])> {
        let (block, offset, size) = (*self.entries.get(index)?)?;
        let bytes = self.blocks.get(&block)?.get(offset..offset.checked_add(size)?)?;
        Some(((block as u64) << 32 | offset as u64, bytes))
    }
}

/// The letter SWORD puts in the index names of a compressed module: `ot.bzs`, `ot.czs`, `ot.vzs`.
fn block_letters(conf: &Conf) -> Vec<char> {
    let declared = match conf.get("BlockType").unwrap_or("CHAPTER").to_ascii_uppercase().as_str() {
        "BOOK" => 'b',
        "VERSE" => 'v',
        _ => 'c',
    };
    let mut letters = vec![declared];
    letters.extend(['b', 'c', 'v'].into_iter().filter(|c| *c != declared));
    letters
}

fn read_testament(files: &mut dyn Files, dir: &str, prefix: &str, conf: &Conf) -> Result<Option<Testament>, String> {
    let driver = conf.get("ModDrv").unwrap_or("zText").to_ascii_lowercase();
    let path = |ext: &str| format!("{dir}{prefix}{ext}");
    match driver.as_str() {
        "ztext" | "ztext4" | "zcom" | "zcom4" => {
            let wide = driver.ends_with('4');
            let mut found = None;
            for letter in block_letters(conf) {
                let (Some(bzs), Some(bzv), Some(bzz)) =
                    (files.read(&path(&format!(".{letter}zs"))), files.read(&path(&format!(".{letter}zv"))), files.read(&path(&format!(".{letter}zz"))))
                else {
                    continue;
                };
                if !bzv.is_empty() {
                    found = Some((bzs, bzv, bzz));
                    break;
                }
            }
            let Some((bzs, bzv, bzz)) = found else { return Ok(None) };
            let compress = conf.get("CompressType").unwrap_or("ZIP");
            let mut blocks: HashMap<u32, Vec<u8>> = HashMap::new();
            let step = if wide { 12 } else { 10 };
            let mut entries = Vec::with_capacity(bzv.len() / step);
            for i in (0..bzv.len().saturating_sub(step - 1)).step_by(step) {
                let block = u32le(&bzv, i).unwrap_or(0);
                let offset = u32le(&bzv, i + 4).unwrap_or(0) as usize;
                let size = if wide { u32le(&bzv, i + 8).unwrap_or(0) as usize } else { u16le(&bzv, i + 8).unwrap_or(0) as usize };
                if size == 0 {
                    entries.push(None);
                    continue;
                }
                if !blocks.contains_key(&block) {
                    let at = block as usize * 12;
                    let (Some(start), Some(len)) = (u32le(&bzs, at), u32le(&bzs, at + 4)) else {
                        entries.push(None);
                        continue;
                    };
                    let (start, len) = (start as usize, len as usize);
                    let raw = bzz.get(start..start.saturating_add(len)).ok_or("block runs past the end of the data file")?;
                    blocks.insert(block, decompress(compress, raw)?);
                }
                entries.push(Some((block, offset, size)));
            }
            Ok(Some(Testament { blocks, entries }))
        }
        "rawtext" | "rawtext4" | "rawcom" | "rawcom4" => {
            let wide = driver.ends_with('4');
            let (Some(vss), Some(text)) = (files.read(&path(".vss")), files.read(&format!("{dir}{prefix}"))) else {
                return Ok(None);
            };
            let step = if wide { 8 } else { 6 };
            let mut entries = Vec::with_capacity(vss.len() / step);
            for i in (0..vss.len().saturating_sub(step - 1)).step_by(step) {
                let start = u32le(&vss, i).unwrap_or(0) as usize;
                let size = if wide { u32le(&vss, i + 4).unwrap_or(0) as usize } else { u16le(&vss, i + 4).unwrap_or(0) as usize };
                entries.push(if size == 0 { None } else { Some((0, start, size)) });
            }
            Ok(Some(Testament { blocks: HashMap::from([(0, text)]), entries }))
        }
        other => Err(format!("module driver {other} is not one GRT Bible reads by verse")),
    }
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

/// What a SWORD module holds, by its driver.
pub fn kind_of(conf: &Conf) -> Option<&'static str> {
    let driver = conf.get("ModDrv").unwrap_or("").to_ascii_lowercase();
    Some(match driver.as_str() {
        "ztext" | "ztext4" | "rawtext" | "rawtext4" => "bible",
        "zcom" | "zcom4" | "rawcom" | "rawcom4" => "commentary",
        "zld" | "rawld" | "rawld4" => "dictionary",
        "rawgenbook" => "book",
        _ => return None,
    })
}

struct Opened {
    files: Box<dyn Files>,
    conf: Conf,
    hash: String,
}

fn load(path: &Path) -> Result<Opened, String> {
    let (files, hash): (Box<dyn Files>, String) = if path.is_dir() {
        (Box::new(DirFiles(path.to_path_buf())), String::new())
    } else {
        if std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) > FILE_LIMIT {
            return Err(format!("{} is larger than any module GRT Bible reads", path.display()));
        }
        let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let hash = sha256_hex(&bytes);
        let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("not a SWORD ZIP: {e}"))?;
        (Box::new(ZipFiles(archive)), hash)
    };
    let mut files = files;
    let confs = find_conf(files.as_ref());
    let conf_name = confs.first().ok_or("no mods.d/*.conf file: this does not look like a SWORD module")?.clone();
    let conf = Conf::parse(&decode(&files.read(&conf_name).ok_or("cannot read the .conf file")?));
    if conf.values.contains_key("CipherKey") {
        return Err("this module is locked with a cipher key and cannot be imported".into());
    }
    Ok(Opened { files, conf, hash })
}

/// The module's folder inside the archive, from `DataPath`.
fn data_dir(conf: &Conf) -> Result<String, String> {
    let raw = conf.get("DataPath").ok_or("the .conf has no DataPath")?.trim_start_matches("./").replace('\\', "/");
    if raw.split('/').any(|part| part == "..") || raw.starts_with('/') {
        return Err("the .conf points outside the module".into());
    }
    Ok(raw)
}

/// The conf alone, to tell what a module is before converting it.
pub fn probe(path: &Path) -> Result<Conf, String> {
    Ok(load(path)?.conf)
}

/// One place in a verse-keyed module, in the order SWORD stores them.
struct Slot<'a> {
    testament: usize,
    osis: &'a str,
    /// 0 for the introduction of the book.
    chapter: u16,
    /// 0 for the introduction of the chapter.
    verse: u16,
    index: usize,
}

fn slots(table: &V11n) -> Vec<Slot<'_>> {
    let mut out = Vec::new();
    for (testament, list) in [&table.ot, &table.nt].into_iter().enumerate() {
        let mut index = 2usize;
        for (osis, chapters) in list.iter() {
            out.push(Slot { testament, osis, chapter: 0, verse: 0, index });
            index += 1;
            for (c, &count) in chapters.iter().enumerate() {
                let chapter = c as u16 + 1;
                out.push(Slot { testament, osis, chapter, verse: 0, index });
                index += 1;
                for verse in 1..=count {
                    out.push(Slot { testament, osis, chapter, verse, index });
                    index += 1;
                }
            }
        }
    }
    out
}

fn testaments(opened: &mut Opened) -> Result<[Option<Testament>; 2], String> {
    let mut dir = data_dir(&opened.conf)?;
    if !dir.ends_with('/') {
        dir.push('/');
    }
    Ok([
        read_testament(opened.files.as_mut(), &dir, "ot", &opened.conf)?,
        read_testament(opened.files.as_mut(), &dir, "nt", &opened.conf)?,
    ])
}

fn versification_of(conf: &Conf) -> Result<&'static V11n, String> {
    let name = conf.get("Versification").unwrap_or("KJV");
    v11n(name).ok_or_else(|| format!("unknown versification {name}"))
}

/// Opens a Bible from a folder holding `mods.d` and `modules`, or a ZIP of one.
pub fn open(path: &Path) -> Result<SwordModule, String> {
    let mut opened = load(path)?;
    if kind_of(&opened.conf) != Some("bible") {
        let driver = opened.conf.get("ModDrv").unwrap_or("").to_ascii_lowercase();
        return Err(format!("'{}' is not a Bible text module (driver {driver})", opened.conf.name));
    }
    let table = versification_of(&opened.conf)?;
    let source = Source::from_conf(opened.conf.get("SourceType"));
    let read = testaments(&mut opened)?;
    let mut warnings = Vec::new();
    let mut books: Vec<UsfmBook> = Vec::new();
    let mut current: Option<(&str, BookContent)> = None;
    let mut next_paragraph = false;
    let mut pending_titles: Vec<(String, String)> = Vec::new();
    let finish = |current: Option<(&str, BookContent)>, books: &mut Vec<UsfmBook>, warnings: &mut Vec<String>| {
        let Some((osis, content)) = current else { return };
        if content.verses.is_empty() {
            return;
        }
        match usfm_code(osis) {
            Some(code) => books.push(UsfmBook { code: code.to_string(), name: osis.to_string(), content }),
            None => warnings.push(format!("{osis} is not in the Catholic canon and was left out")),
        }
    };
    for slot in slots(table) {
        let Some(testament) = &read[slot.testament] else { continue };
        if current.as_ref().map(|(o, _)| *o) != Some(slot.osis) {
            finish(current.take(), &mut books, &mut warnings);
            current = Some((slot.osis, BookContent::default()));
            next_paragraph = false;
        }
        if slot.chapter == 0 {
            continue;
        }
        let raw = testament.get(slot.index).map(|(_, b)| decode(b));
        if slot.verse == 0 {
            pending_titles.clear();
            if let Some(h) = raw {
                pending_titles.extend(plain_entry(&h).titles);
            }
            continue;
        }
        let Some(raw) = raw else { continue };
        let content = &mut current.as_mut().unwrap().1;
        let (chapter, v) = (slot.chapter, slot.verse);
        let e = plain_entry(&raw);
        for (kind, t) in pending_titles.drain(..).chain(e.titles) {
            content.titles.push(TitleRow { chapter, verse: v, kind, text: t });
        }
        for n in e.notes {
            content.notes.push(NoteRow { chapter, verse: v, marker: String::new(), text: n });
        }
        if e.text.is_empty() {
            continue;
        }
        if let Some(markup) = tag_words(&raw, source) {
            content.tagged.push(TaggedRow { chapter, verse: v, markup });
        }
        content.verses.push(VerseRow { chapter, verse: v, text: e.text, paragraph: next_paragraph || e.paragraph_before || v == 1 });
        next_paragraph = e.paragraph_after;
    }
    finish(current.take(), &mut books, &mut warnings);
    if books.is_empty() {
        return Err("the module holds no readable verses".into());
    }
    Ok(SwordModule { conf: opened.conf, books, hash: opened.hash, warnings })
}

/// A commentary, dictionary, set of cross-references or book read from SWORD.
pub struct Resource {
    pub conf: Conf,
    pub kind: &'static str,
    pub entries: Vec<EntryContent>,
    pub links: Vec<LinkGroup>,
    pub hash: String,
    pub warnings: Vec<String>,
}

/// An internal verse for a place in the module's numbering; verse 0 stays an introduction.
fn canon_place(scheme: &Scheme, b: u8, chapter: u16, verse: u16) -> (Ref, Ref) {
    if chapter == 0 {
        return (Ref::new(b, 0, 0), Ref::new(b, 0, 0));
    }
    if verse == 0 {
        let first = scheme.to_canon(Ref::new(b, chapter, 1)).0;
        let at = Ref::new(first.book, first.chapter, 0);
        return (at, at);
    }
    scheme.to_canon(Ref::new(b, chapter, verse))
}

fn canon_ranges(scheme: &Scheme, ranges: Vec<(Ref, Ref)>) -> Vec<(Ref, Ref)> {
    ranges
        .into_iter()
        .map(|(a, b)| (scheme.to_canon(a).0, scheme.to_canon(b).1))
        .filter(|(a, b)| a.in_canon() && b.in_canon() && a <= b)
        .collect()
}

fn resolver<'a>(scheme: &'a Scheme, ctx: Context) -> impl Fn(Target) -> Vec<(Ref, Ref)> + 'a {
    move |t| match t {
        Target::Osis(text) => canon_ranges(scheme, parse_osis(text)),
        Target::Words(text) => canon_ranges(scheme, parse_words(text, ctx)),
    }
}

/// Book index of a SWORD book name, counting the books that live inside canon books.
fn canon_book(osis: &str) -> Option<(u8, u16)> {
    if let Some(b) = crate::canon::book_by_osis(osis) {
        return Some((b.index, 0));
    }
    let (host, chapter) = match osis {
        "EpJer" => ("Bar", 6),
        "Sus" => ("Dan", 13),
        "Bel" => ("Dan", 14),
        _ => return None,
    };
    Some((crate::canon::book_by_osis(host)?.index, chapter))
}

/// Reads a commentary or, read as one, a set of cross-references.
/// Entries, groups of references and warnings.
type ByVerse = (Vec<EntryContent>, Vec<LinkGroup>, Vec<String>);

fn read_by_verse(opened: &mut Opened, as_links: bool, scheme: &Scheme) -> Result<ByVerse, String> {
    let table = versification_of(&opened.conf)?;
    let source = Source::from_conf(opened.conf.get("SourceType"));
    let read = testaments(opened)?;
    let mut entries: Vec<EntryContent> = Vec::new();
    let mut links: Vec<LinkGroup> = Vec::new();
    let mut by_place: HashMap<(usize, u64), (usize, usize)> = HashMap::new();
    let mut outside = BTreeMap::new();
    for slot in slots(table) {
        let Some(testament) = &read[slot.testament] else { continue };
        let Some((place, bytes)) = testament.get(slot.index) else { continue };
        let Some((b, fixed_chapter)) = canon_book(slot.osis) else {
            *outside.entry(slot.osis.to_string()).or_insert(0) += 1;
            continue;
        };
        let chapter = if fixed_chapter > 0 && slot.chapter > 0 { fixed_chapter } else { slot.chapter };
        let (first, last) = canon_place(scheme, b, chapter, slot.verse);
        if !(first.chapter == 0 || first.verse == 0 || (first.in_canon() && last.in_canon())) {
            continue;
        }
        // Verses that share one stored entry are one article spanning all of them.
        if let Some(&(at, previous)) = by_place.get(&(slot.testament, place)) {
            let next_to = previous + 1 == slot.index;
            by_place.insert((slot.testament, place), (at, slot.index));
            let shared = entries[at].spans[0].0;
            if as_links {
                for l in links.iter_mut().filter(|l| l.first == shared) {
                    l.last = l.last.max(last);
                }
            }
            let spans = &mut entries[at].spans;
            match spans.last_mut() {
                Some(span) if next_to && span.1.book == last.book && span.1 <= last => span.1 = last,
                _ => spans.push((first, last)),
            }
            continue;
        }
        let raw = decode(bytes);
        let ctx = Context { book: Some(b), chapter: (chapter > 0).then_some(chapter) };
        let resolve = resolver(scheme, ctx);
        by_place.insert((slot.testament, place), (entries.len(), slot.index));
        if as_links {
            entries.push(EntryContent { spans: vec![(first, last)], ..Default::default() });
            links.extend(link_groups(&raw, source, first, last, &resolve));
            continue;
        }
        let body = to_markup(&raw, source, &resolve);
        if plain_text(&body).is_empty() {
            by_place.remove(&(slot.testament, place));
            continue;
        }
        entries.push(EntryContent { body, spans: vec![(first, last)], ..Default::default() });
    }
    let warnings = outside.into_iter().map(|(osis, n)| format!("{osis} is not in the Catholic canon: {n} entries left out")).collect();
    if as_links {
        entries.clear();
    }
    Ok((entries, links, warnings))
}

/// The groups of a cross-reference entry: the words before each list of references become its label.
fn link_groups(raw: &str, source: Source, first: Ref, last: Ref, resolve: &dyn Fn(Target) -> Vec<(Ref, Ref)>) -> Vec<LinkGroup> {
    let mut groups: Vec<LinkGroup> = Vec::new();
    let mut words = String::new();
    // The last line of words before the current one; the label sits on a line of its own.
    let mut last_line = String::new();
    let mut label = String::new();
    let mut inside: Option<(Option<String>, String)> = None;
    let mut depth_skip = 0usize;
    for token in tokens(raw) {
        match token {
            Token::Text(t) => {
                if depth_skip > 0 {
                    continue;
                }
                match &mut inside {
                    Some((_, text)) => text.push_str(t),
                    None => words.push_str(t),
                }
            }
            Token::Tag { raw_name, closing, empty, attrs } => {
                let name = raw_name.to_ascii_lowercase();
                let is_ref = name == "scripref" || name == "reference" || (name == "ref" && attr(attrs, "osisRef").is_some()) || (source == Source::Gbf && raw_name == "RX");
                if name == "note" && !empty {
                    if closing {
                        depth_skip = depth_skip.saturating_sub(1);
                    } else {
                        depth_skip += 1;
                    }
                    continue;
                }
                if matches!(name.as_str(), "br" | "lb" | "p" | "div" | "l") || (source == Source::Gbf && raw_name == "CL") {
                    if !words.trim().is_empty() {
                        last_line = std::mem::take(&mut words);
                    }
                    continue;
                }
                if is_ref && !closing && !empty {
                    inside = Some((attr(attrs, "osisRef").or_else(|| attr(attrs, "passage")), String::new()));
                    continue;
                }
                let ends_ref = (name == "scripref" || name == "reference" || name == "ref") && closing || (source == Source::Gbf && raw_name == "Rx");
                if ends_ref {
                    let Some((target, text)) = inside.take() else { continue };
                    let shown = decode_entities(&text);
                    // Numbered outline lines ("3 the light;") point inside the chapter and are not references.
                    if target.is_some() && shown.trim().chars().all(|c| c.is_ascii_digit()) {
                        words.clear();
                        last_line.clear();
                        label.clear();
                        continue;
                    }
                    let line = if words.trim().is_empty() { &last_line } else { &words };
                    let lead = decode_entities(line).split_whitespace().collect::<Vec<_>>().join(" ");
                    let lead = lead.trim_end_matches(['.', ':', ',', ';']).trim().to_string();
                    if !lead.is_empty() {
                        label = lead;
                    }
                    words.clear();
                    last_line.clear();
                    let targets = match &target {
                        Some(t) if t.contains('.') => resolve(Target::Osis(t)),
                        Some(t) => resolve(Target::Words(t)),
                        None => resolve(Target::Words(&shown)),
                    };
                    if targets.is_empty() {
                        continue;
                    }
                    match groups.last_mut() {
                        Some(g) if g.label == label => g.targets.extend(targets),
                        _ => groups.push(LinkGroup { first, last, label: label.clone(), targets }),
                    }
                }
            }
        }
    }
    groups
}

/// The share of an entry's words that are references, to tell cross-references from commentary.
pub fn reference_share(entries: &[EntryContent]) -> f32 {
    let mut linked = 0usize;
    let mut total = 0usize;
    for e in entries.iter().take(4000) {
        total += plain_text(&e.body).len();
        let mut rest = e.body.as_str();
        while let Some(i) = rest.find("<a data-ref=") {
            let after = &rest[i..];
            let end = after.find("</a>").unwrap_or(after.len());
            let inner = after[..end].split_once('>').map(|(_, t)| t).unwrap_or("");
            linked += decode_entities(inner).len();
            rest = &after[end..];
        }
    }
    if total == 0 {
        0.0
    } else {
        linked as f32 / total as f32
    }
}

fn dictionary_key(raw: &[u8]) -> String {
    decode(raw).trim_end_matches(['\r', '\n', '\\', ' ']).trim().to_string()
}

/// Headwords and texts of a `zLD`, `RawLD` or `RawLD4` dictionary, in index order.
/// Headwords with their raw text, and warnings.
type Headwords = (Vec<(String, Vec<u8>)>, Vec<String>);

fn read_dictionary(opened: &mut Opened) -> Result<Headwords, String> {
    let driver = opened.conf.get("ModDrv").unwrap_or("").to_ascii_lowercase();
    let base = data_dir(&opened.conf)?;
    let idx = opened.files.read(&format!("{base}.idx")).ok_or("the dictionary has no .idx file")?;
    let dat = opened.files.read(&format!("{base}.dat")).ok_or("the dictionary has no .dat file")?;
    let mut out = Vec::new();
    let mut warnings = Vec::new();
    match driver.as_str() {
        "zld" => {
            let zdx = opened.files.read(&format!("{base}.zdx")).ok_or("the dictionary has no .zdx file")?;
            let zdt = opened.files.read(&format!("{base}.zdt")).ok_or("the dictionary has no .zdt file")?;
            let compress = opened.conf.get("CompressType").unwrap_or("ZIP").to_string();
            let mut blocks: HashMap<u32, Vec<u8>> = HashMap::new();
            for i in (0..idx.len().saturating_sub(7)).step_by(8) {
                let (Some(offset), Some(size)) = (u32le(&idx, i), u32le(&idx, i + 4)) else { break };
                let Some(record) = dat.get(offset as usize..(offset as usize).saturating_add(size as usize)) else { continue };
                let Some(nl) = record.iter().position(|&c| c == b'\n') else { continue };
                let key = dictionary_key(&record[..nl]);
                let rest = &record[nl + 1..];
                let (Some(block), Some(entry)) = (u32le(rest, 0), u32le(rest, 4)) else { continue };
                if !blocks.contains_key(&block) {
                    let at = block as usize * 8;
                    let (Some(start), Some(len)) = (u32le(&zdx, at), u32le(&zdx, at + 4)) else { continue };
                    let Some(raw) = zdt.get(start as usize..(start as usize).saturating_add(len as usize)) else { continue };
                    blocks.insert(block, decompress(&compress, raw)?);
                }
                let data = &blocks[&block];
                let at = 4 + entry as usize * 8;
                let (Some(start), Some(len)) = (u32le(data, at), u32le(data, at + 4)) else { continue };
                let Some(text) = data.get(start as usize..(start as usize).saturating_add(len as usize)) else { continue };
                let text = text.strip_suffix(&[0]).unwrap_or(text);
                out.push((key, text.to_vec()));
            }
        }
        "rawld" | "rawld4" => {
            let wide = driver == "rawld4";
            let step = if wide { 8 } else { 6 };
            for i in (0..idx.len().saturating_sub(step - 1)).step_by(step) {
                let Some(offset) = u32le(&idx, i) else { break };
                let size = if wide { u32le(&idx, i + 4) } else { u16le(&idx, i + 4).map(u32::from) }.unwrap_or(0);
                let Some(record) = dat.get(offset as usize..(offset as usize).saturating_add(size as usize)) else { continue };
                let Some(nl) = record.iter().position(|&c| c == b'\n') else { continue };
                out.push((dictionary_key(&record[..nl]), record[nl + 1..].to_vec()));
            }
        }
        other => return Err(format!("module driver {other} is not a dictionary driver")),
    }
    if out.is_empty() {
        warnings.push("the dictionary index lists no entries".to_string());
    }
    Ok((out, warnings))
}

/// The sections of a `RawGenBook`, depth first, each with the position of its parent.
/// A section's parent, title and raw text.
type Section = (Option<u32>, String, Vec<u8>);

fn read_book(opened: &mut Opened) -> Result<Vec<Section>, String> {
    let base = data_dir(&opened.conf)?;
    let idx = opened.files.read(&format!("{base}.idx")).ok_or("the book has no .idx file")?;
    let dat = opened.files.read(&format!("{base}.dat")).ok_or("the book has no .dat file")?;
    let bdt = opened.files.read(&format!("{base}.bdt")).ok_or("the book has no .bdt file")?;
    struct Node {
        next: i32,
        child: i32,
        name: String,
        text: Vec<u8>,
    }
    let node = |idx_offset: i32| -> Option<Node> {
        let at = u32le(&idx, usize::try_from(idx_offset).ok()?)? as usize;
        let next = u32le(&dat, at + 4)? as i32;
        let child = u32le(&dat, at + 8)? as i32;
        let name_end = at + 12 + dat.get(at + 12..)?.iter().position(|&c| c == 0)?;
        let name = decode(&dat[at + 12..name_end]);
        let size = u16le(&dat, name_end + 1)? as usize;
        let user = dat.get(name_end + 3..name_end + 3 + size)?;
        let text = if size >= 8 {
            let (start, len) = (u32le(user, 0)? as usize, u32le(user, 4)? as usize);
            bdt.get(start..start.saturating_add(len)).unwrap_or(&[]).to_vec()
        } else {
            Vec::new()
        };
        Some(Node { next, child, name, text })
    };
    let limit = idx.len() / 4 + 1;
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let root = node(0).ok_or("the book has no root section")?;
    // Each stack item: the next sibling to read and the position of its parent.
    let mut stack: Vec<(i32, Option<u32>)> = vec![(root.child, None)];
    while let Some((at, parent)) = stack.pop() {
        if at < 0 || !seen.insert(at) || out.len() >= limit {
            continue;
        }
        let Some(n) = node(at) else { continue };
        let position = out.len() as u32;
        stack.push((n.next, parent));
        stack.push((n.child, Some(position)));
        out.push((parent, n.name, n.text));
    }
    Ok(out)
}

/// Reads a module that is not a Bible; `as_kind` reads a commentary as cross-references.
pub fn read_resource(path: &Path, as_kind: Option<&str>, scheme: &Scheme) -> Result<Resource, String> {
    let mut opened = load(path)?;
    let found = kind_of(&opened.conf).ok_or_else(|| {
        format!("'{}' uses the driver {}, which GRT Bible does not read", opened.conf.name, opened.conf.get("ModDrv").unwrap_or("?"))
    })?;
    let kind = match (found, as_kind) {
        ("bible", _) => return Err(format!("'{}' is a Bible: import it as a translation", opened.conf.name)),
        ("commentary", Some("crossrefs")) => "crossrefs",
        (k, _) => k,
    };
    let source = Source::from_conf(opened.conf.get("SourceType"));
    let mut warnings = Vec::new();
    let mut entries = Vec::new();
    let mut links = Vec::new();
    match kind {
        "commentary" | "crossrefs" => {
            let (e, l, w) = read_by_verse(&mut opened, kind == "crossrefs", scheme)?;
            entries = e;
            links = l;
            warnings = w;
        }
        "dictionary" => {
            let (raw, w) = read_dictionary(&mut opened)?;
            warnings = w;
            let none = |_: Target| Vec::new();
            let resolve = resolver(scheme, Context::default());
            for (key, bytes) in raw {
                let text = decode(&bytes);
                let body = match text.trim().strip_prefix("@LINK") {
                    Some(target) => {
                        let target = target.trim();
                        format!("<p>See <a data-key=\"{}\">{}</a>.</p>", escape(target), escape(target))
                    }
                    None => to_markup(&text, source, if source == Source::Plain { &none } else { &resolve }),
                };
                entries.push(EntryContent { key: key.clone(), title: key, body, ..Default::default() });
            }
        }
        _ => {
            let resolve = resolver(scheme, Context::default());
            for (parent, name, bytes) in read_book(&mut opened)? {
                let body = to_markup(&decode(&bytes), source, &resolve);
                entries.push(EntryContent { parent, title: name, body, ..Default::default() });
            }
        }
    }
    if entries.is_empty() && links.is_empty() {
        return Err("the module holds nothing GRT Bible can read".into());
    }
    Ok(Resource { conf: opened.conf, kind, entries, links, hash: opened.hash, warnings })
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

    use std::io::Write;

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    fn module_dir(conf: &str, files: &[(&str, Vec<u8>)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods.d")).unwrap();
        std::fs::write(dir.path().join("mods.d/test.conf"), conf).unwrap();
        for (name, bytes) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        dir
    }

    /// A compressed verse index: each entry is (text index in `texts`, or none).
    fn zverse(slots: &[Option<usize>], texts: &[&str]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut block = Vec::new();
        let mut places = Vec::new();
        for t in texts {
            places.push((block.len() as u32, t.len() as u16));
            block.extend_from_slice(t.as_bytes());
        }
        let compressed = zlib(&block);
        let mut bzs = Vec::new();
        bzs.extend_from_slice(&0u32.to_le_bytes());
        bzs.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        bzs.extend_from_slice(&(block.len() as u32).to_le_bytes());
        let mut bzv = Vec::new();
        for slot in slots {
            let (offset, size) = slot.map(|i| places[i]).unwrap_or((0, 0));
            bzv.extend_from_slice(&0u32.to_le_bytes());
            bzv.extend_from_slice(&offset.to_le_bytes());
            bzv.extend_from_slice(&size.to_le_bytes());
        }
        (bzs, bzv, compressed)
    }

    #[test]
    fn a_chapter_blocked_commentary_with_shared_entries() {
        // Slots: 0, 1 heading, 2 Matthew, 3 chapter 1, then Matt 1:1, 1:2, 1:3.
        let texts = ["<p>Intro to the chapter.</p>", "<p>On the first two verses, see <reference osisRef=\"Luke.3.23\">Luke</reference>.</p>", "<p>Third.</p>"];
        let (bzs, bzv, bzz) = zverse(&[None, None, None, Some(0), Some(1), Some(1), Some(2)], &texts);
        let conf = "[Test]\nDataPath=./modules/comments/zcom/test/\nModDrv=zCom\nBlockType=CHAPTER\nSourceType=OSIS\nVersification=KJV\n";
        let dir = module_dir(conf, &[
            ("modules/comments/zcom/test/nt.czs", bzs),
            ("modules/comments/zcom/test/nt.czv", bzv),
            ("modules/comments/zcom/test/nt.czz", bzz),
        ]);
        assert_eq!(kind_of(&probe(dir.path()).unwrap()), Some("commentary"));
        let read = read_resource(dir.path(), None, crate::versification::scheme("english")).unwrap();
        assert_eq!(read.kind, "commentary");
        let spans: Vec<String> = read.entries.iter().map(|e| crate::module::range_osis(e.spans[0].0, e.spans[0].1)).collect();
        assert_eq!(spans, ["Matt.1.0", "Matt.1.1-Matt.1.2", "Matt.1.3"]);
        assert_eq!(read.entries[1].body, "<p>On the first two verses, see <a data-ref=\"Luke.3.23\">Luke</a>.</p>");
        assert!(open(dir.path()).is_err(), "a commentary is not a Bible");
    }

    #[test]
    fn a_commentary_read_as_cross_references() {
        let texts = ["<br />beginning.<br /><scripRef>Pr 8:22-24; 16:4</scripRef><br /><scripRef>Joh 1:1</scripRef><br />light.<br /><scripRef>3</scripRef>"];
        let (bzs, bzv, bzz) = zverse(&[None, None, None, None, Some(0)], &texts);
        let conf = "[TSK]\nDataPath=./modules/comments/zcom/tsk/\nModDrv=zCom\nBlockType=BOOK\nSourceType=ThML\n";
        let dir = module_dir(conf, &[
            ("modules/comments/zcom/tsk/nt.bzs", bzs),
            ("modules/comments/zcom/tsk/nt.bzv", bzv),
            ("modules/comments/zcom/tsk/nt.bzz", bzz),
        ]);
        let read = read_resource(dir.path(), Some("crossrefs"), crate::versification::scheme("english")).unwrap();
        assert_eq!(read.kind, "crossrefs");
        assert!(read.entries.is_empty());
        let groups: Vec<(String, Vec<String>)> = read
            .links
            .iter()
            .map(|g| (g.label.clone(), g.targets.iter().map(|(a, b)| crate::module::range_osis(*a, *b)).collect()))
            .collect();
        assert_eq!(groups, [
            ("beginning".to_string(), vec!["Prov.8.22-Prov.8.24".to_string(), "Prov.16.4".into(), "John.1.1".into()]),
            ("light".to_string(), vec!["Matt.1.3".to_string()]),
        ]);
        assert_eq!(read.links[0].first.osis(), "Matt.1.1");
    }

    #[test]
    fn a_compressed_dictionary_with_a_link() {
        let texts = ["<entryFree n=\"Aaron\"><title>Aaron</title><p>Brother of Moses (<ref osisRef=\"Bible:Exod.4.14\">Ex 4:14</ref>).</p></entryFree>\0", "@LINK AARON\0"];
        let mut block = Vec::new();
        block.extend_from_slice(&(texts.len() as u32).to_le_bytes());
        let mut offset = 4 + 8 * texts.len() as u32;
        for t in texts {
            block.extend_from_slice(&offset.to_le_bytes());
            block.extend_from_slice(&(t.len() as u32).to_le_bytes());
            offset += t.len() as u32;
        }
        for t in texts {
            block.extend_from_slice(t.as_bytes());
        }
        let zdt = zlib(&block);
        let mut zdx = 0u32.to_le_bytes().to_vec();
        zdx.extend_from_slice(&(zdt.len() as u32).to_le_bytes());
        let mut dat = Vec::new();
        let mut idx = Vec::new();
        for (entry, key) in ["AARON", "AARONITES"].iter().enumerate() {
            let record = [format!("{key}\r\n").into_bytes(), 0u32.to_le_bytes().to_vec(), (entry as u32).to_le_bytes().to_vec()].concat();
            idx.extend_from_slice(&(dat.len() as u32).to_le_bytes());
            idx.extend_from_slice(&(record.len() as u32).to_le_bytes());
            dat.extend_from_slice(&record);
        }
        let conf = "[Dict]\nDataPath=./modules/lexdict/zld/dict/dict\nModDrv=zLD\nSourceType=TEI\nCompressType=ZIP\n";
        let dir = module_dir(conf, &[
            ("modules/lexdict/zld/dict/dict.idx", idx),
            ("modules/lexdict/zld/dict/dict.dat", dat),
            ("modules/lexdict/zld/dict/dict.zdx", zdx),
            ("modules/lexdict/zld/dict/dict.zdt", zdt),
        ]);
        let read = read_resource(dir.path(), None, crate::versification::scheme("english")).unwrap();
        assert_eq!(read.kind, "dictionary");
        assert_eq!(read.entries[0].key, "AARON");
        assert_eq!(read.entries[0].body, "<p>Brother of Moses (<a data-ref=\"Exod.4.14\">Ex 4:14</a>).</p>");
        assert_eq!(read.entries[1].body, "<p>See <a data-key=\"AARON\">AARON</a>.</p>");
    }

    #[test]
    fn a_general_book_keeps_its_tree() {
        // Nodes: root, "Part" with children "One" and "Two".
        let bdt = b"<p>Part text</p><p>First</p><p>Second</p>".to_vec();
        let texts = [(0u32, 16u32), (16, 12), (28, 13)];
        let names = ["", "Part", "One", "Two"];
        // parent, next, first child, as idx offsets (4 bytes per node).
        let links: [(i32, i32, i32); 4] = [(-1, -1, 4), (0, -1, 8), (4, 12, -1), (4, -1, -1)];
        let mut dat = Vec::new();
        let mut idx = Vec::new();
        for (i, name) in names.iter().enumerate() {
            idx.extend_from_slice(&(dat.len() as u32).to_le_bytes());
            let (p, n, c) = links[i];
            for v in [p, n, c] {
                dat.extend_from_slice(&v.to_le_bytes());
            }
            dat.extend_from_slice(name.as_bytes());
            dat.push(0);
            if i == 0 {
                dat.extend_from_slice(&0u16.to_le_bytes());
            } else {
                let (start, len) = texts[i - 1];
                dat.extend_from_slice(&8u16.to_le_bytes());
                dat.extend_from_slice(&start.to_le_bytes());
                dat.extend_from_slice(&len.to_le_bytes());
            }
        }
        let conf = "[Book]\nDataPath=./modules/genbook/rawgenbook/book/book\nModDrv=RawGenBook\nSourceType=ThML\n";
        let dir = module_dir(conf, &[
            ("modules/genbook/rawgenbook/book/book.idx", idx),
            ("modules/genbook/rawgenbook/book/book.dat", dat),
            ("modules/genbook/rawgenbook/book/book.bdt", bdt),
        ]);
        let read = read_resource(dir.path(), None, crate::versification::scheme("english")).unwrap();
        let tree: Vec<(Option<u32>, &str, &str)> = read.entries.iter().map(|e| (e.parent, e.title.as_str(), e.body.as_str())).collect();
        assert_eq!(tree, [(None, "Part", "<p>Part text</p>"), (Some(0), "One", "<p>First</p>"), (Some(0), "Two", "<p>Second</p>")]);
    }

    #[test]
    fn a_data_path_outside_the_module_is_refused() {
        let dir = module_dir("[Bad]\nDataPath=./modules/../../etc/x\nModDrv=zLD\n", &[]);
        let error = read_resource(dir.path(), None, crate::versification::scheme("english")).err().unwrap();
        assert!(error.contains("outside"), "{error}");
    }

    /// The modules in `sources/sword-test`, when they have been downloaded.
    #[test]
    fn real_modules_when_present() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sources/sword-test");
        let english = crate::versification::scheme("english");
        let tsk = dir.join("TSK.zip");
        if !tsk.exists() {
            return;
        }
        let read = read_resource(&tsk, Some("crossrefs"), english).unwrap();
        let first = &read.links[0];
        assert_eq!((first.first.osis().as_str(), first.label.as_str()), ("Gen.1.1", "beginning"));
        let catena = read_resource(&dir.join("Catena.zip"), None, english).unwrap();
        assert_eq!(catena.entries[0].spans[0].0.osis(), "Matt.1.1");
        let easton = read_resource(&dir.join("Easton.zip"), None, english).unwrap();
        assert_eq!(easton.entries[1].key, "AARON");
        let hebrew = read_resource(&dir.join("StrongsHebrew.zip"), None, english).unwrap();
        assert_eq!(hebrew.entries[1].key, "00001");
        let josephus = read_resource(&dir.join("Josephus.zip"), None, english).unwrap();
        assert_eq!(josephus.entries[0].title, "The Life of Flavius Josephus");
        let kjv = open(&dir.join("KJV.zip")).unwrap();
        let genesis = &kjv.books[0].content;
        assert!(genesis.tagged[0].markup.contains("strong:H07225"), "{}", genesis.tagged[0].markup);
    }
}
