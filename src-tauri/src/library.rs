use lofty::{
    file::{AudioFile, TaggedFileExt},
    read_from_path,
    tag::{Accessor, ItemKey},
};
use serde::Serialize;
use std::{fs, path::Path};

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aac", "m4a", "ogg", "opus", "wma", "ape", "alac", "aiff",
    "mp2", "mpc", "wv", "speex",
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    dir_tree: Vec<Node>,
    loca_files_metadata: Vec<Node>,
    count: usize,
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

pub fn scan(folders: &[String]) -> Result<ScanResult, String> {
    let mut metadata_roots = Vec::with_capacity(folders.len());
    let mut count = 0;
    for folder in folders {
        let root = scan_directory(Path::new(folder), &mut count)?;
        metadata_roots.push(root);
    }
    let dir_tree = metadata_roots.iter().map(directory_only).collect();
    Ok(ScanResult {
        dir_tree,
        loca_files_metadata: metadata_roots,
        count,
    })
}

fn scan_directory(path: &Path, count: &mut usize) -> Result<Node, String> {
    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("无法读取 {}: {error}", path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());

    let mut children = Vec::new();
    for entry in entries {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            children.push(scan_directory(&entry_path, count)?);
        } else if is_audio_file(&entry_path) {
            children.push(read_track(&entry_path).unwrap_or_else(|_| fallback_track(&entry_path)));
            *count += 1;
        }
    }

    let date = tag.and_then(Accessor::date).map(|value| value.to_string());
    let year = date
        .as_deref()
        .and_then(|value| value.get(..4))
        .and_then(|value| value.parse().ok());

    Ok(Node {
        name: path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned(),
        dir_path: path.to_string_lossy().into_owned(),
        node_type: "folder".into(),
        children: Some(children),
        id: None,
        common: None,
        format: None,
    })
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| AUDIO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn read_track(path: &Path) -> Result<Node, String> {
    let tagged = read_from_path(path)
        .map_err(|error| format!("无法解析 {}: {error}", path.display()))?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let local_title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let title = tag
        .and_then(Accessor::title)
        .map(|value| value.into_owned())
        .unwrap_or_else(|| local_title.clone());
    let artists = tag
        .and_then(Accessor::artist)
        .map(|value| split_artists(&value))
        .unwrap_or_else(|| vec!["其他".into()]);
    let album = tag
        .and_then(Accessor::album)
        .map(|value| value.into_owned())
        .unwrap_or_else(|| "其他".into());
    let properties = tagged.properties();
    let extension = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_uppercase();
    let file_path = path.to_string_lossy().into_owned();
    let has_lyrics = tagged
        .tags()
        .iter()
        .any(|value| {
            value.get_string(ItemKey::Lyrics).is_some()
                || value.get_string(ItemKey::UnsyncLyrics).is_some()
        })
        || path.with_extension("lrc").is_file();

    Ok(Node {
        name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
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
            albumartist: tag
                .and_then(|value| value.get_string(ItemKey::AlbumArtist))
                .map(str::to_owned),
            date,
            genre: tag
                .and_then(Accessor::genre)
                .map(|value| vec![value.into_owned()])
                .unwrap_or_default(),
            year,
            has_lyrics,
        }),
        format: Some(FormatMetadata {
            bitrate: properties.audio_bitrate().map(|value| value * 1000),
            bits_per_sample: properties.bit_depth(),
            container: extension,
            duration: properties.duration().as_secs_f64(),
            sample_rate: properties.sample_rate(),
        }),
    })
}

fn fallback_track(path: &Path) -> Node {
    let local_title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let file_path = path.to_string_lossy().into_owned();
    let container = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_uppercase();
    Node {
        name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
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
        .split([',', ';', '/'])
        .map(str::trim)
        .filter(|artist| !artist.is_empty())
        .map(str::to_owned)
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
        id: None,
        common: None,
        format: None,
    }
}
