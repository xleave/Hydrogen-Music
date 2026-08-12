use serde_json::{json, Value};

const MAX_TRACKS: usize = 100_000;
const MAX_ID_CHARS: usize = 16_384;
pub const MAX_AUDIO_SECONDS: f64 = 7.0 * 24.0 * 60.0 * 60.0;

fn bounded(value: &str) -> String {
    value.chars().take(MAX_ID_CHARS).collect()
}

fn number(value: Option<&Value>, default: f64, minimum: f64, maximum: f64) -> f64 {
    value
        .and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        })
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(minimum, maximum))
        .unwrap_or(default)
}

fn ids(object: &serde_json::Map<String, Value>, ids_name: &str, legacy_name: &str) -> Vec<Value> {
    if let Some(ids) = object.get(ids_name).and_then(Value::as_array) {
        return ids
            .iter()
            .filter_map(Value::as_str)
            .take(MAX_TRACKS)
            .map(|id| Value::String(bounded(id)))
            .collect();
    }
    object
        .get(legacy_name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|track| track.get("id").and_then(Value::as_str))
        .take(MAX_TRACKS)
        .map(|id| Value::String(bounded(id)))
        .collect()
}

pub fn sanitize(value: &Value) -> Result<Value, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "playlist must be an object".to_string())?;
    let song_ids = ids(object, "songIds", "songList");
    let shuffled_song_ids = ids(object, "shuffledSongIds", "shuffledList");
    let current_song_id = object
        .get("currentSongId")
        .and_then(Value::as_str)
        .map(bounded);
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
    let progress = number(object.get("progress"), 0.0, 0.0, MAX_AUDIO_SECONDS);
    let volume = number(object.get("volume"), 0.3, 0.0, 1.0);
    let play_mode = object
        .get("playMode")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(3);
    let structure_revision = object
        .get("structureRevision")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    Ok(json!({
        "version": 3,
        "structureRevision": structure_revision,
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

pub fn sanitize_update(value: &Value, current: Option<&Value>) -> Result<Value, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "playlist must be an object".to_string())?;
    let has_structure = object.contains_key("songIds") || object.contains_key("songList");
    if has_structure || object.get("version").and_then(Value::as_u64) != Some(4) {
        return sanitize(value);
    }

    let mut merged = current
        .cloned()
        .ok_or_else(|| "playlist structure is required for the first checkpoint".to_string())?;
    let current_revision = merged
        .get("structureRevision")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let requested_revision = object
        .get("structureRevision")
        .and_then(Value::as_u64)
        .unwrap_or(current_revision);
    if requested_revision != current_revision {
        return Err("playlist structure revision is stale".to_string());
    }
    let merged_object = merged
        .as_object_mut()
        .ok_or_else(|| "stored playlist must be an object".to_string())?;
    for key in [
        "currentSongId",
        "currentIndex",
        "shuffleIndex",
        "progress",
        "volume",
        "playMode",
    ] {
        if let Some(next) = object.get(key) {
            merged_object.insert(key.to_string(), next.clone());
        }
    }
    sanitize(&merged)
}

pub fn scalar_state(value: &Value) -> Value {
    json!({
        "version": 4,
        "structureRevision": value.get("structureRevision").and_then(Value::as_u64).unwrap_or(0),
        "currentSongId": value.get("currentSongId").cloned().unwrap_or(Value::Null),
        "currentIndex": value.get("currentIndex").cloned().unwrap_or_else(|| json!(0)),
        "shuffleIndex": value.get("shuffleIndex").cloned().unwrap_or_else(|| json!(0)),
        "progress": value.get("progress").cloned().unwrap_or_else(|| json!(0)),
        "volume": value.get("volume").cloned().unwrap_or_else(|| json!(0.3)),
        "playMode": value.get("playMode").cloned().unwrap_or_else(|| json!(0)),
    })
}

pub fn merge_stored_state(structure: Value, state: Option<&Value>) -> Value {
    let Some(state) = state else {
        return structure;
    };
    sanitize_update(state, Some(&structure)).unwrap_or(structure)
}

#[cfg(test)]
mod tests {
    use super::{merge_stored_state, sanitize, sanitize_update, scalar_state};
    use serde_json::json;

    #[test]
    fn scalar_update_preserves_queue_structure() {
        let current = sanitize(&json!({
            "version": 3,
            "structureRevision": 8,
            "songIds": ["track:a", "track:b"],
            "shuffledSongIds": ["track:b", "track:a"],
            "currentSongId": "track:a",
            "currentIndex": 0,
            "shuffleIndex": 1,
            "progress": 10,
            "volume": 0.3,
            "playMode": 3,
        }))
        .unwrap();
        let updated = sanitize_update(
            &json!({
                "version": 4,
                "structureRevision": 8,
                "currentSongId": "track:b",
                "currentIndex": 1,
                "shuffleIndex": 0,
                "progress": 42.5,
                "volume": 0.6,
                "playMode": 1,
            }),
            Some(&current),
        )
        .unwrap();

        assert_eq!(updated["version"], 3);
        assert_eq!(updated["structureRevision"], 8);
        assert_eq!(updated["songIds"], json!(["track:a", "track:b"]));
        assert_eq!(updated["shuffledSongIds"], json!(["track:b", "track:a"]));
        assert_eq!(updated["currentSongId"], "track:b");
        assert_eq!(updated["progress"], 42.5);
    }

    #[test]
    fn first_checkpoint_requires_structure() {
        let error = sanitize_update(
            &json!({
                "version": 4,
                "structureRevision": 1,
                "progress": 1,
            }),
            None,
        )
        .unwrap_err();

        assert_eq!(
            error,
            "playlist structure is required for the first checkpoint"
        );
    }

    #[test]
    fn scalar_disk_state_excludes_queue_and_stale_state_is_ignored() {
        let structure = sanitize(&json!({
            "version": 3,
            "structureRevision": 12,
            "songIds": ["track:a", "track:b"],
            "shuffledSongIds": [],
            "currentSongId": "track:a",
            "currentIndex": 0,
            "progress": 5,
            "volume": 0.3,
            "playMode": 0,
        }))
        .unwrap();
        let state = scalar_state(&structure);

        assert!(state.get("songIds").is_none());
        assert!(state.get("shuffledSongIds").is_none());
        let stale = json!({
            "version": 4,
            "structureRevision": 11,
            "currentSongId": "track:b",
            "currentIndex": 1,
            "progress": 99,
            "volume": 1,
            "playMode": 1,
        });
        let restored = merge_stored_state(structure, Some(&stale));
        assert_eq!(restored["currentSongId"], "track:a");
        assert_eq!(restored["progress"], 5.0);
    }
}
