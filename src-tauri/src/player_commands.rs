use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Manager, State};

use crate::{
    audio, authorized_file_path_from_folders, bounded_text, configured_music_folders, media,
    track_assets, SettingsState, MAX_AUDIO_SECONDS, MAX_TEXT_CHARS,
};

fn finite(value: f64, name: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{name} must be finite"))
    }
}

fn finite_f32(value: f32, name: &str) -> Result<f32, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{name} must be finite"))
    }
}

#[cfg(target_os = "windows")]
fn media_cover_url(path: &Path) -> Result<String, String> {
    Ok(format!("file://{}", path.to_string_lossy()))
}

#[cfg(not(target_os = "windows"))]
fn media_cover_url(path: &Path) -> Result<String, String> {
    tauri::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| "failed to create system media artwork file URL".to_string())
}

#[tauri::command]
pub(crate) async fn audio_load(
    settings: State<'_, SettingsState>,
    audio: State<'_, audio::AudioState>,
    file_path: String,
    autoplay: bool,
    volume: f32,
) -> Result<audio::AudioStatus, String> {
    let volume = finite_f32(volume, "volume")?.clamp(0.0, 1.0);
    let folders = configured_music_folders(&settings)?;
    let audio = audio.inner().clone();
    // Reserve on the command thread before entering the blocking pool. Task
    // scheduling order can no longer make an older decode supersede a newer one.
    let generation = audio.reserve_load();
    tauri::async_runtime::spawn_blocking(move || {
        let path = authorized_file_path_from_folders(&folders, &file_path)?;
        audio.load_reserved(&path, autoplay, volume, generation)
    })
    .await
    .map_err(|error| error.to_string())?
}

async fn run_audio_blocking<T, F>(audio: audio::AudioState, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&audio::AudioState) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || operation(&audio))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn audio_play(
    audio: State<'_, audio::AudioState>,
) -> Result<audio::AudioStatus, String> {
    run_audio_blocking(audio.inner().clone(), |audio| audio.play()).await
}

#[tauri::command]
pub(crate) async fn audio_pause(
    audio: State<'_, audio::AudioState>,
) -> Result<audio::AudioStatus, String> {
    run_audio_blocking(audio.inner().clone(), |audio| audio.pause()).await
}

#[tauri::command]
pub(crate) async fn audio_seek(
    audio: State<'_, audio::AudioState>,
    position: f64,
) -> Result<audio::AudioStatus, String> {
    let position = finite(position, "position")?.clamp(0.0, MAX_AUDIO_SECONDS);
    run_audio_blocking(audio.inner().clone(), move |audio| audio.seek(position)).await
}

#[tauri::command]
pub(crate) async fn audio_set_volume(
    audio: State<'_, audio::AudioState>,
    volume: f32,
) -> Result<(), String> {
    let volume = finite_f32(volume, "volume")?.clamp(0.0, 1.0);
    run_audio_blocking(audio.inner().clone(), move |audio| audio.set_volume(volume)).await
}

#[tauri::command]
pub(crate) async fn audio_status(
    audio: State<'_, audio::AudioState>,
) -> Result<audio::AudioStatus, String> {
    run_audio_blocking(audio.inner().clone(), |audio| audio.status()).await
}

#[tauri::command]
pub(crate) async fn audio_stop(audio: State<'_, audio::AudioState>) -> Result<(), String> {
    run_audio_blocking(audio.inner().clone(), |audio| audio.stop()).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaMetadataResult {
    applied: bool,
    cover_path: Option<String>,
}

#[tauri::command]
pub(crate) async fn media_set_metadata(
    app: AppHandle,
    media: State<'_, media::MediaState>,
    title: String,
    artist: String,
    album: String,
    duration: f64,
    file_path: Option<String>,
) -> Result<MediaMetadataResult, String> {
    let generation = media.reserve_metadata()?;
    let duration = finite(duration, "duration")?.clamp(0.0, MAX_AUDIO_SECONDS);
    let title = bounded_text(&title, MAX_TEXT_CHARS);
    let artist = bounded_text(&artist, MAX_TEXT_CHARS);
    let album = bounded_text(&album, MAX_TEXT_CHARS);
    let assets = if let Some(file_path) = file_path {
        let settings = app.state::<SettingsState>();
        let folders = configured_music_folders(settings.inner())?;
        let cache_directory = app
            .path()
            .app_cache_dir()
            .map_err(|error| error.to_string())?;
        tauri::async_runtime::spawn_blocking(
            move || -> Result<track_assets::TrackAssets, String> {
                let file_path = authorized_file_path_from_folders(&folders, &file_path)?;
                Ok(
                    match track_assets::read_for_media(&cache_directory, &file_path, generation) {
                        Ok(assets) => assets,
                        Err(error) => {
                            eprintln!("[media artwork] ignored: {error}");
                            track_assets::TrackAssets::default()
                        }
                    },
                )
            },
        )
        .await
        .map_err(|error| error.to_string())??
    } else {
        track_assets::TrackAssets::default()
    };
    let cover_url = assets
        .media_cover_path
        .as_ref()
        .map(|path| media_cover_url(path))
        .transpose()?;
    let applied = media.set_metadata_if_current(
        generation,
        &title,
        &artist,
        &album,
        duration,
        cover_url.as_deref(),
    )?;
    Ok(MediaMetadataResult {
        applied,
        cover_path: if applied {
            assets
                .media_cover_path
                .map(|path| path.to_string_lossy().into_owned())
        } else {
            None
        },
    })
}

#[tauri::command]
pub(crate) async fn media_set_playback(
    audio: State<'_, audio::AudioState>,
    media: State<'_, media::MediaState>,
    playing: bool,
) -> Result<(), String> {
    match run_audio_blocking(audio.inner().clone(), |audio| audio.status_position()).await {
        Ok(position) => media.set_playback(playing, position),
        Err(_) => media.set_stopped(),
    }
}

#[tauri::command]
pub(crate) fn media_set_stopped(media: State<'_, media::MediaState>) -> Result<(), String> {
    media.set_stopped()
}

#[tauri::command]
pub(crate) fn media_clear(media: State<'_, media::MediaState>) -> Result<(), String> {
    media.clear()
}

#[tauri::command]
pub(crate) fn media_set_volume(
    media: State<'_, media::MediaState>,
    volume: f64,
) -> Result<(), String> {
    media.set_volume(finite(volume, "volume")?.clamp(0.0, 1.0))
}
