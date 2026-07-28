mod layer_door;
mod world_exit;

use crate::engine::game_instance::GameInstance;
use engine_core::ecs::component_registry::COMPONENTS;
use engine_core::ecs::entity::Entity;

/// A component's response to an interact event.
pub struct InteractEntry {
    /// Must match the component's `TYPE_NAME`.
    pub type_name: &'static str,
    /// Called when the entity receives an interact event.
    pub on_interact: fn(entity: Entity, game_instance: &mut GameInstance),
}

inventory::collect!(InteractEntry);

/// Dispatches `on_interact` for every component on `entity` that has registered a handler.
pub fn handle_interactions(entity: Entity, game_instance: &mut GameInstance) {
    for entry in inventory::iter::<InteractEntry> {
        let has = {
            let ecs = &game_instance.game.ecs;
            COMPONENTS
                .iter()
                .find(|r| r.type_name == entry.type_name)
                .is_some_and(|r| (r.has)(ecs, entity))
        };
        if has {
            (entry.on_interact)(entity, game_instance);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bishop::prelude::Vec2;
    use crate::engine::game_instance::GameInstance;
    use engine_core::ecs::{CurrentRoom, Interactable, LayerDoor, Player, Transform};
    use engine_core::game::Game;
    use engine_core::worlds::{BackRoomLayer, Room, RoomId, RoomLayer, RoomLayers, RoomVariant, World, WorldId};
    use std::collections::HashMap;

    fn setup_layer_door_game() -> (GameInstance, Entity, Entity) {
        let mut game = Game::default();
        let mut world = World::new(WorldId(1), "Test".to_string(), 16.0);
        let mut room = Room {
            id: RoomId(1),
            ..Default::default()
        };
        room.variants = vec![RoomVariant {
            layers: RoomLayers {
                back: Some(BackRoomLayer::default()),
            },
            ..Default::default()
        }];
        world.add_room(room);
        world.current_room_id = Some(RoomId(1));
        game.add_world(world);

        let player = game
            .ecs
            .create_entity()
            .with(Player)
            .with(Transform::default())
            .with_current_room(RoomId(1))
            .finish();
        let door = game
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Interactable::circle(Vec2::ZERO, 12.0))
            .with(LayerDoor::default())
            .with_current_room(RoomId(1))
            .finish();

        (
            GameInstance {
                game,
                prev_positions: HashMap::new(),
                traversal_residency_diagnostics: None,
            },
            player,
            door,
        )
    }

    #[test]
    fn handle_interactions_toggles_player_layer_through_front_layer_door() {
        let (mut game_instance, player, door) = setup_layer_door_game();

        handle_interactions(door, &mut game_instance);
        assert_eq!(
            game_instance.game.ecs.get::<CurrentRoom>(player).copied(),
            Some(CurrentRoom::new(RoomId(1), RoomLayer::Back))
        );

        handle_interactions(door, &mut game_instance);
        assert_eq!(
            game_instance.game.ecs.get::<CurrentRoom>(player).copied(),
            Some(CurrentRoom::new(RoomId(1), RoomLayer::Front))
        );
    }
}
