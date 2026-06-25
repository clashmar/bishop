use super::*;
use crate::audio::loader::{decode_wav_bytes, wav_path};
use crate::hydration::{EvictError, Hydratable};

impl AudioManager {
    pub(super) fn cached_frames(&self, id: &str) -> Option<Arc<Frames<[f32; 2]>>> {
        self.sound_cache.get(id).cloned()
    }

    /// Returns a cached sound if one is available, otherwise queues a background file read.
    pub(super) fn load_or_cached(&mut self, id: &str) -> Option<Arc<Frames<[f32; 2]>>> {
        if let Some(frames) = self.cached_frames(id) {
            return Some(frames);
        }
        self.queue_sound_load(id);
        None
    }

    pub(super) fn queue_sound_load(&mut self, id: &str) {
        if self.sound_cache.contains_key(id) || self.pending_loads.contains_key(id) {
            return;
        }

        let path = wav_path(id);
        self.pending_loads.insert(id.to_owned(), path.clone());
        #[cfg(test)]
        let _ = &path;
        #[cfg(not(test))]
        self.file_read_pool.queue_read(id.to_owned(), path);
    }

    pub(super) fn poll_pending_loads(&mut self) {
        while let Some(completed) = self.file_read_pool.try_recv_completed() {
            let crate::task::FileReadCompleted { id, path, result } = completed;
            if self.pending_loads.remove(&id).is_none() {
                continue;
            }

            match result {
                Ok(bytes) => match decode_wav_bytes(&path, &bytes) {
                    Ok(frames) => self.finish_sound_load(id, frames),
                    Err(error) => self.fail_sound_load(id, error),
                },
                Err(error) => self.fail_sound_load(id, error),
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn complete_load_for_test(&mut self, id: &str, frames: Arc<Frames<[f32; 2]>>) {
        self.pending_loads.remove(id);
        self.finish_sound_load(id.to_owned(), frames);
    }

    #[cfg(test)]
    pub(crate) fn fail_load_for_test(&mut self, id: &str, error: &str) {
        self.pending_loads.remove(id);
        self.fail_sound_load(id.to_owned(), error.to_owned());
    }

    /// Preloads a sound into the cache without playing it.
    pub(super) fn preload(&mut self, id: &str) {
        self.queue_sound_load(id);
        self.increment_refs(&[id.to_owned()]);
    }

    /// Claims a sound for hydration-managed residency.
    pub(crate) fn claim_sound(&mut self, id: &str) {
        self.increment_refs(&[id.to_owned()]);
    }

    /// Releases a hydration-managed sound claim.
    pub(crate) fn release_claimed_sound(&mut self, id: &str) {
        self.decrement_refs(&[id.to_owned()]);
    }

    /// Evicts a sound from the cache if it has no active references.
    pub(super) fn evict(&mut self, id: &str) {
        if self.ref_counts.get(id).copied().unwrap_or(0) == 0 {
            self.sound_cache.remove(id);
        }
    }

    /// Increments reference counts for the given IDs, loading each sound if not already cached.
    pub(crate) fn increment_refs(&mut self, ids: &[String]) {
        for id in ids {
            *self.ref_counts.entry(id.to_owned()).or_insert(0) += 1;
            self.queue_sound_load(id);
        }
    }

    /// Decrements reference counts for the given IDs. Evicts sounds whose count reaches zero.
    pub(crate) fn decrement_refs(&mut self, ids: &[String]) {
        for id in ids {
            let reached_zero = if let Some(count) = self.ref_counts.get_mut(id.as_str()) {
                *count = count.saturating_sub(1);
                *count == 0
            } else {
                false
            };
            if reached_zero {
                self.ref_counts.remove(id.as_str());
                self.evict(id);
            }
        }
    }

    fn finish_sound_load(&mut self, id: String, frames: Arc<Frames<[f32; 2]>>) {
        self.sound_cache.insert(id, frames);
    }

    fn fail_sound_load(&mut self, id: String, error: String) {
        self.clear_pending_requests_for_sound(&id);
        crate::omni_log!(
            log::Level::Error,
            "AudioManager: failed to load '{id}': {error}"
        );
    }
}

impl Hydratable for AudioManager {
    type Id = String;

    /// Returns the reference count for a sound.
    fn ref_count(&self, id: &Self::Id) -> usize {
        self.ref_counts.get(id).copied().unwrap_or(0)
    }

    /// Increments the reference count.
    fn increment_ref(&mut self, id: Self::Id) {
        self.increment_refs(&[id]);
    }

    /// Decrements the reference count.
    fn decrement_ref(&mut self, id: Self::Id) {
        self.decrement_refs(&[id]);
    }

    /// Attempts eviction. Fails if the ref-count is above zero.
    fn evict(&mut self, id: &Self::Id) -> Result<(), EvictError> {
        let count = self.ref_count(id);
        if count > 0 {
            return Err(EvictError::StillReferenced { count });
        }
        self.evict(id);
        Ok(())
    }
}
