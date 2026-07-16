pub mod draw;
pub mod overlay;

/// Dev tools state for playtest debugging.
pub struct DevTools {
    /// Whether collider outlines are visible.
    pub colliders_visible: bool,
}

impl DevTools {
    /// Create a new DevTools instance with all tools off.
    pub fn new() -> Self {
        Self {
            colliders_visible: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_all_tools_off() {
        let tools = DevTools::new();
        assert!(!tools.colliders_visible);
    }

    #[test]
    fn toggle_colliders_flips_visibility() {
        let mut tools = DevTools::new();
        tools.colliders_visible = !tools.colliders_visible;
        assert!(tools.colliders_visible);
        tools.colliders_visible = !tools.colliders_visible;
        assert!(!tools.colliders_visible);
    }
}
