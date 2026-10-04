//! Rust backend for GRT Bible.

#[cfg(any(target_os = "android", test))]
mod bundled;
mod importer;
mod library;
mod picked;
mod store;
mod videos;

use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use grtb::refs::Ref;
use grtb::userdata::UserData;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use library::{Library, ModuleInfo};
use store::Store;
use videos::Videos;

struct Paths {
    bundled_modules: PathBuf,
    imported_modules: PathBuf,
    bundled_catalog: PathBuf,
    imported_catalog: PathBuf,
}

struct AppState {
    paths: Paths,
    store: Store,
    library: RwLock<Library>,
    user: Mutex<UserData>,
    videos: RwLock<Videos>,
    notices: Vec<String>,
}

type Shared<'a> = State<'a, AppState>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Startup {
    version: &'static str,
    platform: &'static str,
    modules: Vec<ModuleInfo>,
    settings: serde_json::Value,
    user: UserData,
    videos: usize,
    notices: Vec<String>,
    schemes: Vec<&'static str>,
}

fn read_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

fn write_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

fn parse_ref(text: &str) -> Result<Ref, String> {
    Ref::parse(text).ok_or_else(|| format!("not a reference: {text}"))
}

#[tauri::command]
fn startup(state: Shared) -> Startup {
    Startup {
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        modules: read_lock(&state.library).infos(),
        settings: state.store.load_settings(),
        user: state.user.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        videos: read_lock(&state.videos).catalog.videos.len(),
        notices: state.notices.clone(),
        schemes: grtb::versification::GENERAL_SCHEMES.to_vec(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChapterReply {
    #[serde(flatten)]
    view: library::ChapterView,
    videos: Vec<String>,
}

#[tauri::command]
fn chapter(state: Shared, module: String, book: String, chapter: u16) -> Result<ChapterReply, String> {
    let entry = read_lock(&state.library).get(&module)?;
    let view = library::chapter(&entry, &book, chapter)?;
    let videos = match (view.canon_first.as_deref().and_then(Ref::parse), view.canon_last.as_deref().and_then(Ref::parse)) {
        (Some(a), Some(b)) => read_lock(&state.videos).about(Ref { verse: 1, ..a }, Ref { verse: u16::MAX, ..b }),
        _ => Vec::new(),
    };
    Ok(ChapterReply { view, videos })
}

#[tauri::command]
fn resolve(state: Shared, module: String, refs: Vec<String>) -> Result<Vec<Option<library::Place>>, String> {
    let entry = read_lock(&state.library).get(&module)?;
    refs.iter().map(|r| Ok(library::resolve(&entry, parse_ref(r)?))).collect()
}

#[tauri::command]
fn to_canon(state: Shared, module: String, book: String, chapter: u16, verse: u16) -> Result<(String, String), String> {
    let entry = read_lock(&state.library).get(&module)?;
    let (a, b) = entry.scheme().to_canon(Ref::new(library::book_index(&book)?, chapter, verse));
    Ok((a.osis(), b.osis()))
}

#[tauri::command]
fn verse_texts(state: Shared, module: String, refs: Vec<String>) -> Result<Vec<Option<String>>, String> {
    let entry = read_lock(&state.library).get(&module)?;
    refs.iter()
        .map(|r| {
            let canon = parse_ref(r)?;
            Ok(library::resolve(&entry, canon).and_then(|p| {
                let index = library::book_index(p.book).ok()?;
                entry.with(|m| m.verse(index, p.chapter, p.verse).ok().flatten())
            }))
        })
        .collect()
}

#[tauri::command]
fn parallel(state: Shared, modules: Vec<String>, book: String, chapter: u16) -> Result<library::ParallelView, String> {
    let lib = read_lock(&state.library);
    let primary = lib.get(modules.first().ok_or("no module chosen")?)?;
    let others = modules.iter().skip(1).map(|m| lib.get(m)).collect::<Result<Vec<_>, _>>()?;
    library::parallel(&primary, &others, &book, chapter)
}

#[tauri::command]
fn search(state: Shared, query: String, modules: Vec<String>, books: Vec<String>, offset: u32, limit: u32) -> Result<library::SearchView, String> {
    let lib = read_lock(&state.library);
    let entries = if modules.is_empty() {
        lib.all()
    } else {
        modules.iter().map(|m| lib.get(m)).collect::<Result<Vec<_>, _>>()?
    };
    let books = books.iter().map(|b| library::book_index(b)).collect::<Result<Vec<_>, _>>()?;
    library::search(&entries, &query, &books, offset, limit)
}

#[tauri::command]
fn votd(day: u32) -> (String, String) {
    let (a, b) = grtb::votd::for_day(day);
    (a.osis(), b.osis())
}

#[tauri::command]
fn user_save(state: Shared, data: UserData) -> Result<(), String> {
    state.store.save_user(&data)?;
    *state.user.lock().unwrap_or_else(|e| e.into_inner()) = data;
    Ok(())
}

#[tauri::command]
fn user_export(app: AppHandle, state: Shared, path: String) -> Result<(), String> {
    let data = state.user.lock().unwrap_or_else(|e| e.into_inner()).clone();
    picked::write(&app, &path, &data.to_archive()?)
}

#[tauri::command]
fn user_import(app: AppHandle, state: Shared, path: String) -> Result<UserData, String> {
    let bytes = picked::read(&app, &path)?;
    let data = UserData::from_archive(&bytes)?;
    state.store.save_user(&data)?;
    *state.user.lock().unwrap_or_else(|e| e.into_inner()) = data.clone();
    Ok(data)
}

#[tauri::command]
fn settings_save(state: Shared, settings: serde_json::Value) -> Result<(), String> {
    state.store.save_settings(&settings)
}

#[tauri::command]
fn videos_list(state: Shared) -> videos::CatalogView {
    read_lock(&state.videos).views()
}

#[tauri::command]
fn videos_import(app: AppHandle, state: Shared, path: String) -> Result<usize, String> {
    let bytes = picked::read(&app, &path)?;
    let loaded = Videos::from_bytes(&bytes)?;
    store::atomic_write(&state.paths.imported_catalog, &bytes)?;
    let count = loaded.catalog.videos.len();
    *write_lock(&state.videos) = loaded;
    Ok(count)
}

#[tauri::command]
fn video_open(app: AppHandle, state: Shared, id: String) -> Result<(), String> {
    let url = read_lock(&state.videos).link(&id)?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("The browser could not be started: {e}"))
}

#[tauri::command]
fn channel_open(app: AppHandle, state: Shared, id: String) -> Result<(), String> {
    let url = read_lock(&state.videos).channel_link(&id)?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("The browser could not be started: {e}"))
}

#[tauri::command]
async fn import_inspect(app: AppHandle, path: String) -> Result<importer::Inspection, String> {
    tauri::async_runtime::spawn_blocking(move || importer::inspect(&picked::local(&app, &path)?))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn import_module(app: AppHandle, path: String, choices: importer::Choices) -> Result<ModuleInfo, String> {
    let state = app.state::<AppState>();
    if read_lock(&state.library).contains(&choices.abbreviation) {
        return Err(format!("A module called {} is already installed.", choices.abbreviation));
    }
    let folder = state.paths.imported_modules.clone();
    let handle = app.clone();
    let written = tauri::async_runtime::spawn_blocking(move || importer::import(&picked::local(&handle, &path)?, &choices, &folder))
        .await
        .map_err(|e| e.to_string())??;
    let result = write_lock(&state.library).add(&written);
    if result.is_err() {
        let _ = std::fs::remove_file(&written);
    }
    result
}

#[tauri::command]
fn module_remove(state: Shared, module: String) -> Result<(), String> {
    let path = write_lock(&state.library).remove(&module)?;
    std::fs::remove_file(&path).map_err(|e| format!("Cannot delete {}: {e}", path.display()))
}

#[tauri::command]
fn module_report(state: Shared, module: String) -> Result<Option<grtb::integrity::Report>, String> {
    let entry = read_lock(&state.library).get(&module)?;
    Ok(entry.with(|m| m.integrity.clone()))
}

fn build_state(app: &AppHandle) -> Result<AppState, String> {
    let resolver = app.path();
    let resources = resolver.resource_dir().map_err(|e| e.to_string())?;
    let data = resolver.app_data_dir().map_err(|e| e.to_string())?;
    let config = resolver.app_config_dir().map_err(|e| e.to_string())?;
    let mut notices = Vec::new();

    // Android keeps the texts inside the package, and SQLite needs them as ordinary files.
    #[cfg(target_os = "android")]
    let resource_root = {
        use tauri_plugin_fs::{FsExt, OpenOptions};
        let root = data.join("bundled");
        let unpacked = bundled::unpack(&root, bundled::STAMP, bundled::FILES, |name| {
            let mut options = OpenOptions::new();
            options.read(true);
            app.fs().open(resources.join(name), options)
        });
        notices.extend(unpacked.err());
        // The fs plugin leaves its own copy of each asset in the cache.
        if let Ok(cache) = resolver.app_cache_dir() {
            let _ = std::fs::remove_dir_all(cache.join("_assets"));
        }
        root
    };

    // Packages put the texts in the resource folder; a copied binary keeps them beside itself.
    #[cfg(not(target_os = "android"))]
    let mut resource_root = resources.clone();
    #[cfg(not(target_os = "android"))]
    if !resource_root.join("modules").is_dir() {
        let beside_binary = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));
        if let Some(dir) = beside_binary.filter(|d| d.join("modules").is_dir()) {
            resource_root = dir;
        }
        #[cfg(debug_assertions)]
        {
            let source_tree = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
            if !resource_root.join("modules").is_dir() && source_tree.join("modules").is_dir() {
                resource_root = source_tree;
            }
        }
    }
    let paths = Paths {
        bundled_modules: resource_root.join("modules"),
        imported_modules: data.join("modules"),
        bundled_catalog: resource_root.join("catalog").join("videos.grt"),
        imported_catalog: data.join("videos.grt"),
    };
    let store = Store { user_file: data.join("user.grt"), settings_file: config.join("settings.json") };

    let (library, problems) = Library::load(&[(paths.bundled_modules.clone(), true), (paths.imported_modules.clone(), false)]);
    notices.extend(problems);
    let (user, problem) = store.load_user();
    notices.extend(problem);
    let (videos, problem) = Videos::load(&paths.imported_catalog, &paths.bundled_catalog);
    notices.extend(problem);

    Ok(AppState {
        paths,
        store,
        library: RwLock::new(library),
        user: Mutex::new(user),
        videos: RwLock::new(videos),
        notices,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_opener::init());
    #[cfg(target_os = "android")]
    let builder = builder.plugin(tauri_plugin_fs::init());
    builder
        .setup(|app| {
            let state = build_state(app.handle())?;
            app.manage(state);
            // The window opens once the texts are ready, so no command can arrive before them.
            tauri::WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?.build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            startup,
            chapter,
            resolve,
            to_canon,
            verse_texts,
            parallel,
            search,
            votd,
            user_save,
            user_export,
            user_import,
            settings_save,
            videos_list,
            videos_import,
            video_open,
            channel_open,
            import_inspect,
            import_module,
            module_remove,
            module_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running GRT Bible");
}
