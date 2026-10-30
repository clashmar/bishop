use super::AudioManager;
use crate::audio::test_utils::TestBackend;
use crate::constants::paths;
use crate::storage::{ResourceEntry, ResourceManifest, ResourceSource};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[test]
fn claimed_sound_when_wav_is_prefetched_decodes_without_file_read_pool() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(file.path(), spec).unwrap();
    writer.write_sample(12_000_i16).unwrap();
    writer.finalize().unwrap();
    let bytes = fs::read(file.path()).unwrap();
    let relative = Path::new(paths::AUDIO_FOLDER).join("sfx/probe.wav");
    let key = relative.components().map(|part| part.as_os_str().to_str().unwrap())
        .collect::<Vec<_>>().join("/");
    let manifest = ResourceManifest { files: vec![ResourceEntry {
        path: key.clone(), byte_len: bytes.len(),
    }] };
    let source = ResourceSource::from_prefetched(
        manifest, HashMap::from([(key, bytes)]),
    ).unwrap();
    let mut audio = AudioManager::new::<TestBackend>();
    audio.set_source(source);

    audio.claim_sound("sfx/probe");

    let snapshot = audio.diagnostics_snapshot();
    assert_eq!(snapshot.cached_sound_count, 1);
    assert_eq!(snapshot.loading_sound_count, 0);
    assert_eq!(snapshot.entries[0].ref_count, 1);
}
