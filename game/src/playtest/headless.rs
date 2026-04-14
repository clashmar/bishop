use bishop::PlatformContext;
use engine_core::prelude::*;
use game_lib::engine::{Engine, EngineBuilder, EngineEntryMode, GameInstance};
use std::collections::HashMap;

/// Minimal headless playtest session scaffolding.
pub struct HeadlessPlaytestSession {
    session_id: String,
}

impl HeadlessPlaytestSession {
    /// Creates a new headless session.
    pub fn new(session_id: String) -> Self {
        Self { session_id }
    }

    /// Builds a minimal headless engine session.
    pub fn build_engine(&self, ctx: PlatformContext) -> Engine {
        let _ = ctx;
        let mut game = Game {
            name: self.session_id.clone(),
            ..Game::default()
        };

        let mut ecs = Ecs::default();
        let room_id = RoomId::default();
        let world_id = WorldId::default();
        let room = Room::new(&mut ecs, room_id, 16.0);

        game.ecs = ecs;
        game.worlds = vec![World {
            id: world_id,
            name: "HeadlessWorld".to_string(),
            rooms: vec![room],
            current_room_id: Some(room_id),
            starting_room_id: Some(room_id),
            starting_position: None,
            meta: WorldMeta::default(),
            grid_size: 16.0,
        }];
        game.current_world_id = world_id;

        let builder = EngineBuilder::new().entry_mode(EngineEntryMode::Playing);
        let game_instance = GameInstance {
            game,
            prev_positions: HashMap::new(),
        };
        builder.assemble(game_instance, ctx, true)
    }
}
