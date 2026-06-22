use crate::engine::game_instance::GameInstance;
use engine_core::assets::AssetKey;
use engine_core::audio::AudioManager;
use engine_core::diagnostics::{
    PinnedEntitySnapshot, TraversalAssetEvent, TraversalClassCount, TraversalOutcomeSnapshot,
    TraversalResidencySnapshot, WarmRoomSnapshot, WarmWorldSnapshot,
};
use engine_core::ecs::{Active, CurrentRoom, Entity};
use engine_core::game::Game;
use engine_core::hydration::{
    self, DerivedTraversalClaims, HydrationCoordinator, HydrationDriver, HydrationError,
    HydrationScope, ResourceClass,
};
use engine_core::logging::omni_error;
use engine_core::worlds::topology::{RoomEdgeKind, TraversalTopology, extract_topology};
use engine_core::worlds::{RoomId, WorldId};
use mlua::Lua;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Sets Active.value per entity based on current room, respecting pin_count.
pub(crate) fn apply_current_room_default_activation(game: &mut Game) {
    let current_room = game.current_world().current_room_id.unwrap_or_default();

    let mut room_entities: Vec<(RoomId, Vec<Entity>)> = Vec::new();
    for world in game.worlds() {
        for room in world.rooms() {
            let entities: Vec<_> = game.ecs.entities_in_room(room.id).iter().copied().collect();
            room_entities.push((room.id, entities));
        }
    }

    for (room_id, entities) in room_entities {
        for entity in entities {
            if let Some(active) = game.ecs.get_mut::<Active>(entity) {
                if active.pin_count == 0 {
                    active.value = room_id == current_room;
                }
            }
        }
    }
}

#[derive(Default)]
struct ScopeOutcomeAccumulator {
    claimed: BTreeMap<ResourceClass, usize>,
    hydrated: BTreeMap<ResourceClass, usize>,
    evicted: BTreeMap<ResourceClass, usize>,
    failures: usize,
}

#[derive(Default)]
struct WarmScopeReasons {
    room_reasons: HashMap<RoomId, Vec<String>>,
    world_reasons: HashMap<WorldId, Vec<String>>,
}

pub fn refresh_after_traversal(game_instance: &mut GameInstance) {
    refresh_after_traversal_impl(None, None, game_instance);
}

pub fn refresh_after_traversal_runtime(
    lua: &Lua,
    audio_manager: &mut AudioManager,
    game_instance: &mut GameInstance,
) {
    refresh_after_traversal_impl(Some(lua), Some(audio_manager), game_instance);
}

fn refresh_after_traversal_impl(
    lua: Option<&Lua>,
    mut audio_manager: Option<&mut AudioManager>,
    game_instance: &mut GameInstance,
) {
    let topology = extract_topology(&game_instance.game);
    let claims = hydration::derive_traversal_claims(&game_instance.game, &topology);
    let reasons = if game_instance.traversal_residency_diagnostics.is_some() {
        collect_warm_scope_reasons(&game_instance.game, &topology)
    } else {
        WarmScopeReasons::default()
    };

    apply_current_room_default_activation(&mut game_instance.game);

    let diagnostics = &mut game_instance.traversal_residency_diagnostics;
    let previous_scope_labels = diagnostics
        .as_ref()
        .map(|d| scope_labels_from_snapshot(&d.snapshot))
        .unwrap_or_default();
    let desired_scope_labels = scope_labels_from_reasons(&claims, &reasons);
    let previous_assets = current_traversal_scope_assets(&game_instance.game.hydration_coordinator);
    let desired_assets = desired_traversal_scope_assets(&claims);

    let mut events: Vec<TraversalAssetEvent> = Vec::new();
    let mut outcomes: BTreeMap<String, ScopeOutcomeAccumulator> = BTreeMap::new();
    let mut all_scopes: Vec<HydrationScope> = previous_assets
        .keys()
        .chain(desired_assets.keys())
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    all_scopes.sort_by_key(|scope| format!("{:?}", scope));

    for scope in all_scopes {
        let old_set = previous_assets.get(&scope);
        let new_set = desired_assets.get(&scope);
        let previous_label = previous_scope_labels
            .get(&scope)
            .cloned()
            .unwrap_or_else(|| fallback_scope_label(&scope));
        let desired_label = desired_scope_labels
            .get(&scope)
            .cloned()
            .unwrap_or_else(|| previous_label.clone());

        let sets_identical =
            old_set.is_some_and(|old| new_set.is_some_and(|new| old == new));
        if sets_identical && game_instance.game.hydration_coordinator.is_active(&scope) {
            continue;
        }

        if !sets_identical {
            let removed: Vec<AssetKey> = match (old_set, new_set) {
                (Some(old), Some(new)) => {
                    old.iter().filter(|k| !new.contains(k)).copied().collect()
                }
                (Some(old), None) => old.iter().copied().collect(),
                (None, _) => Vec::new(),
            };
            if !removed.is_empty() {
                let bucket = outcomes.entry(previous_label.clone()).or_default();
                for asset in removed {
                    if let Some(audio_manager) = audio_manager.as_deref_mut() {
                        let mut driver = HydrationDriver {
                            coordinator: &game_instance.game.hydration_coordinator,
                            asset_registry: &game_instance.game.asset_registry,
                            sprite_manager: &mut game_instance.game.sprite_manager,
                            script_manager: &mut game_instance.game.script_manager,
                            audio_manager,
                        };
                        driver.dehydrate_asset(asset);
                    }
                    game_instance
                        .game
                        .hydration_coordinator
                        .release_asset(scope.clone(), asset);
                    events.push(TraversalAssetEvent {
                        asset,
                        hydrated: false,
                    });
                    increment_asset_count(&mut bucket.evicted, asset);
                }
            }
        }

        if desired_assets.contains_key(&scope) {
            if !game_instance.game.hydration_coordinator.is_active(&scope) {
                game_instance
                    .game
                    .hydration_coordinator
                    .activate_scope(scope.clone());
            }

            if !sets_identical {
                let added: Vec<AssetKey> = match (old_set, new_set) {
                    (Some(old), Some(new)) => {
                        new.iter().filter(|k| !old.contains(k)).copied().collect()
                    }
                    (None, Some(new)) => new.iter().copied().collect(),
                    (Some(_), None) | (None, None) => Vec::new(),
                };
                if !added.is_empty() {
                    let bucket = outcomes.entry(desired_label.clone()).or_default();
                    for asset in added {
                        game_instance
                            .game
                            .hydration_coordinator
                            .claim_asset(scope.clone(), asset);
                        if let (Some(lua), Some(audio_manager)) =
                            (lua, audio_manager.as_deref_mut())
                        {
                            let result = {
                                let mut driver = HydrationDriver {
                                    coordinator: &game_instance.game.hydration_coordinator,
                                    asset_registry: &game_instance.game.asset_registry,
                                    sprite_manager: &mut game_instance.game.sprite_manager,
                                    script_manager: &mut game_instance.game.script_manager,
                                    audio_manager,
                                };
                                driver.hydrate_asset_runtime(asset, lua)
                            };

                            match result {
                                Ok(()) => {
                                    events.push(TraversalAssetEvent {
                                        asset,
                                        hydrated: true,
                                    });
                                    increment_asset_count(&mut bucket.hydrated, asset);
                                }
                                Err(error) => {
                                    bucket.failures += 1;
                                    log_hydration_error(&scope, asset, &error);
                                }
                            }
                        }
                    }
                }
            }
        } else if game_instance.game.hydration_coordinator.is_active(&scope) {
            game_instance
                .game
                .hydration_coordinator
                .deactivate_scope(scope.clone());
        }
    }

    if let Some(diagnostics) = &mut game_instance.traversal_residency_diagnostics {
        for (scope, assets) in &desired_assets {
            let label = desired_scope_labels
                .get(scope)
                .cloned()
                .unwrap_or_else(|| fallback_scope_label(scope));
            let bucket = outcomes.entry(label).or_default();
            bucket.claimed = class_count_map(assets);
        }
        let snapshot =
            build_traversal_snapshot(&game_instance.game, &claims, &reasons, outcomes);
        diagnostics.record_refresh(snapshot, events);
    }
}

fn build_traversal_snapshot(
    game: &Game,
    claims: &DerivedTraversalClaims,
    reasons: &WarmScopeReasons,
    outcomes: BTreeMap<String, ScopeOutcomeAccumulator>,
) -> TraversalResidencySnapshot {
    let mut rooms: Vec<WarmRoomSnapshot> = claims
        .room_claims
        .iter()
        .map(|(&room_id, assets)| WarmRoomSnapshot {
            room_id,
            reasons: reasons.room_reasons.get(&room_id).cloned().unwrap_or_default(),
            claims: class_counts_from_map(class_count_map(assets)),
        })
        .collect();
    rooms.sort_by_key(|room| room.room_id);

    let mut worlds: Vec<WarmWorldSnapshot> = claims
        .world_claims
        .iter()
        .map(|(&world_id, assets)| WarmWorldSnapshot {
            world_id,
            reasons: reasons.world_reasons.get(&world_id).cloned().unwrap_or_default(),
            claims: class_counts_from_map(class_count_map(assets)),
        })
        .collect();
    worlds.sort_by_key(|world| world.world_id);

    let mut pinned_entities: Vec<PinnedEntitySnapshot> = claims
        .pinned_entity_claims
        .iter()
        .filter_map(|(&entity, assets)| {
            let active = game.ecs.get::<Active>(entity)?;
            Some(PinnedEntitySnapshot {
                entity,
                room_id: game.ecs.get::<CurrentRoom>(entity).map(|room| room.0),
                pin_count: active.pin_count,
                reasons: vec![format!(
                    "pin_count={} blocks traversal deactivation",
                    active.pin_count
                )],
                claims: class_counts_from_map(class_count_map(assets)),
            })
        })
        .collect();
    pinned_entities.sort_by_key(|entity| entity.entity);

    let outcomes = outcomes
        .into_iter()
        .map(|(label, outcome)| TraversalOutcomeSnapshot {
            label,
            claimed: class_counts_from_map(outcome.claimed),
            hydrated: class_counts_from_map(outcome.hydrated),
            evicted: class_counts_from_map(outcome.evicted),
            failures: outcome.failures,
        })
        .collect();

    TraversalResidencySnapshot {
        rooms,
        worlds,
        pinned_entities,
        global_claims: class_counts_from_map(class_count_map(&claims.global_claims)),
        outcomes,
        thrash: Vec::new(),
    }
}

fn collect_warm_scope_reasons(game: &Game, topology: &TraversalTopology) -> WarmScopeReasons {
    let current_room = game.current_world().current_room_id.unwrap_or_default();
    let current_world = game.current_world().id;

    let mut room_reasons: HashMap<RoomId, BTreeSet<String>> = HashMap::new();
    room_reasons
        .entry(current_room)
        .or_default()
        .insert("current room".to_string());

    for edge in topology.room_graph.edges_from(current_room) {
        let reason = match edge.kind {
            RoomEdgeKind::Adjacency => format!("adjacent to Room({})", current_room.0),
            RoomEdgeKind::Exit => format!("exit from Room({})", current_room.0),
            RoomEdgeKind::Portal => format!("portal from Room({})", current_room.0),
        };
        room_reasons.entry(edge.to).or_default().insert(reason);
    }

    let mut world_reasons: HashMap<WorldId, BTreeSet<String>> = HashMap::new();
    world_reasons
        .entry(current_world)
        .or_default()
        .insert("current world".to_string());
    for edge in topology.world_graph.edges_from(current_world) {
        world_reasons
            .entry(edge.to)
            .or_default()
            .insert(format!("world exit from World({})", current_world.0));
    }

    WarmScopeReasons {
        room_reasons: room_reasons
            .into_iter()
            .map(|(room, reasons)| (room, reasons.into_iter().collect()))
            .collect(),
        world_reasons: world_reasons
            .into_iter()
            .map(|(world, reasons)| (world, reasons.into_iter().collect()))
            .collect(),
    }
}

fn desired_traversal_scope_assets(
    claims: &DerivedTraversalClaims,
) -> HashMap<HydrationScope, BTreeSet<AssetKey>> {
    let mut scopes = HashMap::new();

    for (&room_id, assets) in &claims.room_claims {
        scopes.insert(HydrationScope::Room(room_id), assets.clone());
    }
    for (&world_id, assets) in &claims.world_claims {
        scopes.insert(HydrationScope::World(world_id), assets.clone());
    }
    for (&entity, assets) in &claims.pinned_entity_claims {
        scopes.insert(HydrationScope::Entity(entity), assets.clone());
    }
    if !claims.global_claims.is_empty() {
        scopes.insert(HydrationScope::Global, claims.global_claims.clone());
    }

    scopes
}

fn current_traversal_scope_assets(
    coordinator: &HydrationCoordinator,
) -> HashMap<HydrationScope, BTreeSet<AssetKey>> {
    coordinator
        .active_scopes()
        .into_iter()
        .filter(|scope| is_traversal_scope(scope))
        .map(|scope| {
            let assets = coordinator.claimed_assets(&scope).into_iter().collect();
            (scope, assets)
        })
        .collect()
}

fn scope_labels_from_snapshot(
    snapshot: &TraversalResidencySnapshot,
) -> HashMap<HydrationScope, String> {
    let mut labels = HashMap::new();

    for room in &snapshot.rooms {
        labels.insert(
            HydrationScope::Room(room.room_id),
            scope_label(&HydrationScope::Room(room.room_id), &room.reasons),
        );
    }
    for world in &snapshot.worlds {
        labels.insert(
            HydrationScope::World(world.world_id),
            scope_label(&HydrationScope::World(world.world_id), &world.reasons),
        );
    }
    for entity in &snapshot.pinned_entities {
        labels.insert(
            HydrationScope::Entity(entity.entity),
            scope_label(&HydrationScope::Entity(entity.entity), &entity.reasons),
        );
    }
    if !snapshot.global_claims.is_empty() {
        labels.insert(
            HydrationScope::Global,
            scope_label(&HydrationScope::Global, &["global entities".to_string()]),
        );
    }

    labels
}

fn scope_labels_from_reasons(
    claims: &DerivedTraversalClaims,
    reasons: &WarmScopeReasons,
) -> HashMap<HydrationScope, String> {
    let mut labels = HashMap::new();

    for &room_id in claims.room_claims.keys() {
        let scope = HydrationScope::Room(room_id);
        let scope_reasons = reasons.room_reasons.get(&room_id).cloned().unwrap_or_default();
        labels.insert(scope.clone(), scope_label(&scope, &scope_reasons));
    }
    for &world_id in claims.world_claims.keys() {
        let scope = HydrationScope::World(world_id);
        let scope_reasons = reasons
            .world_reasons
            .get(&world_id)
            .cloned()
            .unwrap_or_default();
        labels.insert(scope.clone(), scope_label(&scope, &scope_reasons));
    }
    for (&entity, assets) in &claims.pinned_entity_claims {
        let scope = HydrationScope::Entity(entity);
        let reasons = vec![format!("pinned entity ({} assets)", assets.len())];
        labels.insert(scope.clone(), scope_label(&scope, &reasons));
    }
    if !claims.global_claims.is_empty() {
        labels.insert(
            HydrationScope::Global,
            scope_label(&HydrationScope::Global, &["global entities".to_string()]),
        );
    }

    labels
}

fn scope_label(scope: &HydrationScope, reasons: &[String]) -> String {
    if reasons.is_empty() {
        return fallback_scope_label(scope);
    }
    format!("{} [{}]", fallback_scope_label(scope), reasons.join(", "))
}

fn fallback_scope_label(scope: &HydrationScope) -> String {
    match scope {
        HydrationScope::Room(room_id) => format!("Room({})", room_id.0),
        HydrationScope::World(world_id) => format!("World({})", world_id.0),
        HydrationScope::Entity(entity) => format!("Entity({})", entity.0),
        HydrationScope::Boot => "Boot".to_string(),
        HydrationScope::Global => "Global".to_string(),
    }
}

fn is_traversal_scope(scope: &HydrationScope) -> bool {
    matches!(
        scope,
        HydrationScope::Room(_) | HydrationScope::World(_) | HydrationScope::Entity(_) | HydrationScope::Global
    )
}

fn class_counts_from_map(counts: BTreeMap<ResourceClass, usize>) -> Vec<TraversalClassCount> {
    counts
        .into_iter()
        .map(|(class, count)| TraversalClassCount { class, count })
        .collect()
}

fn class_count_map(assets: &BTreeSet<AssetKey>) -> BTreeMap<ResourceClass, usize> {
    let mut counts = BTreeMap::new();
    for &asset in assets {
        increment_asset_count(&mut counts, asset);
    }
    counts
}

fn increment_asset_count(counts: &mut BTreeMap<ResourceClass, usize>, asset: AssetKey) {
    let Some(class) = ResourceClass::for_asset_key(asset) else {
        return;
    };
    *counts.entry(class).or_default() += 1;
}

fn log_hydration_error(scope: &HydrationScope, asset: AssetKey, error: &HydrationError) {
    omni_error!(
        "Traversal hydration failed for {:?} asset {:?}: {:?}",
        scope,
        asset,
        error
    );
}
