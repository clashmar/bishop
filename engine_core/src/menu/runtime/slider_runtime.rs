use widgets::{HOLD_INITIAL_DELAY, HOLD_REPEAT_RATE};

/// Direction for adjusting a focused slider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SliderAdjustmentDirection {
    Decrease,
    Increase,
}

/// Tracks hold-to-repeat state for focused slider adjustments.
#[derive(Debug, Clone, Default)]
pub(crate) struct SliderRepeatState {
    active_direction: Option<SliderAdjustmentDirection>,
    last_step_time: f64,
    repeat_started: bool,
}

impl SliderRepeatState {
    /// Clears any active repeat tracking.
    pub(crate) fn reset(&mut self) {
        self.active_direction = None;
        self.last_step_time = 0.0;
        self.repeat_started = false;
    }

    /// Returns the adjustment to apply this frame, if any.
    pub(crate) fn next_adjustment(
        &mut self,
        now: f64,
        decrease_pressed: bool,
        decrease_down: bool,
        increase_pressed: bool,
        increase_down: bool,
    ) -> Option<SliderAdjustmentDirection> {
        if decrease_pressed {
            self.active_direction = Some(SliderAdjustmentDirection::Decrease);
            self.last_step_time = now;
            self.repeat_started = false;
            return Some(SliderAdjustmentDirection::Decrease);
        }

        if increase_pressed {
            self.active_direction = Some(SliderAdjustmentDirection::Increase);
            self.last_step_time = now;
            self.repeat_started = false;
            return Some(SliderAdjustmentDirection::Increase);
        }

        let direction = self.active_direction?;

        let still_down = match direction {
            SliderAdjustmentDirection::Decrease => decrease_down,
            SliderAdjustmentDirection::Increase => increase_down,
        };

        if !still_down {
            self.reset();
            return None;
        }

        let elapsed = now - self.last_step_time;
        if (!self.repeat_started && elapsed >= HOLD_INITIAL_DELAY)
            || (self.repeat_started && elapsed >= HOLD_REPEAT_RATE)
        {
            self.last_step_time = now;
            self.repeat_started = true;
            Some(direction)
        } else {
            None
        }
    }
}

/// Returns the adjusted slider value if the step changed it.
pub(crate) fn adjust_slider_value(
    current: f32,
    step: f32,
    min: f32,
    max: f32,
    direction: SliderAdjustmentDirection,
) -> Option<f32> {
    let new_value = match direction {
        SliderAdjustmentDirection::Decrease => (current - step).max(min),
        SliderAdjustmentDirection::Increase => (current + step).min(max),
    };

    if (new_value - current).abs() <= f32::EPSILON {
        None
    } else {
        Some(new_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_repeat_steps_immediately_then_repeats_after_delays() {
        let mut state = SliderRepeatState::default();

        assert_eq!(
            state.next_adjustment(1.0, true, true, false, false),
            Some(SliderAdjustmentDirection::Decrease)
        );
        assert_eq!(state.next_adjustment(1.3, false, true, false, false), None);
        assert_eq!(
            state.next_adjustment(1.5, false, true, false, false),
            Some(SliderAdjustmentDirection::Decrease)
        );
        assert_eq!(state.next_adjustment(1.54, false, true, false, false), None);
        assert_eq!(
            state.next_adjustment(1.55, false, true, false, false),
            Some(SliderAdjustmentDirection::Decrease)
        );
    }

    #[test]
    fn slider_repeat_resets_when_direction_changes() {
        let mut state = SliderRepeatState::default();

        assert_eq!(
            state.next_adjustment(1.0, true, true, false, false),
            Some(SliderAdjustmentDirection::Decrease)
        );
        assert_eq!(
            state.next_adjustment(1.1, false, false, true, true),
            Some(SliderAdjustmentDirection::Increase)
        );
        assert_eq!(state.next_adjustment(1.5, false, false, false, true), None);
        assert_eq!(
            state.next_adjustment(1.6, false, false, false, true),
            Some(SliderAdjustmentDirection::Increase)
        );
    }

    #[test]
    fn slider_repeat_stops_when_input_is_released() {
        let mut state = SliderRepeatState::default();

        assert_eq!(
            state.next_adjustment(1.0, false, false, true, true),
            Some(SliderAdjustmentDirection::Increase)
        );
        assert_eq!(state.next_adjustment(1.1, false, false, false, false), None);
        assert_eq!(state.next_adjustment(1.7, false, false, false, false), None);
    }

    #[test]
    fn adjust_slider_value_returns_none_when_already_clamped() {
        assert_eq!(
            adjust_slider_value(0.0, 0.1, 0.0, 1.0, SliderAdjustmentDirection::Decrease),
            None
        );
        assert_eq!(
            adjust_slider_value(1.0, 0.1, 0.0, 1.0, SliderAdjustmentDirection::Increase),
            None
        );
    }

    #[test]
    fn adjust_slider_value_returns_new_value_when_step_changes_value() {
        assert_eq!(
            adjust_slider_value(0.5, 0.1, 0.0, 1.0, SliderAdjustmentDirection::Decrease),
            Some(0.4)
        );
        assert_eq!(
            adjust_slider_value(0.5, 0.1, 0.0, 1.0, SliderAdjustmentDirection::Increase),
            Some(0.6)
        );
    }
}
