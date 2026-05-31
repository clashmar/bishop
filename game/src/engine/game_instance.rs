use crate::scripting::script_system::ScriptSystem;
use engine_core::prelude::*;
use mlua::Lua;
use mlua::Value;
use mlua::Variadic;
use std::collections::HashMap;

/// Intermediate result after loading game data but before final runtime assembly.
///
/// The bootstrap pipeline can apply save providers between preparation
/// and finalization, separating PreRuntime and PostRuntime restore phases.
pub struct PreparedGameInstance {
    pub game: Game,
    pub room_id: RoomId,
}

/// Top-level orchestrator of the game and systems.
pub struct GameInstance {
    pub game: Game,
    /// Holds the Transform of every entity rendered in the previous frame.
    pub prev_positions: HashMap<Entity, Vec2>,
}

impl GameInstance {
    pub fn from_loaded_game<C: BishopContext>(
        ctx: &mut C,
        game: Game,
        lua: &Lua,
        camera_manager: &mut CameraManager,
    ) -> Self {
        let prepared = Self::prepare_loaded_game(lua, game);
        let instance = Self::from_prepared(ctx, prepared, camera_manager);
        ScriptSystem::init(lua, &instance.game.script_manager.event_bus);
        instance
    }

    pub fn from_loaded_room<C: BishopContext>(
        ctx: &mut C,
        room: Room,
        game: Game,
        lua: &Lua,
        camera_manager: &mut CameraManager,
    ) -> Self {
        let prepared = Self::prepare_loaded_room(lua, room, game);
        let instance = Self::from_prepared(ctx, prepared, camera_manager);
        ScriptSystem::init(lua, &instance.game.script_manager.event_bus);
        instance
    }

    /// Loads a game and determines the starting room, without finalizing
    /// audio wiring, camera setup, or script initialization.
    pub fn prepare_loaded_game(lua: &Lua, mut game: Game) -> PreparedGameInstance {
        game.initialize_runtime(lua);
        let room_id = Self::start_room_id(&game);
        game.ecs.finalize_after_load();
        PreparedGameInstance { game, room_id }
    }

    /// Loads a specific room into a game, without finalizing
    /// audio wiring, camera setup, or script initialization.
    pub fn prepare_loaded_room(lua: &Lua, room: Room, mut game: Game) -> PreparedGameInstance {
        game.initialize_runtime(lua);
        game.ecs.finalize_after_load();
        PreparedGameInstance {
            room_id: room.id,
            game,
        }
    }

    /// Converts a prepared game into a fully wired GameInstance.
    pub fn from_prepared<C: BishopContext>(
        ctx: &mut C,
        prepared: PreparedGameInstance,
        camera_manager: &mut CameraManager,
    ) -> Self {
        let PreparedGameInstance {
            game,
            room_id,
        } = prepared;

        for source in AudioSource::store(&game.ecs).data.values() {
            push_audio_command(AudioCommand::IncrementRefs(sound_command_ids(
                &game.asset_registry,
                source.all_sound_ids(),
            )));
        }

        let ecs = &game.ecs;
        let player_pos = ecs
            .get_player_transform()
            .map(|transform| transform.position)
            .unwrap_or_default();
        let grid_size = game.current_world().grid_size;

        *camera_manager = CameraManager::new(ctx, ecs, room_id, player_pos, grid_size);

        Self {
            game,
            prev_positions: HashMap::new(),
        }
    }

    fn start_room_id(game: &Game) -> RoomId {
        game.current_world()
            .starting_room_id
            .or_else(|| {
                game.worlds()
                    .first()
                    .map(|world| world.starting_room_id.expect("Game has no starting room."))
            })
            .expect("Game has no starting room nor any rooms")
    }

    /// Drains events generated during UI rendering and forwards them to the event bus.
    pub fn drain_ui_events(&self) {
        self.emit_slider_events();
        self.emit_menu_events();
    }

    /// Drains pending menu action events and emits them to the Lua event bus.
    fn emit_menu_events(&self) {
        let events = drain_menu_events();
        for action in events {
            self.game
                .script_manager
                .event_bus
                .emit(format!("menu:{}", action), Variadic::new());
        }
    }

    /// Drains pending slider events and emits them to the Lua event bus.
    fn emit_slider_events(&self) {
        let events = drain_slider_events();
        for (key, value) in events {
            self.game.script_manager.event_bus.emit(
                format!("slider:{key}"),
                Variadic::from_iter([Value::Number(value as f64)]),
            );
        }
    }

    /// Updates the previous position for all entities in the active room.
    pub fn store_previous_positions(&mut self, camera_manager: &mut CameraManager) {
        let ecs = &self.game.ecs;

        // Store the camera target
        camera_manager.previous_position = Some(camera_manager.active.camera.target);

        let Some(current_room_id) = self.game.current_world().current_room_id else {
            self.prev_positions.clear();
            return;
        };

        let trans_store = ecs.get_store::<Transform>();
        let sub_pixel_store = ecs.get_store::<SubPixel>();

        self.prev_positions.clear();
        self.prev_positions.extend(
            ecs.entities_in_room(current_room_id)
                .iter()
                .filter_map(|entity| {
                    ecs.assert_room_membership(current_room_id, *entity);
                    let transform = trans_store.get(*entity)?;
                    Some((
                        *entity,
                        visual_position(transform.position, sub_pixel_store.get(*entity)),
                    ))
                }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_previous_positions_uses_visual_position_with_subpixel_remainder() {
        let room_id = RoomId(1);
        let room = Room {
            id: room_id,
            ..Default::default()
        };

        let mut world = World::default();
        world.current_room_id = Some(room_id);
        world.add_room(room);

        let mut game = Game::default();
        game.add_world(world);

        let entity = game.ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(10.0, 12.0),
                ..Default::default()
            })
            .with(SubPixel { x: 0.25, y: -0.5 })
            .with_current_room(room_id)
            .finish();


        let mut game_instance = GameInstance {
            game,
            prev_positions: HashMap::new(),
        };

        game_instance.store_previous_positions(&mut CameraManager::default());

        assert_eq!(
            game_instance.prev_positions.get(&entity).copied(),
            Some(Vec2::new(10.25, 11.5))
        );
    }
}
