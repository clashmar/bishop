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
}
