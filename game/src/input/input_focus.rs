// game/src/input/input_focus.rs
use std::collections::HashMap;

/// Priority constants for the input focus system.
pub mod focus_priority {
    /// Default player priority — always registered.
    pub const PLAYER: u8 = 0;
    /// Priority for dialogue systems.
    pub const DIALOGUE: u8 = 5;
    /// Priority for menus.
    pub const MENU: u8 = 10;
}

/// Tracks which system currently has input focus.
///
/// The system with the highest registered priority owns input.
/// `"player"` is pre-registered at [`focus_priority::PLAYER`] and acts as the
/// default baseline.
pub struct InputFocusMap {
    map: HashMap<String, u8>,
}

impl Default for InputFocusMap {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert("player".to_string(), focus_priority::PLAYER);
        Self { map }
    }
}

impl InputFocusMap {
    /// Registers `name` with the given `priority`, taking control from any lower-priority entry.
    pub fn take_control(&mut self, name: &str, priority: u8) {
        self.map.insert(name.to_string(), priority);
    }

    /// Removes `name` from the focus map, relinquishing any control it held.
    pub fn release_control(&mut self, name: &str) {
        self.map.remove(name);
    }

    /// Returns `true` if `name` currently holds the highest priority in the map.
    pub fn in_control(&self, name: &str) -> bool {
        let Some(&own_priority) = self.map.get(name) else {
            return false;
        };
        self.map.values().all(|&p| p <= own_priority)
    }

    /// Returns the name of the highest-priority registered entry.
    pub fn active_controller(&self) -> Option<&str> {
        self.map
            .iter()
            .max_by_key(|(_, &p)| p)
            .map(|(name, _)| name.as_str())
    }
}
