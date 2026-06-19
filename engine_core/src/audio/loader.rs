use crate::constants::extensions::WAV;
use crate::constants::paths::AUDIO_FOLDER;
use crate::storage::path_utils::audio_folder;
use oddio::Frames;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// Loads a WAV file by sound ID and decodes it to stereo f32 PCM frames
/// suitable for oddio playback.
///
/// The ID is a path relative to the `audio/` folder without extension,
/// e.g. `"sfx/jump"` resolves to `Resources/audio/sfx/jump.wav`.
pub fn load_wav(id: &str) -> Result<Arc<Frames<[f32; 2]>>, String> {
    let path = wav_path(id);
    let bytes =
        std::fs::read(&path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    decode_wav_bytes(&path, &bytes)
}

/// Returns the WAV path for the given sound ID.
pub fn wav_path(id: &str) -> PathBuf {
    audio_folder().join(id).with_extension(WAV)
}

/// Returns the runtime sound ID for an audio asset path.
pub fn sound_id_from_asset_path(path: &Path) -> Option<String> {
    if path.extension().is_none_or(|ext| ext != WAV) {
        return None;
    }

    let stemless = path.with_extension("");
    let relative = match stemless.components().next() {
        Some(Component::Normal(segment)) if segment == AUDIO_FOLDER => stemless.strip_prefix(AUDIO_FOLDER).ok()?,
        _ => stemless.as_path(),
    };

    Some(relative.to_string_lossy().trim_start_matches('/').replace('\\', "/"))
}

/// Decodes WAV bytes from `path` into stereo f32 PCM frames.
pub fn decode_wav_bytes(path: &Path, bytes: &[u8]) -> Result<Arc<Frames<[f32; 2]>>, String> {
    let cursor = Cursor::new(bytes);
    let reader = hound::WavReader::new(cursor)
        .map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    decode_wav_reader(path, reader)
}

fn decode_wav_reader<R: std::io::Read + std::io::Seek>(
    path: &Path,
    mut reader: hound::WavReader<R>,
) -> Result<Arc<Frames<[f32; 2]>>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_id_from_asset_path_strips_wav_extension() {
        let path = Path::new("music/intro.wav");
        assert_eq!(
            sound_id_from_asset_path(path),
            Some("music/intro".to_string())
        );
    }

    #[test]
    fn sound_id_from_asset_path_strips_audio_prefix() {
        let path = Path::new("audio/music/intro.wav");
        assert_eq!(
            sound_id_from_asset_path(path),
            Some("music/intro".to_string())
        );
    }
}
