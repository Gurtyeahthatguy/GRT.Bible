//! Builds `.grtb` modules and the video catalog package.

mod martini;
mod n1904;
mod rtf;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use grtb::assemble::{assemble, date_from_unix, Description};
use grtb::catalog::{self, Catalog};
use grtb::module::{self, BookContent, Module};
use grtb::source::{arrange, read_usfm, sha256_hex};

const USAGE: &str = "\
Usage:
  grtb-convert usfm    <file|folder|zip> [options] -o <out.grtb>
  grtb-convert martini <file.rtf>        [options] -o <out.grtb>
  grtb-convert n1904   <file.csv>        [options] -o <out.grtb>
  grtb-convert sword   <folder|zip>      [options] -o <out.grtb>
  grtb-convert catalog <folder>                    -o <out.grt>
  grtb-convert info    <module.grtb>

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
        "catalog" => build_catalog(&args.input, args.output.as_deref().ok_or("-o is required")?),
        "info" => {
            let m = Module::open(&args.input)?;
            println!("{}", serde_json::to_string_pretty(&m.manifest).unwrap());
            if let Some(report) = &m.integrity {
                println!("{}", grtb::integrity::summary(report));
            }
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
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
