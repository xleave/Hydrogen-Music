use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use serde::Serialize;
use std::{
    fs::File,
    path::Path,
    sync::Mutex,
    time::Duration,
};

pub struct AudioState(Mutex<Option<AudioOutput>>);

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
        Self(Mutex::new(None))
    }
}

impl AudioState {
    pub fn load(
        &self,
        file_path: &Path,
        autoplay: bool,
        volume: f32,
    ) -> Result<AudioStatus, String> {
        let file = File::open(file_path).map_err(|error| error.to_string())?;
        let decoder = Decoder::try_from(file).map_err(|error| error.to_string())?;
        let duration = decoder.total_duration().unwrap_or_default();
        let mut output = self.0.lock().map_err(|error| error.to_string())?;

        if output.is_none() {
            let device = DeviceSinkBuilder::from_default_device()
                .map_err(|error| error.to_string())?
                .with_buffer_size(rodio::cpal::BufferSize::Fixed(4096))
                .with_error_callback(|error| eprintln!("[native audio] {error}"))
                .open_sink_or_fallback()
                .map_err(|error| error.to_string())?;
            let player = Player::connect_new(device.mixer());
            *output = Some(AudioOutput {
                _device: device,
                player,
                duration,
                loaded: false,
            });
        }

        let output = output.as_mut().expect("audio output was initialized");
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
        self.with_output(|output| {
            output.player.play();
            Ok(output.status())
        })
    }

    pub fn pause(&self) -> Result<AudioStatus, String> {
        self.with_output(|output| {
            output.player.pause();
            Ok(output.status())
        })
    }

    pub fn seek(&self, position: f64) -> Result<AudioStatus, String> {
        self.with_output(|output| {
            output
                .player
                .try_seek(Duration::from_secs_f64(position.max(0.0)))
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
        self.with_output(|output| Ok(output.status()))
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut output = self.0.lock().map_err(|error| error.to_string())?;
        if let Some(output) = output.as_mut() {
            output.player.stop();
            output.loaded = false;
        }
        Ok(())
    }

    fn with_output<T>(
        &self,
        operation: impl FnOnce(&mut AudioOutput) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut output = self.0.lock().map_err(|error| error.to_string())?;
        let output = output
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
