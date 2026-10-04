//! Files the reader chooses in a dialog, which on Android arrive as content URIs rather than paths.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Runtime};

#[cfg(target_os = "android")]
fn android_uri(path: &str) -> Option<tauri_plugin_fs::FilePath> {
    if path.starts_with("content://") || path.starts_with("file://") {
        path.parse().ok()
    } else {
        None
    }
}

#[cfg(target_os = "android")]
fn open_uri<R: Runtime>(app: &AppHandle<R>, uri: tauri_plugin_fs::FilePath, write: bool) -> Result<std::fs::File, String> {
    use tauri_plugin_fs::{FsExt, OpenOptions};
    let mut options = OpenOptions::new();
    if write {
        options.write(true).truncate(true);
    } else {
        options.read(true);
    }
    app.fs().open(uri, options).map_err(|e| format!("The chosen file cannot be opened: {e}"))
}

/// The bytes of a chosen file.
pub fn read<R: Runtime>(app: &AppHandle<R>, path: &str) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "android")]
    if let Some(uri) = android_uri(path) {
        use std::io::Read;
        let mut bytes = Vec::new();
        open_uri(app, uri, false)?.read_to_end(&mut bytes).map_err(|e| format!("The chosen file cannot be read: {e}"))?;
        return Ok(bytes);
    }
    let _ = app;
    std::fs::read(path).map_err(|e| format!("Cannot read {path}: {e}"))
}

/// Writes the bytes where the reader chose to save them.
pub fn write<R: Runtime>(app: &AppHandle<R>, path: &str, bytes: &[u8]) -> Result<(), String> {
    #[cfg(target_os = "android")]
    if let Some(uri) = android_uri(path) {
        use std::io::Write;
        return open_uri(app, uri, true)?.write_all(bytes).map_err(|e| format!("The chosen file cannot be written: {e}"));
    }
    let _ = app;
    crate::store::atomic_write(Path::new(path), bytes)
}

/// A path the importers can open: the chosen file itself or, on Android, a copy of it in the cache.
pub fn local<R: Runtime>(app: &AppHandle<R>, path: &str) -> Result<PathBuf, String> {
    #[cfg(target_os = "android")]
    if let Some(uri) = android_uri(path) {
        use tauri::Manager;
        let name = app
            .path()
            .file_name(path)
            .and_then(|n| Path::new(&n).file_name().map(|f| f.to_string_lossy().to_string()))
            .unwrap_or_else(|| "chosen".into());
        let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?.join("chosen");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
        let target = dir.join(name);
        let mut out = std::fs::File::create(&target).map_err(|e| format!("Cannot write {}: {e}", target.display()))?;
        std::io::copy(&mut open_uri(app, uri, false)?, &mut out).map_err(|e| format!("The chosen file cannot be read: {e}"))?;
        return Ok(target);
    }
    let _ = app;
    Ok(PathBuf::from(path))
}
