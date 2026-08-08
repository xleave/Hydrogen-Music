use std::sync::Mutex;
use tauri::AppHandle;

#[derive(Default)]
struct MetadataGeneration(Mutex<u64>);

impl MetadataGeneration {
    fn reserve(&self) -> Result<u64, String> {
        let mut current = self.0.lock().map_err(|error| error.to_string())?;
        *current += 1;
        Ok(*current)
    }

    fn apply_if_current(
        &self,
        generation: u64,
        operation: impl FnOnce() -> Result<(), String>,
    ) -> Result<bool, String> {
        let current = self.0.lock().map_err(|error| error.to_string())?;
        if *current != generation {
            return Ok(false);
        }
        operation()?;
        Ok(true)
    }
}

pub struct MediaState {
    platform: platform::PlatformMediaState,
    metadata_generation: MetadataGeneration,
}

impl MediaState {
    pub fn new(app: &AppHandle) -> Self {
        Self {
            platform: platform::PlatformMediaState::new(app),
            metadata_generation: MetadataGeneration::default(),
        }
    }

    pub fn reserve_metadata(&self) -> Result<u64, String> {
        self.metadata_generation.reserve()
    }

    pub fn set_metadata_if_current(
        &self,
        generation: u64,
        title: &str,
        artist: &str,
        album: &str,
        duration: f64,
        cover_url: Option<&str>,
    ) -> Result<bool, String> {
        self.metadata_generation.apply_if_current(generation, || {
            self.platform
                .set_metadata(title, artist, album, duration, cover_url)
        })
    }

    pub fn set_playback(&self, playing: bool, position: f64) -> Result<(), String> {
        self.platform.set_playback(playing, position)
    }

    pub fn set_stopped(&self) -> Result<(), String> {
        self.platform.set_stopped()
    }

    pub fn clear(&self) -> Result<(), String> {
        self.metadata_generation.reserve()?;
        self.platform.clear()
    }

    pub fn set_volume(&self, volume: f64) -> Result<(), String> {
        self.platform.set_volume(volume)
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use serde::Serialize;
    use souvlaki::{
        MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition,
        PlatformConfig, SeekDirection,
    };
    use std::{sync::Mutex, time::Duration};
    use tauri::{AppHandle, Emitter, Manager};

    pub struct PlatformMediaState(Mutex<Option<MediaControls>>);

    #[derive(Clone, Serialize)]
    struct MediaCommand {
        action: &'static str,
        value: Option<f64>,
    }

    impl PlatformMediaState {
        pub fn new(app: &AppHandle) -> Self {
            let config = PlatformConfig {
                dbus_name: "hydrogen_music",
                display_name: "Hydrogen Music",
                hwnd: None,
            };
            let controls = (|| -> Result<MediaControls, String> {
                let mut controls = MediaControls::new(config).map_err(|error| error.to_string())?;
                let app_handle = app.clone();
                controls
                    .attach(move |event| handle_event(&app_handle, event))
                    .map_err(|error| error.to_string())?;
                controls
                    .set_playback(MediaPlayback::Stopped)
                    .map_err(|error| error.to_string())?;
                Ok(controls)
            })();

            match controls {
                Ok(controls) => Self(Mutex::new(Some(controls))),
                Err(error) => {
                    eprintln!("[mpris] disabled: {error}");
                    Self(Mutex::new(None))
                }
            }
        }

        fn with_controls(
            &self,
            operation: impl FnOnce(&mut MediaControls) -> Result<(), String>,
        ) -> Result<(), String> {
            let mut guard = self.0.lock().map_err(|error| error.to_string())?;
            let Some(controls) = guard.as_mut() else {
                return Ok(());
            };
            operation(controls)
        }

        pub fn set_metadata(
            &self,
            title: &str,
            artist: &str,
            album: &str,
            duration: f64,
            cover_url: Option<&str>,
        ) -> Result<(), String> {
            self.with_controls(|controls| {
                controls
                    .set_metadata(MediaMetadata {
                        title: Some(title),
                        artist: Some(artist),
                        album: Some(album),
                        // MPRIS consumers run outside the WebView, so artwork must
                        // be a system-readable URL rather than a data/blob URL.
                        cover_url,
                        duration: Some(Duration::from_secs_f64(duration.max(0.0))),
                    })
                    .map_err(|error| error.to_string())
            })
        }

        pub fn set_playback(&self, playing: bool, position: f64) -> Result<(), String> {
            let progress = Some(MediaPosition(Duration::from_secs_f64(position.max(0.0))));
            let playback = if playing {
                MediaPlayback::Playing { progress }
            } else {
                MediaPlayback::Paused { progress }
            };
            self.with_controls(|controls| {
                controls
                    .set_playback(playback)
                    .map_err(|error| error.to_string())
            })
        }

        pub fn set_stopped(&self) -> Result<(), String> {
            self.with_controls(|controls| {
                controls
                    .set_playback(MediaPlayback::Stopped)
                    .map_err(|error| error.to_string())
            })
        }

        pub fn clear(&self) -> Result<(), String> {
            self.with_controls(|controls| {
                controls
                    .set_metadata(MediaMetadata {
                        title: None,
                        artist: None,
                        album: None,
                        cover_url: None,
                        duration: None,
                    })
                    .map_err(|error| error.to_string())?;
                controls
                    .set_playback(MediaPlayback::Stopped)
                    .map_err(|error| error.to_string())
            })
        }

        pub fn set_volume(&self, volume: f64) -> Result<(), String> {
            self.with_controls(|controls| {
                controls
                    .set_volume(volume.clamp(0.0, 1.0))
                    .map_err(|error| error.to_string())
            })
        }
    }

    fn emit(app: &AppHandle, action: &'static str, value: Option<f64>) {
        let _ = app.emit("media-control", MediaCommand { action, value });
    }

    fn handle_event(app: &AppHandle, event: MediaControlEvent) {
        match event {
            MediaControlEvent::Play => emit(app, "play", None),
            MediaControlEvent::Pause => emit(app, "pause", None),
            MediaControlEvent::Stop => emit(app, "stop", None),
            MediaControlEvent::Toggle => emit(app, "toggle", None),
            MediaControlEvent::Next => emit(app, "next", None),
            MediaControlEvent::Previous => emit(app, "previous", None),
            MediaControlEvent::Seek(direction) => emit(app, "seekBy", Some(delta(direction, 5.0))),
            MediaControlEvent::SeekBy(direction, duration) => emit(
                app,
                "seekBy",
                Some(delta(direction, duration.as_secs_f64())),
            ),
            MediaControlEvent::SetPosition(position) => {
                emit(app, "seek", Some(position.0.as_secs_f64()))
            }
            MediaControlEvent::SetVolume(volume) => emit(app, "volume", Some(volume)),
            MediaControlEvent::Raise => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            MediaControlEvent::Quit => crate::request_exit(app),
            MediaControlEvent::OpenUri(_) => {}
        }
    }

    fn delta(direction: SeekDirection, seconds: f64) -> f64 {
        match direction {
            SeekDirection::Forward => seconds,
            SeekDirection::Backward => -seconds,
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use tauri::AppHandle;

    pub struct PlatformMediaState;

    impl PlatformMediaState {
        pub fn new(_app: &AppHandle) -> Self {
            Self
        }
        pub fn set_metadata(
            &self,
            _title: &str,
            _artist: &str,
            _album: &str,
            _duration: f64,
            _cover_url: Option<&str>,
        ) -> Result<(), String> {
            Ok(())
        }
        pub fn set_playback(&self, _playing: bool, _position: f64) -> Result<(), String> {
            Ok(())
        }
        pub fn set_stopped(&self) -> Result<(), String> {
            Ok(())
        }
        pub fn clear(&self) -> Result<(), String> {
            Ok(())
        }
        pub fn set_volume(&self, _volume: f64) -> Result<(), String> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MetadataGeneration;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn only_the_latest_metadata_request_can_apply() {
        let generations = MetadataGeneration::default();
        let first = generations.reserve().unwrap();
        let second = generations.reserve().unwrap();
        let applied = AtomicU64::new(0);

        assert!(!generations
            .apply_if_current(first, || {
                applied.store(first, Ordering::Relaxed);
                Ok(())
            })
            .unwrap());
        assert!(generations
            .apply_if_current(second, || {
                applied.store(second, Ordering::Relaxed);
                Ok(())
            })
            .unwrap());
        assert_eq!(applied.load(Ordering::Relaxed), second);
    }
}
