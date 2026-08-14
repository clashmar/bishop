pub mod draw;
pub mod overlay;

/// Dev tools state for playtest debugging.
#[derive(Default)]
pub struct DevTools {
    /// Whether collider outlines are visible.
    pub colliders_visible: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_defaults_all_tools_off() {
        let tools = DevTools::default();
        assert!(!tools.colliders_visible);
    }

    #[test]
    fn toggle_colliders_flips_visibility() {
        let mut tools = DevTools::default();
        tools.colliders_visible = !tools.colliders_visible;
        assert!(tools.colliders_visible);
        tools.colliders_visible = !tools.colliders_visible;
        assert!(!tools.colliders_visible);
    }
}
