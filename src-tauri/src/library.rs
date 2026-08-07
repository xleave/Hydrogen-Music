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
};

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aac", "m4a", "ogg", "opus", "wma", "ape", "alac", "aiff",
    "mp2", "mpc", "wv", "speex",
];
const MAX_SCAN_DEPTH: usize = 128;

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
    /// 文件最后修改时间（Unix 毫秒时间戳），用于排序
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

pub fn scan(folders: &[PathBuf]) -> Result<ScanResult, String> {
    let results: Result<Vec<(Node, usize)>, String> = folders
        .par_iter()
        .map(|folder| {
            let root = fs::canonicalize(folder)
                .map_err(|error| format!("无法解析 {}: {error}", folder.display()))?;
            if !root.is_dir() {
                return Err(format!("{} 不是目录", root.display()));
            }
            scan_directory(&root, 0)
        })
        .collect();

    let results = results?;
    let count = results.iter().map(|(_, c)| c).sum();
    let metadata_roots: Vec<Node> = results.into_iter().map(|(n, _)| n).collect();
    let dir_tree = metadata_roots.iter().map(directory_only).collect();

    Ok(ScanResult {
        dir_tree,
        loca_files_metadata: metadata_roots,
        count,
    })
}

/// 扫描单个目录，返回 (Node, 文件数量)。
/// 不跟随符号链接，避免扫描越过用户选择的目录或进入链接环。
fn scan_directory(path: &Path, depth: usize) -> Result<(Node, usize), String> {
    if depth > MAX_SCAN_DEPTH {
        return Err(format!("目录层级过深: {}", path.display()));
    }

    let mut entries = fs::read_dir(path)
        .map_err(|e| format!("无法读取 {}: {e}", path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    entries.sort_unstable_by(|a, b| a.file_name().cmp(&b.file_name()));

    let mut sub_dirs = Vec::new();
    let mut audio_paths = Vec::new();
    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|error| format!("无法读取 {} 的文件类型: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        if file_type.is_dir() {
            sub_dirs.push(path);
        } else if file_type.is_file() && is_audio_file(&path) {
            audio_paths.push(path);
        }
    }

    let mut children: Vec<Node> = Vec::with_capacity(sub_dirs.len() + audio_paths.len());
    let mut count = 0usize;
    for sub in sub_dirs {
        let (node, child_count) = scan_directory(&sub, depth + 1)?;
        count += child_count;
        children.push(node);
    }

    let file_count = audio_paths.len();
    let file_nodes: Vec<Node> = audio_paths
        .par_iter()
        .map(|path| read_track(path).unwrap_or_else(|_| fallback_track(path)))
        .collect();

    count += file_count;
    children.extend(file_nodes);

    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();

    Ok((
        Node {
            name,
            dir_path: path.to_string_lossy().into_owned(),
            node_type: "folder".into(),
            children: Some(children),
            id: None,
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

fn read_track(path: &Path) -> Result<Node, String> {
    let tagged = read_from_path(path)
        .map_err(|e| format!("无法解析 {}: {e}", path.display()))?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let local_title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let title = tag
        .and_then(Accessor::title)
        .map(|v| v.into_owned())
        .unwrap_or_else(|| local_title.clone());

    let artists = tag
        .and_then(Accessor::artist)
        .map(|v| split_artists(&v))
        .unwrap_or_else(|| vec!["其他".into()]);

    let album = tag
        .and_then(Accessor::album)
        .map(|v| v.into_owned())
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

    let date = tag.and_then(Accessor::date).map(|v| v.to_string());
    let year = date
        .as_deref()
        .and_then(|v| v.get(..4))
        .and_then(|v| v.parse().ok());

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
            albumartist: tag
                .and_then(|t| t.get_string(ItemKey::AlbumArtist))
                .map(str::to_owned),
            date,
            genre: tag
                .and_then(Accessor::genre)
                .map(|v| vec![v.into_owned()])
                .unwrap_or_default(),
            year,
            has_lyrics,
            modified_at: std::fs::metadata(path)
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
            modified_at: std::fs::metadata(path)
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
