use crate::audio::AudioBackend;
use bytemuck::cast_slice_mut;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

/// Desktop audio backend using cpal. Holds the output stream alive for the
/// lifetime of the AudioManager. Dropping this stops all audio.
/// `_stream` is `None` when no audio device is available (headless / CI).
pub struct CpalBackend {
    /// Keeps the cpal stream alive. None means audio is disabled (no device found).
    _stream: Option<Stream>,
}

// cpal::Stream is not Send because some platform backends (CoreAudio on macOS/iOS,
// AAudio on Android) require the stream to be dropped on the thread that created it.
// This impl is safe in the current architecture because CpalBackend is created and
// dropped exclusively on the main thread (inside Engine, which is single-threaded).
// If this type is ever moved to a worker thread, this impl must be re-evaluated.
unsafe impl Send for CpalBackend {}

impl AudioBackend for CpalBackend {
    fn start<F: FnMut(&mut [[f32; 2]]) + Send + 'static>(mut render_fn: F) -> Self {
        let host = cpal::default_host();

        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                log::error!("no audio output device found — audio disabled");
                return Self { _stream: None };
            }
        };

        let supported = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                log::error!("no default audio output config: {e} — audio disabled");
                return Self { _stream: None };
            }
        };

        if supported.sample_format() != SampleFormat::F32 {
            log::error!(
                "unsupported sample format {:?} — audio disabled",
                supported.sample_format()
            );
            return Self { _stream: None };
        }

        let config = StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(44100),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = match device.build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                // Cast flat interleaved f32 buffer to stereo frames for oddio.
                // Safety: [f32; 2] has the same layout as two consecutive f32s.
                let frames: &mut [[f32; 2]] = cast_slice_mut(data);
                render_fn(frames);
            },
            |err| log::error!("audio stream error: {err}"),
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                log::error!("failed to build audio output stream: {e} — audio disabled");
                return Self { _stream: None };
            }
        };

        if let Err(e) = stream.play() {
            log::error!("failed to start audio stream: {e} — audio disabled");
            return Self { _stream: None };
        }

        Self {
            _stream: Some(stream),
        }
    }
}
