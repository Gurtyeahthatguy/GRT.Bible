//! Turning a SWORD or USFM file the reader already has into a module.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use grtb::assemble::{assemble, date_from_unix, Description};
use grtb::module::BookContent;
use grtb::source::{arrange, read_usfm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub kind: String,
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
}

struct Source {
    kind: &'static str,
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

fn read(path: &Path) -> Result<Source, String> {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if is_sword(path) {
        let sword = grtb::sword::open(path)?;
        let (books, mut warnings) = arrange(sword.books);
        warnings.splice(0..0, sword.warnings);
        let conf = &sword.conf;
        Ok(Source {
            kind: "sword",
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

pub fn inspect(path: &Path) -> Result<Inspection, String> {
    let source = read(path)?;
    if source.books.is_empty() {
        return Err("No book of the Catholic canon was found in this file.".into());
    }
    let ranking: Vec<(String, usize)> =
        grtb::integrity::rank_schemes(&source.books).into_iter().map(|(n, s)| (n.to_string(), s)).collect();
    let versification = source.versification.clone().unwrap_or_else(|| ranking[0].0.clone());
    Ok(Inspection {
        kind: source.kind.to_string(),
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

/// Converts the file and writes the module into `folder`, returning its path.
pub fn import(path: &Path, choices: &Choices, folder: &Path) -> Result<PathBuf, String> {
    validate_abbreviation(&choices.abbreviation)?;
    if !grtb::versification::is_known(&choices.versification) {
        return Err(format!("Unknown versification {}.", choices.versification));
    }
    if choices.title.trim().is_empty() || choices.language.trim().is_empty() {
        return Err("A title and a language are needed.".into());
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
        license: "Supplied by the reader".into(),
        source: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
    };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let content = assemble(&description, source.books, &date_from_unix(now), &source.hash);
    std::fs::create_dir_all(folder).map_err(|e| format!("Cannot create {}: {e}", folder.display()))?;
    let target = folder.join(format!("{}.{}", choices.abbreviation, grtb::module::EXTENSION));
    if target.exists() {
        return Err(format!("A module file called {} already exists.", target.display()));
    }
    grtb::module::write(&target, &content)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let choices = Choices { abbreviation: "MYB".into(), title: "Mine".into(), language: "en".into(), versification: "english".into() };
        let out = import(&src, &choices, &dir.path().join("modules")).unwrap();
        let module = grtb::module::Module::open(&out).unwrap();
        assert_eq!(module.manifest.canon, "partial");
        assert_eq!(module.verse(39, 2, 1).unwrap().unwrap(), "Fish.");
        assert!(import(&src, &choices, &dir.path().join("modules")).is_err());
    }

    #[test]
    fn abbreviations() {
        assert!(validate_abbreviation("KJV").is_ok());
        assert!(validate_abbreviation("K").is_err());
        assert!(validate_abbreviation("K J").is_err());
    }
}
