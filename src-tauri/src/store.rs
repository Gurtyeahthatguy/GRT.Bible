//! The reader's file and the interface preferences on disk.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use grtb::userdata::UserData;

pub struct Store {
    pub user_file: PathBuf,
    pub settings_file: PathBuf,
}

fn temp_beside(target: &Path) -> PathBuf {
    let name = target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    target.with_file_name(format!(".{name}.writing"))
}

/// Replaces a file only once the new bytes are safely on disk.
pub fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
    }
    let temp = temp_beside(target);
    let written = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&temp);
        return Err(format!("Cannot write {}: {e}", target.display()));
    }
    fs::rename(&temp, target).map_err(|e| {
        let _ = fs::remove_file(&temp);
        format!("Cannot replace {}: {e}", target.display())
    })
}

impl Store {
    /// The reader's data, and a message when an unreadable file had to be set aside.
    pub fn load_user(&self) -> (UserData, Option<String>) {
        let Ok(bytes) = fs::read(&self.user_file) else {
            return (UserData::default(), None);
        };
        match UserData::from_archive(&bytes) {
            Ok(data) => (data, None),
            Err(e) => {
                let aside = self.user_file.with_extension("grt.unreadable");
                let _ = fs::rename(&self.user_file, &aside);
                (UserData::default(), Some(format!("Your data file could not be read ({e}). It was kept as {}.", aside.display())))
            }
        }
    }

    pub fn save_user(&self, data: &UserData) -> Result<(), String> {
        atomic_write(&self.user_file, &data.to_archive()?)
    }

    pub fn load_settings(&self) -> serde_json::Value {
        fs::read(&self.settings_file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .filter(serde_json::Value::is_object)
            .unwrap_or_else(|| serde_json::json!({}))
    }

    pub fn save_settings(&self, value: &serde_json::Value) -> Result<(), String> {
        if !value.is_object() {
            return Err("settings must be an object".into());
        }
        let mut text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
        text.push('\n');
        atomic_write(&self.settings_file, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &Path) -> Store {
        Store { user_file: dir.join("user.grt"), settings_file: dir.join("settings.json") }
    }

    #[test]
    fn missing_files_give_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        assert_eq!(s.load_user().0, UserData::default());
        assert_eq!(s.load_settings(), serde_json::json!({}));
    }

    #[test]
    fn an_unreadable_file_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        fs::write(&s.user_file, b"not a zip").unwrap();
        let (data, message) = s.load_user();
        assert_eq!(data, UserData::default());
        assert!(message.unwrap().contains("could not be read"));
        assert!(dir.path().join("user.grt.unreadable").exists());
    }

    #[test]
    fn saves_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        let mut data = UserData::default();
        data.saved_videos.push("x".into());
        s.save_user(&data).unwrap();
        assert_eq!(s.load_user().0, data);
        s.save_settings(&serde_json::json!({"theme": "sepia"})).unwrap();
        assert_eq!(s.load_settings()["theme"], "sepia");
    }
}
