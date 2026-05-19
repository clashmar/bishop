use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;
use engine_core::ecs::entity::Entity;
use engine_core::prelude::Game;

/// Removes room membership from an entity.
pub struct RemoveFromRoomCmd {
    pub entity: Entity,
}

fn remove_entity_from_room(game: &mut Game, entity: Entity) {
    game.ecs.clear_current_room(entity);
}

impl LuaCommand for RemoveFromRoomCmd {
    fn execute(&mut self, engine: &mut Engine) {
        let mut game_instance = engine.game_instance.borrow_mut();
        remove_entity_from_room(&mut game_instance.game, self.entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::prelude::{CurrentRoom, Room, RoomId, World};

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
    fn remove_from_room_command_clears_membership() {
        let mut game = game_with_room(RoomId(7));
        let entity = game.ecs.create_entity().finish();
        game.ecs.set_current_room(entity, RoomId(7));

        remove_entity_from_room(&mut game, entity);

        assert!(!game.ecs.has::<CurrentRoom>(entity));
        assert!(!game.ecs.entities_in_room(RoomId(7)).contains(&entity));
    }
}
