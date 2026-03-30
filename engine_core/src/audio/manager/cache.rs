use super::*;
use crate::audio::loader::{decode_wav_bytes, wav_path};

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

    fn finish_sound_load(&mut self, id: String, frames: Arc<Frames<[f32; 2]>>) {
        self.sound_cache.insert(id, frames);
    }

    fn fail_sound_load(&mut self, id: String, error: String) {
        self.clear_pending_requests_for_sound(&id);
        crate::onscreen_log!(
            log::Level::Error,
            "AudioManager: failed to load '{id}': {error}"
        );
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

    /// Preloads a sound into the cache without playing it and pins it against auto-eviction.
    pub(super) fn preload(&mut self, id: &str) {
        self.queue_sound_load(id);
        self.pinned.insert(id.to_owned());
    }

    /// Evicts a sound from the cache if it is not pinned.
    pub(super) fn evict(&mut self, id: &str) {
        if !self.pinned.contains(id) {
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

    /// Decrements reference counts for the given IDs. Evicts unpinned sounds whose count reaches zero.
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
}
