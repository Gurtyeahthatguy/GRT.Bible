//! The video catalog: a `.grt` package with a JSON list and local thumbnails.

use std::collections::{BTreeMap, HashSet};

use grt_container::{read_archive, write_archive, Entry, Manifest, Part, README_NAME, README_TEXT};
use serde::{Deserialize, Serialize};

use crate::refs::Ref;

pub const KIND: &str = "bible-catalog";
const CATALOG_PATH: &str = "content/catalog.json";

/// Hosts a video link may point at.
pub const ALLOWED_HOSTS: [&str; 5] = ["youtube.com", "www.youtube.com", "m.youtube.com", "youtu.be", "www.youtube-nocookie.com"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoRef {
    pub start: Ref,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<Ref>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub url: String,
    /// Id of the channel in `Catalog::channels`, when it came from one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub channel: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub language: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_s: Option<u32>,
    #[serde(default)]
    pub description: String,
    /// Path of the thumbnail inside the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    #[serde(default)]
    pub refs: Vec<VideoRef>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    pub version: u32,
    #[serde(default)]
    pub channels: Vec<Channel>,
    pub videos: Vec<Video>,
}

/// The host of an `https` URL, lower-cased.
pub fn host_of(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.contains('@') {
        return None;
    }
    let host = authority.split(':').next()?.to_lowercase();
    (!host.is_empty()).then_some(host)
}

pub fn url_allowed(url: &str) -> bool {
    host_of(url).map(|h| ALLOWED_HOSTS.contains(&h.as_str())).unwrap_or(false)
}

/// Problems that make a catalog unusable, one message each.
pub fn validate(catalog: &Catalog, thumbs: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut problems = Vec::new();
    let mut channels = HashSet::new();
    for c in &catalog.channels {
        if c.id.is_empty() || !channels.insert(c.id.clone()) {
            problems.push(format!("channel id '{}' is empty or repeated", c.id));
        }
        if !url_allowed(&c.url) {
            problems.push(format!("channel {}: {} is not on an allowed host", c.id, c.url));
        }
    }
    let mut ids = HashSet::new();
    for v in &catalog.videos {
        if v.id.is_empty() || !ids.insert(v.id.clone()) {
            problems.push(format!("video id '{}' is empty or repeated", v.id));
        }
        if !url_allowed(&v.url) {
            problems.push(format!("{}: {} is not on an allowed host", v.id, v.url));
        }
        if !v.channel.is_empty() && !channels.contains(&v.channel) {
            problems.push(format!("{}: channel {} is not listed", v.id, v.channel));
        }
        for r in &v.refs {
            let end = r.end.unwrap_or(r.start);
            if !r.start.in_canon() || !end.in_canon() || end < r.start {
                problems.push(format!("{}: reference {} is not a valid passage", v.id, r.start));
            }
        }
        if let Some(t) = &v.thumb {
            if !thumbs.contains_key(t) {
                problems.push(format!("{}: thumbnail {t} is not in the package", v.id));
            }
        }
    }
    problems
}

pub fn write_package(catalog: &Catalog, thumbs: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, String> {
    let mut manifest = Manifest::new(KIND);
    manifest.parts.push(Part { path: CATALOG_PATH.into(), media_type: "application/json".into() });
    for name in thumbs.keys() {
        let media = if name.ends_with(".png") { "image/png" } else if name.ends_with(".webp") { "image/webp" } else { "image/jpeg" };
        manifest.parts.push(Part { path: name.clone(), media_type: media.into() });
    }
    let mut json = serde_json::to_string_pretty(catalog).map_err(|e| e.to_string())?;
    json.push('\n');
    let mut entries = vec![
        Entry::new(README_NAME, README_TEXT.as_bytes().to_vec()),
        Entry::new("manifest.json", manifest.to_json().map_err(|e| e.to_string())?.into_bytes()),
        Entry::new(CATALOG_PATH, json.into_bytes()),
    ];
    for (name, bytes) in thumbs {
        entries.push(Entry::new(name.clone(), bytes.clone()));
    }
    write_archive(entries).map_err(|e| e.to_string())
}

pub fn read_package(bytes: &[u8]) -> Result<(Catalog, BTreeMap<String, Vec<u8>>), String> {
    let entries = read_archive(bytes).map_err(|e| e.to_string())?;
    let manifest = entries
        .iter()
        .find(|e| e.name == "manifest.json")
        .ok_or("the package has no manifest")
        .and_then(|e| Manifest::from_json(&String::from_utf8_lossy(&e.data)).map_err(|_| "unreadable manifest"))?;
    if manifest.kind != KIND {
        return Err(format!("this is a '{}' file, not a video catalog", manifest.kind));
    }
    let catalog: Catalog = entries
        .iter()
        .find(|e| e.name == CATALOG_PATH)
        .ok_or("the package has no catalog")
        .and_then(|e| serde_json::from_slice(&e.data).map_err(|_| "unreadable catalog"))?;
    let thumbs = entries
        .into_iter()
        .filter(|e| e.name.starts_with("resources/"))
        .map(|e| (e.name, e.data))
        .collect();
    Ok((catalog, thumbs))
}

/// Whether a video speaks about any verse between two internal references.
pub fn touches(video: &Video, first: Ref, last: Ref) -> bool {
    video.refs.iter().any(|r| {
        let end = r.end.unwrap_or(r.start);
        r.start <= last && end >= first
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_listed_hosts() {
        assert!(url_allowed("https://www.youtube.com/watch?v=abc"));
        assert!(url_allowed("https://youtu.be/abc"));
        assert!(!url_allowed("http://www.youtube.com/watch?v=abc"));
        assert!(!url_allowed("https://www.youtube.com.evil.example/watch"));
        assert!(!url_allowed("https://user@www.youtube.com/"));
        assert!(!url_allowed("https://example.com/"));
    }

    #[test]
    fn package_round_trip_is_stable() {
        let catalog = Catalog {
            version: 1,
            channels: vec![Channel { id: "c1".into(), name: "A channel".into(), url: "https://www.youtube.com/@a".into() }],
            videos: vec![Video {
                id: "ps23".into(),
                title: "Psalm 23".into(),
                url: "https://www.youtube.com/watch?v=x".into(),
                channel: "c1".into(),
                category: "Theology".into(),
                source: String::new(),
                language: "it".into(),
                duration_s: Some(300),
                description: String::new(),
                thumb: Some("resources/thumbs/ps23.jpg".into()),
                refs: vec![VideoRef { start: Ref::parse("Ps.23.1").unwrap(), end: Ref::parse("Ps.23.6") }],
                tags: vec!["psalms".into()],
            }],
        };
        let mut thumbs = BTreeMap::new();
        thumbs.insert("resources/thumbs/ps23.jpg".to_string(), vec![0xff, 0xd8, 0xff]);
        assert!(validate(&catalog, &thumbs).is_empty());
        let a = write_package(&catalog, &thumbs).unwrap();
        let b = write_package(&catalog, &thumbs).unwrap();
        assert_eq!(a, b);
        let (back, back_thumbs) = read_package(&a).unwrap();
        assert_eq!(back, catalog);
        assert_eq!(back_thumbs, thumbs);
    }

    #[test]
    fn chapter_overlap() {
        let video = |start: &str, end: Option<&str>| Video {
            id: "v".into(), title: "t".into(), url: "https://youtu.be/x".into(), channel: String::new(),
            category: String::new(), source: String::new(),
            language: String::new(), duration_s: None, description: String::new(), thumb: None,
            refs: vec![VideoRef { start: Ref::parse(start).unwrap(), end: end.and_then(Ref::parse) }], tags: vec![],
        };
        let first = Ref::parse("John.3.1").unwrap();
        let last = Ref::parse("John.3.36").unwrap();
        assert!(touches(&video("John.3.16", None), first, last));
        assert!(touches(&video("John.2.1", Some("John.3.2")), first, last));
        assert!(!touches(&video("John.4.1", None), first, last));
    }

    #[test]
    fn channels_must_be_listed_and_allowed() {
        let mut catalog = Catalog { version: 1, channels: vec![], videos: vec![] };
        catalog.channels.push(Channel { id: "c".into(), name: "C".into(), url: "https://example.com/c".into() });
        catalog.videos.push(Video {
            id: "v".into(), title: "t".into(), url: "https://youtu.be/x".into(), channel: "missing".into(),
            category: String::new(), source: String::new(), language: String::new(), duration_s: None,
            description: String::new(), thumb: None, refs: vec![], tags: vec![],
        });
        let problems = validate(&catalog, &BTreeMap::new());
        assert_eq!(problems.len(), 2, "{problems:?}");
    }
}
