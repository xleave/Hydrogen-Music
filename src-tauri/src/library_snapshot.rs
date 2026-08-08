use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_SNAPSHOT_BYTES: u64 = 128 * 1024 * 1024;
const SNAPSHOT_VERSION: u64 = 1;

fn cache_directory() -> Option<PathBuf> {
    if let Some(cache_home) = std::env::var_os("XDG_CACHE_HOME") {
        if !cache_home.is_empty() {
            return Some(PathBuf::from(cache_home).join("hydrogen-music"));
        }
    }
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .map(|home| home.join(".cache/hydrogen-music"))
}

fn snapshot_path() -> Option<PathBuf> {
    cache_directory().map(|directory| directory.join("library-snapshot.json"))
}

fn normalized_roots(roots: &[PathBuf]) -> Vec<String> {
    let mut roots: Vec<String> = roots
        .iter()
        .map(|root| root.to_string_lossy().into_owned())
        .collect();
    roots.sort();
    roots.dedup();
    roots
}

fn quarantine(path: &Path) {
    let corrupt = path.with_extension("corrupt.json");
    let _ = fs::remove_file(&corrupt);
    let _ = fs::rename(path, corrupt);
}

pub fn load(roots: &[PathBuf]) -> Result<Option<Value>, String> {
    let Some(path) = snapshot_path() else {
        return Ok(None);
    };
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if metadata.len() > MAX_SNAPSHOT_BYTES {
        quarantine(&path);
        return Ok(None);
    }

    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => {
            quarantine(&path);
            return Ok(None);
        }
    };
    if value.get("version").and_then(Value::as_u64) != Some(SNAPSHOT_VERSION) {
        return Ok(None);
    }
    let stored_roots = value
        .get("roots")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if stored_roots != normalized_roots(roots) {
        return Ok(None);
    }
    Ok(value.get("result").cloned())
}

pub fn save<T: Serialize>(roots: &[PathBuf], result: &T) -> Result<(), String> {
    let result = serde_json::to_value(result).map_err(|error| error.to_string())?;
    if result
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        return Ok(());
    }

    let Some(path) = snapshot_path() else {
        return Ok(());
    };
    let parent = path
        .parent()
        .ok_or_else(|| "invalid library snapshot path".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let payload = json!({
        "version": SNAPSHOT_VERSION,
        "roots": normalized_roots(roots),
        "result": result,
    });
    let bytes = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
        return Ok(());
    }

    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if cfg!(windows) && path.exists() {
        let _ = fs::remove_file(&path);
    }
    fs::rename(&temporary, &path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_signatures_ignore_order() {
        let first = vec![PathBuf::from("/b"), PathBuf::from("/a")];
        let second = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        assert_eq!(normalized_roots(&first), normalized_roots(&second));
    }
}
