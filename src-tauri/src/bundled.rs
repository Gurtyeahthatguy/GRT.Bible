//! The texts and the video catalog that come with the program, as Android needs them unpacked.

use std::fs::{self, File};
use std::io;
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/bundled.rs"));

/// Copies every bundled file into `dir`, unless the stamp there shows this set is already unpacked.
pub fn unpack(dir: &Path, stamp: &str, files: &[&str], mut open: impl FnMut(&str) -> io::Result<File>) -> Result<(), String> {
    let stamp_file = dir.join("stamp");
    let current = fs::read_to_string(&stamp_file).is_ok_and(|s| s == stamp);
    if current && files.iter().all(|f| dir.join(f).is_file()) {
        return Ok(());
    }
    let _ = fs::remove_dir_all(dir);
    for name in files {
        let target = dir.join(name);
        let temp = dir.join(format!("{name}.unpacking"));
        let copied = (|| -> io::Result<()> {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut source = open(name)?;
            let mut out = File::create(&temp)?;
            io::copy(&mut source, &mut out)?;
            out.sync_all()?;
            fs::rename(&temp, &target)
        })();
        if let Err(e) = copied {
            let _ = fs::remove_file(&temp);
            return Err(format!("The texts that come with the program could not be unpacked ({name}: {e})."));
        }
    }
    crate::store::atomic_write(&stamp_file, stamp.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(dir: &Path) -> impl FnMut(&str) -> io::Result<File> + '_ {
        move |name| File::open(dir.join(name))
    }

    #[test]
    fn unpacks_once_per_stamp() {
        let src = tempfile::tempdir().unwrap();
        fs::create_dir_all(src.path().join("modules")).unwrap();
        fs::write(src.path().join("modules/A.grtb"), b"first").unwrap();
        fs::write(src.path().join("modules/B.grtb"), b"second").unwrap();
        let out = tempfile::tempdir().unwrap();
        let dir = out.path().join("bundled");

        unpack(&dir, "one", &["modules/A.grtb", "modules/B.grtb"], source(src.path())).unwrap();
        assert_eq!(fs::read(dir.join("modules/B.grtb")).unwrap(), b"second");

        let mut opened = 0;
        unpack(&dir, "one", &["modules/A.grtb", "modules/B.grtb"], |_| {
            opened += 1;
            Err(io::Error::other("not needed"))
        })
        .unwrap();
        assert_eq!(opened, 0);

        fs::write(src.path().join("modules/A.grtb"), b"changed").unwrap();
        unpack(&dir, "two", &["modules/A.grtb"], source(src.path())).unwrap();
        assert_eq!(fs::read(dir.join("modules/A.grtb")).unwrap(), b"changed");
        assert!(!dir.join("modules/B.grtb").exists());
    }

    #[test]
    fn a_failed_copy_leaves_no_stamp() {
        let out = tempfile::tempdir().unwrap();
        let dir = out.path().join("bundled");
        let result = unpack(&dir, "one", &["modules/A.grtb"], |_| Err(io::Error::other("gone")));
        assert!(result.unwrap_err().contains("modules/A.grtb"));
        assert!(!dir.join("stamp").exists());
        assert!(!dir.join("modules/A.grtb.unpacking").exists());
    }
}
