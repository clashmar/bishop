use std::collections::HashMap;
use std::collections::HashSet;

/// Per-frame virtual input overlay keyed by canonical engine input names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VirtualInputState {
    down: HashMap<&'static str, bool>,
    pressed: HashMap<&'static str, bool>,
    released: HashMap<&'static str, bool>,
    transient_press_down: HashSet<&'static str>,
    transient_release_down: HashSet<&'static str>,
}

impl VirtualInputState {
    /// Builds virtual input with the provided canonical names marked down.
    pub fn from_down_inputs(inputs: impl IntoIterator<Item = &'static str>) -> Self {
        let mut state = Self::default();
        for input in inputs {
            state.down.insert(input, true);
        }

        state
    }

    /// Builds virtual input for a one-frame press of a canonical input name.
    pub fn single_frame_press(input: &'static str) -> Self {
        let mut state = Self::default();
        state.down.insert(input, true);
        state.pressed.insert(input, true);
        state.transient_press_down.insert(input);
        state
    }

    /// Builds virtual input for a one-frame release of a canonical input name.
    pub fn single_frame_release(input: &'static str) -> Self {
        let mut state = Self::default();
        state.down.insert(input, false);
        state.released.insert(input, true);
        state.transient_release_down.insert(input);
        state
    }

    /// Marks a canonical input name as held down or not down.
    pub fn set_down(&mut self, input: &'static str, value: bool) {
        self.down.insert(input, value);
    }

    /// Marks a canonical input name as pressed for the current frame.
    pub fn set_pressed(&mut self, input: &'static str, value: bool) {
        self.pressed.insert(input, value);
        if value {
            self.transient_press_down.insert(input);
        } else {
            self.transient_press_down.remove(input);
        }
    }

    /// Marks a canonical input name as released for the current frame.
    pub fn set_released(&mut self, input: &'static str, value: bool) {
        self.released.insert(input, value);
        if value {
            self.transient_release_down.insert(input);
        } else {
            self.transient_release_down.remove(input);
        }
    }

    /// Returns the virtual down-input map.
    pub fn down(&self) -> &HashMap<&'static str, bool> {
        &self.down
    }

    /// Returns the virtual pressed-input map.
    pub fn pressed(&self) -> &HashMap<&'static str, bool> {
        &self.pressed
    }

    /// Returns the virtual released-input map.
    pub fn released(&self) -> &HashMap<&'static str, bool> {
        &self.released
    }

    /// Clears one-frame pressed and released edges while preserving held down state.
    pub fn clear_edges(&mut self) {
        for input in self.transient_press_down.drain() {
            self.down.remove(input);
        }

        for input in self.transient_release_down.drain() {
            self.down.remove(input);
        }

        self.pressed.clear();
        self.released.clear();
    }
}
