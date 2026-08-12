mod audio;
mod collections;
mod file_replace;
mod library;
mod library_index;
mod library_model;
mod library_snapshot;
mod media;
mod player_commands;
mod playlist_snapshot;
mod storage;
mod system_fonts;
mod track_assets;

use lofty::{file::TaggedFileExt, read_from_path, tag::ItemKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[cfg(desktop)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const MAX_LYRICS_BYTES: u64 = 2 * 1024 * 1024;
const MAX_FRONTEND_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_CRASH_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SETTINGS_BYTES: usize = 1024 * 1024;
const MAX_PLAYLIST_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const MAX_TEXT_CHARS: usize = 4096;
pub(crate) use playlist_snapshot::MAX_AUDIO_SECONDS;
const EXIT_FALLBACK_DELAY: Duration = Duration::from_millis(1500);
const PROJECT_URL: &str = "https://github.com/xleave/Hydrogen-Music";
const PLAYLIST_STRUCTURE_FILE: &str = "last-playlist.json";
const PLAYLIST_STATE_FILE: &str = "last-playlist-state.json";

pub(crate) struct SettingsState(RwLock<Value>);
struct ScanState(Arc<AtomicU64>);

#[derive(Clone, Default)]
pub(crate) struct PersistenceState(pub(crate) Arc<Mutex<()>>);

#[derive(Clone, Default)]
struct PlaybackSnapshotState(Arc<RwLock<Option<Value>>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapState {
    settings: Value,
    last_playlist: Option<Value>,
    collections: collections::Collections,
}

#[derive(Clone, Default)]
struct ExitState(Arc<AtomicBool>);

#[cfg(desktop)]
#[derive(Clone, Default)]
struct ShortcutRegistry(Arc<RwLock<HashMap<u32, String>>>);

#[cfg(not(desktop))]
#[derive(Clone, Default)]
struct ShortcutRegistry;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShortcutBinding {
    id: String,
    shortcut: String,
}

pub(crate) fn bounded_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn stored_music_folders_from_value(settings: &Value) -> Vec<PathBuf> {
    settings
        .get("local")
        .and_then(|local| local.get("localFolder"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect()
}

fn available_music_folders_from_value(settings: &Value) -> Vec<PathBuf> {
    stored_music_folders_from_value(settings)
        .into_iter()
        .filter_map(|folder| {
            let metadata = std::fs::symlink_metadata(&folder).ok()?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return None;
            }
            std::fs::canonicalize(folder).ok()
        })
        .collect()
}

pub(crate) fn configured_music_folders(settings: &SettingsState) -> Result<Vec<PathBuf>, String> {
    let settings = settings.0.read().map_err(|error| error.to_string())?;
    Ok(available_music_folders_from_value(&settings))
}

fn replace_local_folders(settings: &mut Value, folders: &[PathBuf]) -> Result<(), String> {
    let local = settings
        .get_mut("local")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "settings.local is missing".to_string())?;
    local.insert(
        "localFolder".to_string(),
        Value::Array(
            folders
                .iter()
                .map(|folder| Value::String(folder.to_string_lossy().into_owned()))
                .collect(),
        ),
    );
    Ok(())
}

pub(crate) fn authorized_file_path_from_folders(
    folders: &[PathBuf],
    file_path: &str,
) -> Result<PathBuf, String> {
    let file_path = std::fs::canonicalize(file_path).map_err(|error| error.to_string())?;
    if !file_path.is_file() {
        return Err("path is not a file".to_string());
    }
    if !folders.iter().any(|folder| file_path.starts_with(folder)) {
        return Err("file is outside the configured music folders".to_string());
    }
    Ok(file_path)
}

fn json_number(value: Option<&Value>, default: f64, min: f64, max: f64) -> f64 {
    value
        .and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        })
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn sanitize_shortcuts(value: Option<&Value>) -> Value {
    let defaults = storage::default_settings()
        .get("shortcuts")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let Some(items) = value.and_then(Value::as_array) else {
        return defaults;
    };
    let sanitized: Vec<Value> = items
        .iter()
        .take(64)
        .filter_map(|item| {
            let object = item.as_object()?;
            let id = bounded_text(object.get("id")?.as_str()?, 64);
            let name = bounded_text(object.get("name")?.as_str()?, 128);
            let shortcut = bounded_text(
                object.get("shortcut").and_then(Value::as_str).unwrap_or(""),
                128,
            );
            let global = bounded_text(
                object
                    .get("globalShortcut")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                128,
            );
            Some(json!({
                "id": id,
                "name": name,
                "shortcut": shortcut,
                "globalShortcut": global,
            }))
        })
        .collect();
    Value::Array(sanitized)
}

fn sanitize_settings(requested: &Value, authorized_folders: &[PathBuf]) -> Result<Value, String> {
    let music = requested.get("music");
    let other = requested.get("other");
    let requested_folders = stored_music_folders_from_value(requested);
    let retained: Vec<PathBuf> = requested_folders
        .into_iter()
        .filter(|folder| authorized_folders.contains(folder))
        .collect();

    let quit_app = match other
        .and_then(|value| value.get("quitApp"))
        .and_then(Value::as_str)
    {
        Some("quit") => "quit",
        _ => "minimize",
    };
    let custom_font = other
        .and_then(|value| value.get("customFont"))
        .and_then(Value::as_str)
        .map(|value| bounded_text(value, 256))
        .unwrap_or_default();
    let global_shortcuts = other
        .and_then(|value| value.get("globalShortcuts"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut settings = json!({
        "music": {
            "lyricSize": json_number(music.and_then(|v| v.get("lyricSize")), 20.0, 8.0, 96.0),
            "tlyricSize": json_number(music.and_then(|v| v.get("tlyricSize")), 14.0, 8.0, 96.0),
            "rlyricSize": json_number(music.and_then(|v| v.get("rlyricSize")), 12.0, 8.0, 96.0),
            "lyricInterlude": json_number(music.and_then(|v| v.get("lyricInterlude")), 13.0, 1.0, 120.0),
        },
        "local": { "localFolder": [] },
        "shortcuts": sanitize_shortcuts(requested.get("shortcuts")),
        "other": {
            "globalShortcuts": global_shortcuts,
            "quitApp": quit_app,
            "customFont": custom_font,
        }
    });
    replace_local_folders(&mut settings, &retained)?;
    Ok(settings)
}

pub(crate) async fn write_json_async(
    app: AppHandle,
    writer: Arc<Mutex<()>>,
    name: &'static str,
    value: Value,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = writer.lock().map_err(|error| error.to_string())?;
        storage::write_json(&app, name, &value)
    })
    .await
    .map_err(|error| error.to_string())?
}

async fn write_playlist_checkpoint_async(
    app: AppHandle,
    writer: Arc<Mutex<()>>,
    value: Value,
    persist_structure: bool,
) -> Result<(), String> {
    let state = playlist_snapshot::scalar_state(&value);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = writer.lock().map_err(|error| error.to_string())?;
        if persist_structure {
            storage::write_json(&app, PLAYLIST_STRUCTURE_FILE, &value)?;
        }
        storage::write_json(&app, PLAYLIST_STATE_FILE, &state)
    })
    .await
    .map_err(|error| error.to_string())?
}

fn snapshot_with_native_position(
    snapshot: &PlaybackSnapshotState,
    audio: &audio::AudioState,
) -> Option<Value> {
    let mut value = snapshot.0.read().ok()?.clone()?;
    let object = value.as_object_mut()?;
    let has_tracks = object
        .get("songIds")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    let position = if has_tracks { audio.position() } else { 0.0 };
    object.insert("progress".to_string(), json!(position));
    Some(value)
}

fn persist_native_snapshot_blocking(
    app: &AppHandle,
    writer: &Arc<Mutex<()>>,
    snapshot: &PlaybackSnapshotState,
    audio: &audio::AudioState,
) -> Result<(), String> {
    let Some(value) = snapshot_with_native_position(snapshot, audio) else {
        return Ok(());
    };
    let _guard = writer.lock().map_err(|error| error.to_string())?;
    storage::write_json(
        app,
        PLAYLIST_STATE_FILE,
        &playlist_snapshot::scalar_state(&value),
    )
}

#[tauri::command]
async fn select_local_folder(
    app: AppHandle,
    state: State<'_, SettingsState>,
    persistence: State<'_, PersistenceState>,
) -> Result<Option<String>, String> {
    let Some(selected) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let selected = selected.into_path().map_err(|error| error.to_string())?;
    let metadata = std::fs::symlink_metadata(&selected).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("selected path is not a directory".to_string());
    }
    let selected = std::fs::canonicalize(selected).map_err(|error| error.to_string())?;

    let mut next = state.0.read().map_err(|error| error.to_string())?.clone();
    let mut folders = stored_music_folders_from_value(&next);
    if !folders.contains(&selected) {
        folders.push(selected.clone());
    }
    replace_local_folders(&mut next, &folders)?;
    write_json_async(app, persistence.0.clone(), "settings.json", next.clone()).await?;
    *state.0.write().map_err(|error| error.to_string())? = next;
    Ok(Some(selected.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn get_cached_library(
    app: AppHandle,
    settings: State<'_, SettingsState>,
) -> Result<Option<Value>, String> {
    let stored = settings
        .0
        .read()
        .map_err(|error| error.to_string())
        .map(|value| stored_music_folders_from_value(&value))?;
    let folders = configured_music_folders(&settings)?;
    if folders.len() != stored.len() {
        return Ok(None);
    }
    let cache_directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || library_snapshot::load(&cache_directory, &folders))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn scan_local_music(
    app: AppHandle,
    settings: State<'_, SettingsState>,
    scan: State<'_, ScanState>,
    known_revision: Option<String>,
) -> Result<library::ScanResponse, String> {
    let stored = settings
        .0
        .read()
        .map_err(|error| error.to_string())
        .map(|value| stored_music_folders_from_value(&value))?;
    let folders = configured_music_folders(&settings)?;
    let roots_complete = folders.len() == stored.len();
    let latest = scan.0.clone();
    // Native ownership prevents a WebView reload from resetting the generation
    // below the long-lived Rust process state.
    let request_id = latest.fetch_add(1, Ordering::AcqRel) + 1;
    let cache_directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let index_path = cache_directory.join("library-index.sqlite3");
        let result = library::scan(&folders, request_id, &latest, roots_complete, &index_path)?;
        if !result.matches_revision(known_revision.as_deref()) {
            if let Err(error) = library_snapshot::save(&cache_directory, &folders, &result) {
                eprintln!("[library snapshot] failed to persist cache: {error}");
            }
        }
        Ok(result.into_response(known_revision.as_deref()))
    })
    .await
    .map_err(|error| error.to_string())?
}

fn read_cover_blocking(
    folders: Vec<PathBuf>,
    cache_directory: PathBuf,
    cache: track_assets::CoverCache,
    file_path: String,
) -> Result<Option<String>, String> {
    let file_path = authorized_file_path_from_folders(&folders, &file_path)?;
    cache
        .read(&cache_directory, &file_path)
        .map(|path| path.map(|path| path.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn read_cover(
    app: AppHandle,
    settings: State<'_, SettingsState>,
    cache: State<'_, track_assets::CoverCache>,
    file_path: String,
) -> Result<Option<String>, String> {
    let folders = configured_music_folders(&settings)?;
    let cache_directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        read_cover_blocking(folders, cache_directory, cache, file_path)
    })
    .await
    .map_err(|error| error.to_string())?
}

fn read_lyrics_blocking(
    folders: Vec<PathBuf>,
    file_path: String,
) -> Result<Option<String>, String> {
    let audio_path = authorized_file_path_from_folders(&folders, &file_path)?;
    if let Ok(tagged) = read_from_path(&audio_path) {
        if let Some(lyrics) = tagged.tags().iter().find_map(|tag| {
            tag.get_string(ItemKey::Lyrics)
                .or_else(|| tag.get_string(ItemKey::UnsyncLyrics))
        }) {
            if !lyrics.trim().is_empty() {
                if lyrics.len() as u64 > MAX_LYRICS_BYTES {
                    return Err("embedded lyrics are too large".to_string());
                }
                return Ok(Some(lyrics.to_owned()));
            }
        }
    }

    let lrc_path = audio_path.with_extension("lrc");
    if !lrc_path.exists() {
        return Ok(None);
    }
    let lrc_path = authorized_file_path_from_folders(&folders, &lrc_path.to_string_lossy())?;
    if std::fs::metadata(&lrc_path)
        .map_err(|error| error.to_string())?
        .len()
        > MAX_LYRICS_BYTES
    {
        return Err("lyrics file is too large".to_string());
    }
    std::fs::read_to_string(lrc_path)
        .map(Some)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn read_lyrics(
    settings: State<'_, SettingsState>,
    file_path: String,
) -> Result<Option<String>, String> {
    let folders = configured_music_folders(&settings)?;
    tauri::async_runtime::spawn_blocking(move || read_lyrics_blocking(folders, file_path))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
fn open_project_page(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(PROJECT_URL, None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn reveal_music_file(
    app: AppHandle,
    settings: State<'_, SettingsState>,
    file_path: String,
) -> Result<(), String> {
    let folders = configured_music_folders(&settings)?;
    let path = tauri::async_runtime::spawn_blocking(move || {
        authorized_file_path_from_folders(&folders, &file_path)
    })
    .await
    .map_err(|error| error.to_string())??;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_settings(settings: State<'_, SettingsState>) -> Result<Value, String> {
    settings
        .0
        .read()
        .map(|value| value.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_bootstrap_state(
    settings: State<'_, SettingsState>,
    snapshot: State<'_, PlaybackSnapshotState>,
    collections: State<'_, collections::CollectionsState>,
) -> Result<BootstrapState, String> {
    let settings = settings
        .0
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    let last_playlist = snapshot
        .0
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    let collections = collections
        .0
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    Ok(BootstrapState {
        settings,
        last_playlist,
        collections,
    })
}

#[tauri::command]
async fn set_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    persistence: State<'_, PersistenceState>,
    settings: String,
) -> Result<(), String> {
    if settings.len() > MAX_SETTINGS_BYTES {
        return Err("settings payload is too large".to_string());
    }
    let requested: Value = serde_json::from_str(&settings).map_err(|error| error.to_string())?;
    let authorized = state
        .0
        .read()
        .map_err(|error| error.to_string())
        .map(|current| stored_music_folders_from_value(&current))?;
    let value = sanitize_settings(&requested, &authorized)?;
    write_json_async(app, persistence.0.clone(), "settings.json", value.clone()).await?;
    *state.0.write().map_err(|error| error.to_string())? = value;
    Ok(())
}

#[tauri::command]
async fn report_frontend_error(
    app: AppHandle,
    source: String,
    detail: String,
) -> Result<(), String> {
    let source = bounded_text(&source, 256).replace(['\r', '\n'], " ");
    let detail = bounded_text(&detail, 64 * 1024);
    tauri::async_runtime::spawn_blocking(move || {
        let directory = app
            .path()
            .app_log_dir()
            .map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let path = directory.join("frontend-errors.log");
        let truncate = std::fs::metadata(&path)
            .map(|metadata| metadata.len() >= MAX_FRONTEND_LOG_BYTES)
            .unwrap_or(false);
        let mut options = OpenOptions::new();
        options.create(true).write(true);
        if truncate {
            options.truncate(true);
        } else {
            options.append(true);
        }
        let mut file = options.open(path).map_err(|error| error.to_string())?;
        writeln!(file, "[{source}] {detail}").map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn get_last_playlist(snapshot: State<'_, PlaybackSnapshotState>) -> Result<Option<Value>, String> {
    snapshot
        .0
        .read()
        .map(|value| value.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn save_last_playlist(
    app: AppHandle,
    snapshot: State<'_, PlaybackSnapshotState>,
    persistence: State<'_, PersistenceState>,
    playlist: String,
) -> Result<(), String> {
    if playlist.len() > MAX_PLAYLIST_BYTES {
        return Err("playlist payload is too large".to_string());
    }
    let raw: Value = serde_json::from_str(&playlist).map_err(|error| error.to_string())?;
    let persist_structure = raw
        .as_object()
        .is_some_and(|object| object.contains_key("songIds") || object.contains_key("songList"));
    let current = snapshot
        .0
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    let value = playlist_snapshot::sanitize_update(&raw, current.as_ref())?;
    *snapshot.0.write().map_err(|error| error.to_string())? = Some(value.clone());
    write_playlist_checkpoint_async(app, persistence.0.clone(), value, persist_structure).await
}

fn allowed_shortcut_action(id: &str) -> bool {
    matches!(
        id,
        "play" | "last" | "next" | "volumeUp" | "volumeDown" | "processForward" | "processBack"
    )
}

#[tauri::command]
fn register_shortcuts(
    app: AppHandle,
    registry: State<'_, ShortcutRegistry>,
    shortcuts: Vec<ShortcutBinding>,
) -> Result<(), String> {
    #[cfg(desktop)]
    {
        let manager = app.global_shortcut();
        manager
            .unregister_all()
            .map_err(|error| error.to_string())?;
        registry
            .0
            .write()
            .map_err(|error| error.to_string())?
            .clear();

        let mut bindings = HashMap::new();
        for binding in shortcuts.into_iter().take(64) {
            if !allowed_shortcut_action(&binding.id) || binding.shortcut.trim().is_empty() {
                continue;
            }
            let shortcut = Shortcut::try_from(binding.shortcut.as_str())
                .map_err(|error| format!("invalid shortcut {}: {error}", binding.shortcut))?;
            let shortcut_id = shortcut.id;
            if let Err(error) = manager.register(shortcut) {
                let _ = manager.unregister_all();
                registry
                    .0
                    .write()
                    .map_err(|lock_error| lock_error.to_string())?
                    .clear();
                return Err(error.to_string());
            }
            bindings.insert(shortcut_id, bounded_text(&binding.id, 64));
        }
        *registry.0.write().map_err(|error| error.to_string())? = bindings;
        Ok(())
    }

    #[cfg(not(desktop))]
    {
        let _ = (app, registry, shortcuts);
        Ok(())
    }
}

#[tauri::command]
fn unregister_shortcuts(
    app: AppHandle,
    registry: State<'_, ShortcutRegistry>,
) -> Result<(), String> {
    #[cfg(desktop)]
    {
        app.global_shortcut()
            .unregister_all()
            .map_err(|error| error.to_string())?;
        registry
            .0
            .write()
            .map_err(|error| error.to_string())?
            .clear();
        Ok(())
    }

    #[cfg(not(desktop))]
    {
        let _ = (app, registry);
        Ok(())
    }
}

fn is_minimize_to_tray(settings: &SettingsState) -> bool {
    settings
        .0
        .read()
        .ok()
        .and_then(|value| {
            value
                .get("other")
                .and_then(|other| other.get("quitApp"))
                .and_then(Value::as_str)
                .map(|mode| mode == "minimize")
        })
        .unwrap_or(true)
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub(crate) fn request_exit(app: &AppHandle) {
    let exit_state = app.state::<ExitState>();
    if exit_state.0.swap(true, Ordering::AcqRel) {
        return;
    }

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("app-exit-requested", ());
    }

    // The WebView normally flushes settings/playlist and calls quit_app. If it
    // is wedged, Rust still owns enough playback state to persist a safe final
    // checkpoint and terminate the process without waiting on renderer IPC.
    let app_handle = app.clone();
    let writer = app.state::<PersistenceState>().0.clone();
    let snapshot = app.state::<PlaybackSnapshotState>().inner().clone();
    let audio = app.state::<audio::AudioState>().inner().clone();
    std::thread::spawn(move || {
        std::thread::sleep(EXIT_FALLBACK_DELAY);
        let _ = persist_native_snapshot_blocking(&app_handle, &writer, &snapshot, &audio);
        app_handle.exit(0);
    });
}

#[tauri::command]
async fn quit_app(
    app: AppHandle,
    exit: State<'_, ExitState>,
    persistence: State<'_, PersistenceState>,
    snapshot: State<'_, PlaybackSnapshotState>,
    audio: State<'_, audio::AudioState>,
) -> Result<(), String> {
    exit.0.store(true, Ordering::Release);
    let app_handle = app.clone();
    let writer = persistence.0.clone();
    let snapshot = snapshot.inner().clone();
    let audio = audio.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        persist_native_snapshot_blocking(&app_handle, &writer, &snapshot, &audio)
    })
    .await
    .map_err(|error| error.to_string())?;
    app.exit(0);
    result
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Single-instance must be the first plugin so a second process exits
    // before it can claim the desktop media service name.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _, _| {
        show_window(app);
    }));

    let builder = builder
        .manage(audio::AudioState::default())
        .manage(ScanState(Arc::new(AtomicU64::new(0))))
        .manage(PersistenceState::default())
        .manage(ExitState::default())
        .manage(ShortcutRegistry::default());

    #[cfg(desktop)]
    let builder = builder.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                if event.state != ShortcutState::Pressed {
                    return;
                }
                let action = app
                    .state::<ShortcutRegistry>()
                    .0
                    .read()
                    .ok()
                    .and_then(|bindings| bindings.get(&shortcut.id).cloned());
                if let Some(action) = action {
                    let _ = app.emit("shortcut-action", action);
                }
            })
            .build(),
    );

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let raw_settings = storage::read_json_with_limit(
                app.handle(),
                "settings.json",
                storage::default_settings(),
                MAX_SETTINGS_BYTES as u64,
            )
            .map_err(std::io::Error::other)?;
            let authorized = stored_music_folders_from_value(&raw_settings);
            let settings =
                sanitize_settings(&raw_settings, &authorized).map_err(std::io::Error::other)?;
            storage::write_json(app.handle(), "settings.json", &settings)
                .map_err(std::io::Error::other)?;
            app.manage(SettingsState(RwLock::new(settings)));

            let playlist_structure = storage::read_optional_json_with_limit(
                app.handle(),
                PLAYLIST_STRUCTURE_FILE,
                MAX_PLAYLIST_BYTES as u64,
            )
            .map_err(std::io::Error::other)?
            .and_then(|value| playlist_snapshot::sanitize(&value).ok());
            let playlist_state = storage::read_optional_json_with_limit(
                app.handle(),
                PLAYLIST_STATE_FILE,
                MAX_PLAYLIST_BYTES as u64,
            )
            .map_err(std::io::Error::other)?;
            let initial_playlist = playlist_structure.map(|structure| {
                playlist_snapshot::merge_stored_state(structure, playlist_state.as_ref())
            });
            app.manage(PlaybackSnapshotState(Arc::new(RwLock::new(
                initial_playlist,
            ))));

            let collections = collections::load(app.handle()).map_err(std::io::Error::other)?;
            app.manage(collections::CollectionsState(RwLock::new(collections)));

            // System media integration is optional. D-Bus/SMTC initialization
            // failure must never make the native audio player itself fail.
            app.manage(media::MediaState::new(app.handle()));
            app.manage(track_assets::CoverCache::default());

            let log_directory = app.path().app_log_dir()?;
            std::fs::create_dir_all(&log_directory)?;
            let default_panic_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic_info| {
                default_panic_hook(panic_info);
                let crash_path = log_directory.join("crash.log");
                if std::fs::metadata(&crash_path)
                    .map(|metadata| metadata.len() >= MAX_CRASH_LOG_BYTES)
                    .unwrap_or(false)
                {
                    let _ = std::fs::remove_file(&crash_path);
                }
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(crash_path)
                {
                    let _ = writeln!(file, "{panic_info}");
                }
            }));

            let show_item =
                MenuItem::with_id(app, "show", "显示 Hydrogen Music", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Hydrogen Music")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_window(app),
                    "quit" => request_exit(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if is_minimize_to_tray(&window.state::<SettingsState>()) {
                    let _ = window.hide();
                    let _ = window.emit("tray-hide", ());
                } else {
                    request_exit(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            select_local_folder,
            get_cached_library,
            scan_local_music,
            read_cover,
            read_lyrics,
            player_commands::audio_load,
            player_commands::audio_play,
            player_commands::audio_pause,
            player_commands::audio_seek,
            player_commands::audio_set_volume,
            player_commands::audio_status,
            player_commands::audio_stop,
            player_commands::media_set_metadata,
            player_commands::media_set_playback,
            player_commands::media_set_stopped,
            player_commands::media_clear,
            player_commands::media_set_volume,
            system_fonts::list_system_fonts,
            open_project_page,
            reveal_music_file,
            get_settings,
            get_bootstrap_state,
            set_settings,
            get_last_playlist,
            save_last_playlist,
            collections::get_collections,
            collections::save_collections,
            report_frontend_error,
            register_shortcuts,
            unregister_shortcuts,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Hydrogen Music");
}
