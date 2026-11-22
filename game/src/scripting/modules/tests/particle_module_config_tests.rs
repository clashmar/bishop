use super::particle_module_test_support::{owner_id, particle_script, setup_particle_lua};
use crate::game_global::drain_commands;
use crate::scripting::modules::particle_module::ParticleModule;
use bishop::types::{Color, Vec2};
use engine_core::animation::ClipId;
use engine_core::ecs::{
    Active, CurrentFrameSnapshot, CurrentRoom, ParticleEmissionMode, ParticleEmitter,
    ParticleEmitterConfig, ParticleVisual, SpriteId, Transform,
};
use engine_core::scripting::lua_constants::{lua_engine, lua_globals, lua_particles};
use engine_core::scripting::{LuaApi, LuaApiWriter};
use engine_core::worlds::{RoomId, RoomLayer};
use mlua::AnyUserData;

#[test]
fn canonical_particle_config_deserialization_rejects_unknown_fields() {
    let result = serde_json::from_value::<ParticleEmitterConfig>(serde_json::json!({
        "unrecognized": 1,
    }));
    assert!(result.is_err());
}

#[test]
fn create_uses_shared_defaults_and_active_front_room_membership() {
    let (lua, game_instance) = setup_particle_lua();
    let owner: AnyUserData = lua
        .load(particle_script("return $engine.$particles.$create({ $x = 12, $y = -4 })"))
        .eval()
        .expect("default particle emitter should be created");
    let owner_id = owner_id(&owner);
    let game_instance = game_instance.borrow();
    let ecs = &game_instance.game.ecs;
    let transform = ecs
        .get::<Transform>(owner_id)
        .expect("emitter should have a transform");
    let membership = ecs
        .get::<CurrentRoom>(owner_id)
        .expect("emitter should have room membership");
    let emitter = ecs
        .get::<ParticleEmitter>(owner_id)
        .expect("emitter should have particle settings");

    assert_eq!(transform.position, Vec2::new(12.0, -4.0));
    assert_eq!(transform.z, 0);
    assert_eq!(membership.room_id, RoomId(1));
    assert_eq!(membership.layer, RoomLayer::Front);
    assert!(ecs.entities_in_room(RoomId(1)).contains(&owner_id));
    assert!(ecs
        .entities_in_room_layer(RoomId(1), RoomLayer::Front)
        .contains(&owner_id));
    assert!(ecs.get::<Active>(owner_id).is_some());
    assert_eq!(emitter.config, ParticleEmitterConfig::default());
}

#[test]
fn create_decodes_every_shared_config_field_and_enum_value() {
    let (lua, game_instance) = setup_particle_lua();
    let owner: AnyUserData = lua
        .load(particle_script(
            r#"
            return $engine.$particles.$create({ $x = 5, $y = 9 }, {
                $mode = "$mode_continuous",
                $visual = "$visual_line",
                $area_size = { 7, 11 },
                $line_length = 13,
                $lifetime_min = 0.4,
                $lifetime_max = 1.7,
                $velocity_min = { $x = -3, $y = -8 },
                $velocity_max = { $x = 6, $y = 12 },
                $scale_start = 0.5,
                $scale_end = 2.5,
                $color = { $r = 0.1, $g = 0.2, $b = 0.3, $a = 0.4 },
                $opacity_start = 0.25,
                $opacity_end = 0.75,
                $burst_count = 17,
                $emission_rate = 23.5,
                $max_particles = 39,
                $seed = 41,
            })
            "#,
        ))
        .eval()
        .expect("complete particle configuration should decode");
    let emitter = game_instance
        .borrow()
        .game
        .ecs
        .get::<ParticleEmitter>(owner_id(&owner))
        .expect("created entity should have a particle emitter")
        .clone();

    let expected = ParticleEmitterConfig {
        mode: ParticleEmissionMode::Continuous,
        visual: ParticleVisual::Line,
        area_size: Vec2::new(7.0, 11.0),
        line_length: 13,
        lifetime_min: 0.4,
        lifetime_max: 1.7,
        velocity_min: Vec2::new(-3.0, -8.0),
        velocity_max: Vec2::new(6.0, 12.0),
        scale_start: 0.5,
        scale_end: 2.5,
        color: Color::new(0.1, 0.2, 0.3, 0.4),
        opacity_start: 0.25,
        opacity_end: 0.75,
        burst_count: 17,
        emission_rate: 23.5,
        max_particles: 39,
        seed: 41,
    };
    let ron_config = ron::from_str::<ParticleEmitterConfig>(
        &ron::to_string(&expected).expect("expected particle config should serialize to RON"),
    )
    .expect("RON particle config should deserialize");
    assert_eq!(emitter.config, ron_config);
}

#[test]
fn create_decodes_sprite_frame_visual_with_named_vector_fields() {
    let (lua, game_instance) = setup_particle_lua();
    let owner: AnyUserData = lua
        .load(particle_script(
            r#"
            return $engine.$particles.$create({ $x = 0, $y = 0 }, {
                $visual = {
                    $visual_sprite = {
                        $frame = {
                            $clip_id = "Idle",
                            $col = 2,
                            $row = 3,
                            $offset = { $x = 1, $y = -2 },
                            $sprite_id = 5,
                            $frame_size = { $x = 8, $y = 9 },
                            $flip_x = true,
                        },
                    },
                },
            })
            "#,
        ))
        .eval()
        .expect("sprite frame visual should decode");
    let config = game_instance
        .borrow()
        .game
        .ecs
        .get::<ParticleEmitter>(owner_id(&owner))
        .expect("created entity should have a particle emitter")
        .config
        .clone();
    assert_eq!(
        config.visual,
        ParticleVisual::Sprite {
            frame: CurrentFrameSnapshot {
                clip_id: ClipId::Idle,
                col: 2,
                row: 3,
                offset: Vec2::new(1.0, -2.0),
                sprite_id: SpriteId(5),
                frame_size: Vec2::new(8.0, 9.0),
                flip_x: true,
            },
        }
    );
}

#[test]
fn configure_queues_valid_settings_and_rejects_invalid_opacity_atomically() {
    let _ = drain_commands();
    let (lua, game_instance) = setup_particle_lua();
    let owner: AnyUserData = lua
        .load(particle_script("return $engine.$particles.$create({ $x = 0, $y = 0 })"))
        .eval()
        .expect("particle emitter should be created");
    lua.globals()
        .set("owner", owner.clone())
        .expect("particle handle should enter Lua globals");
    lua.load(particle_script(
        r#"
        $owner:$configure({
            $mode = "$mode_continuous",
            $emission_rate = 5,
            $opacity_start = 0.8,
            $opacity_end = 0.2,
        })
        "#,
    ))
    .exec()
    .expect("valid configuration should queue settings");
    assert_eq!(drain_commands().count(), 1);
    let owner_id = owner_id(&owner);
    let configured = game_instance
        .borrow()
        .game
        .ecs
        .get::<ParticleEmitter>(owner_id)
        .expect("emitter settings should remain present")
        .config
        .clone();
    assert_eq!(
        configured,
        ParticleEmitterConfig::default(),
    );

    let result = lua
        .load(particle_script(
            r#"
            $owner:$configure({
                $mode = "$mode_continuous",
                $emission_rate = 7,
                $opacity_start = 1.01,
            })
            "#,
        ))
        .exec();
    assert!(result.is_err());
    assert_eq!(drain_commands().count(), 0);
    let after_rejection = game_instance
        .borrow()
        .game
        .ecs
        .get::<ParticleEmitter>(owner_id)
        .expect("emitter settings should remain present")
        .config
        .clone();
    assert_eq!(after_rejection, configured);
}

#[test]
fn emit_api_documents_particle_config_fields_and_return_types() {
    let mut out = LuaApiWriter::default();
    ParticleModule.emit_api(&mut out);
    crate::scripting::modules::entity_module::EntityModule.emit_api(&mut out);

    for field in [
        lua_particles::MODE,
        lua_particles::VISUAL,
        lua_particles::AREA_SIZE,
        lua_particles::LINE_LENGTH,
        lua_particles::LIFETIME_MIN,
        lua_particles::LIFETIME_MAX,
        lua_particles::VELOCITY_MIN,
        lua_particles::VELOCITY_MAX,
        lua_particles::SCALE_START,
        lua_particles::SCALE_END,
        lua_particles::COLOR,
        lua_particles::OPACITY_START,
        lua_particles::OPACITY_END,
        lua_particles::BURST_COUNT,
        lua_particles::EMISSION_RATE,
        lua_particles::MAX_PARTICLES,
        lua_particles::SEED,
    ] {
        assert!(out.buf.contains(field));
    }
    for signature in [
        format!(
            "function {}.{}.{}({}, {}) end",
            lua_engine::ENGINE,
            lua_particles::MODULE,
            lua_particles::CREATE,
            lua_particles::POSITION,
            lua_particles::CONFIG,
        ),
        format!(
            "function {}:{}({}) end",
            lua_globals::ENTITY,
            lua_particles::ENTITY_CONFIGURE,
            lua_particles::CONFIG,
        ),
        format!("function {}:{}() end", lua_globals::ENTITY, lua_particles::ENTITY_BURST),
        format!("function {}:{}() end", lua_globals::ENTITY, lua_particles::ENTITY_START),
        format!("function {}:{}() end", lua_globals::ENTITY, lua_particles::ENTITY_STOP),
    ] {
        assert!(out.buf.contains(&signature));
    }
    assert!(out
        .buf
        .contains(&format!("---@return {}", lua_globals::ENTITY)));
    assert!(out.buf.contains("---@return nil"));
    for documentation in [
        "--- Replaces this entity's particle emitter settings.",
        "--- Emits a burst when this entity uses burst mode.",
        "--- Starts continuous emission when this entity uses continuous mode.",
        "--- Stops continuous emission without removing live particles.",
    ] {
        assert!(out.buf.contains(documentation));
    }
}
