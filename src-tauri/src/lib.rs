mod library;
mod storage;

use base64::{engine::general_purpose::STANDARD, Engine};
use lofty::{
    file::TaggedFileExt,
    read_from_path,
    tag::ItemKey,
};
use serde_json::Value;
use std::path::Path;
use tauri::{
    AppHandle, Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

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
    let mime = cover_mime(picture.data());
    Ok(Some(format!(
        "data:{mime};base64,{}",
        STANDARD.encode(picture.data())
    )))
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

/// 判断当前设置是否为"最小化到托盘"模式
fn is_minimize_to_tray(app: &AppHandle) -> bool {
    storage::read_json(app, "settings.json", storage::default_settings())
        .ok()
        .and_then(|v| v.get("other").and_then(|o| o.get("quitApp")).and_then(|q| q.as_str()).map(|s| s == "minimize"))
        .unwrap_or(true) // 默认最小化到托盘
}

/// 显示主窗口（从托盘恢复）
fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 真正退出应用（保存播放列表后退出）
#[tauri::command]
async fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 创建托盘右键菜单
            let show_item = MenuItem::with_id(app, "show", "显示 Hydrogen Music", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // 创建托盘图标
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Hydrogen Music")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    // 双击或单击托盘图标 → 显示窗口
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
            // 拦截窗口关闭请求
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if is_minimize_to_tray(app) {
                    // 最小化到托盘：隐藏窗口，阻止真正关闭
                    api.prevent_close();
                    let _ = window.hide();
                    // 通知前端保存播放列表
                    let _ = window.emit("tray-hide", ());
                }
                // 否则正常关闭（直接退出）
            }
        })
        .invoke_handler(tauri::generate_handler![
            scan_local_music,
            read_cover,
            read_lyrics,
            get_settings,
            set_settings,
            get_last_playlist,
            save_last_playlist,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Hydrogen Music");
}
