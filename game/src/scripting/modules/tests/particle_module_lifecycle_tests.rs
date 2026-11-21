use super::particle_module_test_support::{owner_id, particle_script, setup_particle_lua};
use crate::game_global::drain_commands;
use crate::scripting::modules::entity_module::EntityHandle;
use bishop::types::Vec2;
use engine_core::ecs::{Entity, ParticleEmitter, ParticleEmitterConfig, Transform};
use mlua::AnyUserData;

#[test]
fn configured_entity_methods_queue_emitter_operations() {
    let _ = drain_commands();
    let (lua, game_instance) = setup_particle_lua();
    let owner: AnyUserData = lua
        .load(particle_script(
            r#"
            return $engine.$particles.$create({ $x = 10, $y = 20 }, {
                $mode = "$mode_continuous",
                $emission_rate = 4,
                $lifetime_min = 10,
                $lifetime_max = 10,
            })
            "#,
        ))
        .eval()
        .expect("continuous emitter should be created");
    lua.globals()
        .set("owner", owner.clone())
        .expect("emitter handle should enter Lua globals");

    lua.load(particle_script(
        "owner:$start(); owner:$stop(); owner:$burst()",
    ))
    .exec()
    .expect("entity particle methods should queue");

    assert_eq!(drain_commands().count(), 3);
    assert_eq!(
        game_instance
            .borrow()
            .game
            .ecs
            .get::<Transform>(owner_id(&owner))
            .expect("emitter should have a transform")
            .position,
        Vec2::new(10.0, 20.0),
    );
}

#[test]
fn prefab_entity_with_emitter_can_be_triggered_directly() {
    let _ = drain_commands();
    let (lua, game_instance) = setup_particle_lua();
    let owner = game_instance
        .borrow_mut()
        .game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(ParticleEmitter {
            config: ParticleEmitterConfig::default(),
        })
        .finish();
    lua.globals()
        .set(
            "prefab_owner",
            lua.create_userdata(EntityHandle { entity: owner })
                .expect("prefab handle should create"),
        )
        .expect("prefab handle should enter Lua globals");

    lua.load(particle_script("prefab_owner:$burst()"))
        .exec()
        .expect("prefab emitter should be triggerable through its entity");

    assert_eq!(drain_commands().count(), 1);
}

#[test]
fn emitter_methods_reject_plain_and_stale_entities() {
    let _ = drain_commands();
    let (lua, game_instance) = setup_particle_lua();
    let plain_entity = game_instance
        .borrow_mut()
        .game
        .ecs
        .create_entity()
        .with(Transform::default())
        .finish();
    lua.globals()
        .set(
            "plain",
            lua.create_userdata(EntityHandle {
                entity: plain_entity,
            })
            .expect("plain entity handle should create"),
        )
        .expect("plain entity handle should enter Lua globals");
    assert!(lua
        .load(particle_script("plain:$burst()"))
        .exec()
        .is_err());

    lua.globals()
        .set(
            "stale",
            lua.create_userdata(EntityHandle { entity: Entity(999) })
                .expect("stale entity handle should create"),
        )
        .expect("stale entity handle should enter Lua globals");
    assert!(lua
        .load(particle_script("stale:$start()"))
        .exec()
        .is_err());
    assert_eq!(drain_commands().count(), 0);
}

#[test]
fn entity_configure_validates_before_queueing_and_replaces_no_partial_state() {
    let _ = drain_commands();
    let (lua, game_instance) = setup_particle_lua();
    let owner: AnyUserData = lua
        .load(particle_script(
            "return $engine.$particles.$create({ $x = 0, $y = 0 })",
        ))
        .eval()
        .expect("particle emitter should be created");
    lua.globals()
        .set("owner", owner.clone())
        .expect("particle handle should enter Lua globals");

    lua.load(particle_script(
        "owner:$configure({ $mode = \"$mode_continuous\", $emission_rate = 5 })",
    ))
    .exec()
    .expect("valid settings should queue");
    assert_eq!(drain_commands().count(), 1);

    let result = lua
        .load(particle_script(
            "owner:$configure({ $mode = \"$mode_continuous\", $emission_rate = 7, $opacity_start = 1.01 })",
        ))
        .exec();
    assert!(result.is_err());
    assert_eq!(drain_commands().count(), 0);
    assert_eq!(
        game_instance
            .borrow()
            .game
            .ecs
            .get::<ParticleEmitter>(owner_id(&owner))
            .expect("emitter settings should remain present")
            .config,
        ParticleEmitterConfig::default(),
    );
}
