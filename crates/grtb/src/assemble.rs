//! From arranged books to a module ready to write.

use std::collections::BTreeMap;

use crate::integrity;
use crate::module::{missing_books, BookContent, Manifest, ModuleContent, SCHEMA_VERSION};
use crate::versification::scheme;

/// What a person says about a module; the rest is worked out from the text.
#[derive(Debug, Clone, Default)]
pub struct Description {
    pub id: String,
    pub abbreviation: String,
    pub title: String,
    pub language: String,
    pub direction: String,
    pub year: Option<i32>,
    pub canon: String,
    pub versification: String,
    pub license: String,
    pub source: String,
}

pub fn assemble(description: &Description, books: BTreeMap<u8, BookContent>, generated_at: &str, source_hash: &str) -> ModuleContent {
    let report = integrity::check(&books, scheme(&description.versification));
    let direction = if description.direction.is_empty() {
        if matches!(description.language.as_str(), "hbo" | "he" | "heb" | "ar" | "syc") { "rtl" } else { "ltr" }
    } else {
        description.direction.as_str()
    };
    let manifest = Manifest {
        id: description.id.clone(),
        abbreviation: description.abbreviation.clone(),
        title: description.title.clone(),
        language: description.language.clone(),
        direction: direction.to_string(),
        year: description.year,
        canon: description.canon.clone(),
        versification: description.versification.clone(),
        license: description.license.clone(),
        source: description.source.clone(),
        books_present: books.len() as u32,
        source_notes: books.values().any(|b| !b.notes.is_empty()),
        schema_version: SCHEMA_VERSION,
        generated_at: generated_at.to_string(),
        source_hash: source_hash.to_string(),
        missing_books: missing_books(&books),
    };
    ModuleContent { manifest, books, integrity: Some(report) }
}

/// The civil date of a Unix time, as `YYYY-MM-DD`.
pub fn date_from_unix(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Day of the year, from 1, for a Unix time.
pub fn day_of_year(seconds: i64) -> u32 {
    let date = date_from_unix(seconds);
    let year: i64 = date[..4].parse().unwrap();
    let start = days_from_civil(year, 1, 1);
    (seconds.div_euclid(86_400) - start) as u32 + 1
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(date_from_unix(0), "1970-01-01");
        assert_eq!(date_from_unix(1_789_000_000), "2026-09-10");
        assert_eq!(day_of_year(0), 1);
        assert_eq!(day_of_year(days_from_civil(2024, 12, 31) * 86_400), 366);
    }
}
