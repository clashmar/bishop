use super::*;
use crate::shared::scene_ui::inspector::test_utils::FakeInspectorModule;

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
