use engine_core::storage::path_utils::resources_folder_current;
use std::path::PathBuf;

/// Tracks relative directory navigation within Resources/.
/// Stores path segments relative to the Resources root, resolved
/// fresh at scan time via `resources_folder_current()` so the panel
/// works correctly even when constructed before a game is loaded.
pub struct Navigation {
    segments: Vec<String>,
}

impl Navigation {
    /// Creates a new navigation starting at the Resources root.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Returns the current absolute directory path.
    pub fn current(&self) -> PathBuf {
        let mut path = resources_folder_current();
        for seg in &self.segments {
            path = path.join(seg);
        }
        path
    }

    /// Navigates into the given subdirectory.
    pub fn push(&mut self, dir_name: &str) {
        self.segments.push(dir_name.to_string());
    }

    /// Navigates back to the parent directory. Returns true if we went back,
    /// false if we're already at root.
    pub fn pop(&mut self) -> bool {
        if self.segments.is_empty() {
            return false;
        }
        self.segments.pop();
        true
    }

    /// Returns true if we are at the root directory (can't go back further).
    pub fn is_at_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Returns the current navigation depth (0 at root).
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Truncates navigation to the given depth (0 = root).
    pub fn truncate_to(&mut self, depth: usize) {
        self.segments.truncate(depth);
    }

    /// Returns the segment at the given depth index, if it exists.
    pub fn segment(&self, index: usize) -> Option<&str> {
        self.segments.get(index).map(|s| s.as_str())
    }
}
