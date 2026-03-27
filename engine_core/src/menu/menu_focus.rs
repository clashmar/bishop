use crate::menu::*;

/// Navigation direction for menu focus movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Tracks the current focus position in a menu.
///
/// Uses a two-level scheme: `node` indexes a top-level element (button, slider, or layout group),
/// and `child` optionally indexes a focusable child within a layout group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuFocus {
    /// Element index in the template.
    pub node: usize,
    /// Child index within a layout group (None for standalone buttons or sliders).
    pub child: Option<usize>,
}

impl MenuFocus {
    /// Creates a new focus state pointing at the given node.
    pub fn new(node: usize) -> Self {
        Self { node, child: None }
    }

    /// Resets focus to the first focusable element in the template.
    pub fn reset(&mut self, template: &MenuTemplate) {
        self.node = 0;
        self.child = None;

        for (i, element) in template.elements.iter().enumerate() {
            if !element.enabled || !element.visible {
                continue;
            }
            match &element.kind {
                MenuElementKind::Button(_) | MenuElementKind::Slider(_) => {
                    self.node = i;
                    return;
                }
                MenuElementKind::LayoutGroup(_) => {
                    if template.focusable_child_count(i) > 0 {
                        self.node = i;
                        self.child = Some(0);
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    /// Navigates focus in the given direction based on the template structure.
    pub fn navigate(&mut self, dir: NavDirection, template: &MenuTemplate) {
        let Some(element) = template.elements.get(self.node) else {
            return;
        };

        match &element.kind {
            MenuElementKind::Button(button) => {
                if let Some(target_idx) = Self::get_nav_target(&button.nav_targets, dir) {
                    self.enter_element(target_idx, dir, template);
                }
            }
            MenuElementKind::Slider(slider) => {
                if let Some(target_idx) = Self::get_nav_target(&slider.nav_targets, dir) {
                    self.enter_element(target_idx, dir, template);
                }
            }
            MenuElementKind::LayoutGroup(group) => {
                let is_along_axis = Self::direction_along_axis(dir, group.layout.direction);

                if is_along_axis {
                    self.navigate_within_group(dir, group, template);
                } else {
                    let target = Self::group_nav_field(dir, group);
                    if let Some(target_idx) = target {
                        self.enter_element(target_idx, dir, template);
                    }
                }
            }
            _ => {}
        }
    }

    /// Returns true if the direction aligns with the layout direction's primary axis.
    fn direction_along_axis(dir: NavDirection, layout_dir: LayoutDirection) -> bool {
        match layout_dir {
            LayoutDirection::Vertical => matches!(dir, NavDirection::Up | NavDirection::Down),
            LayoutDirection::Horizontal => matches!(dir, NavDirection::Left | NavDirection::Right),
            LayoutDirection::Grid { .. } => true,
        }
    }

    /// Returns the navigation target index for the given direction.
    fn get_nav_target(targets: &NavTargets, dir: NavDirection) -> Option<usize> {
        match dir {
            NavDirection::Up => targets.up,
            NavDirection::Down => targets.down,
            NavDirection::Left => targets.left,
            NavDirection::Right => targets.right,
        }
    }

    /// Gets the nav field for a direction from a layout group.
    fn group_nav_field(dir: NavDirection, group: &LayoutGroupElement) -> Option<usize> {
        Self::get_nav_target(&group.nav_targets, dir)
    }

    /// Navigates within a layout group along its primary axis.
    fn navigate_within_group(
        &mut self,
        dir: NavDirection,
        group: &LayoutGroupElement,
        template: &MenuTemplate,
    ) {
        let child_count = template.focusable_child_count(self.node);
        if child_count == 0 {
            return;
        }

        let current_child = self.child.unwrap_or(0);
        let is_forward = matches!(dir, NavDirection::Down | NavDirection::Right);

        if is_forward {
            if current_child + 1 < child_count {
                self.child = Some(current_child + 1);
            } else {
                // At boundary, try exit nav or wrap to start
                let target = Self::group_nav_field(dir, group);
                if let Some(target_idx) = target {
                    self.enter_element(target_idx, dir, template);
                } else {
                    self.child = Some(0);
                }
            }
        } else if current_child > 0 {
            self.child = Some(current_child - 1);
        } else {
            // At boundary, try exit nav or wrap to end
            let target = Self::group_nav_field(dir, group);
            if let Some(target_idx) = target {
                self.enter_element(target_idx, dir, template);
            } else {
                self.child = Some(child_count - 1);
            }
        }
    }

    /// Enters an element, setting up child focus if it's a layout group.
    fn enter_element(&mut self, target_idx: usize, dir: NavDirection, template: &MenuTemplate) {
        let Some(target_element) = template.elements.get(target_idx) else {
            return;
        };

        if !target_element.enabled || !target_element.visible {
            return;
        }

        match &target_element.kind {
            MenuElementKind::Button(_) | MenuElementKind::Slider(_) => {
                self.node = target_idx;
                self.child = None;
            }
            MenuElementKind::LayoutGroup(_) => {
                let child_count = template.focusable_child_count(target_idx);
                if child_count == 0 {
                    return;
                }
                self.node = target_idx;
                // Enter from the appropriate end based on direction
                let enter_from_end = matches!(dir, NavDirection::Up | NavDirection::Left);
                self.child = if enter_from_end {
                    Some(child_count - 1)
                } else {
                    Some(0)
                };
            }
            _ => {}
        }
    }
}
