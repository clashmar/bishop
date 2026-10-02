use bishop::prelude::*;
use crate::scripting::script_system::ScriptSystem;
use engine_core::camera::{get_room_cameras, CameraManager};
use engine_core::diagnostics::TraversalResidencyDiagnostics;
use engine_core::ecs::{Active, CurrentRoom, Entity, SubPixel, Transform, WorldEntry};
use engine_core::game::Game;
use engine_core::menu::{drain_menu_events, drain_slider_events};
use engine_core::rendering::{visual_position, RoomRenderState};
use engine_core::storage::hydrate_initial_payloads_for_runtime;
use engine_core::worlds::{Room, RoomId, RoomLayer};
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
    pub room_layer: RoomLayer,
}

/// Top-level orchestrator of the game and systems.
pub struct GameInstance {
    pub game: Game,
    /// Holds the visual position of every active entity from the previous frame.
    pub prev_positions: HashMap<Entity, Vec2>,
    /// Runtime traversal-residency diagnostics (playtest only).
    pub traversal_residency_diagnostics: Option<TraversalResidencyDiagnostics>,
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

    /// Loads a game, determines the starting room, and hydrates the startup payloads
    /// without finalizing audio wiring, camera setup, or script initialization.
    pub fn prepare_loaded_game(lua: &Lua, mut game: Game) -> PreparedGameInstance {
        game.initialize_runtime(lua);
        let (room_id, room_layer) = Self::start_room_and_layer(&game);
        if let Some(world) = game.current_world_mut() {
            world.current_room_id = Some(room_id);
        }
        hydrate_initial_payloads_for_runtime(&mut game)
            .expect("initial payload hydration should succeed during startup");
        game.ecs.finalize_after_load();
        game.sync_all_tile_placements();
        PreparedGameInstance {
            game,
            room_id,
            room_layer,
        }
    }

    /// Loads a specific room into a game, without finalizing
    /// audio wiring, camera setup, or script initialization.
    pub fn prepare_loaded_room(lua: &Lua, room: Room, mut game: Game) -> PreparedGameInstance {
        game.initialize_runtime(lua);
        if let Some(world) = game.current_world_mut() {
            world.current_room_id = Some(room.id);
        }
        game.ecs.finalize_after_load();
        game.sync_all_tile_placements();
        PreparedGameInstance {
            room_id: room.id,
            room_layer: RoomLayer::Front,
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
            room_layer,
        } = prepared;

        let ecs = &game.ecs;
        let player_pos = ecs
            .get_player_transform()
            .map(|transform| transform.position)
            .unwrap_or_default();
        let grid_size = game.current_world().grid_size;

        *camera_manager = CameraManager::new(ctx, ecs, room_id, room_layer, player_pos, grid_size);

        Self {
            game,
            prev_positions: HashMap::new(),
            traversal_residency_diagnostics: None,
        }
    }

    pub fn current_render_state(&self) -> RoomRenderState {
        if let Some(player) = self.game.ecs.get_player_entity() {
            if let Some(current_room) = self.game.ecs.get::<CurrentRoom>(player).copied() {
                return RoomRenderState {
                    current_layer: current_room.layer,
                    viewpoint_position: self.game.ecs.get::<Transform>(player).map(|transform| transform.position),
                    show_all_back_bounds: false,
                };
            }
        }

        let fallback = self.game.current_world().current_room_id.and_then(|room_id| {
            let ecs = &self.game.ecs;
            let preferred_layer = Self::preferred_room_layer(&self.game, room_id);
            get_room_cameras(ecs, room_id, preferred_layer)
                .into_iter()
                .chain(get_room_cameras(ecs, room_id, preferred_layer.opposite()))
                .find_map(|(entity, _)| {
                    let current_room = ecs.get::<CurrentRoom>(entity).copied()?;
                    let viewpoint_position = ecs.get::<Transform>(entity).map(|transform| transform.position);
                    Some((current_room.layer, viewpoint_position))
                })
        });

        let (current_layer, viewpoint_position) =
            fallback.unwrap_or((RoomLayer::Front, None));

        RoomRenderState {
            current_layer,
            viewpoint_position,
            show_all_back_bounds: false,
        }
    }

    /// Drains events generated during UI rendering and forwards them to the event bus.
    pub fn drain_ui_events(&self) {
        self.emit_slider_events();
        self.emit_menu_events();
    }

    /// Updates the previous position for all active entities.
    pub fn store_previous_positions(&mut self, camera_manager: &mut CameraManager) {
        let ecs = &self.game.ecs;

        // Store the camera target
        camera_manager.previous_position = Some(camera_manager.active.camera.target);

        let trans_store = ecs.get_store::<Transform>();
        let sub_pixel_store = ecs.get_store::<SubPixel>();

        self.prev_positions.clear();
        self.prev_positions.extend(
            trans_store
                .data
                .keys()
                .filter(|entity| ecs.get::<Active>(**entity).is_some_and(Active::is_enabled))
                .filter_map(|entity| {
                    let transform = trans_store.get(*entity)?;
                    Some((
                        *entity,
                        visual_position(transform.position, sub_pixel_store.get(*entity)),
                    ))
                }),
        );
    }

    fn start_room_and_layer(game: &Game) -> (RoomId, RoomLayer) {
        let world = game.current_world();
        // Prefer the start WorldEntry's authored room/layer, then fall back to the first room on Front
        let entries = game.ecs.get_store::<WorldEntry>();
        for (&entity, entry) in entries.data.iter() {
            if !entry.is_start {
                continue;
            }
            let Some(current_room) = game.ecs.get::<CurrentRoom>(entity).copied() else {
                continue;
            };
            if world.get_room(current_room.room_id).is_some() {
                return (current_room.room_id, current_room.layer);
            }
        }
        (
            world.rooms().first().map(|r| r.id).unwrap_or_default(),
            RoomLayer::Front,
        )
    }

    fn preferred_room_layer(game: &Game, room_id: RoomId) -> RoomLayer {
        let world = game.current_world();
        let entries = game.ecs.get_store::<WorldEntry>();

        for (&entity, entry) in entries.data.iter() {
            if !entry.is_start {
                continue;
            }
            let Some(current_room) = game.ecs.get::<CurrentRoom>(entity).copied() else {
                continue;
            };
            if current_room.room_id == room_id && world.get_room(current_room.room_id).is_some() {
                return current_room.layer;
            }
        }

        RoomLayer::Front
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
}
