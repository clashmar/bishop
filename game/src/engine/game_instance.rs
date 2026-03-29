// game/src/engine/game_instance.rs
use crate::scripting::script_system::ScriptSystem;
use engine_core::prelude::*;
use mlua::Lua;
use mlua::Value;
use mlua::Variadic;
use std::collections::HashMap;

/// Top level orchestrator of the game and systems.
pub struct GameInstance {
    /// The whole game.
    pub game: Game,
    /// Holds the Transform of every entity rendered in the previous frame.
    pub prev_positions: HashMap<Entity, Vec2>,
}

impl GameInstance {
    // TODO: Make game creation DRYer
    pub fn new<C: BishopContext>(
        ctx: &mut C,
        lua: &Lua,
        camera_manager: &mut CameraManager,
    ) -> Self {
        // Allows the shared engine features to make decisions
        set_engine_mode(EngineMode::Game);

        let game = match load_game_ron() {
            Ok(game) => game,
            Err(e) => panic!("{e}"),
        };

        Self::from_loaded_game_blocking(ctx, game, lua, camera_manager)
    }

    pub fn for_room<C: BishopContext>(
        ctx: &mut C,
        room: Room,
        game: Game,
        lua: &Lua,
        camera_manager: &mut CameraManager,
    ) -> Self {
        // Playtest mode is set in playtest_main.rs before this is called,
        // so we only set Game mode if not already in Playtest mode
        if get_engine_mode() != EngineMode::Playtest {
            set_engine_mode(EngineMode::Game);
        }

        Self::from_loaded_room_blocking(ctx, room, game, lua, camera_manager)
    }

    pub fn from_loaded_game<C: BishopContext>(
        ctx: &mut C,
        game: Game,
        lua: &Lua,
        camera_manager: &mut CameraManager,
    ) -> Self {
        let start_room_id = game
            .current_world()
            .starting_room_id
            .or_else(|| {
                game.worlds
                    .first()
                    .map(|world| world.starting_room_id.expect("Game has no starting room."))
            })
            .expect("Game has no starting room nor any rooms");

        Self::from_loaded_room_id(ctx, start_room_id, game, lua, camera_manager, true)
    }

    fn from_loaded_game_blocking<C: BishopContext>(
        ctx: &mut C,
        game: Game,
        lua: &Lua,
        camera_manager: &mut CameraManager,
    ) -> Self {
        let start_room_id = game
            .current_world()
            .starting_room_id
            .or_else(|| {
                game.worlds
                    .first()
                    .map(|world| world.starting_room_id.expect("Game has no starting room."))
            })
            .expect("Game has no starting room nor any rooms");

        Self::from_loaded_room_id(ctx, start_room_id, game, lua, camera_manager, false)
    }

    pub fn from_loaded_room<C: BishopContext>(
        ctx: &mut C,
        room: Room,
        game: Game,
        lua: &Lua,
        camera_manager: &mut CameraManager,
    ) -> Self {
        Self::from_loaded_room_id(ctx, room.id, game, lua, camera_manager, true)
    }

    fn from_loaded_room_blocking<C: BishopContext>(
        ctx: &mut C,
        room: Room,
        game: Game,
        lua: &Lua,
        camera_manager: &mut CameraManager,
    ) -> Self {
        Self::from_loaded_room_id(ctx, room.id, game, lua, camera_manager, false)
    }

    fn from_loaded_room_id<C: BishopContext>(
        ctx: &mut C,
        room_id: RoomId,
        mut game: Game,
        lua: &Lua,
        camera_manager: &mut CameraManager,
        runtime_loading: bool,
    ) -> Self {
        if runtime_loading {
            game.initialize_runtime(lua);
        } else {
            game.initialize(ctx, lua);
        }

        // Warm the audio cache for all AudioSource components that were loaded from the
        // save file. Ecs::deserialize bypasses post_create hooks (serde has no GameCtxMut),
        // so we push IncrementRefs manually here.
        // TODO(save-load): replace with a proper post-load hook once the Save/Load sprint
        // adds runtime save files and a generalised post-deserialize callback.
        for source in AudioSource::store(&game.ecs).data.values() {
            push_audio_command(AudioCommand::IncrementRefs(source.all_sound_ids()));
        }

        let ecs = &game.ecs;
        let player_pos = ecs
            .get_player_transform()
            .map(|transform| transform.position)
            .unwrap_or_default();
        let grid_size = game.current_world().grid_size;

        *camera_manager = CameraManager::new(ctx, ecs, room_id, player_pos, grid_size);

        ScriptSystem::init(lua, &game.script_manager.event_bus);

        Self {
            game,
            prev_positions: HashMap::new(),
        }
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
        let trans_store = ecs.get_store::<Transform>();
        let room_store = ecs.get_store::<CurrentRoom>();

        // Store the camera target
        camera_manager.previous_position = Some(camera_manager.active.camera.target);

        self.prev_positions.clear();
        self.prev_positions
            .extend(trans_store.data.iter().filter_map(|(entity, transform)| {
                room_store
                    .get(*entity)
                    .filter(|cr| cr.0 == self.game.current_world().current_room_id.unwrap()) // TODO: handle unwrap
                    .map(|_| (*entity, transform.position))
            }));
    }
}
