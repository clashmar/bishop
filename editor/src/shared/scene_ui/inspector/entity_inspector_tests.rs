use super::*;
use crate::shared::scene_ui::inspector::test_utils::FakeInspectorModule;

fn entity_inspector_with_modules(modules: Vec<Box<dyn InspectorModule>>) -> EntityInspector {
    EntityInspector {
        target: None,
        modules,
        widget_ids: WidgetIds {
            add_component_dropdown_id: WidgetId::default(),
        },
        component_edits: std::collections::HashMap::new(),
        interactive_rects: Vec::new(),
        command_mode: None,
    }
}

#[test]
fn collapse_all_collapses_each_expandable_entity_module() {
    let mut inspector = entity_inspector_with_modules(vec![
        Box::new(FakeInspectorModule::new(Some(true))),
        Box::new(FakeInspectorModule::new(None)),
        Box::new(FakeInspectorModule::new(Some(true))),
    ]);

    assert!(inspector.has_collapsible_modules());
    inspector.collapse_all();

    assert_eq!(inspector.modules[0].expansion_state(), Some(false));
    assert_eq!(inspector.modules[1].expansion_state(), None);
    assert_eq!(inspector.modules[2].expansion_state(), Some(false));
}

#[test]
fn inspector_expansion_reveal_rect_requires_collapsed_to_expanded_transition() {
    let mut module = FakeInspectorModule::new(Some(false));
    let rect = Rect::new(10.0, 20.0, 100.0, 24.0);

    let previous_state = module.expansion_state();
    module.set_expanded(true);

    assert_eq!(
        module.expansion_reveal_rect(previous_state, rect),
        Some(Rect::new(rect.x, rect.y, rect.w, module.height()))
    );
    assert_eq!(module.expansion_reveal_rect(Some(true), rect), None);
    assert_eq!(module.expansion_reveal_rect(None, rect), None);
}

#[test]
fn entity_inspector_without_collapsible_modules_reports_none() {
    let inspector = entity_inspector_with_modules(vec![Box::new(FakeInspectorModule::new(None))]);

    assert!(!inspector.has_collapsible_modules());
}
