use crate::engine::game_instance::GameInstance;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::particle_module::ParticleModule;
use engine_core::ecs::Entity;
use engine_core::game::Game;
use engine_core::scripting::lua_constants::{lua_color, lua_engine, lua_fields, lua_particles};
use engine_core::scripting::LuaModule;
use engine_core::worlds::{Room, RoomId, World, WorldId};
use mlua::{AnyUserData, Lua};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn setup_particle_lua() -> (Lua, Rc<RefCell<GameInstance>>) {
    let lua = Lua::new();
    let mut game = Game::default();
    let mut world = World::new(WorldId(1), "Particle Test".to_string(), 16.0);
    world.current_room_id = Some(RoomId(1));
    world.add_room(Room::new(&mut game.ecs, RoomId(1), 16.0));
    world.add_room(Room::new(&mut game.ecs, RoomId(2), 16.0));
    game.add_world(world);
    game.current_world_id = Some(WorldId(1));

    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    }));
    LuaGameCtx {
        game_instance: game_instance.clone(),
    }
    .set_lua_ctx(&lua)
    .expect("Lua game context should register");
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().expect("engine table should create"))
        .expect("engine table should register");
    ParticleModule
        .register(&lua)
        .expect("particle module should register");

    (lua, game_instance)
}

pub(super) fn particle_script(source: &str) -> String {
    let mut replacements = [
        ("$engine", lua_engine::ENGINE),
        ("$particles", lua_particles::MODULE),
        ("$create", lua_particles::CREATE),
        ("$configure", lua_particles::ENTITY_CONFIGURE),
        ("$burst", lua_particles::ENTITY_BURST),
        ("$start", lua_particles::ENTITY_START),
        ("$stop", lua_particles::ENTITY_STOP),
        ("$owner", "owner"),
        ("$position", lua_particles::POSITION),
        ("$config", lua_particles::CONFIG),
        ("$mode", lua_particles::MODE),
        ("$mode_burst", lua_particles::MODE_BURST),
        ("$mode_continuous", lua_particles::MODE_CONTINUOUS),
        ("$visual", lua_particles::VISUAL),
        ("$visual_pixel", lua_particles::VISUAL_PIXEL),
        ("$visual_line", lua_particles::VISUAL_LINE),
        ("$visual_sprite", lua_particles::VISUAL_SPRITE),
        ("$area_size", lua_particles::AREA_SIZE),
        ("$line_length", lua_particles::LINE_LENGTH),
        ("$lifetime_min", lua_particles::LIFETIME_MIN),
        ("$lifetime_max", lua_particles::LIFETIME_MAX),
        ("$velocity_min", lua_particles::VELOCITY_MIN),
        ("$velocity_max", lua_particles::VELOCITY_MAX),
        ("$scale_start", lua_particles::SCALE_START),
        ("$scale_end", lua_particles::SCALE_END),
        ("$color", lua_particles::COLOR),
        ("$opacity_start", lua_particles::OPACITY_START),
        ("$opacity_end", lua_particles::OPACITY_END),
        ("$burst_count", lua_particles::BURST_COUNT),
        ("$emission_rate", lua_particles::EMISSION_RATE),
        ("$max_particles", lua_particles::MAX_PARTICLES),
        ("$seed", lua_particles::SEED),
        ("$frame", lua_particles::FRAME),
        ("$clip_id", lua_particles::CLIP_ID),
        ("$col", lua_particles::COL),
        ("$row", lua_particles::ROW),
        ("$offset", lua_particles::OFFSET),
        ("$sprite_id", lua_particles::SPRITE_ID),
        ("$frame_size", lua_particles::FRAME_SIZE),
        ("$flip_x", lua_particles::FLIP_X),
        ("$x", lua_fields::X),
        ("$y", lua_fields::Y),
        ("$r", lua_color::R),
        ("$g", lua_color::G),
        ("$b", lua_color::B),
        ("$a", lua_color::A),
    ];
    replacements.sort_by_key(|(token, _)| std::cmp::Reverse(token.len()));
    replacements
        .into_iter()
        .fold(source.to_string(), |script, (token, value)| {
            script.replace(token, value)
        })
}

pub(super) fn owner_id(owner: &AnyUserData) -> Entity {
    owner
        .borrow::<crate::scripting::modules::entity_module::EntityHandle>()
        .expect("particle owner handle should be valid")
        .entity
}
