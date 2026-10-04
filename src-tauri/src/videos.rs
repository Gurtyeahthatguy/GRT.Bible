//! The video catalog as the interface sees it.

use std::collections::BTreeMap;
use std::path::Path;

use grtb::catalog::{read_package, touches, url_allowed, Catalog, VideoRef};
use grtb::refs::Ref;
use serde::Serialize;

#[derive(Default)]
pub struct Videos {
    pub catalog: Catalog,
    thumbs: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelView {
    pub id: String,
    pub name: String,
    pub url: String,
    pub videos: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogView {
    pub channels: Vec<ChannelView>,
    pub videos: Vec<VideoView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoView {
    pub id: String,
    pub title: String,
    pub host: String,
    pub channel: String,
    pub category: String,
    pub source: String,
    pub language: String,
    pub duration_s: Option<u32>,
    pub description: String,
    pub thumb: Option<String>,
    pub refs: Vec<VideoRef>,
    pub tags: Vec<String>,
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

impl Videos {
    /// The imported catalog if there is a readable one, otherwise the bundled one.
    pub fn load(imported: &Path, bundled: &Path) -> (Videos, Option<String>) {
        let mut problem = None;
        for path in [imported, bundled] {
            let Ok(bytes) = std::fs::read(path) else { continue };
            match read_package(&bytes) {
                Ok((catalog, thumbs)) => return (Videos { catalog, thumbs }, problem),
                Err(e) => problem = Some(format!("{}: {e}", path.display())),
            }
        }
        (Videos::default(), problem)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Videos, String> {
        let (catalog, thumbs) = read_package(bytes)?;
        let problems = grtb::catalog::validate(&catalog, &thumbs);
        if !problems.is_empty() {
            return Err(problems.join("\n"));
        }
        Ok(Videos { catalog, thumbs })
    }

    pub fn views(&self) -> CatalogView {
        let channels = self
            .catalog
            .channels
            .iter()
            .map(|c| ChannelView {
                id: c.id.clone(),
                name: c.name.clone(),
                url: c.url.clone(),
                videos: self.catalog.videos.iter().filter(|v| v.channel == c.id).count(),
            })
            .collect();
        let videos = self
            .catalog
            .videos
            .iter()
            .map(|v| VideoView {
                id: v.id.clone(),
                title: v.title.clone(),
                host: grtb::catalog::host_of(&v.url).unwrap_or_default(),
                channel: v.channel.clone(),
                category: v.category.clone(),
                source: v.source.clone(),
                language: v.language.clone(),
                duration_s: v.duration_s,
                description: v.description.clone(),
                thumb: v.thumb.as_ref().and_then(|t| {
                    let bytes = self.thumbs.get(t)?;
                    let media = if t.ends_with(".png") { "image/png" } else if t.ends_with(".webp") { "image/webp" } else { "image/jpeg" };
                    Some(format!("data:{media};base64,{}", base64(bytes)))
                }),
                refs: v.refs.clone(),
                tags: v.tags.clone(),
            })
            .collect();
        CatalogView { channels, videos }
    }

    /// Ids of the videos about any verse in an internal range.
    pub fn about(&self, first: Ref, last: Ref) -> Vec<String> {
        self.catalog.videos.iter().filter(|v| touches(v, first, last)).map(|v| v.id.clone()).collect()
    }

    /// The link for a video, only if its host is on the allow list.
    pub fn link(&self, id: &str) -> Result<String, String> {
        let video = self.catalog.videos.iter().find(|v| v.id == id).ok_or("This video is no longer in the catalog.")?;
        if !url_allowed(&video.url) {
            return Err("This link points outside the allowed sites and was not opened.".into());
        }
        Ok(video.url.clone())
    }

    /// The page of a channel, only if its host is on the allow list.
    pub fn channel_link(&self, id: &str) -> Result<String, String> {
        let channel = self.catalog.channels.iter().find(|c| c.id == id).ok_or("This channel is no longer in the catalog.")?;
        if !url_allowed(&channel.url) {
            return Err("This link points outside the allowed sites and was not opened.".into());
        }
        Ok(channel.url.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn empty_when_nothing_is_there() {
        let dir = tempfile::tempdir().unwrap();
        let (videos, problem) = Videos::load(&dir.path().join("a.grt"), &dir.path().join("b.grt"));
        assert!(videos.catalog.videos.is_empty());
        assert!(problem.is_none());
        assert!(videos.link("x").is_err());
    }

    #[test]
    fn the_shipped_catalog_is_sorted_into_channels_and_categories() {
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../catalog/videos.grt");
        let (videos, problem) = Videos::load(Path::new("/nonexistent"), &bundled);
        assert!(problem.is_none(), "{problem:?}");
        let view = videos.views();
        for video in &view.videos {
            assert!(view.channels.iter().any(|c| c.id == video.channel), "{} has no channel", video.id);
            assert!(["Theology", "Doctrine", "Debates"].contains(&video.category.as_str()), "{}", video.category);
        }
        for channel in &view.channels {
            assert!(videos.channel_link(&channel.id).unwrap().starts_with("https://www.youtube.com/@"));
        }
        if let Some(first) = view.videos.first() {
            assert!(videos.link(&first.id).unwrap().starts_with("https://www.youtube.com/watch?v="));
        }
    }
}
