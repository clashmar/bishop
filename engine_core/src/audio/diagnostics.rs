use oddio::Frames;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Snapshot entry for a single sound ID.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AudioDiagnosticsEntry {
    /// Sound identifier.
    pub id: String,
    /// Whether the sound is cached.
    pub cached: bool,
    /// Whether the sound is currently loading in the background.
    pub loading: bool,
    /// Whether the sound is pinned against eviction.
    pub pinned: bool,
    /// Reference count tracked for the sound.
    pub ref_count: usize,
}

/// Snapshot of audio cache and reference state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AudioDiagnosticsSnapshot {
    /// Number of cached sounds.
    pub cached_sound_count: usize,
    /// Number of sounds currently loading.
    pub loading_sound_count: usize,
    /// Number of pinned sounds.
    pub pinned_sound_count: usize,
    /// Number of reference-count entries.
    pub ref_count_entry_count: usize,
    /// Snapshot entries, sorted by sound ID.
    pub entries: Vec<AudioDiagnosticsEntry>,
}

pub(crate) fn snapshot_from_state(
    sound_cache: &HashMap<String, Arc<Frames<[f32; 2]>>>,
    ref_counts: &HashMap<String, usize>,
    pinned: &HashSet<String>,
    loading: &HashSet<String>,
) -> AudioDiagnosticsSnapshot {
    let mut ids: HashSet<String> = HashSet::new();
    ids.extend(sound_cache.keys().cloned());
    ids.extend(ref_counts.keys().cloned());
    ids.extend(pinned.iter().cloned());
    ids.extend(loading.iter().cloned());

    let mut entries = ids
        .into_iter()
        .map(|id| AudioDiagnosticsEntry {
            cached: sound_cache.contains_key(&id),
            loading: loading.contains(&id),
            pinned: pinned.contains(&id),
            ref_count: ref_counts.get(&id).copied().unwrap_or(0),
            id,
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.id.cmp(&right.id));

    AudioDiagnosticsSnapshot {
        cached_sound_count: sound_cache.len(),
        loading_sound_count: loading.len(),
        pinned_sound_count: pinned.len(),
        ref_count_entry_count: ref_counts.len(),
        entries,
    }
}
