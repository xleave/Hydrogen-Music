use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

fn data_file(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join(name))
}

pub fn read_json(app: &AppHandle, name: &str, default: Value) -> Result<Value, String> {
    let path = data_file(app, name)?;
    if !path.is_file() {
        write_json(app, name, &default)?;
        return Ok(default);
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

pub fn read_optional_json(app: &AppHandle, name: &str) -> Result<Option<Value>, String> {
    let path = data_file(app, name)?;
    if !path.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub fn write_json(app: &AppHandle, name: &str, value: &Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(data_file(app, name)?, content).map_err(|error| error.to_string())
}

pub fn default_settings() -> Value {
    json!({
        "music": {
            "level": "lossless",
            "lyricSize": "20",
            "tlyricSize": "14",
            "rlyricSize": "12",
            "lyricInterlude": 13
        },
        "local": {
            "videoFolder": null,
            "downloadFolder": null,
            "localFolder": []
        },
        "shortcuts": [
            { "id": "play", "name": "播放/暂停", "shortcut": "CommandOrControl+P", "globalShortcut": "CommandOrControl+Alt+P" },
            { "id": "last", "name": "上一首", "shortcut": "CommandOrControl+Left", "globalShortcut": "CommandOrControl+Alt+Left" },
            { "id": "next", "name": "下一首", "shortcut": "CommandOrControl+Right", "globalShortcut": "CommandOrControl+Alt+Right" },
            { "id": "volumeUp", "name": "增加音量", "shortcut": "CommandOrControl+Up", "globalShortcut": "CommandOrControl+Alt+Up" },
            { "id": "volumeDown", "name": "减少音量", "shortcut": "CommandOrControl+Down", "globalShortcut": "CommandOrControl+Alt+Down" },
            { "id": "processForward", "name": "快进(3s)", "shortcut": "CommandOrControl+]", "globalShortcut": "CommandOrControl+Alt+]" },
            { "id": "processBack", "name": "后退(3s)", "shortcut": "CommandOrControl+[", "globalShortcut": "CommandOrControl+Alt+[" }
        ],
        "other": {
            "globalShortcuts": false,
            "quitApp": "quit",
            "customFont": ""
        }
    })
}

