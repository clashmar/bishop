use crate::ecs::{CurrentRoom, Ecs, Transform, set_parent};
use crate::worlds::{RoomId, RoomLayer};

#[test]
fn set_current_room_layer_updates_room_and_layer_for_descendants() {
    let mut ecs = Ecs::default();
    let root = ecs.create_entity().with(Transform::default()).finish();
    let child = ecs.create_entity().with(Transform::default()).finish();
    let grandchild = ecs.create_entity().with(Transform::default()).finish();
    set_parent(&mut ecs, child, root);
    set_parent(&mut ecs, grandchild, child);
    let old_room = RoomId(1);
    let new_room = RoomId(2);
    for entity in [root, child, grandchild] {
        ecs.set_current_room_layer(entity, old_room, RoomLayer::Front);
    }

    ecs.set_current_room_layer(root, new_room, RoomLayer::Back);

    for entity in [root, child, grandchild] {
        assert_eq!(
            ecs.get::<CurrentRoom>(entity),
            Some(&CurrentRoom::new(new_room, RoomLayer::Back))
        );
        assert!(ecs.entities_in_room(new_room).contains(&entity));
        assert!(ecs.entities_in_room_layer(new_room, RoomLayer::Back).contains(&entity));
        assert!(!ecs.entities_in_room(old_room).contains(&entity));
        assert!(!ecs.entities_in_room_layer(old_room, RoomLayer::Front).contains(&entity));
    }
}

#[test]
fn set_current_room_layer_repairs_descendants_when_root_already_matches() {
    let mut ecs = Ecs::default();
    let root = ecs.create_entity().finish();
    let child = ecs.create_entity().finish();
    set_parent(&mut ecs, child, root);
    let room_id = RoomId(1);
    ecs.set_current_room_layer(root, room_id, RoomLayer::Back);
    ecs.set_current_room_layer(child, room_id, RoomLayer::Front);

    ecs.set_current_room_layer(root, room_id, RoomLayer::Back);

    assert_eq!(
        ecs.get::<CurrentRoom>(child),
        Some(&CurrentRoom::new(room_id, RoomLayer::Back))
    );
}
