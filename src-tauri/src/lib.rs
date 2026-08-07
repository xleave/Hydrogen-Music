mod audio;
mod library;
mod media;
mod storage;

use base64::{engine::general_purpose::STANDARD, Engine};
use lofty::{file::TaggedFileExt, read_from_path, tag::ItemKey};
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::RwLock,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_dialog::DialogExt;

const MAX_COVER_BYTES: usize = 16 * 1024 * 1024;
const MAX_LYRICS_BYTES: u64 = 2 * 1024 * 1024;
const MAX_FRONTEND_LOG_BYTES: u64 = 2 * 1024 * 1024;

struct SettingsState(RwLock<Value>);

fn configured_music_folders_from_value(settings: &Value) -> Vec<PathBuf> {
    settings
        .get("local")
        .and_then(|local| local.get("localFolder"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|folder| {
            let folder = PathBuf::from(folder);
            let metadata = std::fs::symlink_metadata(&folder).ok()?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return None;
            }
            Some(folder)
        })
        .collect()
}

fn configured_music_folders(settings: &SettingsState) -> Result<Vec<PathBuf>, String> {
    let settings = settings.0.read().map_err(|error| error.to_string())?;
    Ok(configured_music_folders_from_value(&settings))
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

fn authorized_file_path(settings: &SettingsState, file_path: &str) -> Result<PathBuf, String> {
    let file_path = std::fs::canonicalize(file_path).map_err(|error| error.to_string())?;
    if !file_path.is_file() {
        return Err("path is not a file".to_string());
    }
    let folders = configured_music_folders(settings)?;
    if !folders.iter().any(|folder| file_path.starts_with(folder)) {
        return Err("file is outside the configured music folders".to_string());
    }
    Ok(file_path)
}

#[tauri::command]
async fn select_local_folder(
    app: AppHandle,
    state: State<'_, SettingsState>,
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

    let mut current = state.0.write().map_err(|error| error.to_string())?;
    let mut next = current.clone();
    let mut folders = configured_music_folders_from_value(&next);
    if !folders.contains(&selected) {
        folders.push(selected.clone());
    }
    replace_local_folders(&mut next, &folders)?;
    storage::write_json(&app, "settings.json", &next)?;
    *current = next;
    Ok(Some(selected.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn scan_local_music(settings: State<'_, SettingsState>) -> Result<library::ScanResult, String> {
    let folders = configured_music_folders(&settings)?;
    tauri::async_runtime::spawn_blocking(move || library::scan(&folders))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
fn read_cover(
    settings: State<'_, SettingsState>,
    file_path: String,
) -> Result<Option<String>, String> {
    let file_path = authorized_file_path(&settings, &file_path)?;
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

#[tauri::command]
fn read_lyrics(
    settings: State<'_, SettingsState>,
    file_path: String,
) -> Result<Option<String>, String> {
    let audio_path = authorized_file_path(&settings, &file_path)?;
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
    let lrc_path = authorized_file_path(&settings, &lrc_path.to_string_lossy())?;
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
fn audio_load(
    settings: State<'_, SettingsState>,
    audio: State<'_, audio::AudioState>,
    file_path: String,
    autoplay: bool,
    volume: f32,
    request_id: u64,
) -> Result<audio::AudioStatus, String> {
    let path = authorized_file_path(&settings, &file_path)?;
    audio.load(&path, autoplay, volume, request_id)
}

#[tauri::command]
fn audio_play(audio: State<'_, audio::AudioState>) -> Result<audio::AudioStatus, String> { audio.play() }
#[tauri::command]
fn audio_pause(audio: State<'_, audio::AudioState>) -> Result<audio::AudioStatus, String> { audio.pause() }
#[tauri::command]
fn audio_seek(audio: State<'_, audio::AudioState>, position: f64) -> Result<audio::AudioStatus, String> { audio.seek(position) }
#[tauri::command]
fn audio_set_volume(audio: State<'_, audio::AudioState>, volume: f32) -> Result<(), String> { audio.set_volume(volume) }
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
    media.set_metadata(&title, &artist, &album, duration)
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
fn media_set_volume(media: State<'_, media::MediaState>, volume: f64) -> Result<(), String> { media.set_volume(volume) }

#[tauri::command]
fn get_settings(settings: State<'_, SettingsState>) -> Result<Value, String> {
    settings.0.read().map(|value| value.clone()).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_settings(app: AppHandle, state: State<'_, SettingsState>, settings: String) -> Result<(), String> {
    let mut value: Value = serde_json::from_str(&settings).map_err(|error| error.to_string())?;
    let mut current = state.0.write().map_err(|error| error.to_string())?;
    let authorized = configured_music_folders_from_value(&current);
    let requested = configured_music_folders_from_value(&value);
    let retained: Vec<PathBuf> = requested.into_iter().filter(|folder| authorized.contains(folder)).collect();
    replace_local_folders(&mut value, &retained)?;
    storage::write_json(&app, "settings.json", &value)?;
    *current = value;
    Ok(())
}

#[tauri::command]
fn report_frontend_error(app: AppHandle, source: String, detail: String) -> Result<(), String> {
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
fn get_last_playlist(app: AppHandle) -> Result<Option<Value>, String> { storage::read_optional_json(&app, "last-playlist.json") }
#[tauri::command]
fn save_last_playlist(app: AppHandle, playlist: String) -> Result<(), String> {
    let value = serde_json::from_str(&playlist).map_err(|error| error.to_string())?;
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let settings = storage::read_json(app.handle(), "settings.json", storage::default_settings()).map_err(std::io::Error::other)?;
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
            get_settings, set_settings, get_last_playlist, save_last_playlist, report_frontend_error, quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Hydrogen Music");
}
