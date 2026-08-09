use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::RwLock;
use tauri::{AppHandle, State};

use crate::{storage, write_json_async, PersistenceState};

pub const MAX_COLLECTIONS_BYTES: usize = 16 * 1024 * 1024;
const MAX_PLAYLISTS: usize = 1_000;
const MAX_TRACKS_PER_PLAYLIST: usize = 100_000;
const MAX_TRACK_ID_CHARS: usize = 256;
const MAX_PLAYLIST_NAME_CHARS: usize = 80;
const MAX_PLAYLIST_ID_CHARS: usize = 64;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Playlist {
    id: String,
    name: String,
    track_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collections {
    version: u8,
    next_playlist_id: u64,
    favorite_track_ids: Vec<String>,
    playlists: Vec<Playlist>,
}

impl Default for Collections {
    fn default() -> Self {
        Self {
            version: 1,
            next_playlist_id: 1,
            favorite_track_ids: Vec::new(),
            playlists: Vec::new(),
        }
    }
}

pub struct CollectionsState(pub RwLock<Collections>);

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn sanitize_track_ids(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter_map(|value| {
            let id = bounded(value.trim(), MAX_TRACK_ID_CHARS);
            if id.is_empty() || !seen.insert(id.clone()) {
                None
            } else {
                Some(id)
            }
        })
        .take(MAX_TRACKS_PER_PLAYLIST)
        .collect()
}

fn playlist_number(id: &str) -> Option<u64> {
    id.strip_prefix("playlist:")?
        .parse()
        .ok()
        .filter(|id| *id > 0)
}

pub fn sanitize(value: Value) -> Collections {
    let requested = serde_json::from_value::<Collections>(value).unwrap_or_default();
    let mut used_ids = HashSet::new();
    let mut highest_id = 0;
    let playlists = requested
        .playlists
        .into_iter()
        .filter_map(|playlist| {
            let id = bounded(playlist.id.trim(), MAX_PLAYLIST_ID_CHARS);
            let number = playlist_number(&id)?;
            if !used_ids.insert(id.clone()) {
                return None;
            }
            highest_id = highest_id.max(number);
            let name = bounded(playlist.name.trim(), MAX_PLAYLIST_NAME_CHARS);
            Some(Playlist {
                id,
                name: if name.is_empty() {
                    format!("歌单 {number}")
                } else {
                    name
                },
                track_ids: sanitize_track_ids(playlist.track_ids),
            })
        })
        .take(MAX_PLAYLISTS)
        .collect();

    Collections {
        version: 1,
        next_playlist_id: requested
            .next_playlist_id
            .max(highest_id.saturating_add(1))
            .max(1),
        favorite_track_ids: sanitize_track_ids(requested.favorite_track_ids),
        playlists,
    }
}

pub fn load(app: &AppHandle) -> Result<Collections, String> {
    let raw = storage::read_json_with_limit(
        app,
        "collections.json",
        serde_json::to_value(Collections::default()).map_err(|error| error.to_string())?,
        MAX_COLLECTIONS_BYTES as u64,
    )?;
    let collections = sanitize(raw);
    storage::write_json(
        app,
        "collections.json",
        &serde_json::to_value(&collections).map_err(|error| error.to_string())?,
    )?;
    Ok(collections)
}

#[tauri::command]
pub fn get_collections(state: State<'_, CollectionsState>) -> Result<Collections, String> {
    state
        .0
        .read()
        .map(|value| value.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn save_collections(
    app: AppHandle,
    state: State<'_, CollectionsState>,
    persistence: State<'_, PersistenceState>,
    collections: String,
) -> Result<Collections, String> {
    if collections.len() > MAX_COLLECTIONS_BYTES {
        return Err("collections payload is too large".to_string());
    }
    let raw: Value = serde_json::from_str(&collections).map_err(|error| error.to_string())?;
    let value = sanitize(raw);
    *state.0.write().map_err(|error| error.to_string())? = value.clone();
    write_json_async(
        app,
        persistence.0.clone(),
        "collections.json",
        serde_json::to_value(&value).map_err(|error| error.to_string())?,
    )
    .await?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::sanitize;
    use serde_json::json;

    #[test]
    fn preserves_playlist_order_and_deduplicates_track_ids() {
        let value = sanitize(json!({
            "version": 9,
            "nextPlaylistId": 1,
            "favoriteTrackIds": ["track:a", "track:a", "track:b"],
            "playlists": [
                {"id": "playlist:4", "name": "  驾车  ", "trackIds": ["track:b", "track:a", "track:b"]},
                {"id": "playlist:2", "name": "安静", "trackIds": ["track:c"]}
            ]
        }));
        let encoded = serde_json::to_value(value).unwrap();

        assert_eq!(encoded["version"], 1);
        assert_eq!(encoded["nextPlaylistId"], 5);
        assert_eq!(encoded["favoriteTrackIds"], json!(["track:a", "track:b"]));
        assert_eq!(encoded["playlists"][0]["name"], "驾车");
        assert_eq!(
            encoded["playlists"][0]["trackIds"],
            json!(["track:b", "track:a"])
        );
        assert_eq!(encoded["playlists"][1]["id"], "playlist:2");
    }
}
