use lofty::{
    file::{AudioFile, TaggedFileExt},
    read_from_path,
    tag::{Accessor, ItemKey},
};
use rayon::prelude::*;
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

pub fn scan(folders: &[String]) -> Result<ScanResult, String> {
    // 顶层文件夹并行扫描
    let results: Result<Vec<(Node, usize)>, String> = folders
        .par_iter()
        .map(|folder| scan_directory(Path::new(folder)))
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

/// 扫描单个目录，返回 (Node, 文件数量)
/// 子目录递归扫描，目录内音频文件并行解析 tag
fn scan_directory(path: &Path) -> Result<(Node, usize), String> {
    let mut entries = fs::read_dir(path)
        .map_err(|e| format!("无法读取 {}: {e}", path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 按文件名排序，保证结果一致
    entries.sort_unstable_by(|a, b| a.file_name().cmp(&b.file_name()));

    // 分离子目录和音频文件
    let mut sub_dirs = Vec::new();
    let mut audio_paths = Vec::new();
    for entry in entries {
        let p = entry.path();
        if p.is_dir() {
            sub_dirs.push(p);
        } else if is_audio_file(&p) {
            audio_paths.push(p);
        }
    }

    // 子目录：递归扫描（串行，保持树结构）
    let mut children: Vec<Node> = Vec::with_capacity(sub_dirs.len() + audio_paths.len());
    let mut count = 0usize;
    for sub in sub_dirs {
        let (node, c) = scan_directory(&sub)?;
        count += c;
        children.push(node);
    }

    // 音频文件：rayon 并行解析 lofty tag（最大瓶颈所在）
    let file_count = audio_paths.len();
    let file_nodes: Vec<Node> = audio_paths
        .par_iter()
        .map(|p| read_track(p).unwrap_or_else(|_| fallback_track(p)))
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

    let has_lyrics = tagged
        .tags()
        .iter()
        .any(|t| {
            t.get_string(ItemKey::Lyrics).is_some()
                || t.get_string(ItemKey::UnsyncLyrics).is_some()
        })
        || path.with_extension("lrc").is_file();

    let date = tag.and_then(Accessor::date).map(|v| v.to_string());
    let year = date
        .as_deref()
        .and_then(|v| v.get(..4))
        .and_then(|v| v.parse().ok());

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
        .split([',', ';', '/'])
        .map(str::trim)
        .filter(|a| !a.is_empty())
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
