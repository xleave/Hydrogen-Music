mod library;
mod storage;

use base64::{engine::general_purpose::STANDARD, Engine};
use lofty::{
    file::TaggedFileExt,
    picture::MimeType,
    read_from_path,
    tag::ItemKey,
};
use serde_json::Value;
use std::path::Path;
use tauri::AppHandle;

#[tauri::command]
fn scan_local_music(folders: Vec<String>) -> Result<library::ScanResult, String> {
    library::scan(&folders)
}

#[tauri::command]
fn read_cover(file_path: String) -> Result<Option<String>, String> {
    let Ok(tagged) = read_from_path(&file_path) else {
        return Ok(None);
    };
    let picture = tagged.tags().iter().find_map(|tag| tag.pictures().first());
    let Some(picture) = picture else {
        return Ok(None);
    };
    let mime = match picture.mime_type() {
        Some(MimeType::Png) => "image/png",
        Some(MimeType::Jpeg) => "image/jpeg",
        Some(MimeType::Tiff) => "image/tiff",
        Some(MimeType::Bmp) => "image/bmp",
        Some(MimeType::Gif) => "image/gif",
        _ => "application/octet-stream",
    };
    Ok(Some(format!(
        "data:{mime};base64,{}",
        STANDARD.encode(picture.data())
    )))
}

#[tauri::command]
fn read_lyrics(file_path: String) -> Result<Option<String>, String> {
    if let Ok(tagged) = read_from_path(&file_path) {
        if let Some(lyrics) = tagged
            .tags()
            .iter()
            .find_map(|tag| {
                tag.get_string(ItemKey::Lyrics)
                    .or_else(|| tag.get_string(ItemKey::UnsyncLyrics))
            })
        {
            if !lyrics.trim().is_empty() {
                return Ok(Some(lyrics.to_owned()));
            }
        }
    }

    let audio_path = Path::new(&file_path);
    let lrc_path = audio_path.with_extension("lrc");
    if !lrc_path.is_file() {
        return Ok(None);
    }
    std::fs::read_to_string(lrc_path)
        .map(Some)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<Value, String> {
    storage::read_json(&app, "settings.json", storage::default_settings())
}

#[tauri::command]
fn set_settings(app: AppHandle, settings: String) -> Result<(), String> {
    let value = serde_json::from_str(&settings).map_err(|error| error.to_string())?;
    storage::write_json(&app, "settings.json", &value)
}

#[tauri::command]
fn get_last_playlist(app: AppHandle) -> Result<Option<Value>, String> {
    storage::read_optional_json(&app, "last-playlist.json")
}

#[tauri::command]
fn save_last_playlist(app: AppHandle, playlist: String) -> Result<(), String> {
    let value = serde_json::from_str(&playlist).map_err(|error| error.to_string())?;
    storage::write_json(&app, "last-playlist.json", &value)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan_local_music,
            read_cover,
            read_lyrics,
            get_settings,
            set_settings,
            get_last_playlist,
            save_last_playlist,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Hydrogen Music");
}
