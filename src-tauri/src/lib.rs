mod audio;
mod library;
mod media;
mod storage;

use base64::{engine::general_purpose::STANDARD, Engine};
use lofty::{file::TaggedFileExt, read_from_path, tag::ItemKey};
use serde_json::{json, Value};
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

const MAX_COVER_BYTES: usize = 16 * 1024 * 1024;
const MAX_LYRICS_BYTES: u64 = 2 * 1024 * 1024;
const MAX_FRONTEND_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SETTINGS_BYTES: usize = 1024 * 1024;
const MAX_PLAYLIST_BYTES: usize = 4 * 1024 * 1024;
const MAX_PLAYLIST_TRACKS: usize = 100_000;
const MAX_TEXT_CHARS: usize = 4096;
const MAX_PATH_CHARS: usize = 16_384;
const MAX_AUDIO_SECONDS: f64 = 7.0 * 24.0 * 60.0 * 60.0;
const PROJECT_URL: &str = "https://github.com/xleave/Hydrogen-Music";

struct SettingsState(RwLock<Value>);
struct ScanState(Arc<AtomicU64>);

fn bounded_text(value: &str, max_chars: usize) -> String {
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

fn configured_music_folders(settings: &SettingsState) -> Result<Vec<PathBuf>, String> {
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

fn authorized_file_path_from_folders(folders: &[PathBuf], file_path: &str) -> Result<PathBuf, String> {
    let file_path = std::fs::canonicalize(file_path).map_err(|error| error.to_string())?;
    if !file_path.is_file() {
        return Err("path is not a file".to_string());
    }
    if !folders.iter().any(|folder| file_path.starts_with(folder)) {
        return Err("file is outside the configured music folders".to_string());
    }
    Ok(file_path)
}

fn finite(value: f64, name: &str) -> Result<f64, String> {
    if value.is_finite() { Ok(value) } else { Err(format!("{name} must be finite")) }
}

fn finite_f32(value: f32, name: &str) -> Result<f32, String> {
    if value.is_finite() { Ok(value) } else { Err(format!("{name} must be finite")) }
}

fn json_number(value: Option<&Value>, default: f64, min: f64, max: f64) -> f64 {
    value
        .and_then(|value| value.as_f64().or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok())))
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
            let shortcut = bounded_text(object.get("shortcut").and_then(Value::as_str).unwrap_or(""), 128);
            let global = bounded_text(object.get("globalShortcut").and_then(Value::as_str).unwrap_or(""), 128);
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

fn playlist_ids(object: &serde_json::Map<String, Value>, ids_name: &str, legacy_name: &str) -> Vec<Value> {
    if let Some(ids) = object.get(ids_name).and_then(Value::as_array) {
        return ids
            .iter()
            .filter_map(Value::as_str)
            .take(MAX_PLAYLIST_TRACKS)
            .map(|id| Value::String(bounded_text(id, MAX_PATH_CHARS)))
            .collect();
    }
    object
        .get(legacy_name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|track| track.get("id").and_then(Value::as_str))
        .take(MAX_PLAYLIST_TRACKS)
        .map(|id| Value::String(bounded_text(id, MAX_PATH_CHARS)))
        .collect()
}

fn sanitize_playlist(value: &Value) -> Result<Value, String> {
    let object = value.as_object().ok_or_else(|| "playlist must be an object".to_string())?;
    let song_ids = playlist_ids(object, "songIds", "songList");
    let shuffled_song_ids = playlist_ids(object, "shuffledSongIds", "shuffledList");
    let current_song_id = object
        .get("currentSongId")
        .and_then(Value::as_str)
        .map(|id| bounded_text(id, MAX_PATH_CHARS));
    let current_index = object
        .get("currentIndex")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(song_ids.len().saturating_sub(1) as u64);
    let shuffle_index = object
        .get("shuffleIndex")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(shuffled_song_ids.len().saturating_sub(1) as u64);
    let progress = json_number(object.get("progress"), 0.0, 0.0, MAX_AUDIO_SECONDS);
    let volume = json_number(object.get("volume"), 0.3, 0.0, 1.0);
    let play_mode = object.get("playMode").and_then(Value::as_u64).unwrap_or(0).min(3);

    Ok(json!({
        "version": 3,
        "songIds": song_ids,
        "shuffledSongIds": shuffled_song_ids,
        "currentSongId": current_song_id,
        "currentIndex": current_index,
        "shuffleIndex": shuffle_index,
        "progress": progress,
        "volume": volume,
        "playMode": play_mode,
    }))
}

#[tauri::command]
async fn select_local_folder(app: AppHandle, state: State<'_, SettingsState>) -> Result<Option<String>, String> {
    let Some(selected) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let selected = selected.into_path().map_err(|error| error.to_string())?;
    let metadata = std::fs::symlink_metadata(&selected).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("selected path is not a directory".to_string());
    }
    let selected = std::fs::canonicalize(selected).map_err(|error| error.to_string())?;

    let mut current = state.0.write().map_err(|error| error.to_string())?;
    let mut next = current.clone();
    let mut folders = stored_music_folders_from_value(&next);
    if !folders.contains(&selected) {
        folders.push(selected.clone());
    }
    replace_local_folders(&mut next, &folders)?;
    storage::write_json(&app, "settings.json", &next)?;
    *current = next;
    Ok(Some(selected.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn scan_local_music(
    settings: State<'_, SettingsState>,
    scan: State<'_, ScanState>,
    request_id: u64,
) -> Result<library::ScanResult, String> {
    let folders = configured_music_folders(&settings)?;
    let latest = scan.0.clone();
    latest.store(request_id, Ordering::Release);
    tauri::async_runtime::spawn_blocking(move || library::scan(&folders, request_id, &latest))
        .await
        .map_err(|error| error.to_string())?
}

fn read_cover_blocking(folders: Vec<PathBuf>, file_path: String) -> Result<Option<String>, String> {
    let file_path = authorized_file_path_from_folders(&folders, &file_path)?;
    let Ok(tagged) = read_from_path(&file_path) else {
        return Ok(None);
    };
    let picture = tagged.tags().iter().find_map(|tag| tag.pictures().first());
    let Some(picture) = picture else {
        return Ok(None);
    };
    if picture.data().len() > MAX_COVER_BYTES {
        return Err("embedded cover is too large".to_string());
    }
    let mime = cover_mime(picture.data());
    Ok(Some(format!("data:{mime};base64,{}", STANDARD.encode(picture.data()))))
}

#[tauri::command]
async fn read_cover(settings: State<'_, SettingsState>, file_path: String) -> Result<Option<String>, String> {
    let folders = configured_music_folders(&settings)?;
    tauri::async_runtime::spawn_blocking(move || read_cover_blocking(folders, file_path))
        .await
        .map_err(|error| error.to_string())?
}

fn cover_mime(data: &[u8]) -> &'static str {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if data.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if data.starts_with(b"GIF8") {
        "image/gif"
    } else if data.starts_with(b"BM") {
        "image/bmp"
    } else if data.starts_with(b"II*\0") || data.starts_with(b"MM\0*") {
        "image/tiff"
    } else {
        "application/octet-stream"
    }
}

fn read_lyrics_blocking(folders: Vec<PathBuf>, file_path: String) -> Result<Option<String>, String> {
    let audio_path = authorized_file_path_from_folders(&folders, &file_path)?;
    if let Ok(tagged) = read_from_path(&audio_path) {
        if let Some(lyrics) = tagged.tags().iter().find_map(|tag| {
            tag.get_string(ItemKey::Lyrics).or_else(|| tag.get_string(ItemKey::UnsyncLyrics))
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
    if std::fs::metadata(&lrc_path).map_err(|error| error.to_string())?.len() > MAX_LYRICS_BYTES {
        return Err("lyrics file is too large".to_string());
    }
    std::fs::read_to_string(lrc_path).map(Some).map_err(|error| error.to_string())
}

#[tauri::command]
async fn read_lyrics(settings: State<'_, SettingsState>, file_path: String) -> Result<Option<String>, String> {
    let folders = configured_music_folders(&settings)?;
    tauri::async_runtime::spawn_blocking(move || read_lyrics_blocking(folders, file_path))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn audio_load(
    settings: State<'_, SettingsState>,
    audio: State<'_, audio::AudioState>,
    file_path: String,
    autoplay: bool,
    volume: f32,
    request_id: u64,
) -> Result<audio::AudioStatus, String> {
    let volume = finite_f32(volume, "volume")?.clamp(0.0, 1.0);
    let folders = configured_music_folders(&settings)?;
    let audio = audio.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = authorized_file_path_from_folders(&folders, &file_path)?;
        audio.load(&path, autoplay, volume, request_id)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn audio_play(audio: State<'_, audio::AudioState>) -> Result<audio::AudioStatus, String> { audio.play() }
#[tauri::command]
fn audio_pause(audio: State<'_, audio::AudioState>) -> Result<audio::AudioStatus, String> { audio.pause() }
#[tauri::command]
fn audio_seek(audio: State<'_, audio::AudioState>, position: f64) -> Result<audio::AudioStatus, String> {
    audio.seek(finite(position, "position")?.clamp(0.0, MAX_AUDIO_SECONDS))
}
#[tauri::command]
fn audio_set_volume(audio: State<'_, audio::AudioState>, volume: f32) -> Result<(), String> {
    audio.set_volume(finite_f32(volume, "volume")?.clamp(0.0, 1.0))
}
#[tauri::command]
fn audio_status(audio: State<'_, audio::AudioState>) -> Result<audio::AudioStatus, String> { audio.status() }
#[tauri::command]
fn audio_stop(audio: State<'_, audio::AudioState>) -> Result<(), String> { audio.stop() }

#[tauri::command]
fn media_set_metadata(
    media: State<'_, media::MediaState>,
    title: String,
    artist: String,
    album: String,
    duration: f64,
) -> Result<(), String> {
    let duration = finite(duration, "duration")?.clamp(0.0, MAX_AUDIO_SECONDS);
    media.set_metadata(
        &bounded_text(&title, MAX_TEXT_CHARS),
        &bounded_text(&artist, MAX_TEXT_CHARS),
        &bounded_text(&album, MAX_TEXT_CHARS),
        duration,
    )
}

#[tauri::command]
fn media_set_playback(
    audio: State<'_, audio::AudioState>,
    media: State<'_, media::MediaState>,
    playing: bool,
) -> Result<(), String> {
    media.set_playback(playing, audio.position())
}

#[tauri::command]
fn media_set_volume(media: State<'_, media::MediaState>, volume: f64) -> Result<(), String> {
    media.set_volume(finite(volume, "volume")?.clamp(0.0, 1.0))
}

fn list_system_fonts_blocking() -> Result<Vec<String>, String> {
    #[cfg(target_os = "linux")]
    {
        use std::collections::BTreeSet;
        use std::process::Command;

        let program = if std::path::Path::new("/usr/bin/fc-list").is_file() {
            "/usr/bin/fc-list"
        } else {
            "fc-list"
        };
        let output = Command::new(program)
            .arg("--format=%{family}\n")
            .output()
            .map_err(|error| format!("failed to execute fc-list: {error}"))?;
        if !output.status.success() {
            return Err("fc-list returned a non-zero status".to_string());
        }

        let mut families = BTreeSet::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            for family in line.split(',') {
                let family = family.trim();
                if !family.is_empty() {
                    families.insert(bounded_text(family, 256));
                }
            }
        }
        Ok(families.into_iter().collect())
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(Vec::new())
    }
}

#[tauri::command]
async fn list_system_fonts() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(list_system_fonts_blocking)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
fn open_project_page(app: AppHandle) -> Result<(), String> {
    app.opener().open_url(PROJECT_URL, None::<&str>).map_err(|error| error.to_string())
}

#[tauri::command]
async fn reveal_music_file(
    app: AppHandle,
    settings: State<'_, SettingsState>,
    file_path: String,
) -> Result<(), String> {
    let folders = configured_music_folders(&settings)?;
    let path = tauri::async_runtime::spawn_blocking(move || authorized_file_path_from_folders(&folders, &file_path))
        .await
        .map_err(|error| error.to_string())??;
    app.opener().reveal_item_in_dir(path).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_settings(settings: State<'_, SettingsState>) -> Result<Value, String> {
    settings.0.read().map(|value| value.clone()).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_settings(app: AppHandle, state: State<'_, SettingsState>, settings: String) -> Result<(), String> {
    if settings.len() > MAX_SETTINGS_BYTES {
        return Err("settings payload is too large".to_string());
    }
    let requested: Value = serde_json::from_str(&settings).map_err(|error| error.to_string())?;
    let mut current = state.0.write().map_err(|error| error.to_string())?;
    let authorized = stored_music_folders_from_value(&current);
    let value = sanitize_settings(&requested, &authorized)?;
    storage::write_json(&app, "settings.json", &value)?;
    *current = value;
    Ok(())
}

#[tauri::command]
fn report_frontend_error(app: AppHandle, source: String, detail: String) -> Result<(), String> {
    let source = bounded_text(&source, 256).replace('\r', " ").replace('\n', " ");
    let detail = bounded_text(&detail, 64 * 1024);
    let directory = app.path().app_log_dir().map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join("frontend-errors.log");
    let truncate = std::fs::metadata(&path).map(|metadata| metadata.len() >= MAX_FRONTEND_LOG_BYTES).unwrap_or(false);
    let mut options = OpenOptions::new();
    options.create(true).write(true);
    if truncate { options.truncate(true); } else { options.append(true); }
    let mut file = options.open(path).map_err(|error| error.to_string())?;
    writeln!(file, "[{source}] {detail}").map_err(|error| error.to_string())
}

#[tauri::command]
fn get_last_playlist(app: AppHandle) -> Result<Option<Value>, String> {
    let Some(value) = storage::read_optional_json(&app, "last-playlist.json")? else {
        return Ok(None);
    };
    sanitize_playlist(&value).map(Some)
}

#[tauri::command]
fn save_last_playlist(app: AppHandle, playlist: String) -> Result<(), String> {
    if playlist.len() > MAX_PLAYLIST_BYTES {
        return Err("playlist payload is too large".to_string());
    }
    let raw: Value = serde_json::from_str(&playlist).map_err(|error| error.to_string())?;
    let value = sanitize_playlist(&raw)?;
    storage::write_json(&app, "last-playlist.json", &value)
}

fn is_minimize_to_tray(settings: &SettingsState) -> bool {
    settings.0.read().ok().and_then(|value| {
        value.get("other").and_then(|other| other.get("quitApp")).and_then(Value::as_str).map(|mode| mode == "minimize")
    }).unwrap_or(true)
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn request_exit(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("app-exit-requested", ());
    } else {
        app.exit(0);
    }
}

#[tauri::command]
async fn quit_app(app: AppHandle) { app.exit(0); }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(audio::AudioState::default())
        .manage(ScanState(Arc::new(AtomicU64::new(0))))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let raw_settings = storage::read_json(app.handle(), "settings.json", storage::default_settings()).map_err(std::io::Error::other)?;
            let authorized = stored_music_folders_from_value(&raw_settings);
            let settings = sanitize_settings(&raw_settings, &authorized).map_err(std::io::Error::other)?;
            storage::write_json(app.handle(), "settings.json", &settings).map_err(std::io::Error::other)?;
            app.manage(SettingsState(RwLock::new(settings)));
            let media = media::MediaState::new(app.handle()).map_err(std::io::Error::other)?;
            app.manage(media);

            let log_directory = app.path().app_log_dir()?;
            std::fs::create_dir_all(&log_directory)?;
            let default_panic_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic_info| {
                default_panic_hook(panic_info);
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_directory.join("crash.log")) {
                    let _ = writeln!(file, "{panic_info}");
                }
            }));

            let show_item = MenuItem::with_id(app, "show", "显示 Hydrogen Music", true, None::<&str>)?;
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
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
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
                    let _ = window.emit("app-exit-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            select_local_folder, scan_local_music, read_cover, read_lyrics,
            audio_load, audio_play, audio_pause, audio_seek, audio_set_volume, audio_status, audio_stop,
            media_set_metadata, media_set_playback, media_set_volume,
            list_system_fonts, open_project_page, reveal_music_file,
            get_settings, set_settings, get_last_playlist, save_last_playlist, report_frontend_error, quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Hydrogen Music");
}
