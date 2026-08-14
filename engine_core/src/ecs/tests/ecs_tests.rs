use super::*;
use crate::assets::asset_registry::AssetRegistry;
use crate::assets::sprite_manager::SpriteManager;
use crate::game::GameCtxMut;
use crate::prefab::PrefabManager;
use crate::scripting::script_manager::ScriptManager;
use crate::tiles::{TileDefId, TileMap, TileRegistry};
use crate::worlds::{
    BackRoomLayer,
    InteriorZone,
    InteriorZoneBounds,
    InteriorZoneId,
    InteriorZoneScope,
    LayerCompositionMode,
    Room,
    RoomId,
    RoomLayer,
    RoomLayers,
    RoomVariant,
    World,
    WorldId,
};
use bishop::prelude::Vec2;
use std::collections::HashMap;

/// Declare a minimal GameCtxMut for tests that need remove_entity/remove_component.
/// Variables are placed in the enclosing scope so borrows live long enough.
macro_rules! make_game_ctx {
    ($ecs:expr, $ctx:ident) => {
        let mut _mgc_ar = AssetRegistry::default();
        let mut _mgc_tr = TileRegistry::default();
        let mut _mgc_sm = SpriteManager::default();
        let mut _mgc_scm = ScriptManager::default();
        let _mgc_pm = PrefabManager::default();
        let mut $ctx = GameCtxMut {
            ecs: $ecs,
            world: None,
            world_directory: Vec::new(),
            room_world_map: std::collections::HashMap::new(),
            asset_registry: &mut _mgc_ar,
            tile_registry: &mut _mgc_tr,
            sprite_manager: &mut _mgc_sm,
            script_manager: &mut _mgc_scm,
            prefab_manager: &_mgc_pm,
        };
    };
}

// ---- room_entities tests ----

#[test]
fn current_room_round_trip_preserves_room_and_layer() {
    let current_room = CurrentRoom {
        room_id: RoomId(42),
        layer: RoomLayer::Back,
    };

    let ron = ron::ser::to_string_pretty(&current_room, ron::ser::PrettyConfig::new()).unwrap();
    let parsed: CurrentRoom = ron::from_str(&ron).unwrap();

    assert_eq!(parsed.room_id, RoomId(42));
    assert_eq!(parsed.layer, RoomLayer::Back);
}

#[test]
fn current_room_insert_tracks_room_entities() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_id = RoomId(42);

    ecs.insert_component(entity, CurrentRoom::front(room_id));

    let entities = ecs.entities_in_room(room_id);
    assert_eq!(entities.len(), 1, "exactly one entity in room");
    assert!(entities.contains(&entity), "entity should be tracked in room_entities");
}

#[test]
fn current_room_remove_untracks_room_entities() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_id = RoomId(42);

    ecs.insert_component(entity, CurrentRoom::front(room_id));

    make_game_ctx!(&mut ecs, ctx);
    Ecs::remove_component::<CurrentRoom>(&mut ctx, entity);

    let entities = ecs.entities_in_room(room_id);
    assert!(
        entities.is_empty() || !entities.contains(&entity),
        "entity should not be tracked after CurrentRoom removed"
    );
    // Also verify the entity has no CurrentRoom at all
    assert!(!ecs.has::<CurrentRoom>(entity));
}

#[test]
fn clear_current_room_removes_component_and_untracks_entity() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_id = RoomId(9);

    ecs.set_current_room(entity, room_id);
    ecs.clear_current_room(entity);

    assert!(!ecs.has::<CurrentRoom>(entity));
    assert!(!ecs.entities_in_room(room_id).contains(&entity));
}

#[test]
fn set_current_room_rehomes_entity_without_leaving_stale_membership() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_a = RoomId(1);
    let room_b = RoomId(2);

    ecs.set_current_room(entity, room_a);
    ecs.set_current_room(entity, room_b);

    assert!(
        !ecs.entities_in_room(room_a).contains(&entity),
        "entity should be removed from old room"
    );
    assert!(
        ecs.entities_in_room(room_b).contains(&entity),
        "entity should be added to new room"
    );
}

#[test]
fn tile_placement_insert_tracks_room_layer_tile_entities() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_id = RoomId(5);

    ecs.insert_component(entity, TilePlacement::new(TileDefId(3), 2, 4));
    ecs.insert_component(entity, CurrentRoom::front(room_id));

    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Front, 2, 4), Some(entity));
    assert_eq!(
        ecs.tile_placement_at(room_id, RoomLayer::Front, 2, 4)
            .map(|tile| tile.definition),
        Some(TileDefId(3))
    );
}

#[test]
fn tile_placement_insert_tracks_definition_link_entities() {
    let mut ecs = Ecs::default();
    let first = ecs.create_entity().finish();
    let second = ecs.create_entity().finish();
    let tile_id = TileDefId(12);

    ecs.insert_component(first, TilePlacement::new(tile_id, 2, 4));
    ecs.insert_component(second, TilePlacement::new(tile_id, 3, 4));

    let linked = ecs.tile_entities_for_definition(tile_id);
    assert_eq!(linked.len(), 2);
    assert!(linked.contains(&first));
    assert!(linked.contains(&second));
}

#[test]
fn tile_placement_remove_untracks_room_layer_tile_entities() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_id = RoomId(6);

    ecs.insert_component(entity, CurrentRoom::front(room_id));
    ecs.insert_component(entity, TilePlacement::new(TileDefId(4), 1, 3));

    make_game_ctx!(&mut ecs, ctx);
    Ecs::remove_component::<TilePlacement>(&mut ctx, entity);

    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Front, 1, 3), None);
}

#[test]
fn tile_placement_remove_untracks_definition_link_entities() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let tile_id = TileDefId(13);

    ecs.insert_component(entity, TilePlacement::new(tile_id, 1, 3));

    make_game_ctx!(&mut ecs, ctx);
    Ecs::remove_component::<TilePlacement>(&mut ctx, entity);

    assert!(!ecs.tile_entities_for_definition(tile_id).contains(&entity));
}

#[test]
fn remove_entity_when_tile_placement_exists_then_definition_link_index_is_cleared() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let tile_id = TileDefId(15);

    ecs.insert_component(entity, TilePlacement::new(tile_id, 1, 3));

    make_game_ctx!(&mut ecs, ctx);
    Ecs::remove_entity(&mut ctx, entity);

    assert!(!ecs.tile_entities_for_definition(tile_id).contains(&entity));
}

#[test]
fn set_current_room_tracks_entity_in_room_entities() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_a = RoomId(1);
    let room_b = RoomId(2);

    // Move to room_a — inserts CurrentRoom and tracks in room_entities
    ecs.set_current_room(entity, room_a);
    assert!(ecs.entities_in_room(room_a).contains(&entity));
    assert!(!ecs.entities_in_room(room_b).contains(&entity));

    // Move to room_b — updates CurrentRoom and room_entities
    ecs.set_current_room(entity, room_b);
    assert!(!ecs.entities_in_room(room_a).contains(&entity),
        "entity should be removed from old room");
    assert!(ecs.entities_in_room(room_b).contains(&entity),
        "entity should be added to new room");
}

#[test]
fn set_current_room_rehomes_tile_indexed_entity() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_a = RoomId(7);
    let room_b = RoomId(8);

    ecs.insert_component(entity, TilePlacement::new(TileDefId(9), 4, 5));
    ecs.set_current_room(entity, room_a);
    assert_eq!(ecs.tile_entity_at(room_a, RoomLayer::Front, 4, 5), Some(entity));

    ecs.set_current_room(entity, room_b);
    assert_eq!(ecs.tile_entity_at(room_a, RoomLayer::Front, 4, 5), None);
    assert_eq!(ecs.tile_entity_at(room_b, RoomLayer::Front, 4, 5), Some(entity));
}

#[test]
fn tile_entity_at_filters_by_room_layer_and_cell() {
    let mut ecs = Ecs::default();
    let room_id = RoomId(70);
    let front = ecs.create_entity().finish();
    let back = ecs.create_entity().finish();

    ecs.insert_component(front, TilePlacement::new(TileDefId(1), 2, 3));
    ecs.insert_component(
        front,
        CurrentRoom {
            room_id,
            layer: RoomLayer::Front,
        },
    );

    ecs.insert_component(back, TilePlacement::new(TileDefId(2), 2, 3));
    ecs.insert_component(
        back,
        CurrentRoom {
            room_id,
            layer: RoomLayer::Back,
        },
    );

    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Front, 2, 3), Some(front));
    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Back, 2, 3), Some(back));
    assert_eq!(
        ecs.tile_placement_at(room_id, RoomLayer::Front, 2, 3)
            .map(|placement| placement.definition),
        Some(TileDefId(1))
    );
    assert_eq!(
        ecs.tile_placement_at(room_id, RoomLayer::Back, 2, 3)
            .map(|placement| placement.definition),
        Some(TileDefId(2))
    );
}

#[test]
fn finalize_after_load_rebuilds_room_entities_for_current_room() {
    // Simulate a loaded ECS: insert CurrentRoom directly into store,
    // bypassing lifecycle hooks. Then call finalize_after_load.
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_id = RoomId(42);

    // Bypass lifecycle hooks: insert directly into the store
    ecs.get_store_mut::<CurrentRoom>().insert(entity, CurrentRoom::front(room_id));

    // room_entities should be empty since on_insert didn't fire
    assert!(ecs.entities_in_room(room_id).is_empty(), "room_entities should be empty before finalize");

    ecs.finalize_after_load();

    let entities = ecs.entities_in_room(room_id);
    assert_eq!(entities.len(), 1, "exactly one entity in room after finalize");
    assert!(entities.contains(&entity), "entity should be tracked after finalize");
}

#[test]
fn finalize_after_load_rebuilds_room_layer_tile_entities_for_tile_placements() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    let room_id = RoomId(43);

    ecs.get_store_mut::<TilePlacement>()
        .insert(entity, TilePlacement::new(TileDefId(2), 3, 1));
    ecs.get_store_mut::<CurrentRoom>().insert(
        entity,
        CurrentRoom {
            room_id,
            layer: RoomLayer::Front,
        },
    );

    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Front, 3, 1), None);

    ecs.finalize_after_load();

    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Front, 3, 1), Some(entity));
}

#[test]
fn finalize_after_load_rebuilds_room_layer_indexes() {
    let mut ecs = Ecs::default();
    let front = ecs.create_entity().finish();
    let back = ecs.create_entity().finish();
    let room_id = RoomId(71);

    ecs.get_store_mut::<TilePlacement>()
        .insert(front, TilePlacement::new(TileDefId(8), 1, 2));
    ecs.get_store_mut::<CurrentRoom>().insert(
        front,
        CurrentRoom {
            room_id,
            layer: RoomLayer::Front,
        },
    );
    ecs.get_store_mut::<TilePlacement>()
        .insert(back, TilePlacement::new(TileDefId(9), 1, 2));
    ecs.get_store_mut::<CurrentRoom>().insert(
        back,
        CurrentRoom {
            room_id,
            layer: RoomLayer::Back,
        },
    );

    assert!(ecs.entities_in_room_layer(room_id, RoomLayer::Front).is_empty());
    assert!(ecs.entities_in_room_layer(room_id, RoomLayer::Back).is_empty());
    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Front, 1, 2), None);
    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Back, 1, 2), None);

    ecs.finalize_after_load();

    assert!(ecs.entities_in_room_layer(room_id, RoomLayer::Front).contains(&front));
    assert!(ecs.entities_in_room_layer(room_id, RoomLayer::Back).contains(&back));
    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Front, 1, 2), Some(front));
    assert_eq!(ecs.tile_entity_at(room_id, RoomLayer::Back, 1, 2), Some(back));
}

#[test]
fn finalize_after_load_rebuilds_definition_link_entities_for_tile_placements() {
    let mut ecs = Ecs::default();
    let first = ecs.create_entity().finish();
    let second = ecs.create_entity().finish();
    let tile_id = TileDefId(14);

    ecs.get_store_mut::<TilePlacement>()
        .insert(first, TilePlacement::new(tile_id, 3, 1));
    ecs.get_store_mut::<TilePlacement>()
        .insert(second, TilePlacement::new(tile_id, 4, 1));

    assert!(ecs.tile_entities_for_definition(tile_id).is_empty());

    ecs.finalize_after_load();

    let linked = ecs.tile_entities_for_definition(tile_id);
    assert_eq!(linked.len(), 2);
    assert!(linked.contains(&first));
    assert!(linked.contains(&second));
}

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

    let entity = ecs.create_entity().with(Transform::default()).finish();
    ecs.insert_component(entity, LifecycleMarker::default());

    make_game_ctx!(&mut ecs, ctx);

    Ecs::remove_entity(&mut ctx, entity);

    assert!(!ecs.get_store::<LifecycleMarker>().contains(entity));
}

#[test]
fn remove_component_removes_component() {
    let mut ecs = Ecs::default();

    let entity = ecs.create_entity().with(Transform::default()).finish();
    ecs.insert_component(entity, LifecycleMarker::default());

    make_game_ctx!(&mut ecs, ctx);

    Ecs::remove_component::<LifecycleMarker>(&mut ctx, entity);

    assert!(!ecs.get_store::<LifecycleMarker>().contains(entity));
}

#[test]
fn remove_component_by_type_name_removes_component() {
    let mut ecs = Ecs::default();

    let entity = ecs.create_entity().with(Transform::default()).finish();
    ecs.insert_component(entity, LifecycleMarker::default());

    make_game_ctx!(&mut ecs, ctx);

    Ecs::remove_component_by_type_name(&mut ctx, entity, LifecycleMarker::TYPE_NAME);

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
fn component_has_dependents_grounded_after_physics_body_removed_returns_false() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();
    generic_inserter::<PhysicsBody>(&mut ecs, entity, Box::new(PhysicsBody));

    make_game_ctx!(&mut ecs, ctx);
    Ecs::remove_component_by_type_name(&mut ctx, entity, PhysicsBody::TYPE_NAME);

    assert!(!component_has_dependents(Grounded::TYPE_NAME, entity, &ecs));
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
        room_entities: HashMap::new(),
        room_layer_entities: HashMap::new(),
        room_tile_entities: HashMap::new(),
        tile_definition_entities: HashMap::new(),
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
fn serialize_pretty_with_transform_store_embeds_nested_component_data() {
    let mut ecs = Ecs::default();
    ecs.create_entity().with(Transform::default()).finish();

    let ron = ron::ser::to_string_pretty(&ecs, ron::ser::PrettyConfig::new()).unwrap();
    let normalized_ron = ron.replace("\r\n", "\n");

    assert!(normalized_ron.contains("type_name: \"Transform\""));
    assert!(normalized_ron.contains("data: {\n                (1): ("));
    assert!(normalized_ron.contains("position: (0.0, 0.0)"));
    assert!(!normalized_ron.contains("data: \"{\\n"));

    let deserialized: Ecs = ron::de::from_str(&ron).unwrap();
    assert!(deserialized.has::<Transform>(Entity(1)));
}

#[test]
fn roundtrip_serde_empty_ecs() {
    let ecs = Ecs::default();
    let ron = ron::ser::to_string(&ecs).unwrap();
    let deserialized: Ecs = ron::de::from_str(&ron).unwrap();
    assert_eq!(deserialized.next_entity_id, 1);
}

#[test]
fn room_camera_conflicts_with_world_entry() {
    let group = COMPONENT_CONFLICT_GROUPS
        .iter()
        .find(|g| g.contains(&RoomCamera::TYPE_NAME) && g.contains(&WorldEntry::TYPE_NAME));
    assert!(
        group.is_some(),
        "RoomCamera and WorldEntry must be in a conflict group together"
    );
}

#[test]
fn room_camera_conflicts_with_world_exit() {
    let group = COMPONENT_CONFLICT_GROUPS
        .iter()
        .find(|g| g.contains(&RoomCamera::TYPE_NAME) && g.contains(&WorldExit::TYPE_NAME));
    assert!(
        group.is_some(),
        "RoomCamera and WorldExit must be in a conflict group together"
    );
}

#[test]
fn world_entry_and_world_exit_are_not_in_conflict() {
    let group = COMPONENT_CONFLICT_GROUPS
        .iter()
        .find(|g| g.contains(&WorldEntry::TYPE_NAME) && g.contains(&WorldExit::TYPE_NAME));
    assert!(
        group.is_none(),
        "WorldEntry and WorldExit must NOT be in a conflict group (portals need both)"
    );
}

#[test]
fn layer_door_conflicts_with_cover() {
    let group = COMPONENT_CONFLICT_GROUPS
        .iter()
        .find(|g| g.contains(&LayerDoor::TYPE_NAME) && g.contains(&Cover::TYPE_NAME));
    assert!(
        group.is_some(),
        "LayerDoor and Cover must be in a conflict group together"
    );
}

#[test]
fn layer_door_conflicts_with_tile_placement() {
    let group = COMPONENT_CONFLICT_GROUPS
        .iter()
        .find(|g| g.contains(&LayerDoor::TYPE_NAME) && g.contains(&TilePlacement::TYPE_NAME));
    assert!(
        group.is_some(),
        "LayerDoor and TilePlacement must be in a conflict group together"
    );
}

#[test]
fn entity_builder_with_layer_door_auto_adds_interactable_dependency() {
    let mut ecs = Ecs::default();

    let entity = ecs.create_entity()
        .with(LayerDoor::default())
        .finish();

    assert!(ecs.has::<LayerDoor>(entity));
    assert!(ecs.has::<Interactable>(entity));
}

#[test]
fn validate_layer_door_when_interactable_area_extends_outside_back_zone_union_then_reports_issue() {
    let room_id = RoomId(7);
    let world = World::from_rooms(
        WorldId(1),
        "Test".to_string(),
        vec![Room {
            id: room_id,
            position: Vec2::ZERO,
            size: Vec2::new(4.0, 4.0),
            variants: vec![RoomVariant {
                tilemap: TileMap::new(4, 4),
                layers: RoomLayers {
                    back: Some(BackRoomLayer {
                        composition_mode: LayerCompositionMode::Hidden,
                        zone_scope: InteriorZoneScope::Occupied,
                        interior_zones: vec![InteriorZone {
                            id: InteriorZoneId(1),
                            bounds: InteriorZoneBounds::new(0, 0, 32, 32),
                        }],
                    }),
                },
                ..Default::default()
            }],
            ..Default::default()
        }],
        16.0,
    );

    let mut ecs = Ecs::default();
    let entity = ecs.create_entity()
        .with(Transform {
            position: Vec2::new(28.0, 8.0),
            ..Default::default()
        })
        .with(Interactable::rect(Vec2::ZERO, Vec2::new(16.0, 16.0)))
        .with(LayerDoor::default())
        .with_current_room(room_id)
        .finish();

    assert_eq!(
        validate_layer_door(&ecs, &world, entity).err(),
        Some(LayerDoorValidationIssue::InteractableOutsideBackBounds)
    );
}
