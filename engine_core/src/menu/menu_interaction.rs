use crate::menu::*;
use bishop::prelude::*;
use widgets::{HOLD_INITIAL_DELAY, HOLD_REPEAT_RATE};

/// Direction for adjusting a focused slider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderAdjustmentDirection {
    Decrease,
    Increase,
}

/// A focusable menu target resolved into screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct FocusTarget {
    /// Focus state to apply when this target is selected.
    pub focus: MenuFocus,
    /// Screen-space rectangle that can be hit-tested.
    pub rect: Rect,
}

/// Tracks hold-to-repeat state for focused slider adjustments.
#[derive(Debug, Clone, Default)]
pub struct SliderRepeatState {
    active_direction: Option<SliderAdjustmentDirection>,
    last_step_time: f64,
    repeat_started: bool,
}

impl SliderRepeatState {
    /// Clears any active repeat tracking.
    pub fn reset(&mut self) {
        self.active_direction = None;
        self.last_step_time = 0.0;
        self.repeat_started = false;
    }

    /// Returns the adjustment to apply this frame, if any.
    pub fn next_adjustment(
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

/// Returns all enabled, visible focusable targets in render order.
pub fn collect_focus_targets(
    template: &MenuTemplate,
    canvas_origin: Vec2,
    canvas_size: Vec2,
) -> Vec<FocusTarget> {
    let mut targets = Vec::new();

    for element_index in template.sorted_element_indices() {
        let element = &template.elements[element_index];
        if !element.visible {
            continue;
        }

        match &element.kind {
            MenuElementKind::Button(_) | MenuElementKind::Slider(_) if element.enabled => {
                targets.push(FocusTarget {
                    focus: MenuFocus::new(element_index),
                    rect: normalized_rect_to_screen(element.rect, canvas_origin, canvas_size),
                });
            }
            MenuElementKind::LayoutGroup(group) => {
                let resolved = resolve_layout(group, element.rect);
                let mut child_focus_index = 0;

                for (child, rect) in group.children.iter().zip(resolved.iter()) {
                    if !child.element.visible {
                        continue;
                    }

                    let is_focusable = matches!(
                        child.element.kind,
                        MenuElementKind::Button(_) | MenuElementKind::Slider(_)
                    );

                    if is_focusable && child.element.enabled {
                        targets.push(FocusTarget {
                            focus: MenuFocus {
                                node: element_index,
                                child: Some(child_focus_index),
                            },
                            rect: normalized_rect_to_screen(*rect, canvas_origin, canvas_size),
                        });
                        child_focus_index += 1;
                    }
                }
            }
            _ => {}
        }
    }

    targets
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
    fn collect_focus_targets_includes_top_level_and_layout_children() {
        let mut template = MenuTemplate::new("settings".to_string());
        template.elements.push(MenuElement::label(
            "title".to_string(),
            Rect::new(0.0, 0.0, 1.0, 0.1),
        ));
        template.elements.push(MenuElement::button(
            "resume".to_string(),
            MenuAction::Resume,
            Rect::new(0.1, 0.2, 0.2, 0.1),
        ));
        template.elements.push(MenuElement::layout_group(
            LayoutGroupElement {
                layout: LayoutConfig::vertical()
                    .with_item_size(200.0, 40.0)
                    .with_spacing(10.0),
                children: vec![
                    LayoutChild {
                        element: MenuElement::label(
                            "header".to_string(),
                            Rect::new(0.0, 0.0, 0.0, 0.0),
                        ),
                        managed: true,
                    },
                    LayoutChild {
                        element: MenuElement::button(
                            "apply".to_string(),
                            MenuAction::CloseMenu,
                            Rect::new(0.0, 0.0, 0.0, 0.0),
                        ),
                        managed: true,
                    },
                    LayoutChild {
                        element: MenuElement::slider(
                            "volume".to_string(),
                            "master".to_string(),
                            0.0,
                            1.0,
                            0.1,
                            0.5,
                            Rect::new(0.0, 0.0, 0.0, 0.0),
                        ),
                        managed: true,
                    },
                ],
                ..Default::default()
            },
            Rect::new(0.0, 0.3, 1.0, 0.6),
        ));

        let targets = collect_focus_targets(&template, Vec2::ZERO, Vec2::new(1000.0, 500.0));

        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].focus, MenuFocus::new(1));
        assert_eq!(
            targets[1].focus,
            MenuFocus {
                node: 2,
                child: Some(0),
            }
        );
        assert_eq!(
            targets[2].focus,
            MenuFocus {
                node: 2,
                child: Some(1),
            }
        );
    }
}
