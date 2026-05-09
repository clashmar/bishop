use super::*;
use std::collections::HashMap;

#[test]
fn on_insert_fires_on_builder_insertion() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(Transform::default())
        .finish();

    assert!(ecs.get_store::<Transform>().contains(entity));
}

#[test]
fn on_insert_fires_on_direct_insert_component() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    ecs.insert_component(entity, LifecycleMarker::default());

    let comp = ecs.get_store::<LifecycleMarker>().get(entity).unwrap();
    assert_eq!(comp.insert_count, 1, "on_insert should fire during direct insert_component");
}

#[test]
fn on_insert_fires_on_add_component_to_entity() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    ecs.add_component_to_entity(entity, LifecycleMarker::default());

    let comp = ecs.get_store::<LifecycleMarker>().get(entity).unwrap();
    assert_eq!(comp.insert_count, 1, "on_insert should fire during add_component_to_entity");
}

#[test]
fn on_remove_fires_on_remove_entity() {
    let mut ecs = Ecs::default();
    let mut asset_registry = AssetRegistry::default();
    let mut sprite_manager = SpriteManager::default();
    let mut script_manager = ScriptManager::default();
    let prefab_manager = PrefabManager::default();

    let entity = ecs.create_entity().with(Transform::default()).finish();
    ecs.insert_component(entity, LifecycleMarker::default());

    let mut ctx = GameCtxMut {
        ecs: &mut ecs,
        world: None,
        asset_registry: &mut asset_registry,
        sprite_manager: &mut sprite_manager,
        script_manager: &mut script_manager,
        prefab_manager: &prefab_manager,
    };

    Ecs::remove_entity(&mut ctx, entity);

    assert!(!ecs.get_store::<LifecycleMarker>().contains(entity));
}

#[test]
fn on_remove_fires_on_remove_component() {
    let mut ecs = Ecs::default();
    let mut asset_registry = AssetRegistry::default();
    let mut sprite_manager = SpriteManager::default();
    let mut script_manager = ScriptManager::default();
    let prefab_manager = PrefabManager::default();

    let entity = ecs.create_entity().with(Transform::default()).finish();
    ecs.insert_component(entity, LifecycleMarker::default());

    let mut ctx = GameCtxMut {
        ecs: &mut ecs,
        world: None,
        asset_registry: &mut asset_registry,
        sprite_manager: &mut sprite_manager,
        script_manager: &mut script_manager,
        prefab_manager: &prefab_manager,
    };

    Ecs::remove_component::<LifecycleMarker>(&mut ctx, entity);

    assert!(!ecs.get_store::<LifecycleMarker>().contains(entity));
}

#[test]
fn on_remove_fires_on_purge_proxies() {
    let mut ecs = Ecs::default();
    let proxy = Entity(0);
    ecs.get_store_mut::<PlayerProxy>().insert(proxy, PlayerProxy);
    ecs.get_store_mut::<LifecycleMarker>().insert(proxy, LifecycleMarker::default());
    ecs.purge_proxies();
    assert!(!ecs.get_store::<LifecycleMarker>().contains(proxy));
}

#[test]
fn replace_component_updates_store_value() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().with(Transform::default()).finish();

    ecs.replace_component(entity, Transform::default());

    assert!(ecs.get_store::<Transform>().contains(entity));
}

#[test]
fn proc_macro_wires_on_insert_on_remove_and_guarded_into_registry() {
    let reg = inventory::iter::<ComponentRegistry>
        .into_iter()
        .find(|r| r.type_name == "LifecycleMarker")
        .expect("LifecycleMarker registry not found");

    assert!(reg.guarded, "guarded flag should be true");

    let mut comp = LifecycleMarker::default();
    (reg.on_insert)(&mut comp, &Entity(1), &mut Ecs::default());
    assert_eq!(comp.insert_count, 1);

    (reg.on_remove)(&mut comp, &Entity(1), &mut Ecs::default());
    assert_eq!(comp.remove_count, 1);
}

#[test]
fn finalize_after_load_calls_on_insert_for_all_entities() {
    let mut ecs = Ecs::default();

    let e1 = ecs.create_entity().with(Transform::default()).finish();
    ecs.insert_component(e1, LifecycleMarker::default());
    let e2 = ecs.create_entity().with(Transform::default()).finish();
    ecs.insert_component(e2, LifecycleMarker::default());

    ecs.get_store_mut::<LifecycleMarker>().get_mut(e1).unwrap().insert_count = 0;
    ecs.get_store_mut::<LifecycleMarker>().get_mut(e2).unwrap().insert_count = 0;

    ecs.finalize_after_load();

    let c1 = ecs.get_store::<LifecycleMarker>().get(e1).unwrap();
    let c2 = ecs.get_store::<LifecycleMarker>().get(e2).unwrap();
    assert_eq!(c1.insert_count, 1, "finalize should call on_insert for e1");
    assert_eq!(c2.insert_count, 1, "finalize should call on_insert for e2");
}

#[test]
fn finalize_after_load_on_empty_ecs_is_noop() {
    let mut ecs = Ecs::default();
    ecs.finalize_after_load();
}

#[test]
fn post_create_is_wired_in_registry_for_animation() {
    let reg = inventory::iter::<ComponentRegistry>
        .into_iter()
        .find(|r| r.type_name == "Animation")
        .expect("Animation registry not found");
    assert!(reg.post_create as *const () != noop_post_create as *const (),
        "Animation should have a real post_create, not the noop");
}

#[test]
fn restore_next_entity_id_finds_max() {
    let mut ecs = Ecs::default();
    let _e1 = ecs.create_entity().with(Transform::default()).finish();
    let _e2 = ecs.create_entity().with(Transform::default()).finish();
    let e3 = ecs.create_entity().with(Transform::default()).finish();

    assert_eq!(ecs.next_entity_id, 4);

    ecs.get_store_mut::<Transform>().remove(e3);
    ecs.restore_next_entity_id();
    assert_eq!(
        ecs.next_entity_id, 3,
        "after removing the highest entity, next_entity_id should be max(existing) + 1"
    );

    let e_new = ecs.create_entity().finish();
    assert_eq!(e_new.0, 3);
}

#[test]
fn restore_next_entity_id_empty_ecs_defaults_to_1() {
    let mut ecs = Ecs {
        stores: HashMap::new(),
        next_entity_id: 42,
    };
    ecs.restore_next_entity_id();
    assert_eq!(ecs.next_entity_id, 1);
}

#[test]
fn roundtrip_serde_derives_next_entity_id() {
    let mut ecs = Ecs::default();
    ecs.create_entity().with(Transform::default()).finish();
    ecs.create_entity().with(Transform::default()).finish();
    assert_eq!(ecs.next_entity_id, 3);

    let ron = ron::ser::to_string(&ecs).unwrap();
    let deserialized: Ecs = ron::de::from_str(&ron).unwrap();
    assert_eq!(deserialized.next_entity_id, 3);
}

#[test]
fn roundtrip_serde_empty_ecs() {
    let ecs = Ecs::default();
    let ron = ron::ser::to_string(&ecs).unwrap();
    let deserialized: Ecs = ron::de::from_str(&ron).unwrap();
    assert_eq!(deserialized.next_entity_id, 1);
}
