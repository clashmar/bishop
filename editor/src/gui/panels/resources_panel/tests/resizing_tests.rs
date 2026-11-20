use super::super::{ResourcesPanel, RESOURCES_PANEL_MIN_HEIGHT, RESOURCES_PANEL_MIN_WIDTH};
use crate::gui::panels::generic_panel::PanelDefinition;

#[test]
fn resources_panel_has_resizable_panel_limits() {
    let panel = ResourcesPanel::new();
    let Some(limits) = panel.resize_limits() else {
        panic!("Resources panel should support resizing");
    };

    assert_eq!(
        (limits.min_width, limits.min_height),
        (RESOURCES_PANEL_MIN_WIDTH, RESOURCES_PANEL_MIN_HEIGHT)
    );
}
