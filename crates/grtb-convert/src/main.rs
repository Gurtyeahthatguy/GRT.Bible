//! Builds `.grtb` modules and the video catalog package.

mod bdb;
mod commentary;
mod martini;
mod morphgnt;
mod n1904;
mod oshb;
mod rtf;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use grtb::assemble::{assemble, assemble_resource, date_from_unix, Description};
use grtb::catalog::{self, Catalog};
use grtb::module::{self, BookContent, Module};
use grtb::source::{arrange, read_usfm, sha256_hex};

const USAGE: &str = "\
Usage:
  grtb-convert usfm    <file|folder|zip> [options] -o <out.grtb>
  grtb-convert martini <file.rtf>        [options] -o <out.grtb>
  grtb-convert n1904   <file.csv>        [options] -o <out.grtb>
  grtb-convert sword   <folder|zip>      [options] -o <out.grtb>
                       a commentary, dictionary or book keeps its kind;
                       --as crossrefs reads a commentary as cross-references
  grtb-convert morphgnt <folder> --lexemes <lexemes.yaml> [options] -o <out.grtb>
  grtb-convert oshb    <folder> --index <LexicalIndex.xml> --aug <AugIndex.xml> [options] -o <out.grtb>
  grtb-convert bdb     <BrownDriverBriggs.xml> --index <LexicalIndex.xml> [options] -o <out.grtb>
  grtb-convert haydock <haydock.tsv>      [options] -o <out.grtb>
  grtb-convert fathers <commentaries.sqlite> --excerpts catena|fathers [options] -o <out.grtb>
  grtb-convert liturgy <YYYY-MM-DD> --to <YYYY-MM-DD>
                       prints the liturgical day and readings of each date as a line of JSON
  grtb-convert plans  plans
                       prints every reading plan, day by day, as JSON
  grtb-convert catalog <folder>                    -o <out.grt>
  grtb-convert info    <module.grtb>
  grtb-convert measure <folder> [--packs <folder>]
                       opens every module and reads from each, then prints the time and peak memory

Options:
  --id ID  --abbreviation ABBR  --title TITLE  --language CODE
  --direction ltr|rtl  --year YEAR  --canon catholic|protestant|partial
  --versification SCHEME  --license TEXT  --source TEXT
  --date YYYY-MM-DD      generation date; SOURCE_DATE_EPOCH is used otherwise
  --strip-brackets       remove [ and ] from verse text
  --hebrew-paragraphs    turn setumah and petuchah marks into paragraph breaks
";

struct Args {
    command: String,
    input: PathBuf,
    output: Option<PathBuf>,
    options: BTreeMap<String, String>,
    flags: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut raw = std::env::args().skip(1);
    let command = raw.next().ok_or(USAGE)?;
    let input = PathBuf::from(raw.next().ok_or(USAGE)?);
    let mut output = None;
    let mut options = BTreeMap::new();
    let mut flags = Vec::new();
    let rest: Vec<String> = raw.collect();
    let mut i = 0;
    while i < rest.len() {
        let arg = &rest[i];
        match arg.as_str() {
            "-o" | "--output" => {
                output = Some(PathBuf::from(rest.get(i + 1).ok_or("-o needs a path")?));
                i += 2;
            }
            "--strip-brackets" | "--hebrew-paragraphs" => {
                flags.push(arg.trim_start_matches("--").to_string());
                i += 1;
            }
            a if a.starts_with("--") => {
                let value = rest.get(i + 1).ok_or_else(|| format!("{a} needs a value"))?;
                options.insert(a.trim_start_matches("--").to_string(), value.clone());
                i += 2;
            }
            other => return Err(format!("unexpected argument {other}\n\n{USAGE}")),
        }
    }
    Ok(Args { command, input, output, options, flags })
}

fn description(args: &Args) -> Result<Description, String> {
    let need = |k: &str| args.options.get(k).cloned().ok_or_else(|| format!("--{k} is required"));
    let versification = need("versification")?;
    if !grtb::versification::is_known(&versification) {
        return Err(format!("unknown versification '{versification}'; known: {}", grtb::versification::names().join(", ")));
    }
    Ok(Description {
        id: need("id")?,
        abbreviation: need("abbreviation")?,
        title: need("title")?,
        language: need("language")?,
        direction: args.options.get("direction").cloned().unwrap_or_default(),
        year: args.options.get("year").and_then(|y| y.parse().ok()),
        canon: args.options.get("canon").cloned().unwrap_or_else(|| "catholic".into()),
        versification,
        license: need("license")?,
        source: args.options.get("source").cloned().unwrap_or_default(),
    })
}

fn generation_date(args: &Args) -> String {
    if let Some(d) = args.options.get("date") {
        return d.clone();
    }
    let seconds = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64);
    date_from_unix(seconds)
}

fn post_process(books: &mut BTreeMap<u8, BookContent>, flags: &[String]) {
    let strip = flags.iter().any(|f| f == "strip-brackets");
    let hebrew = flags.iter().any(|f| f == "hebrew-paragraphs");
    for body in books.values_mut() {
        let mut next_paragraph = false;
        for v in &mut body.verses {
            if strip {
                v.text = v.text.replace(['[', ']'], "").split(' ').filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
            }
            if hebrew {
                if next_paragraph {
                    v.paragraph = true;
                }
                next_paragraph = false;
                let trimmed = v.text.trim_end();
                for mark in [" ס", " פ", " {ס}", " {פ}"] {
                    if let Some(t) = trimmed.strip_suffix(mark) {
                        v.text = t.trim_end().to_string();
                        next_paragraph = true;
                        break;
                    }
                }
            }
        }
    }
}

fn write_module(args: &Args, books: BTreeMap<u8, BookContent>, hash: &str) -> Result<(), String> {
    let output = args.output.as_ref().ok_or("-o is required")?;
    let content = assemble(&description(args)?, books, &generation_date(args), hash);
    module::write(output, &content)?;
    let report = content.integrity.as_ref().unwrap();
    println!("{}: {}", output.display(), grtb::integrity::summary(report));
    if !report.unmapped.is_empty() {
        println!("  {} verses map outside the canon, first {}", report.unmapped.len(), report.unmapped[0]);
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    match args.command.as_str() {
        "usfm" => {
            let loaded = read_usfm(&args.input)?;
            for w in &loaded.warnings {
                eprintln!("warning: {w}");
            }
            let (mut books, warnings) = arrange(loaded.books);
            for w in warnings {
                eprintln!("warning: {w}");
            }
            post_process(&mut books, &args.flags);
            write_module(&args, books, &loaded.hash)
        }
        "martini" => {
            let bytes = std::fs::read(&args.input).map_err(|e| e.to_string())?;
            let mut parsed = martini::parse(&bytes)?;
            martini::correct(&mut parsed.books, &mut parsed.warnings);
            for w in &parsed.warnings {
                eprintln!("warning: {w}");
            }
            write_module(&args, parsed.books, &sha256_hex(&bytes))
        }
        "n1904" => {
            let bytes = std::fs::read(&args.input).map_err(|e| e.to_string())?;
            let books = n1904::parse(&String::from_utf8_lossy(&bytes))?;
            write_module(&args, books, &sha256_hex(&bytes))
        }
        "sword" if grtb::sword::kind_of(&grtb::sword::probe(&args.input)?) != Some("bible") => {
            let conf = grtb::sword::probe(&args.input)?;
            let mut options = args.options.clone();
            options.entry("versification".into()).or_insert_with(|| grtb::sword::scheme_for(conf.get("Versification").unwrap_or("KJV")).into());
            let args = Args { options, ..args };
            let numbering = grtb::versification::scheme(&args.options["versification"]);
            let read = grtb::sword::read_resource(&args.input, args.options.get("as").map(String::as_str), numbering)?;
            for w in &read.warnings {
                eprintln!("warning: {w}");
            }
            let features = read.conf.values.get("Feature").map(|f| f.lines().map(str::to_string).collect()).unwrap_or_default();
            let output = args.output.as_ref().ok_or("-o is required")?;
            let content = assemble_resource(&description(&args)?, read.kind, read.entries, read.links, features, &generation_date(&args), &read.hash);
            module::write(output, &content)?;
            println!("{}: {} with {} entries and {} groups of references", output.display(), read.kind, content.entries.len(), content.links.len());
            Ok(())
        }
        "sword" => {
            let sword = grtb::sword::open(&args.input)?;
            for w in &sword.warnings {
                eprintln!("warning: {w}");
            }
            let (books, warnings) = arrange(sword.books);
            for w in warnings {
                eprintln!("warning: {w}");
            }
            write_module(&args, books, &sword.hash)
        }
        "morphgnt" => {
            let lexemes = std::fs::read_to_string(args.options.get("lexemes").ok_or("--lexemes is required")?).map_err(|e| e.to_string())?;
            let books = morphgnt::parse(&args.input, &morphgnt::lexemes(&lexemes))?;
            write_module(&args, books, &hash_dir(&args.input)?)
        }
        "oshb" => {
            let index = std::fs::read_to_string(args.options.get("index").ok_or("--index is required")?).map_err(|e| e.to_string())?;
            let aug = std::fs::read_to_string(args.options.get("aug").ok_or("--aug is required")?).map_err(|e| e.to_string())?;
            let books = oshb::parse(&args.input, &oshb::lexical_index(&index, &aug))?;
            write_module(&args, books, &hash_dir(&args.input)?)
        }
        "bdb" => {
            let index = std::fs::read_to_string(args.options.get("index").ok_or("--index is required")?).map_err(|e| e.to_string())?;
            let xml = std::fs::read(&args.input).map_err(|e| e.to_string())?;
            let entries = bdb::parse(&String::from_utf8_lossy(&xml), &bdb::strongs_by_entry(&index));
            let output = args.output.as_ref().ok_or("-o is required")?;
            let content = assemble_resource(&description(&args)?, "dictionary", entries, Vec::new(), vec!["HebrewDef".into()], &generation_date(&args), &sha256_hex(&xml));
            module::write(output, &content)?;
            println!("{}: dictionary with {} entries", output.display(), content.entries.len());
            Ok(())
        }
        "haydock" | "fathers" => {
            let (entries, hash) = if args.command == "haydock" {
                let tsv = std::fs::read(&args.input).map_err(|e| e.to_string())?;
                (commentary::haydock(&String::from_utf8_lossy(&tsv))?, sha256_hex(&tsv))
            } else {
                let which = match args.options.get("excerpts").map(String::as_str) {
                    Some("catena") => commentary::Excerpts::Catena,
                    Some("fathers") => commentary::Excerpts::Fathers,
                    _ => return Err("--excerpts must be catena or fathers".into()),
                };
                let bytes = std::fs::read(&args.input).map_err(|e| e.to_string())?;
                (commentary::fathers(&args.input, which)?, sha256_hex(&bytes))
            };
            let output = args.output.as_ref().ok_or("-o is required")?;
            let content = assemble_resource(&description(&args)?, "commentary", entries, Vec::new(), Vec::new(), &generation_date(&args), &hash);
            module::write(output, &content)?;
            println!("{}: commentary with {} entries", output.display(), content.entries.len());
            Ok(())
        }
        "liturgy" => {
            let parse = |text: &str| grtb::liturgy::parse_date(text).ok_or_else(|| format!("not a date: {text}"));
            let from = parse(&args.input.to_string_lossy())?;
            let to = parse(args.options.get("to").ok_or("--to is required")?)?;
            for n in from..=to {
                let (y, m, d) = grtb::liturgy::civil_from_days(n);
                println!("{}", serde_json::to_string(&grtb::liturgy::day(y, m, d)).map_err(|e| e.to_string())?);
            }
            Ok(())
        }
        "plans" => {
            let all: serde_json::Map<String, serde_json::Value> = grtb::plans::PLANS
                .iter()
                .map(|p| {
                    let days: Vec<Vec<String>> = p.schedule().into_iter().map(|d| d.into_iter().map(|(a, b)| module::range_osis(a, b)).collect()).collect();
                    (p.id.to_string(), serde_json::json!({ "title": p.title, "days": days }))
                })
                .collect();
            println!("{}", serde_json::Value::Object(all));
            Ok(())
        }
        "measure" => measure(&args.input, args.options.get("packs").map(PathBuf::from)),
        "catalog" => build_catalog(&args.input, args.output.as_deref().ok_or("-o is required")?),
        "info" => {
            let m = Module::open(&args.input)?;
            println!("{}", serde_json::to_string_pretty(&m.manifest).unwrap());
            if m.kind() != "bible" {
                println!("{} entries", m.entry_count()?);
            }
            if let Some(report) = &m.integrity {
                println!("{}", grtb::integrity::summary(report));
            }
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

/// Peak resident memory of this process, in kilobytes, where the system says.
fn peak_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|l| l.strip_prefix("VmHWM:")).and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
}

/// Opens the modules the way the program does at start, then reads what a reader would.
fn measure(modules: &Path, packs: Option<PathBuf>) -> Result<(), String> {
    let start = std::time::Instant::now();
    let mut paths: Vec<PathBuf> = Vec::new();
    for dir in std::iter::once(modules.to_path_buf()).chain(packs) {
        let mut found: Vec<PathBuf> = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "grtb").unwrap_or(false)).collect();
        found.sort();
        paths.extend(found);
    }
    let opened: Vec<Module> = paths.iter().map(|p| Module::open(p)).collect::<Result<_, _>>()?;
    for m in &opened {
        if m.kind() == "bible" {
            m.books()?;
        }
    }
    let open_ms = start.elapsed().as_millis();
    let read = std::time::Instant::now();
    let john = grtb::canon::book_by_osis("John").unwrap().index;
    let genesis = grtb::canon::book_by_osis("Gen").unwrap().index;
    for m in &opened {
        match m.kind() {
            "bible" => {
                m.chapter(john, 3)?;
                m.chapter(genesis, 1)?;
                m.chapter_words(john, 3)?;
                if let Some(fts) = grtb::text::fts_query("lux", &m.manifest.language) {
                    m.search(&fts, &[], 50, 0)?;
                }
                if m.has_words()? {
                    grtb::query::run(m, &grtb::query::parse("G25 OR H430").unwrap())?;
                }
            }
            "dictionary" => {
                m.lookup("a", 50)?;
            }
            _ => {}
        }
    }
    println!("modules {}", opened.len());
    println!("open_ms {open_ms}");
    println!("read_ms {}", read.elapsed().as_millis());
    if let Some(kb) = peak_kb() {
        println!("peak_kb {kb}");
    }
    Ok(())
}

/// One checksum over the files of a folder, by name.
fn hash_dir(dir: &Path) -> Result<String, String> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir).map_err(|e| e.to_string())?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_file()).collect();
    paths.sort();
    let mut all = Vec::new();
    for p in paths {
        all.extend_from_slice(p.file_name().unwrap().to_string_lossy().as_bytes());
        all.push(0);
        all.extend(std::fs::read(&p).map_err(|e| e.to_string())?);
    }
    Ok(sha256_hex(&all))
}

/// A folder with `catalog.json` and the thumbnails it names.
fn build_catalog(dir: &Path, output: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(dir.join("catalog.json")).map_err(|e| format!("{}: {e}", dir.join("catalog.json").display()))?;
    let raw: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("catalog.json: {e}"))?;
    let scheme = raw.get("reference_scheme").and_then(|s| s.as_str()).unwrap_or("hebrew").to_string();
    if !grtb::versification::is_known(&scheme) {
        return Err(format!("catalog.json: unknown reference_scheme {scheme}"));
    }
    let mut catalog: Catalog = serde_json::from_value(raw).map_err(|e| format!("catalog.json: {e}"))?;
    let numbering = grtb::versification::scheme(&scheme);
    for video in &mut catalog.videos {
        for r in &mut video.refs {
            let end = r.end.unwrap_or(r.start);
            r.start = numbering.to_canon(r.start).0;
            r.end = Some(numbering.to_canon(end).1).filter(|e| *e != r.start && e.in_canon() && *e > r.start);
        }
        video.refs.retain(|r| r.start.in_canon());
    }
    for channel in &mut catalog.channels {
        if let Some(rest) = channel.url.strip_prefix("http://") {
            channel.url = format!("https://{rest}");
        }
    }
    let mut thumbs = BTreeMap::new();
    for video in &mut catalog.videos {
        if let Some(thumb) = video.thumb.take() {
            let bytes = std::fs::read(dir.join(&thumb)).map_err(|e| format!("{thumb}: {e}"))?;
            let name = format!("resources/{}", thumb.trim_start_matches("./"));
            thumbs.insert(name.clone(), bytes);
            video.thumb = Some(name);
        }
    }
    let problems = catalog::validate(&catalog, &thumbs);
    if !problems.is_empty() {
        return Err(problems.join("\n"));
    }
    let bytes = catalog::write_package(&catalog, &thumbs)?;
    std::fs::write(output, bytes).map_err(|e| e.to_string())?;
    println!("{}: {} videos", output.display(), catalog.videos.len());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
