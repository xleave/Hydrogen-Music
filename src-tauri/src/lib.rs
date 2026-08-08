mod audio;
mod library;
mod library_snapshot;
mod media;
mod storage;

use base64::{engine::general_purpose::STANDARD, Engine};
use lofty::{file::TaggedFileExt, read_from_path, tag::ItemKey};
use serde::Deserialize;
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

const MAX_COVER_BYTES: usize = 16 * 1024 * 1024;
const MAX_COVER_DIMENSION: u32 = 8192;
const MAX_COVER_PIXELS: u64 = 32 * 1024 * 1024;
const MAX_LYRICS_BYTES: u64 = 2 * 1024 * 1024;
const MAX_FRONTEND_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_CRASH_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SETTINGS_BYTES: usize = 1024 * 1024;
const MAX_PLAYLIST_BYTES: usize = 32 * 1024 * 1024;
const MAX_PLAYLIST_TRACKS: usize = 100_000;
const MAX_TEXT_CHARS: usize = 4096;
const MAX_PATH_CHARS: usize = 16_384;
const MAX_AUDIO_SECONDS: f64 = 7.0 * 24.0 * 60.0 * 60.0;
const EXIT_FALLBACK_DELAY: Duration = Duration::from_millis(1500);
const PROJECT_URL: &str = "https://github.com/xleave/Hydrogen-Music";

struct SettingsState(RwLock<Value>);
struct ScanState(Arc<AtomicU64>);

#[derive(Clone, Default)]
struct PersistenceState(Arc<Mutex<()>>);

#[derive(Clone, Default)]
struct PlaybackSnapshotState(Arc<RwLock<Option<Value>>>);

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

fn authorized_file_path_from_folders(
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

fn finite(value: f64, name: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{name} must be finite"))
    }
}

fn finite_f32(value: f32, name: &str) -> Result<f32, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{name} must be finite"))
    }
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
                object
                    .get("shortcut")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
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

fn playlist_ids(
    object: &serde_json::Map<String, Value>,
    ids_name: &str,
    legacy_name: &str,
) -> Vec<Value> {
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
    let object = value
        .as_object()
        .ok_or_else(|| "playlist must be an object".to_string())?;
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
    let play_mode = object
        .get("playMode")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(3);

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

async fn write_json_async(
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
    storage::write_json(app, "last-playlist.json", &value)
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

    let mut next = state
        .0
        .read()
        .map_err(|error| error.to_string())?
        .clone();
    let mut folders = stored_music_folders_from_value(&next);
    if !folders.contains(&selected) {
        folders.push(selected.clone());
    }
    replace_local_folders(&mut next, &folders)?;
    write_json_async(
        app,
        persistence.0.clone(),
        "settings.json",
        next.clone(),
    )
    .await?;
    *state.0.write().map_err(|error| error.to_string())? = next;
    Ok(Some(selected.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn get_cached_library(
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
    tauri::async_runtime::spawn_blocking(move || library_snapshot::load(&folders))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn scan_local_music(
    settings: State<'_, SettingsState>,
    scan: State<'_, ScanState>,
    request_id: u64,
) -> Result<library::ScanResult, String> {
    let stored = settings
        .0
        .read()
        .map_err(|error| error.to_string())
        .map(|value| stored_music_folders_from_value(&value))?;
    let folders = configured_music_folders(&settings)?;
    let may_snapshot = folders.len() == stored.len();
    let latest = scan.0.clone();
    let previous = latest.fetch_max(request_id, Ordering::AcqRel);
    if previous > request_id {
        return Err(library::STALE_SCAN.to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let result = library::scan(&folders, request_id, &latest)?;
        if may_snapshot {
            if let Err(error) = library_snapshot::save(&folders, &result) {
                eprintln!("[library snapshot] failed to persist cache: {error}");
            }
        }
        Ok(result)
    })
    .await
    .map_err(|error| error.to_string())?
}

fn cover_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") && data.len() >= 24 {
        return Some((
            u32::from_be_bytes(data[16..20].try_into().ok()?),
            u32::from_be_bytes(data[20..24].try_into().ok()?),
        ));
    }
    if data.starts_with(b"GIF8") && data.len() >= 10 {
        return Some((
            u16::from_le_bytes(data[6..8].try_into().ok()?) as u32,
            u16::from_le_bytes(data[8..10].try_into().ok()?) as u32,
        ));
    }
    if data.starts_with(b"BM") && data.len() >= 26 {
        let width = i32::from_le_bytes(data[18..22].try_into().ok()?).unsigned_abs();
        let height = i32::from_le_bytes(data[22..26].try_into().ok()?).unsigned_abs();
        return Some((width, height));
    }
    if data.starts_with(b"\xff\xd8\xff") {
        let mut cursor = 2usize;
        while cursor + 4 <= data.len() {
            if data[cursor] != 0xff {
                cursor += 1;
                continue;
            }
            while cursor < data.len() && data[cursor] == 0xff {
                cursor += 1;
            }
            if cursor >= data.len() {
                break;
            }
            let marker = data[cursor];
            cursor += 1;
            if marker == 0xd8 || marker == 0xd9 || marker == 0x01 {
                continue;
            }
            if cursor + 2 > data.len() {
                break;
            }
            let segment_len = u16::from_be_bytes(data[cursor..cursor + 2].try_into().ok()?) as usize;
            if segment_len < 2 || cursor + segment_len > data.len() {
                break;
            }
            if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf)
                && segment_len >= 7
            {
                let height = u16::from_be_bytes(data[cursor + 3..cursor + 5].try_into().ok()?) as u32;
                let width = u16::from_be_bytes(data[cursor + 5..cursor + 7].try_into().ok()?) as u32;
                return Some((width, height));
            }
            cursor += segment_len;
        }
    }
    None
}

fn validate_cover_dimensions(data: &[u8]) -> Result<(), String> {
    let Some((width, height)) = cover_dimensions(data) else {
        return Ok(());
    };
    let pixels = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > MAX_COVER_DIMENSION
        || height > MAX_COVER_DIMENSION
        || pixels > MAX_COVER_PIXELS
    {
        return Err("embedded cover dimensions are too large".to_string());
    }
    Ok(())
}

fn read_cover_blocking(
    folders: Vec<PathBuf>,
    file_path: String,
) -> Result<Option<String>, String> {
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
    validate_cover_dimensions(picture.data())?;
    let mime = cover_mime(picture.data());
    Ok(Some(format!(
        "data:{mime};base64,{}",
        STANDARD.encode(picture.data())
    )))
}

#[tauri::command]
async fn read_cover(
    settings: State<'_, SettingsState>,
    file_path: String,
) -> Result<Option<String>, String> {
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
async fn audio_load(
    settings: State<'_, SettingsState>,
    audio: State<'_, audio::AudioState>,
    file_path: String,
    autoplay: bool,
    volume: f32,
) -> Result<audio::AudioStatus, String> {
    let volume = finite_f32(volume, "volume")?.clamp(0.0, 1.0);
    let folders = configured_music_folders(&settings)?;
    let audio = audio.inner().clone();
    // Reserve on the command thread before entering the blocking pool. Task
    // scheduling order can no longer make an older decode supersede a newer one.
    let generation = audio.reserve_load();
    tauri::async_runtime::spawn_blocking(move || {
        let path = authorized_file_path_from_folders(&folders, &file_path)?;
        audio.load_reserved(&path, autoplay, volume, generation)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn audio_play(audio: State<'_, audio::AudioState>) -> Result<audio::AudioStatus, String> {
    audio.play()
}

#[tauri::command]
fn audio_pause(audio: State<'_, audio::AudioState>) -> Result<audio::AudioStatus, String> {
    audio.pause()
}

#[tauri::command]
fn audio_seek(
    audio: State<'_, audio::AudioState>,
    position: f64,
) -> Result<audio::AudioStatus, String> {
    audio.seek(finite(position, "position")?.clamp(0.0, MAX_AUDIO_SECONDS))
}

#[tauri::command]
fn audio_set_volume(audio: State<'_, audio::AudioState>, volume: f32) -> Result<(), String> {
    audio.set_volume(finite_f32(volume, "volume")?.clamp(0.0, 1.0))
}

#[tauri::command]
fn audio_status(audio: State<'_, audio::AudioState>) -> Result<audio::AudioStatus, String> {
    audio.status()
}

#[tauri::command]
fn audio_stop(audio: State<'_, audio::AudioState>) -> Result<(), String> {
    audio.stop()
}

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
    if audio.status().is_err() {
        return media.set_stopped();
    }
    media.set_playback(playing, audio.position())
}

#[tauri::command]
fn media_set_stopped(media: State<'_, media::MediaState>) -> Result<(), String> {
    media.set_stopped()
}

#[tauri::command]
fn media_clear(media: State<'_, media::MediaState>) -> Result<(), String> {
    media.clear()
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
    write_json_async(
        app,
        persistence.0.clone(),
        "settings.json",
        value.clone(),
    )
    .await?;
    *state.0.write().map_err(|error| error.to_string())? = value;
    Ok(())
}

#[tauri::command]
async fn report_frontend_error(
    app: AppHandle,
    source: String,
    detail: String,
) -> Result<(), String> {
    let source = bounded_text(&source, 256)
        .replace('\r', " ")
        .replace('\n', " ");
    let detail = bounded_text(&detail, 64 * 1024);
    tauri::async_runtime::spawn_blocking(move || {
        let directory = app.path().app_log_dir().map_err(|error| error.to_string())?;
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
fn get_last_playlist(
    snapshot: State<'_, PlaybackSnapshotState>,
) -> Result<Option<Value>, String> {
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
    let value = sanitize_playlist(&raw)?;
    *snapshot.0.write().map_err(|error| error.to_string())? = Some(value.clone());
    write_json_async(
        app,
        persistence.0.clone(),
        "last-playlist.json",
        value,
    )
    .await
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
        manager.unregister_all().map_err(|error| error.to_string())?;
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
    let builder = tauri::Builder::default()
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

            let initial_playlist = storage::read_optional_json_with_limit(
                app.handle(),
                "last-playlist.json",
                MAX_PLAYLIST_BYTES as u64,
            )
            .map_err(std::io::Error::other)?
            .and_then(|value| sanitize_playlist(&value).ok());
            app.manage(PlaybackSnapshotState(Arc::new(RwLock::new(initial_playlist))));

            // MPRIS is an optional Linux desktop integration. D-Bus failure must
            // never make the native audio player itself fail to start.
            app.manage(media::MediaState::new(app.handle()));

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
            audio_load,
            audio_play,
            audio_pause,
            audio_seek,
            audio_set_volume,
            audio_status,
            audio_stop,
            media_set_metadata,
            media_set_playback,
            media_set_stopped,
            media_clear,
            media_set_volume,
            list_system_fonts,
            open_project_page,
            reveal_music_file,
            get_settings,
            set_settings,
            get_last_playlist,
            save_last_playlist,
            report_frontend_error,
            register_shortcuts,
            unregister_shortcuts,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Hydrogen Music");
}
