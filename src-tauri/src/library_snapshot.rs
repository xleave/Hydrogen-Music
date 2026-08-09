use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use crate::file_replace;

const MAX_SNAPSHOT_BYTES: u64 = 128 * 1024 * 1024;
const SNAPSHOT_VERSION: u64 = 2;

fn snapshot_path(cache_directory: &Path) -> PathBuf {
    cache_directory.join("library-snapshot.json")
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

pub fn load(cache_directory: &Path, roots: &[PathBuf]) -> Result<Option<Value>, String> {
    let path = snapshot_path(cache_directory);
    file_replace::recover_backup(&path)?;
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
    let result = value.get("result").cloned();
    if result
        .as_ref()
        .and_then(|result| result.get("complete"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Ok(None);
    }
    Ok(result)
}

pub fn save<T: Serialize>(
    cache_directory: &Path,
    roots: &[PathBuf],
    result: &T,
) -> Result<(), String> {
    let result = serde_json::to_value(result).map_err(|error| error.to_string())?;
    if result.get("complete").and_then(Value::as_bool) != Some(true)
        || result
            .get("truncated")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    {
        return Ok(());
    }

    let path = snapshot_path(cache_directory);
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
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(&bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
    }
    file_replace::replace(&temporary, &path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_CACHE: AtomicU64 = AtomicU64::new(0);

    fn temporary_cache_directory() -> PathBuf {
        let id = NEXT_CACHE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "hydrogen-music-library-snapshot-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn root_signatures_ignore_order() {
        let first = vec![PathBuf::from("/b"), PathBuf::from("/a")];
        let second = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        assert_eq!(normalized_roots(&first), normalized_roots(&second));
    }

    #[test]
    fn snapshot_round_trip_uses_the_supplied_cache_directory() {
        let cache_directory = temporary_cache_directory();
        let roots = vec![PathBuf::from("/music")];
        let result = json!({
            "complete": true,
            "truncated": false,
            "count": 0,
            "dirTree": [],
            "locaFilesMetadata": [],
        });

        save(&cache_directory, &roots, &result).unwrap();

        assert_eq!(load(&cache_directory, &roots).unwrap(), Some(result));
        assert!(cache_directory.join("library-snapshot.json").is_file());
        fs::remove_dir_all(cache_directory).unwrap();
    }
}
