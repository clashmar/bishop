use engine_core::worlds::RoomLayer;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoomLayerState {
    pub active_layer: RoomLayer,
}

impl RoomLayerState {
    pub fn toggle(&mut self) {
        self.active_layer = match self.active_layer {
            RoomLayer::Front => RoomLayer::Back,
            RoomLayer::Back => RoomLayer::Front,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_switches_front_to_back_and_back_to_front() {
        let mut state = RoomLayerState::default();

        assert_eq!(state.active_layer, RoomLayer::Front);

        state.toggle();
        assert_eq!(state.active_layer, RoomLayer::Back);

        state.toggle();
        assert_eq!(state.active_layer, RoomLayer::Front);
    }
}
