use crate::engine::game_instance::GameInstance;
use crate::scripting::interact::InteractEntry;
use engine_core::ecs::entity::Entity;
use engine_core::ecs::{CurrentRoom, LayerDoor};
use engine_core::worlds::RoomLayer;

fn on_interact(entity: Entity, game_instance: &mut GameInstance) {
    let Some(layer_door) = game_instance.game.ecs.get::<LayerDoor>(entity).copied() else {
        return;
    };
    if !layer_door.usable {
        return;
    }

    let Some(door_room) = game_instance.game.ecs.get::<CurrentRoom>(entity).copied() else {
        return;
    };
    let Some(room) = game_instance.game.current_world().get_room(door_room.room_id) else {
        return;
    };
    if room.current_variant().layers.back.is_none() {
        return;
    }

    let Some(player) = game_instance.game.ecs.get_player_entity() else {
        return;
    };
    let Some(player_room) = game_instance.game.ecs.get::<CurrentRoom>(player).copied() else {
        return;
    };
    if player_room.room_id != door_room.room_id {
        return;
    }

    let next_layer = match player_room.layer {
        RoomLayer::Front => RoomLayer::Back,
        RoomLayer::Back => RoomLayer::Front,
    };
    game_instance
        .game
        .ecs
        .set_current_room_layer(player, door_room.room_id, next_layer);
}

inventory::submit! {
    InteractEntry {
        type_name: LayerDoor::TYPE_NAME,
        on_interact,
    }
}
