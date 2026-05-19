use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::entity::Entity;
use engine_core::prelude::{Game, RoomId};

/// Moves an entity to a specific room if that room exists.
pub struct MoveToRoomCmd {
    pub entity: Entity,
    pub room_id: RoomId,
}

fn move_entity_to_room(game: &mut Game, entity: Entity, room_id: RoomId) -> bool {
    let room_exists = game
        .current_world()
        .rooms()
        .iter()
        .any(|room| room.id == room_id);

    if !room_exists {
        return false;
    }

    game.ecs.set_current_room(entity, room_id);
    true
}

impl LuaCommand for MoveToRoomCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        if !move_entity_to_room(&mut game_instance.game, self.entity, self.room_id) {
            engine_core::onscreen_error!("Unknown room id {:?}", self.room_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::prelude::{CurrentRoom, Room, World};

    fn game_with_room(room_id: RoomId) -> Game {
        let mut game = Game::default();
        let mut world = World::default();
        world.add_room(Room {
            id: room_id,
            ..Default::default()
        });
        world.current_room_id = Some(room_id);
        game.add_world(world);
        game
    }

    #[test]
    fn move_to_room_command_rejects_unknown_room_id() {
        let mut game = game_with_room(RoomId(7));
        let entity = game.ecs.create_entity().finish();

        let changed = move_entity_to_room(&mut game, entity, RoomId(999));

        assert!(!changed);
        assert!(game.ecs.get::<CurrentRoom>(entity).is_none());
    }
}
