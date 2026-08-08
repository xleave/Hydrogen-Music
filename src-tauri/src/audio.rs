use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use serde::Serialize;
use std::{
    fs::File,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

const STALE_LOAD: &str = "stale audio load";
const DEVICE_FAULT: &str = "audio output device faulted";

#[derive(Clone)]
pub struct AudioState {
    core: Arc<Mutex<AudioCore>>,
    load_generation: Arc<AtomicU64>,
    device_faulted: Arc<AtomicBool>,
}

#[derive(Default)]
struct AudioCore {
    output: Option<AudioOutput>,
}

struct AudioOutput {
    _device: MixerDeviceSink,
    player: Player,
    duration: Duration,
    loaded: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStatus {
    position: f64,
    duration: f64,
    playing: bool,
    ended: bool,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            core: Arc::new(Mutex::new(AudioCore::default())),
            load_generation: Arc::new(AtomicU64::new(0)),
            device_faulted: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl AudioState {
    /// Reserve a native generation before any blocking decode work starts.
    /// The renderer never owns this counter, so WebView reloads cannot make
    /// otherwise valid loads look stale to the long-lived Rust process.
    pub fn reserve_load(&self) -> u64 {
        self.load_generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn invalidate_pending_loads(&self) {
        self.load_generation.fetch_add(1, Ordering::AcqRel);
    }

    fn rebuild_output_if_needed(&self, core: &mut AudioCore, duration: Duration) -> Result<(), String> {
        if core.output.is_some() && !self.device_faulted.load(Ordering::Acquire) {
            return Ok(());
        }

        core.output = None;
        // Clear first; the error callback can set the bit again immediately if
        // the newly opened stream itself is already unhealthy.
        self.device_faulted.store(false, Ordering::Release);
        let faulted = self.device_faulted.clone();
        let device = DeviceSinkBuilder::from_default_device()
            .map_err(|error| error.to_string())?
            .with_buffer_size(rodio::cpal::BufferSize::Fixed(4096))
            .with_error_callback(move |error| {
                faulted.store(true, Ordering::Release);
                eprintln!("[native audio] {error}");
            })
            .open_sink_or_fallback()
            .map_err(|error| error.to_string())?;
        let player = Player::connect_new(device.mixer());
        core.output = Some(AudioOutput {
            _device: device,
            player,
            duration,
            loaded: false,
        });
        Ok(())
    }

    pub fn load_reserved(
        &self,
        file_path: &Path,
        autoplay: bool,
        volume: f32,
        generation: u64,
    ) -> Result<AudioStatus, String> {
        if generation != self.load_generation.load(Ordering::Acquire) {
            return Err(STALE_LOAD.to_string());
        }

        let file = File::open(file_path).map_err(|error| error.to_string())?;
        let decoder = Decoder::try_from(file).map_err(|error| error.to_string())?;
        let duration = decoder.total_duration().unwrap_or_default();

        let mut core = self.core.lock().map_err(|error| error.to_string())?;
        if generation != self.load_generation.load(Ordering::Acquire) {
            return Err(STALE_LOAD.to_string());
        }

        self.rebuild_output_if_needed(&mut core, duration)?;
        if generation != self.load_generation.load(Ordering::Acquire) {
            return Err(STALE_LOAD.to_string());
        }

        let output = core.output.as_mut().expect("audio output was initialized");
        output.player.pause();
        output.player.stop();
        output.player.set_volume(volume.clamp(0.0, 1.0));
        output.player.append(decoder);
        output.duration = duration;
        output.loaded = true;
        if autoplay {
            output.player.play();
        }
        Ok(output.status())
    }

    pub fn play(&self) -> Result<AudioStatus, String> {
        if self.device_faulted.load(Ordering::Acquire) {
            return Err(DEVICE_FAULT.to_string());
        }
        self.with_output(|output| {
            output.player.play();
            Ok(output.status())
        })
    }

    pub fn pause(&self) -> Result<AudioStatus, String> {
        self.invalidate_pending_loads();
        self.with_output(|output| {
            output.player.pause();
            Ok(output.status())
        })
    }

    pub fn seek(&self, position: f64) -> Result<AudioStatus, String> {
        self.with_output(|output| {
            let target = Duration::from_secs_f64(position.max(0.0)).min(output.duration);
            output
                .player
                .try_seek(target)
                .map_err(|error| error.to_string())?;
            Ok(output.status())
        })
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), String> {
        self.with_output(|output| {
            output.player.set_volume(volume.clamp(0.0, 1.0));
            Ok(())
        })
    }

    pub fn status(&self) -> Result<AudioStatus, String> {
        if self.device_faulted.load(Ordering::Acquire) {
            return Err(DEVICE_FAULT.to_string());
        }
        self.with_output(|output| Ok(output.status()))
    }

    pub fn position(&self) -> f64 {
        let Ok(mut core) = self.core.lock() else {
            return 0.0;
        };
        core.output
            .as_mut()
            .filter(|output| output.loaded)
            .map(|output| output.player.get_pos().min(output.duration).as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn stop(&self) -> Result<(), String> {
        self.invalidate_pending_loads();
        let mut core = self.core.lock().map_err(|error| error.to_string())?;
        if let Some(output) = core.output.as_mut() {
            output.player.stop();
            output.loaded = false;
        }
        Ok(())
    }

    fn with_output<T>(
        &self,
        operation: impl FnOnce(&mut AudioOutput) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut core = self.core.lock().map_err(|error| error.to_string())?;
        let output = core
            .output
            .as_mut()
            .filter(|output| output.loaded)
            .ok_or_else(|| "no audio is loaded".to_string())?;
        operation(output)
    }
}

impl AudioOutput {
    fn status(&self) -> AudioStatus {
        let ended = self.loaded && self.player.empty();
        let position = if ended {
            self.duration
        } else {
            self.player.get_pos().min(self.duration)
        };
        AudioStatus {
            position: position.as_secs_f64(),
            duration: self.duration.as_secs_f64(),
            playing: self.loaded && !ended && !self.player.is_paused(),
            ended,
        }
    }
}
