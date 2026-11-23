use super::*;
use crate::shared::scene_ui::inspector::test_utils::FakeInspectorModule;

#[test]
fn inspector_nested_details_report_first_expanded_component_bounds() {
    let mut expanded_rect = None;
    let mut nested_module = FakeInspectorModule::new(Some(false));
    let rect = Rect::new(10.0, 20.0, 100.0, 24.0);
    let previous_expansion = nested_module.expansion_state();
    nested_module.set_expanded(true);

    capture_expanded_module_rect(
        &mut expanded_rect,
        &nested_module,
        previous_expansion,
        rect,
    );
    assert_eq!(
        expanded_rect,
        Some(Rect::new(rect.x, rect.y, rect.w, nested_module.height()))
    );

    let mut another_module = FakeInspectorModule::new(Some(false));
    let previous_expansion = another_module.expansion_state();
    another_module.set_expanded(true);
    capture_expanded_module_rect(
        &mut expanded_rect,
        &another_module,
        previous_expansion,
        Rect::new(10.0, 80.0, 100.0, 24.0),
    );
    assert_eq!(
        expanded_rect,
        Some(Rect::new(rect.x, rect.y, rect.w, nested_module.height()))
    );
}

#[test]
fn collapse_all_collapses_nested_component_modules() {
    let mut details = TilemapDetailsModule::new();
    details.component_editor.modules = vec![
        Box::new(FakeInspectorModule::new(Some(true))),
        Box::new(FakeInspectorModule::new(None)),
    ];

    details.collapse_all();

    assert_eq!(
        details.component_editor.modules[0].expansion_state(),
        Some(false)
    );
    assert_eq!(
        details.component_editor.modules[1].expansion_state(),
        None
    );
}
