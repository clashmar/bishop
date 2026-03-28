use super::*;

impl AudioManager {
    /// Loads sound `id` from disk if not cached, returning a shared reference.
    pub(super) fn load_or_cached(&mut self, id: &str) -> Option<Arc<Frames<[f32; 2]>>> {
        if let Some(frames) = self.sound_cache.get(id) {
            return Some(frames.clone());
        }
        if self.pending_loads.contains_key(id) {
            return None;
        }
        match load_wav(id) {
            Ok(frames) => {
                self.sound_cache.insert(id.to_owned(), frames.clone());
                Some(frames)
            }
            Err(e) => {
                log::error!("AudioManager: failed to load '{id}': {e}");
                None
            }
        }
    }

    pub(super) fn queue_sound_load(&mut self, id: &str) {
        if self.sound_cache.contains_key(id) || self.pending_loads.contains_key(id) {
            return;
        }

        let id = id.to_owned();
        self.pending_loads
            .insert(id.clone(), BackgroundTask::spawn(move || load_wav(&id)));
    }

    fn finish_sound_load(&mut self, id: String, result: Result<Arc<Frames<[f32; 2]>>, String>) {
        match result {
            Ok(frames) => {
                self.sound_cache.insert(id, frames);
            }
            Err(e) => {
                log::error!("AudioManager: failed to load '{id}': {e}");
            }
        }
    }

    pub(super) fn poll_pending_loads(&mut self) {
        let mut completed = Vec::new();

        for (id, task) in &mut self.pending_loads {
            if let Some(result) = task.poll() {
                completed.push((id.clone(), result));
            }
        }

        for (id, result) in completed {
            self.pending_loads.remove(&id);
            self.finish_sound_load(id, result);
        }
    }

    #[cfg(test)]
    pub(crate) fn complete_load_for_test(&mut self, id: &str, frames: Arc<Frames<[f32; 2]>>) {
        self.pending_loads.remove(id);
        self.finish_sound_load(id.to_owned(), Ok(frames));
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
