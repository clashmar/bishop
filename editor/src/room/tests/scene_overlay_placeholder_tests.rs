use super::*;

#[test]
fn placeholder_eligibility_requires_a_visible_root_entity() {
    let mut ecs = Ecs::default();
    let root = ecs
        .create_entity()
        .with(Transform::default())
        .finish();
    let child = ecs
        .create_entity()
        .with(Transform::default())
        .with(Parent(root))
        .finish();
    let invisible_root = ecs
        .create_entity()
        .with(Transform {
            visible: false,
            ..Default::default()
        })
        .finish();

    assert!(ecs
        .get::<Transform>(root)
        .is_some_and(|transform| should_draw_entity_placeholder(&ecs, root, transform)));
    assert!(ecs
        .get::<Transform>(child)
        .is_some_and(|transform| !should_draw_entity_placeholder(&ecs, child, transform)));
    assert!(ecs
        .get::<Transform>(invisible_root)
        .is_some_and(|transform| !should_draw_entity_placeholder(&ecs, invisible_root, transform)));
}
