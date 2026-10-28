//! Turning a SWORD or USFM file the reader already has into a module.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use grtb::assemble::{assemble, assemble_resource, date_from_unix, Description};
use grtb::module::BookContent;
use grtb::source::{arrange, read_usfm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub kind: String,
    /// What the module holds: `bible`, `commentary`, `crossrefs`, `dictionary` or `book`.
    pub module_kind: String,
    /// Kinds the reader may choose between, when the file can be read more than one way.
    pub kinds: Vec<String>,
    pub entries: usize,
    pub title: String,
    pub abbreviation: String,
    pub language: String,
    pub versification: String,
    pub ranking: Vec<(String, usize)>,
    pub books: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choices {
    pub abbreviation: String,
    pub title: String,
    pub language: String,
    pub versification: String,
    #[serde(default)]
    pub kind: Option<String>,
}

/// Share of a commentary's words in references above which it is offered as cross-references first.
const MOSTLY_REFERENCES: f32 = 0.25;

struct Source {
    kind: &'static str,
    module_kind: &'static str,
    license: String,
    origin: String,
    books: BTreeMap<u8, BookContent>,
    hash: String,
    title: String,
    abbreviation: String,
    language: String,
    versification: Option<String>,
    warnings: Vec<String>,
}

fn is_sword(path: &Path) -> bool {
    if path.is_dir() {
        return path.join("mods.d").is_dir();
    }
    let Ok(file) = std::fs::File::open(path) else { return false };
    let mut head = [0u8; 2];
    if file.take(2).read_exact(&mut head).is_err() || &head != b"PK" {
        return false;
    }
    std::fs::read(path)
        .ok()
        .and_then(|bytes| zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok())
        .map(|a| a.file_names().any(|n| n.to_lowercase().starts_with("mods.d/")))
        .unwrap_or(false)
}

fn suggested_abbreviation(text: &str) -> String {
    let letters: String = text.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    letters.chars().take(8).collect::<String>().to_uppercase()
}

fn sword_origin(conf: &grtb::sword::Conf) -> String {
    match conf.get("Version") {
        Some(v) => format!("SWORD module {}, version {v}", conf.name),
        None => format!("SWORD module {}", conf.name),
    }
}

/// A SWORD module that is not a Bible, read as `kind`.
fn read_resource(path: &Path, kind: Option<&str>, versification: Option<&str>) -> Result<(grtb::sword::Resource, String), String> {
    let conf = grtb::sword::probe(path)?;
    let scheme_name = versification.map(str::to_string).unwrap_or_else(|| grtb::sword::scheme_for(conf.get("Versification").unwrap_or("KJV")).to_string());
    let read = grtb::sword::read_resource(path, kind, grtb::versification::scheme(&scheme_name))?;
    Ok((read, scheme_name))
}

fn read(path: &Path) -> Result<Source, String> {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if is_sword(path) {
        let sword = grtb::sword::open(path)?;
        let (books, mut warnings) = arrange(sword.books);
        warnings.splice(0..0, sword.warnings);
        let conf = &sword.conf;
        Ok(Source {
            kind: "sword",
            module_kind: "bible",
            license: conf.get("DistributionLicense").unwrap_or("Supplied by the reader").to_string(),
            origin: sword_origin(conf),
            books,
            hash: sword.hash,
            title: conf.get("Description").unwrap_or(&conf.name).to_string(),
            abbreviation: suggested_abbreviation(&conf.name),
            language: conf.get("Lang").unwrap_or("").to_string(),
            versification: Some(grtb::sword::scheme_for(conf.get("Versification").unwrap_or("KJV")).to_string()),
            warnings,
        })
    } else {
        let loaded = read_usfm(path)?;
        let (books, mut warnings) = arrange(loaded.books);
        warnings.splice(0..0, loaded.warnings);
        Ok(Source {
            kind: "usfm",
            module_kind: "bible",
            license: "Supplied by the reader".into(),
            origin: String::new(),
            books,
            hash: loaded.hash,
            title: stem.replace(['_', '-'], " "),
            abbreviation: suggested_abbreviation(&stem.split(['_', '-']).next().unwrap_or(&stem)),
            language: String::new(),
            versification: None,
            warnings,
        })
    }
}

fn inspect_resource(path: &Path, conf: &grtb::sword::Conf, found: &str) -> Result<Inspection, String> {
    let (read, scheme) = read_resource(path, None, None)?;
    let mut kinds = vec![found.to_string()];
    let mut module_kind = found.to_string();
    if found == "commentary" {
        kinds.push("crossrefs".into());
        if grtb::sword::reference_share(&read.entries) > MOSTLY_REFERENCES {
            kinds.reverse();
            module_kind = "crossrefs".into();
        }
    }
    let by_verse = matches!(found, "commentary" | "crossrefs");
    Ok(Inspection {
        kind: "sword".into(),
        module_kind,
        kinds,
        entries: read.entries.len(),
        title: conf.get("Description").unwrap_or(&conf.name).to_string(),
        abbreviation: suggested_abbreviation(&conf.name),
        language: conf.get("Lang").unwrap_or("").to_string(),
        versification: scheme.clone(),
        ranking: if by_verse { grtb::versification::GENERAL_SCHEMES.iter().map(|s| (s.to_string(), 0)).collect() } else { Vec::new() },
        books: 0,
        warnings: read.warnings,
    })
}

pub fn inspect(path: &Path) -> Result<Inspection, String> {
    if is_sword(path) {
        let conf = grtb::sword::probe(path)?;
        match grtb::sword::kind_of(&conf) {
            Some("bible") => {}
            Some(found) => return inspect_resource(path, &conf, found),
            None => return Err(format!("'{}' uses the driver {}, which GRT Bible does not read.", conf.name, conf.get("ModDrv").unwrap_or("?"))),
        }
    }
    let source = read(path)?;
    if source.books.is_empty() {
        return Err("No book of the Catholic canon was found in this file.".into());
    }
    let ranking: Vec<(String, usize)> =
        grtb::integrity::rank_schemes(&source.books).into_iter().map(|(n, s)| (n.to_string(), s)).collect();
    let versification = source.versification.clone().unwrap_or_else(|| ranking[0].0.clone());
    Ok(Inspection {
        kind: source.kind.to_string(),
        module_kind: source.module_kind.to_string(),
        kinds: vec![source.module_kind.to_string()],
        entries: 0,
        title: source.title,
        abbreviation: source.abbreviation,
        language: source.language,
        versification,
        ranking,
        books: source.books.len(),
        warnings: source.warnings,
    })
}

pub fn validate_abbreviation(abbreviation: &str) -> Result<(), String> {
    let ok = (2..=10).contains(&abbreviation.len()) && abbreviation.chars().all(|c| c.is_ascii_alphanumeric());
    if ok {
        Ok(())
    } else {
        Err("The abbreviation must be 2 to 10 letters or digits.".into())
    }
}

pub fn today() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    date_from_unix(now)
}

fn target(folder: &Path, abbreviation: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(folder).map_err(|e| format!("Cannot create {}: {e}", folder.display()))?;
    let target = folder.join(format!("{abbreviation}.{}", grtb::module::EXTENSION));
    if target.exists() {
        return Err(format!("A module file called {} already exists.", target.display()));
    }
    Ok(target)
}

fn import_resource(path: &Path, choices: &Choices, folder: &Path) -> Result<PathBuf, String> {
    let kind = choices.kind.as_deref();
    let (read, scheme) = read_resource(path, kind, Some(&choices.versification))?;
    let conf = &read.conf;
    let description = Description {
        id: format!("imported-{}", choices.abbreviation.to_lowercase()),
        abbreviation: choices.abbreviation.clone(),
        title: choices.title.trim().to_string(),
        language: choices.language.trim().to_lowercase(),
        direction: if conf.get("Direction").map(|d| d.eq_ignore_ascii_case("RtoL")).unwrap_or(false) { "rtl".into() } else { String::new() },
        year: None,
        canon: String::new(),
        versification: scheme,
        license: conf.get("DistributionLicense").unwrap_or("Supplied by the reader").to_string(),
        source: sword_origin(conf),
    };
    let features = conf.values.get("Feature").map(|f| f.lines().map(str::to_string).collect()).unwrap_or_default();
    let content = assemble_resource(&description, read.kind, read.entries, read.links, features, &today(), &read.hash);
    let target = target(folder, &choices.abbreviation)?;
    grtb::module::write(&target, &content)?;
    Ok(target)
}

/// Converts the file and writes the module into `folder`, returning its path.
pub fn import(path: &Path, choices: &Choices, folder: &Path) -> Result<PathBuf, String> {
    validate_abbreviation(&choices.abbreviation)?;
    if !grtb::versification::is_known(&choices.versification) {
        return Err(format!("Unknown versification {}.", choices.versification));
    }
    if choices.title.trim().is_empty() || choices.language.trim().is_empty() {
        return Err("A title and a language are needed.".into());
    }
    if is_sword(path) && grtb::sword::kind_of(&grtb::sword::probe(path)?) != Some("bible") {
        return import_resource(path, choices, folder);
    }
    let source = read(path)?;
    let count = source.books.len();
    let deutero = source.books.keys().any(|i| grtb::canon::book(*i).map(|b| b.deutero).unwrap_or(false));
    let canon = if count == 73 { "catholic" } else if count == 66 && !deutero { "protestant" } else { "partial" };
    let description = Description {
        id: format!("imported-{}", choices.abbreviation.to_lowercase()),
        abbreviation: choices.abbreviation.clone(),
        title: choices.title.trim().to_string(),
        language: choices.language.trim().to_lowercase(),
        direction: String::new(),
        year: None,
        canon: canon.into(),
        versification: choices.versification.clone(),
        license: source.license.clone(),
        source: if source.origin.is_empty() { path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default() } else { source.origin.clone() },
    };
    let content = assemble(&description, source.books, &today(), &source.hash);
    let target = target(folder, &choices.abbreviation)?;
    grtb::module::write(&target, &content)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use grtb::refs::Ref;

    #[test]
    fn a_usfm_folder_becomes_a_module() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("my-bible");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("01-JON.usfm"), "\\id JON\n\\c 1\n\\p\n\\v 1 Word.\n\\v 2 Arise.\n\\c 2\n\\p\n\\v 1 Fish.\n\\v 2 Prayed.\n").unwrap();
        let inspection = inspect(&src).unwrap();
        assert_eq!(inspection.kind, "usfm");
        assert_eq!(inspection.books, 1);
        assert_eq!(inspection.abbreviation, "MY");
        let choices = Choices { abbreviation: "MYB".into(), title: "Mine".into(), language: "en".into(), versification: "english".into(), kind: None };
        let out = import(&src, &choices, &dir.path().join("modules")).unwrap();
        let module = grtb::module::Module::open(&out).unwrap();
        assert_eq!(module.manifest.canon, "partial");
        assert_eq!(module.verse(39, 2, 1).unwrap().unwrap(), "Fish.");
        assert!(import(&src, &choices, &dir.path().join("modules")).is_err());
    }

    #[test]
    fn a_commentary_of_references_is_offered_as_cross_references() {
        let tsk = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sources/sword-test/TSK.zip");
        if !tsk.exists() {
            return;
        }
        let found = inspect(&tsk).unwrap();
        assert_eq!((found.module_kind.as_str(), found.kinds.len()), ("crossrefs", 2));
        let dir = tempfile::tempdir().unwrap();
        let choices = Choices { abbreviation: "TSK".into(), title: "Treasury".into(), language: "en".into(), versification: "english".into(), kind: Some("crossrefs".into()) };
        let out = import(&tsk, &choices, dir.path()).unwrap();
        let module = grtb::module::Module::open(&out).unwrap();
        assert_eq!(module.kind(), "crossrefs");
        assert_eq!(module.manifest.license, "Public Domain");
        assert!(!module.links_at(Ref::parse("John.3.16").unwrap()).unwrap().is_empty());
    }

    #[test]
    fn abbreviations() {
        assert!(validate_abbreviation("KJV").is_ok());
        assert!(validate_abbreviation("K").is_err());
        assert!(validate_abbreviation("K J").is_err());
    }
}
