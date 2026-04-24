use std::path::{Path, PathBuf};

/// Tracks the current directory path and supports navigating in/out of directories.
pub struct Navigation {
    stack: Vec<PathBuf>,
}

impl Navigation {
    /// Creates a new navigation starting at the given root path.
    pub fn new(root: PathBuf) -> Self {
        Self { stack: vec![root] }
    }

    /// Returns the current directory path.
    pub fn current(&self) -> &Path {
        self.stack
            .last()
            .expect("navigation stack is empty")
            .as_path()
    }

    /// Navigates into the given subdirectory. Pushes it onto the stack.
    pub fn push(&mut self, dir_name: &str) {
        let next = self.current().join(dir_name);
        self.stack.push(next);
    }

    /// Navigates back to the parent directory. Returns true if we went back,
    /// false if we're already at root.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() <= 1 {
            return false;
        }
        self.stack.pop();
        true
    }

    /// Returns true if we are at the root directory (can't go back further).
    pub fn is_at_root(&self) -> bool {
        self.stack.len() == 1
    }
}
