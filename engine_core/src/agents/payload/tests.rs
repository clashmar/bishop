use super::*;
use crate::camera::game_camera::{CameraMode, RoomCamera, ZoomMode};
use crate::ecs::component::comp_type_name;
use crate::ecs::transform::{Pivot, Transform};
use crate::worlds::room::RoomId;
use bishop::prelude::Vec2;
use mlua::Lua;

fn transform_ron(position: Vec2, pivot: Pivot) -> String {
    ron::to_string(&Transform {
        visible: true,
        position,
        pivot,
    })
    .unwrap()
}

fn room_camera_ron(room_id: RoomId) -> String {
    ron::to_string(&RoomCamera {
        zoom: Vec2::new(0.015625, 0.027777778),
        room_id,
        zoom_mode: ZoomMode::Step,
        camera_mode: CameraMode::Fixed,
    })
    .unwrap()
}

#[test]
fn agent_payload_builder_rejects_unknown_component_type() {
    let payload = AgentPayloadSpec::synthetic("TestGame");
    let result = payload
        .add_entity("Player")
        .attach_component("Player", "NoSuchComponent", "()")
        .build();

    assert!(result.is_err());
}

#[test]
fn agent_payload_builder_can_build_seeded_and_synthetic_payloads() {
    let player_transform = transform_ron(Vec2::ZERO, Pivot::BottomCenter);
    let synthetic = AgentPayloadSpec::synthetic("TestGame")
        .add_room("TestRoom")
        .add_entity("Player")
        .attach_component("Player", "Transform", &player_transform)
        .attach_script("Player", "player_main")
        .build()
        .unwrap();

    assert_eq!(synthetic.game.name, "TestGame");
    assert_eq!(synthetic.room.name, "TestRoom");
}

#[test]
fn built_agent_payload_can_materialize_camera_and_player_entities() {
    let lua = Lua::new();
    let room_id = RoomId(1);
    let camera_transform = transform_ron(Vec2::new(64.0, 36.0), Pivot::CenterLeft);
    let camera_component = room_camera_ron(room_id);
    let player_transform = transform_ron(Vec2::new(28.0, 72.0), Pivot::BottomCenter);
    let built = AgentPayloadSpec::synthetic("TestGame")
        .add_room("TestRoom")
        .add_entity("Camera 1")
        .attach_component("Camera 1", comp_type_name::<Transform>(), &camera_transform)
        .attach_component(
            "Camera 1",
            comp_type_name::<RoomCamera>(),
            &camera_component,
        )
        .add_entity("Player")
        .attach_component("Player", comp_type_name::<Transform>(), &player_transform)
        .build()
        .unwrap();

    let materialized = built.materialize(&lua).unwrap();
    let names = materialized.game.ecs.get_store::<Name>();
    let transforms = materialized.game.ecs.get_store::<Transform>();
    let cameras = materialized.game.ecs.get_store::<RoomCamera>();

    let camera_entity = names
        .data
        .iter()
        .find_map(|(&entity, name)| (name.0 == "Camera 1").then_some(entity))
        .unwrap();
    let player_entity = names
        .data
        .iter()
        .find_map(|(&entity, name)| (name.0 == "Player").then_some(entity))
        .unwrap();

    assert_eq!(
        transforms.get(camera_entity).unwrap().pivot,
        Pivot::CenterLeft
    );
    assert_eq!(
        transforms.get(player_entity).unwrap().pivot,
        Pivot::BottomCenter
    );
    assert!(cameras.get(camera_entity).is_some());
}
