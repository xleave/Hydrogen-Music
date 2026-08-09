use lofty::{
    file::{AudioFile, TaggedFileExt},
    read_from_path,
    tag::{Accessor, ItemKey},
};
use rayon::prelude::*;
use rusqlite::{params, Connection};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Mutex,
    },
};

#[cfg(unix)]
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};

use crate::library_model::{CommonMetadata, FormatMetadata, Node};
pub use crate::library_model::{ScanResponse, ScanResult};

pub const STALE_SCAN: &str = "stale music scan";
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aac", "m4a", "ogg", "opus", "wma", "ape", "alac", "aiff", "mp2", "mpc",
    "wv", "speex",
];
const MAX_SCAN_DEPTH: usize = 128;
const MAX_TRACKS: usize = 100_000;
const MAX_DIRECTORIES: usize = 100_000;
const MAX_VISITED_ENTRIES: usize = 1_000_000;
const MAX_METADATA_CHARS: usize = 4096;
const MAX_ARTISTS: usize = 32;
const MAX_GENRES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fingerprint {
    size: u64,
    modified_ms: u64,
    lrc_size: u64,
    lrc_modified_ms: u64,
}

#[derive(Clone)]
struct CachedTrack {
    fingerprint: Fingerprint,
    file_key: Option<String>,
    node: Node,
}

struct ScanCache {
    entries: HashMap<String, CachedTrack>,
    path_by_file_key: HashMap<String, String>,
    updates: Mutex<Vec<(String, CachedTrack)>>,
    seen: Mutex<HashMap<String, Option<Fingerprint>>>,
}

impl ScanCache {
    fn new(entries: HashMap<String, CachedTrack>) -> Self {
        let path_by_file_key = entries
            .iter()
            .filter_map(|(path, entry)| {
                entry
                    .file_key
                    .as_ref()
                    .map(|file_key| (file_key.clone(), path.clone()))
            })
            .collect();
        Self {
            entries,
            path_by_file_key,
            updates: Mutex::new(Vec::new()),
            seen: Mutex::new(HashMap::new()),
        }
    }

    fn remember_seen(&self, path: &str, fingerprint: Option<Fingerprint>) {
        if let Ok(mut seen) = self.seen.lock() {
            seen.insert(path.to_owned(), fingerprint);
        }
    }

    fn queue_update(&self, path: String, entry: CachedTrack) {
        if let Ok(mut updates) = self.updates.lock() {
            updates.push((path, entry));
        }
    }

    fn moved_entry(&self, current_path: &str, file_key: Option<&str>) -> Option<&CachedTrack> {
        let previous_path = self.path_by_file_key.get(file_key?)?;
        if previous_path == current_path || Path::new(previous_path).exists() {
            return None;
        }
        self.entries.get(previous_path)
    }
}

struct ScanBudget {
    tracks: AtomicUsize,
    directories: AtomicUsize,
    entries: AtomicUsize,
    truncated: AtomicBool,
    incomplete: AtomicBool,
}

impl ScanBudget {
    fn new() -> Self {
        Self {
            tracks: AtomicUsize::new(0),
            directories: AtomicUsize::new(0),
            entries: AtomicUsize::new(0),
            truncated: AtomicBool::new(false),
            incomplete: AtomicBool::new(false),
        }
    }

    fn reserve(counter: &AtomicUsize, max: usize, truncated: &AtomicBool) -> bool {
        if counter.fetch_add(1, Ordering::Relaxed) < max {
            true
        } else {
            counter.fetch_sub(1, Ordering::Relaxed);
            truncated.store(true, Ordering::Release);
            false
        }
    }

    fn reserve_track(&self) -> bool {
        Self::reserve(&self.tracks, MAX_TRACKS, &self.truncated)
    }

    fn reserve_directory(&self) -> bool {
        Self::reserve(&self.directories, MAX_DIRECTORIES, &self.truncated)
    }

    fn reserve_entry(&self) -> bool {
        Self::reserve(&self.entries, MAX_VISITED_ENTRIES, &self.truncated)
    }

    fn mark_incomplete(&self) {
        self.incomplete.store(true, Ordering::Release);
    }

    fn may_prune_cache(&self) -> bool {
        !self.truncated.load(Ordering::Acquire) && !self.incomplete.load(Ordering::Acquire)
    }
}

pub fn scan(
    folders: &[PathBuf],
    request_id: u64,
    latest_request_id: &AtomicU64,
    roots_complete: bool,
    index_path: &Path,
) -> Result<ScanResult, String> {
    ensure_current(request_id, latest_request_id)?;
    let cache_entries = load_index(index_path).unwrap_or_else(|error| {
        eprintln!("[library index] ignoring cache: {error}");
        HashMap::new()
    });
    let cache = ScanCache::new(cache_entries);
    let budget = ScanBudget::new();

    let roots: Result<Vec<Option<(Node, usize)>>, String> = folders
        .par_iter()
        .map(|folder| {
            ensure_current(request_id, latest_request_id)?;
            let root = match fs::canonicalize(folder) {
                Ok(root) => root,
                Err(error) => {
                    budget.mark_incomplete();
                    eprintln!("[library scan] 无法解析 {}: {error}", folder.display());
                    return Ok(None);
                }
            };
            if !root.is_dir() {
                budget.mark_incomplete();
                eprintln!("[library scan] {} 不是目录", root.display());
                return Ok(None);
            }
            if !budget.reserve_directory() {
                return Ok(None);
            }
            match scan_directory(&root, 0, request_id, latest_request_id, &budget, &cache) {
                Ok(result) => Ok(Some(result)),
                Err(error) if error == STALE_SCAN => Err(error),
                Err(error) => {
                    budget.mark_incomplete();
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
    let revision = library_revision(&metadata_roots, &cache);

    let complete = roots_complete && budget.may_prune_cache();
    if let Err(error) = persist_index(index_path, &cache, complete) {
        eprintln!("[library index] failed to persist cache: {error}");
    }

    Ok(ScanResult {
        dir_tree,
        loca_files_metadata: metadata_roots,
        count,
        truncated: budget.truncated.load(Ordering::Acquire),
        complete,
        revision,
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
    budget: &ScanBudget,
    cache: &ScanCache,
) -> Result<(Node, usize), String> {
    ensure_current(request_id, latest_request_id)?;
    if depth > MAX_SCAN_DEPTH {
        budget.truncated.store(true, Ordering::Release);
        return Err(format!("目录层级过深: {}", path.display()));
    }

    let read_dir = match fs::read_dir(path) {
        Ok(read_dir) => read_dir,
        Err(error) => {
            budget.mark_incomplete();
            return Err(format!("无法读取 {}: {error}", path.display()));
        }
    };
    let mut entries = Vec::new();
    for entry in read_dir {
        ensure_current(request_id, latest_request_id)?;
        if !budget.reserve_entry() {
            break;
        }
        match entry {
            Ok(entry) => entries.push(entry),
            Err(error) => {
                budget.mark_incomplete();
                eprintln!("[library scan] 无法读取目录项 {}: {error}", path.display());
            }
        }
    }
    entries.sort_unstable_by_key(|a| a.file_name());

    let mut sub_dirs = Vec::new();
    let mut audio_paths = Vec::new();
    for entry in entries {
        ensure_current(request_id, latest_request_id)?;
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                budget.mark_incomplete();
                eprintln!(
                    "[library scan] 无法读取 {} 的文件类型: {error}",
                    entry.path().display()
                );
                continue;
            }
        };
        if file_type.is_symlink() {
            continue;
        }

        let entry_path = entry.path();
        if file_type.is_dir() {
            if budget.reserve_directory() {
                sub_dirs.push(entry_path);
            }
        } else if file_type.is_file() && is_audio_file(&entry_path) && budget.reserve_track() {
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
            budget,
            cache,
        ) {
            Ok((node, child_count)) => {
                count += child_count;
                children.push(node);
            }
            Err(error) if error == STALE_SCAN => return Err(error),
            Err(error) => {
                budget.mark_incomplete();
                eprintln!("[library scan] {error}");
            }
        }
    }

    ensure_current(request_id, latest_request_id)?;
    let file_nodes: Vec<Node> = audio_paths
        .par_iter()
        .filter_map(|audio_path| {
            if latest_request_id.load(Ordering::Acquire) != request_id {
                return None;
            }
            Some(read_track_cached(audio_path, cache))
        })
        .collect();

    ensure_current(request_id, latest_request_id)?;
    count += file_nodes.len();
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

fn fnv1a(bytes: impl Iterator<Item = u8>, mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn stable_track_id(path: &Path) -> String {
    #[cfg(unix)]
    let bytes = path.as_os_str().as_bytes();
    #[cfg(not(unix))]
    let owned = path.to_string_lossy().into_owned();
    #[cfg(not(unix))]
    let bytes = owned.as_bytes();

    let first = fnv1a(bytes.iter().copied(), 0xcbf29ce484222325);
    let second = fnv1a(bytes.iter().rev().copied(), 0x84222325cbf29ce4);
    format!("track:{first:016x}{second:016x}")
}

fn hash_revision_field(value: &[u8], first: &mut u64, second: &mut u64) {
    let length = (value.len() as u64).to_le_bytes();
    *first = fnv1a(length.into_iter().chain(value.iter().copied()), *first);
    *second = fnv1a(value.iter().rev().copied().chain(length), *second);
}

fn hash_revision_nodes(
    nodes: &[Node],
    seen: &HashMap<String, Option<Fingerprint>>,
    first: &mut u64,
    second: &mut u64,
) {
    for node in nodes {
        hash_revision_field(node.node_type.as_bytes(), first, second);
        hash_revision_field(node.dir_path.as_bytes(), first, second);
        if let Some(Some(fingerprint)) = seen.get(&node.dir_path) {
            for value in [
                fingerprint.size,
                fingerprint.modified_ms,
                fingerprint.lrc_size,
                fingerprint.lrc_modified_ms,
            ] {
                hash_revision_field(&value.to_le_bytes(), first, second);
            }
        }
        if let Some(children) = node.children.as_deref() {
            hash_revision_nodes(children, seen, first, second);
        }
    }
}

fn library_revision(nodes: &[Node], cache: &ScanCache) -> String {
    let mut first = 0xcbf29ce484222325;
    let mut second = 0x84222325cbf29ce4;
    if let Ok(seen) = cache.seen.lock() {
        hash_revision_nodes(nodes, &seen, &mut first, &mut second);
    } else {
        hash_revision_nodes(nodes, &HashMap::new(), &mut first, &mut second);
    }
    format!("{first:016x}{second:016x}")
}

fn modified_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(unix)]
fn native_file_key(metadata: &fs::Metadata) -> Option<String> {
    Some(format!("{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn native_file_key(_metadata: &fs::Metadata) -> Option<String> {
    None
}

fn file_state(path: &Path) -> Option<(Fingerprint, Option<String>)> {
    let metadata = fs::metadata(path).ok()?;
    let lrc = fs::metadata(path.with_extension("lrc")).ok();
    let fingerprint = Fingerprint {
        size: metadata.len(),
        modified_ms: modified_ms(&metadata),
        lrc_size: lrc.as_ref().map(fs::Metadata::len).unwrap_or(0),
        lrc_modified_ms: lrc.as_ref().map(modified_ms).unwrap_or(0),
    };
    Some((fingerprint, native_file_key(&metadata)))
}

fn relocate_cached_track(mut node: Node, path: &Path) -> Node {
    let file_path = path.to_string_lossy().into_owned();
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let local_title = bounded_text(&path.file_stem().unwrap_or_default().to_string_lossy());
    node.name = file_name;
    node.dir_path = file_path.clone();
    if let Some(common) = node.common.as_mut() {
        if common.title == common.local_title {
            common.title = local_title.clone();
        }
        common.local_title = local_title;
        common.file_url = file_path;
    }
    node
}

fn read_track_cached(path: &Path, cache: &ScanCache) -> Node {
    let path_string = path.to_string_lossy().into_owned();
    let current = file_state(path);
    cache.remember_seen(
        &path_string,
        current.as_ref().map(|(fingerprint, _)| *fingerprint),
    );

    if let Some((current, file_key)) = current {
        if let Some(cached) = cache.entries.get(&path_string) {
            if cached.fingerprint == current {
                if cached.file_key != file_key {
                    cache.queue_update(
                        path_string,
                        CachedTrack {
                            fingerprint: current,
                            file_key,
                            node: cached.node.clone(),
                        },
                    );
                }
                return cached.node.clone();
            }
        }

        let moved = cache.moved_entry(&path_string, file_key.as_deref());
        let mut node = match moved {
            Some(entry) if entry.fingerprint == current => {
                relocate_cached_track(entry.node.clone(), path)
            }
            _ => read_track(path).unwrap_or_else(|_| fallback_track(path)),
        };
        if let Some(id) = moved.and_then(|entry| entry.node.id.clone()) {
            node.id = Some(id);
        }
        cache.queue_update(
            path_string,
            CachedTrack {
                fingerprint: current,
                file_key,
                node: node.clone(),
            },
        );
        return node;
    }

    fallback_track(path)
}

fn open_index(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS tracks (
                 path TEXT PRIMARY KEY NOT NULL,
                 size INTEGER NOT NULL,
                 modified_ms INTEGER NOT NULL,
                 lrc_size INTEGER NOT NULL,
                 lrc_modified_ms INTEGER NOT NULL,
                 file_key TEXT,
                 node_json TEXT NOT NULL
             );",
        )
        .map_err(|error| error.to_string())?;
    let has_file_key = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('tracks') WHERE name = 'file_key')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| error.to_string())?;
    if !has_file_key {
        connection
            .execute("ALTER TABLE tracks ADD COLUMN file_key TEXT", [])
            .map_err(|error| error.to_string())?;
    }
    Ok(connection)
}

fn load_index(path: &Path) -> Result<HashMap<String, CachedTrack>, String> {
    let connection = open_index(path)?;

    let mut statement = connection
        .prepare(
            "SELECT path, size, modified_ms, lrc_size, lrc_modified_ms, file_key, node_json
             FROM tracks",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let size: i64 = row.get(1)?;
            let modified_ms: i64 = row.get(2)?;
            let lrc_size: i64 = row.get(3)?;
            let lrc_modified_ms: i64 = row.get(4)?;
            let file_key: Option<String> = row.get(5)?;
            let node_json: String = row.get(6)?;
            Ok((
                path,
                size,
                modified_ms,
                lrc_size,
                lrc_modified_ms,
                file_key,
                node_json,
            ))
        })
        .map_err(|error| error.to_string())?;

    let mut cache = HashMap::new();
    for row in rows {
        let Ok((path, size, modified_ms, lrc_size, lrc_modified_ms, file_key, node_json)) = row
        else {
            continue;
        };
        let Ok(node) = serde_json::from_str::<Node>(&node_json) else {
            continue;
        };
        if size < 0 || modified_ms < 0 || lrc_size < 0 || lrc_modified_ms < 0 {
            continue;
        }
        cache.insert(
            path,
            CachedTrack {
                fingerprint: Fingerprint {
                    size: size as u64,
                    modified_ms: modified_ms as u64,
                    lrc_size: lrc_size as u64,
                    lrc_modified_ms: lrc_modified_ms as u64,
                },
                file_key,
                node,
            },
        );
    }
    Ok(cache)
}

fn persist_index(path: &Path, cache: &ScanCache, prune_stale: bool) -> Result<(), String> {
    let mut connection = open_index(path)?;

    let updates = cache
        .updates
        .lock()
        .map_err(|error| error.to_string())?
        .clone();
    let stale = if prune_stale {
        let seen = cache
            .seen
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        cache
            .entries
            .keys()
            .filter(|path| !seen.contains_key(*path))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    {
        let mut upsert = transaction
            .prepare_cached(
                "INSERT INTO tracks(
                    path, size, modified_ms, lrc_size, lrc_modified_ms, file_key, node_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(path) DO UPDATE SET
                    size=excluded.size,
                    modified_ms=excluded.modified_ms,
                    lrc_size=excluded.lrc_size,
                    lrc_modified_ms=excluded.lrc_modified_ms,
                    file_key=excluded.file_key,
                    node_json=excluded.node_json",
            )
            .map_err(|error| error.to_string())?;
        for (path, entry) in updates {
            let node_json =
                serde_json::to_string(&entry.node).map_err(|error| error.to_string())?;
            upsert
                .execute(params![
                    path,
                    entry.fingerprint.size as i64,
                    entry.fingerprint.modified_ms as i64,
                    entry.fingerprint.lrc_size as i64,
                    entry.fingerprint.lrc_modified_ms as i64,
                    entry.file_key,
                    node_json,
                ])
                .map_err(|error| error.to_string())?;
        }
    }
    {
        let mut delete = transaction
            .prepare_cached("DELETE FROM tracks WHERE path = ?1")
            .map_err(|error| error.to_string())?;
        for path in stale {
            delete.execute([path]).map_err(|error| error.to_string())?;
        }
    }
    transaction.commit().map_err(|error| error.to_string())
}

fn read_track(path: &Path) -> Result<Node, String> {
    let tagged = read_from_path(path).map_err(|e| format!("无法解析 {}: {e}", path.display()))?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let local_title = bounded_text(&path.file_stem().unwrap_or_default().to_string_lossy());

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

    let date = tag
        .and_then(Accessor::date)
        .map(|v| bounded_text(&v.to_string()));
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
        id: Some(stable_track_id(path)),
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
                .map(|metadata| modified_ms(&metadata)),
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
    let local_title = bounded_text(&path.file_stem().unwrap_or_default().to_string_lossy());
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
        id: Some(stable_track_id(path)),
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
                .map(|metadata| modified_ms(&metadata)),
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

    static NEXT_LIBRARY_TEST: AtomicU64 = AtomicU64::new(0);

    fn temporary_library_directory() -> PathBuf {
        let id = NEXT_LIBRARY_TEST.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "hydrogen-music-library-test-{}-{id}",
            std::process::id()
        ))
    }

    fn first_track(result: &ScanResult) -> &Node {
        fn visit(nodes: &[Node]) -> Option<&Node> {
            for node in nodes {
                if node.node_type == "music" {
                    return Some(node);
                }
                if let Some(track) = node.children.as_deref().and_then(visit) {
                    return Some(track);
                }
            }
            None
        }
        visit(&result.loca_files_metadata).unwrap()
    }

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

    #[test]
    fn track_ids_are_stable_and_compact() {
        let path = Path::new("/music/example/song.flac");
        let first = stable_track_id(path);
        let second = stable_track_id(path);
        assert_eq!(first, second);
        assert!(first.starts_with("track:"));
        assert_eq!(first.len(), 38);
    }

    #[test]
    fn library_revision_changes_when_a_track_fingerprint_changes() {
        let path = Path::new("/music/example/song.flac");
        let path_string = path.to_string_lossy();
        let nodes = vec![fallback_track(path)];
        let first_cache = ScanCache::new(HashMap::new());
        first_cache.remember_seen(
            &path_string,
            Some(Fingerprint {
                size: 100,
                modified_ms: 200,
                lrc_size: 0,
                lrc_modified_ms: 0,
            }),
        );
        let second_cache = ScanCache::new(HashMap::new());
        second_cache.remember_seen(
            &path_string,
            Some(Fingerprint {
                size: 101,
                modified_ms: 200,
                lrc_size: 0,
                lrc_modified_ms: 0,
            }),
        );

        assert_ne!(
            library_revision(&nodes, &first_cache),
            library_revision(&nodes, &second_cache)
        );
    }

    #[test]
    fn matching_revision_returns_only_the_scan_summary() {
        let result = ScanResult {
            dir_tree: vec![fallback_track(Path::new("/music/example/song.flac"))],
            loca_files_metadata: vec![fallback_track(Path::new("/music/example/song.flac"))],
            count: 1,
            truncated: false,
            complete: true,
            revision: "revision-1".into(),
        };

        let response = serde_json::to_value(result.into_response(Some("revision-1"))).unwrap();

        assert_eq!(
            response
                .get("unchanged")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            response.get("count").and_then(serde_json::Value::as_u64),
            Some(1)
        );
        assert!(response.get("locaFilesMetadata").is_none());
        assert!(response.get("dirTree").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn moving_a_track_on_the_same_filesystem_preserves_its_id() {
        let temporary = temporary_library_directory();
        let music = temporary.join("music");
        let cache = temporary.join("cache/library-index.sqlite3");
        fs::create_dir_all(&music).unwrap();
        let original = music.join("before.mp3");
        let moved = music.join("after.mp3");
        fs::write(&original, b"not-a-real-mp3").unwrap();

        let generation = AtomicU64::new(1);
        let first = scan(std::slice::from_ref(&music), 1, &generation, true, &cache).unwrap();
        let first_id = first_track(&first).id.clone();
        fs::rename(&original, &moved).unwrap();
        generation.store(2, Ordering::Release);
        let second = scan(std::slice::from_ref(&music), 2, &generation, true, &cache).unwrap();
        let second_track = first_track(&second);

        assert_eq!(second_track.id, first_id);
        assert_eq!(
            second_track.common.as_ref().unwrap().file_url,
            moved.to_string_lossy()
        );
        fs::remove_dir_all(temporary).unwrap();
    }

    #[test]
    fn budget_only_marks_truncated_after_a_real_overflow() {
        let budget = ScanBudget::new();
        for _ in 0..MAX_TRACKS {
            assert!(budget.reserve_track());
        }
        assert!(!budget.truncated.load(Ordering::Acquire));
        assert!(!budget.reserve_track());
        assert!(budget.truncated.load(Ordering::Acquire));
        assert!(!budget.may_prune_cache());
    }

    #[test]
    fn incomplete_scan_never_prunes_cache() {
        let budget = ScanBudget::new();
        assert!(budget.may_prune_cache());
        budget.mark_incomplete();
        assert!(!budget.may_prune_cache());
    }
}
