use crate::storage::path_utils::audio_folder;
use oddio::Frames;
use std::sync::Arc;

/// Loads a WAV file by sound ID and decodes it to stereo f32 PCM frames
/// suitable for oddio playback.
///
/// The ID is a path relative to the `audio/` folder without extension,
/// e.g. `"sfx/jump"` resolves to `Resources/audio/sfx/jump.wav`.
pub fn load_wav(id: &str) -> Result<Arc<Frames<[f32; 2]>>, String> {
    let path = audio_folder().join(id).with_extension("wav");
    let mut reader = hound::WavReader::open(&path)
        .map_err(|e| format!("failed to open {}: {e}", path.display()))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels as usize;

    // Decode all samples to f32, regardless of source bit depth.
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max).map_err(|e| e.to_string()))
                .collect::<Result<_, _>>()?
        }
    };

    // Interleave or mix down to stereo [f32; 2] frames.
    let frames: Vec<[f32; 2]> = match channels {
        1 => samples.iter().map(|&s| [s, s]).collect(),
        2 => samples.chunks_exact(2).map(|c| [c[0], c[1]]).collect(),
        n => {
            return Err(format!(
                "unsupported channel count {n} in {}",
                path.display()
            ));
        }
    };

    Ok(Frames::from_slice(sample_rate, &frames))
}
