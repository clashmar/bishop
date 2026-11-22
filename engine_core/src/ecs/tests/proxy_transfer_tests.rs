use bishop::prelude::Vec2;
use crate::ecs::{
    CurrentRoom, Ecs, Player, PlayerProxy, Transform, get_parent, set_parent,
    update_entity_position,
};
use crate::worlds::{RoomId, RoomLayer};

#[test]
fn set_player_spawn_from_proxy_transfers_only_that_proxys_children() {
    let mut ecs = Ecs::default();
    let player = ecs
        .create_entity()
        .with(Player)
        .with(Transform::default())
        .finish();
    let proxy_position = Vec2::new(68.0, 72.0);
    let proxy = ecs
        .create_entity()
        .with(PlayerProxy)
        .with(Transform {
            position: proxy_position,
            ..Default::default()
        })
        .with_current_room_layer(RoomId(1), RoomLayer::Front)
        .finish();
    let emitter_offset = Vec2::new(0.0, 4.0);
    let emitter = ecs
        .create_entity()
        .with(Transform {
            position: proxy_position + emitter_offset,
            ..Default::default()
        })
        .with_current_room_layer(RoomId(1), RoomLayer::Front)
        .finish();
    set_parent(&mut ecs, emitter, proxy);

    let other_proxy = ecs
        .create_entity()
        .with(PlayerProxy)
        .with(Transform {
            position: Vec2::new(120.0, 30.0),
            ..Default::default()
        })
        .with_current_room(RoomId(2))
        .finish();
    let other_child = ecs.create_entity().with(Transform::default()).finish();
    set_parent(&mut ecs, other_child, other_proxy);

    ecs.set_player_spawn_from_proxy(RoomId(1));

    assert_eq!(
        ecs.get::<Transform>(player).map(|transform| transform.position),
        Some(proxy_position)
    );
    assert_eq!(get_parent(&ecs, emitter), Some(player));
    assert_eq!(get_parent(&ecs, other_child), Some(other_proxy));
    assert_eq!(
        ecs.get::<CurrentRoom>(emitter),
        ecs.get::<CurrentRoom>(player)
    );

    let moved_position = proxy_position + Vec2::new(10.0, -6.0);
    update_entity_position(&mut ecs, player, moved_position);
    assert_eq!(
        ecs.get::<Transform>(emitter).map(|transform| transform.position),
        Some(moved_position + emitter_offset)
    );
}
