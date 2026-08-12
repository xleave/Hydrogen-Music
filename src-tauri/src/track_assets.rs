use lofty::{file::TaggedFileExt, read_from_path};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::SystemTime,
};

const MAX_COVER_BYTES: usize = 16 * 1024 * 1024;
const MAX_COVER_DIMENSION: u32 = 8192;
const MAX_COVER_PIXELS: u64 = 32 * 1024 * 1024;
const MAX_MEMORY_CACHE_ENTRIES: usize = 64;

struct EmbeddedCover {
    data: Vec<u8>,
    extension: &'static str,
}

#[derive(Clone, PartialEq, Eq)]
struct SourceStamp {
    bytes: u64,
    modified: Option<SystemTime>,
}

#[derive(Clone)]
struct CachedCover {
    source: SourceStamp,
    path: Option<PathBuf>,
    last_used: u64,
}

#[derive(Clone, Default)]
pub struct CoverCache {
    next_generation: Arc<AtomicU64>,
    entries: Arc<Mutex<HashMap<PathBuf, CachedCover>>>,
}

fn cover_format(data: &[u8]) -> Option<(&'static str, &'static str)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("image/png", "png"))
    } else if data.starts_with(b"\xff\xd8\xff") {
        Some(("image/jpeg", "jpg"))
    } else if data.starts_with(b"GIF8") {
        Some(("image/gif", "gif"))
    } else if data.starts_with(b"BM") {
        Some(("image/bmp", "bmp"))
    } else {
        None
    }
}

fn cover_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") && data.len() >= 24 {
        return Some((
            u32::from_be_bytes(data[16..20].try_into().ok()?),
            u32::from_be_bytes(data[20..24].try_into().ok()?),
        ));
    }
    if data.starts_with(b"GIF8") && data.len() >= 10 {
        return Some((
            u16::from_le_bytes(data[6..8].try_into().ok()?) as u32,
            u16::from_le_bytes(data[8..10].try_into().ok()?) as u32,
        ));
    }
    if data.starts_with(b"BM") && data.len() >= 26 {
        let width = i32::from_le_bytes(data[18..22].try_into().ok()?).unsigned_abs();
        let height = i32::from_le_bytes(data[22..26].try_into().ok()?).unsigned_abs();
        return Some((width, height));
    }
    if data.starts_with(b"\xff\xd8\xff") {
        let mut cursor = 2usize;
        while cursor + 4 <= data.len() {
            if data[cursor] != 0xff {
                cursor += 1;
                continue;
            }
            while cursor < data.len() && data[cursor] == 0xff {
                cursor += 1;
            }
            if cursor >= data.len() {
                break;
            }
            let marker = data[cursor];
            cursor += 1;
            if marker == 0xd8 || marker == 0xd9 || marker == 0x01 {
                continue;
            }
            if cursor + 2 > data.len() {
                break;
            }
            let segment_length =
                u16::from_be_bytes(data[cursor..cursor + 2].try_into().ok()?) as usize;
            if segment_length < 2 || cursor + segment_length > data.len() {
                break;
            }
            if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf)
                && segment_length >= 7
            {
                let height =
                    u16::from_be_bytes(data[cursor + 3..cursor + 5].try_into().ok()?) as u32;
                let width =
                    u16::from_be_bytes(data[cursor + 5..cursor + 7].try_into().ok()?) as u32;
                return Some((width, height));
            }
            cursor += segment_length;
        }
    }
    None
}

fn validate_cover(data: &[u8]) -> Result<(), String> {
    if data.len() > MAX_COVER_BYTES {
        return Err("embedded cover is too large".to_string());
    }
    if cover_format(data).is_none() {
        return Err("embedded cover format is unsupported".to_string());
    }
    let (width, height) = cover_dimensions(data)
        .ok_or_else(|| "embedded cover dimensions are invalid".to_string())?;
    let pixels = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > MAX_COVER_DIMENSION
        || height > MAX_COVER_DIMENSION
        || pixels > MAX_COVER_PIXELS
    {
        return Err("embedded cover dimensions are too large".to_string());
    }
    Ok(())
}

fn read_embedded_cover(file_path: &Path) -> Result<Option<EmbeddedCover>, String> {
    let Ok(tagged) = read_from_path(file_path) else {
        return Ok(None);
    };
    let picture = tagged.tags().iter().find_map(|tag| tag.pictures().first());
    let Some(picture) = picture else {
        return Ok(None);
    };
    validate_cover(picture.data())?;
    let (_, extension) = cover_format(picture.data()).expect("validated cover format");
    Ok(Some(EmbeddedCover {
        data: picture.data().to_vec(),
        extension,
    }))
}

fn materialize_cover(
    cache_directory: &Path,
    cache_key: &str,
    cover: &EmbeddedCover,
) -> Result<PathBuf, String> {
    let directory = cache_directory.join("mpris-artwork");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let target = directory.join(format!("{cache_key}.{}", cover.extension));
    fs::write(&target, &cover.data).map_err(|error| error.to_string())?;

    let mut cached = fs::read_dir(&directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path == target || !path.is_file() {
                return None;
            }
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect::<Vec<_>>();
    cached.sort_by_key(|(modified, _)| *modified);
    let remove_count = cached.len().saturating_sub(31);
    for (_, path) in cached.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
    Ok(target)
}

impl CoverCache {
    fn generation(&self) -> u64 {
        self.next_generation.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn read(
        &self,
        cache_directory: &Path,
        file_path: &Path,
    ) -> Result<Option<PathBuf>, String> {
        let metadata = fs::metadata(file_path).map_err(|error| error.to_string())?;
        let source = SourceStamp {
            bytes: metadata.len(),
            modified: metadata.modified().ok(),
        };
        let generation = self.generation();
        if let Some(path) = self
            .entries
            .lock()
            .map_err(|error| error.to_string())?
            .get_mut(file_path)
            .filter(|entry| {
                entry.source == source
                    && entry
                        .path
                        .as_ref()
                        .map(|path| path.is_file())
                        .unwrap_or(true)
            })
            .map(|entry| {
                entry.last_used = generation;
                entry.path.clone()
            })
        {
            return Ok(path);
        }

        let path = read_embedded_cover(file_path)?
            .map(|cover| materialize_cover(cache_directory, &format!("cover-{generation}"), &cover))
            .transpose()?;
        let mut entries = self.entries.lock().map_err(|error| error.to_string())?;
        if entries.len() >= MAX_MEMORY_CACHE_ENTRIES && !entries.contains_key(file_path) {
            if let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(path, _)| path.clone())
            {
                entries.remove(&oldest);
            }
        }
        entries.insert(
            file_path.to_path_buf(),
            CachedCover {
                source,
                path: path.clone(),
                last_used: generation,
            },
        );
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_tiff_without_a_supported_dimension_parser() {
        assert_eq!(
            validate_cover(b"II*\0unused").unwrap_err(),
            "embedded cover format is unsupported"
        );
    }

    #[test]
    fn rejects_png_dimensions_above_the_limit() {
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[16..20].copy_from_slice(&(MAX_COVER_DIMENSION + 1).to_be_bytes());
        png[20..24].copy_from_slice(&1u32.to_be_bytes());

        assert_eq!(
            validate_cover(&png).unwrap_err(),
            "embedded cover dimensions are too large"
        );
    }

    #[test]
    fn remembers_tracks_without_embedded_artwork_and_invalidates_on_change() {
        let directory =
            std::env::temp_dir().join(format!("hydrogen-cover-cache-test-{}", std::process::id()));
        let audio_path = directory.join("track.bin");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&audio_path, b"not tagged audio").unwrap();
        let cache = CoverCache::default();

        assert_eq!(cache.read(&directory, &audio_path).unwrap(), None);
        assert_eq!(cache.read(&directory, &audio_path).unwrap(), None);
        assert_eq!(cache.entries.lock().unwrap().len(), 1);

        fs::write(&audio_path, b"changed untagged audio").unwrap();
        assert_eq!(cache.read(&directory, &audio_path).unwrap(), None);
        assert_eq!(cache.entries.lock().unwrap()[&audio_path].source.bytes, 22);
        let _ = fs::remove_dir_all(directory);
    }
}
