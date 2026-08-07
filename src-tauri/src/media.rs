#[cfg(target_os = "linux")]
mod platform {
    use serde::Serialize;
    use souvlaki::{
        MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition,
        PlatformConfig, SeekDirection,
    };
    use std::{sync::Mutex, time::Duration};
    use tauri::{AppHandle, Emitter, Manager};

    pub struct MediaState(Mutex<MediaControls>);

    #[derive(Clone, Serialize)]
    struct MediaCommand {
        action: &'static str,
        value: Option<f64>,
    }

    impl MediaState {
        pub fn new(app: &AppHandle) -> Result<Self, String> {
            let config = PlatformConfig {
                dbus_name: "hydrogen_music",
                display_name: "Hydrogen Music",
                hwnd: None,
            };
            let mut controls = MediaControls::new(config).map_err(|error| error.to_string())?;
            let app_handle = app.clone();
            controls
                .attach(move |event| handle_event(&app_handle, event))
                .map_err(|error| error.to_string())?;
            controls
                .set_playback(MediaPlayback::Stopped)
                .map_err(|error| error.to_string())?;
            Ok(Self(Mutex::new(controls)))
        }

        pub fn set_metadata(&self, title: &str, artist: &str, album: &str, duration: f64) -> Result<(), String> {
            self.0.lock().map_err(|error| error.to_string())?.set_metadata(MediaMetadata {
                title: Some(title), artist: Some(artist), album: Some(album), cover_url: None,
                duration: Some(Duration::from_secs_f64(duration.max(0.0))),
            }).map_err(|error| error.to_string())
        }

        pub fn set_playback(&self, playing: bool, position: f64) -> Result<(), String> {
            let progress = Some(MediaPosition(Duration::from_secs_f64(position.max(0.0))));
            let playback = if playing { MediaPlayback::Playing { progress } } else { MediaPlayback::Paused { progress } };
            self.0.lock().map_err(|error| error.to_string())?.set_playback(playback).map_err(|error| error.to_string())
        }

        pub fn set_volume(&self, volume: f64) -> Result<(), String> {
            self.0.lock().map_err(|error| error.to_string())?.set_volume(volume.clamp(0.0, 1.0)).map_err(|error| error.to_string())
        }
    }

    fn emit(app: &AppHandle, action: &'static str, value: Option<f64>) {
        let _ = app.emit("media-control", MediaCommand { action, value });
    }

    fn handle_event(app: &AppHandle, event: MediaControlEvent) {
        match event {
            MediaControlEvent::Play => emit(app, "play", None),
            MediaControlEvent::Pause | MediaControlEvent::Stop => emit(app, "pause", None),
            MediaControlEvent::Toggle => emit(app, "toggle", None),
            MediaControlEvent::Next => emit(app, "next", None),
            MediaControlEvent::Previous => emit(app, "previous", None),
            MediaControlEvent::Seek(direction) => emit(app, "seekBy", Some(delta(direction, 5.0))),
            MediaControlEvent::SeekBy(direction, duration) => emit(app, "seekBy", Some(delta(direction, duration.as_secs_f64()))),
            MediaControlEvent::SetPosition(position) => emit(app, "seek", Some(position.0.as_secs_f64())),
            MediaControlEvent::SetVolume(volume) => emit(app, "volume", Some(volume)),
            MediaControlEvent::Raise => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            MediaControlEvent::Quit => { let _ = app.emit("app-exit-requested", ()); }
            MediaControlEvent::OpenUri(_) => {}
        }
    }

    fn delta(direction: SeekDirection, seconds: f64) -> f64 {
        match direction { SeekDirection::Forward => seconds, SeekDirection::Backward => -seconds }
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use tauri::AppHandle;
    pub struct MediaState;
    impl MediaState {
        pub fn new(_app: &AppHandle) -> Result<Self, String> { Ok(Self) }
        pub fn set_metadata(&self, _title: &str, _artist: &str, _album: &str, _duration: f64) -> Result<(), String> { Ok(()) }
        pub fn set_playback(&self, _playing: bool, _position: f64) -> Result<(), String> { Ok(()) }
        pub fn set_volume(&self, _volume: f64) -> Result<(), String> { Ok(()) }
    }
}

pub use platform::MediaState;
