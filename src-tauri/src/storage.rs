use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

fn data_file(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let directory = app.path().app_data_dir().map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join(name))
}

fn quarantine_corrupt(path: &Path) {
    let corrupt = path.with_extension("corrupt.json");
    let _ = fs::remove_file(&corrupt);
    let _ = fs::rename(path, corrupt);
}

pub fn read_json(app: &AppHandle, name: &str, default: Value) -> Result<Value, String> {
    let path = data_file(app, name)?;
    if !path.is_file() {
        write_json(app, name, &default)?;
        return Ok(default);
    }
    let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    match serde_json::from_str(&content) {
        Ok(value) => Ok(value),
        Err(_) => {
            quarantine_corrupt(&path);
            write_json(app, name, &default)?;
            Ok(default)
        }
    }
}

pub fn read_optional_json(app: &AppHandle, name: &str) -> Result<Option<Value>, String> {
    let path = data_file(app, name)?;
    if !path.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    match serde_json::from_str(&content) {
        Ok(value) => Ok(Some(value)),
        Err(_) => {
            quarantine_corrupt(&path);
            Ok(None)
        }
    }
}

pub fn write_json(app: &AppHandle, name: &str, value: &Value) -> Result<(), String> {
    let path = data_file(app, name)?;
    let content = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let tmp = path.with_extension("tmp");
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)
            .map_err(|error| error.to_string())?;
        file.write_all(&content).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
    }
    if cfg!(windows) && path.exists() {
        fs::remove_file(&path).map_err(|error| error.to_string())?;
    }
    fs::rename(&tmp, &path).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        if let Ok(directory) = OpenOptions::new().read(true).open(parent) {
            let _ = directory.sync_all();
        }
    }
    Ok(())
}

pub fn default_settings() -> Value {
    json!({
        "music": {
            "lyricSize": 20,
            "tlyricSize": 14,
            "rlyricSize": 12,
            "lyricInterlude": 13
        },
        "local": {
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
            "quitApp": "minimize",
            "customFont": ""
        }
    })
}
