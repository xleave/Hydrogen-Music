use lofty::{
    file::{AudioFile, TaggedFileExt},
    read_from_path,
    tag::{Accessor, ItemKey},
};
use rayon::prelude::*;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};

pub const STALE_SCAN: &str = "stale music scan";
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aac", "m4a", "ogg", "opus", "wma", "ape", "alac", "aiff",
    "mp2", "mpc", "wv", "speex",
];
const MAX_SCAN_DEPTH: usize = 128;
const MAX_TRACKS: usize = 100_000;
const MAX_METADATA_CHARS: usize = 4096;
const MAX_ARTISTS: usize = 32;
const MAX_GENRES: usize = 32;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    dir_tree: Vec<Node>,
    loca_files_metadata: Vec<Node>,
    count: usize,
    truncated: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Node {
    name: String,
    dir_path: String,
    #[serde(rename = "type")]
    node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<Node>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    common: Option<CommonMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<FormatMetadata>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommonMetadata {
    local_title: String,
    file_url: String,
    title: String,
    artists: Vec<String>,
    album: String,
    albumartist: Option<String>,
    date: Option<String>,
    genre: Vec<String>,
    year: Option<u32>,
    has_lyrics: bool,
    modified_at: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FormatMetadata {
    bitrate: Option<u32>,
    bits_per_sample: Option<u8>,
    container: String,
    duration: f64,
    sample_rate: Option<u32>,
}

pub fn scan(
    folders: &[PathBuf],
    request_id: u64,
    latest_request_id: &AtomicU64,
) -> Result<ScanResult, String> {
    ensure_current(request_id, latest_request_id)?;
    let total_tracks = AtomicUsize::new(0);
    let roots: Result<Vec<Option<(Node, usize)>>, String> = folders
        .par_iter()
        .map(|folder| {
            ensure_current(request_id, latest_request_id)?;
            let root = match fs::canonicalize(folder) {
                Ok(root) => root,
                Err(error) => {
                    eprintln!("[library scan] 无法解析 {}: {error}", folder.display());
                    return Ok(None);
                }
            };
            if !root.is_dir() {
                eprintln!("[library scan] {} 不是目录", root.display());
                return Ok(None);
            }
            match scan_directory(
                &root,
                0,
                request_id,
                latest_request_id,
                &total_tracks,
            ) {
                Ok(result) => Ok(Some(result)),
                Err(error) if error == STALE_SCAN => Err(error),
                Err(error) => {
                    eprintln!("[library scan] {error}");
                    Ok(None)
                }
            }
        })
        .collect();

    ensure_current(request_id, latest_request_id)?;
    let results: Vec<(Node, usize)> = roots?.into_iter().flatten().collect();
    let count = results.iter().map(|(_, c)| c).sum();
    let metadata_roots: Vec<Node> = results.into_iter().map(|(n, _)| n).collect();
    let dir_tree = metadata_roots.iter().map(directory_only).collect();

    Ok(ScanResult {
        dir_tree,
        loca_files_metadata: metadata_roots,
        count,
        truncated: total_tracks.load(Ordering::Relaxed) >= MAX_TRACKS,
    })
}

fn ensure_current(request_id: u64, latest_request_id: &AtomicU64) -> Result<(), String> {
    if latest_request_id.load(Ordering::Acquire) != request_id {
        Err(STALE_SCAN.to_string())
    } else {
        Ok(())
    }
}

fn scan_directory(
    path: &Path,
    depth: usize,
    request_id: u64,
    latest_request_id: &AtomicU64,
    total_tracks: &AtomicUsize,
) -> Result<(Node, usize), String> {
    ensure_current(request_id, latest_request_id)?;
    if depth > MAX_SCAN_DEPTH {
        return Err(format!("目录层级过深: {}", path.display()));
    }

    let read_dir = fs::read_dir(path)
        .map_err(|e| format!("无法读取 {}: {e}", path.display()))?;
    let mut entries = Vec::new();
    for entry in read_dir {
        ensure_current(request_id, latest_request_id)?;
        if let Ok(entry) = entry {
            entries.push(entry);
        }
    }
    entries.sort_unstable_by(|a, b| a.file_name().cmp(&b.file_name()));

    let mut sub_dirs = Vec::new();
    let mut audio_paths = Vec::new();
    for entry in entries {
        ensure_current(request_id, latest_request_id)?;
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }

        let entry_path = entry.path();
        if file_type.is_dir() {
            sub_dirs.push(entry_path);
        } else if file_type.is_file() && is_audio_file(&entry_path) {
            if total_tracks.fetch_add(1, Ordering::Relaxed) >= MAX_TRACKS {
                total_tracks.fetch_sub(1, Ordering::Relaxed);
                break;
            }
            audio_paths.push(entry_path);
        }
    }

    let mut children: Vec<Node> = Vec::with_capacity(sub_dirs.len() + audio_paths.len());
    let mut count = 0usize;
    for sub in sub_dirs {
        ensure_current(request_id, latest_request_id)?;
        match scan_directory(
            &sub,
            depth + 1,
            request_id,
            latest_request_id,
            total_tracks,
        ) {
            Ok((node, child_count)) => {
                count += child_count;
                children.push(node);
            }
            Err(error) if error == STALE_SCAN => return Err(error),
            Err(error) => eprintln!("[library scan] {error}"),
        }
    }

    ensure_current(request_id, latest_request_id)?;
    let file_count = audio_paths.len();
    let file_nodes: Vec<Node> = audio_paths
        .par_iter()
        .filter_map(|audio_path| {
            if latest_request_id.load(Ordering::Acquire) != request_id {
                return None;
            }
            Some(read_track(audio_path).unwrap_or_else(|_| fallback_track(audio_path)))
        })
        .collect();

    ensure_current(request_id, latest_request_id)?;
    count += file_count;
    children.extend(file_nodes);

    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();
    let path_string = path.to_string_lossy().into_owned();

    Ok((
        Node {
            name,
            dir_path: path_string.clone(),
            node_type: "folder".into(),
            children: Some(children),
            id: Some(path_string),
            common: None,
            format: None,
        },
        count,
    ))
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn bounded_text(value: &str) -> String {
    value.chars().take(MAX_METADATA_CHARS).collect()
}

fn read_track(path: &Path) -> Result<Node, String> {
    let tagged = read_from_path(path)
        .map_err(|e| format!("无法解析 {}: {e}", path.display()))?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let local_title = bounded_text(
        &path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy(),
    );

    let title = tag
        .and_then(Accessor::title)
        .map(|v| bounded_text(&v))
        .unwrap_or_else(|| local_title.clone());

    let artists = tag
        .and_then(Accessor::artist)
        .map(|v| split_artists(&v))
        .filter(|artists| !artists.is_empty())
        .unwrap_or_else(|| vec!["其他".into()]);

    let album = tag
        .and_then(Accessor::album)
        .map(|v| bounded_text(&v))
        .unwrap_or_else(|| "其他".into());

    let properties = tagged.properties();
    let extension = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_uppercase();

    let file_path = path.to_string_lossy().into_owned();

    let has_lyrics = tagged.tags().iter().any(|t| {
        t.get_string(ItemKey::Lyrics).is_some() || t.get_string(ItemKey::UnsyncLyrics).is_some()
    }) || path.with_extension("lrc").is_file();

    let date = tag.and_then(Accessor::date).map(|v| bounded_text(&v.to_string()));
    let year = date
        .as_deref()
        .and_then(|v| v.get(..4))
        .and_then(|v| v.parse().ok());

    let albumartist = tag
        .and_then(|t| t.get_string(ItemKey::AlbumArtist))
        .map(bounded_text);
    let genre = tag
        .and_then(Accessor::genre)
        .map(|value| {
            value
                .split([',', ';'])
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .take(MAX_GENRES)
                .map(bounded_text)
                .collect()
        })
        .unwrap_or_default();

    Ok(Node {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        dir_path: file_path.clone(),
        node_type: "music".into(),
        children: None,
        id: Some(file_path.clone()),
        common: Some(CommonMetadata {
            local_title,
            file_url: file_path,
            title,
            artists,
            album,
            albumartist,
            date,
            genre,
            year,
            has_lyrics,
            modified_at: fs::metadata(path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64),
        }),
        format: Some(FormatMetadata {
            bitrate: properties.audio_bitrate().map(|v| v * 1000),
            bits_per_sample: properties.bit_depth(),
            container: extension,
            duration: properties.duration().as_secs_f64(),
            sample_rate: properties.sample_rate(),
        }),
    })
}

fn fallback_track(path: &Path) -> Node {
    let local_title = bounded_text(
        &path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy(),
    );
    let file_path = path.to_string_lossy().into_owned();
    let container = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_uppercase();
    Node {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        dir_path: file_path.clone(),
        node_type: "music".into(),
        children: None,
        id: Some(file_path.clone()),
        common: Some(CommonMetadata {
            local_title: local_title.clone(),
            file_url: file_path,
            title: local_title,
            artists: vec!["其他".into()],
            album: "其他".into(),
            albumartist: None,
            date: None,
            genre: Vec::new(),
            year: None,
            has_lyrics: path.with_extension("lrc").is_file(),
            modified_at: fs::metadata(path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64),
        }),
        format: Some(FormatMetadata {
            bitrate: None,
            bits_per_sample: None,
            container,
            duration: 0.0,
            sample_rate: None,
        }),
    }
}

fn split_artists(value: &str) -> Vec<String> {
    value
        .split([',', ';'])
        .flat_map(|part| part.split(" / "))
        .map(str::trim)
        .filter(|artist| !artist.is_empty())
        .take(MAX_ARTISTS)
        .map(bounded_text)
        .collect()
}

fn directory_only(node: &Node) -> Node {
    let children = node.children.as_ref().map(|items| {
        items
            .iter()
            .filter(|item| item.node_type == "folder")
            .map(directory_only)
            .collect()
    });
    Node {
        name: node.name.clone(),
        dir_path: node.dir_path.clone(),
        node_type: node.node_type.clone(),
        children,
        id: node.id.clone(),
        common: None,
        format: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artist_split_does_not_break_slashes_inside_names() {
        assert_eq!(split_artists("AC/DC"), vec!["AC/DC"]);
        assert_eq!(split_artists("A / B; C"), vec!["A", "B", "C"]);
    }

    #[test]
    fn metadata_is_bounded() {
        let long = "x".repeat(MAX_METADATA_CHARS + 100);
        assert_eq!(bounded_text(&long).chars().count(), MAX_METADATA_CHARS);
    }
}
