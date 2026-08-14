use crate::audio::scoped_playback::reconcile_scoped_audio;
use crate::engine::game_instance::GameInstance;
use crate::scripting::script_system::ScriptSystem;
use engine_core::audio::AudioManager;
use engine_core::diagnostics::{
    PinnedEntitySnapshot, TraversalAssetEvent, TraversalClassCount, TraversalOutcomeSnapshot,
    TraversalResidencySnapshot, WarmRoomSnapshot, WarmWorldSnapshot,
};
use engine_core::ecs::{Active, CurrentRoom, Entity, Global};
use engine_core::game::Game;
use engine_core::hydration::{
    self, DerivedTraversalClaims, HydrationCoordinator, HydrationDriver, HydrationError,
    HydrationScope, ScopeKey, ResourceClass, ResidencyKey,
};
use engine_core::logging::omni_error;
use engine_core::worlds::topology::{extract_topology, RoomEdgeKind, TraversalTopology};
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
                    active.active = room_id == current_room;
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
    let previous_keys = current_traversal_scope_keys(&game_instance.game.hydration_coordinator);
    let desired_keys = desired_traversal_scope_keys(&claims);
    let previous_owner_counts = key_owner_counts(&previous_keys);
    let desired_owner_counts = key_owner_counts(&desired_keys);

    let mut events: Vec<TraversalAssetEvent> = Vec::new();
    let mut outcomes: BTreeMap<String, ScopeOutcomeAccumulator> = BTreeMap::new();
    let mut all_scopes: Vec<HydrationScope> = previous_keys
        .keys()
        .chain(desired_keys.keys())
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    all_scopes.sort_by_key(|scope| format!("{:?}", scope));

    for scope in all_scopes {
        let old_set = previous_keys.get(&scope);
        let new_set = desired_keys.get(&scope);
        let previous_label = previous_scope_labels
            .get(&scope)
            .cloned()
            .unwrap_or_else(|| fallback_scope_label(&scope));
        let desired_label = desired_scope_labels
            .get(&scope)
            .cloned()
            .unwrap_or_else(|| previous_label.clone());

        let sets_identical = old_set.is_some_and(|old| new_set.is_some_and(|new| old == new));
        if sets_identical && game_instance.game.hydration_coordinator.is_active(&scope) {
            continue;
        }

        if !sets_identical {
            let removed: Vec<ResidencyKey> = match (old_set, new_set) {
                (Some(old), Some(new)) => old.iter().filter(|k| !new.contains(k)).copied().collect(),
                (Some(old), None) => old.iter().copied().collect(),
                (None, _) => Vec::new(),
            };
            if !removed.is_empty() {
                let bucket = outcomes.entry(previous_label.clone()).or_default();
                for key in removed {
                    let should_dehydrate_payload = matches!(key, ResidencyKey::Scope(_))
                        && desired_owner_counts.get(&key).copied().unwrap_or(0) == 0
                        && is_primary_scope_for_key(&previous_keys, &scope, key);

                    if should_dehydrate_payload {
                        if let ResidencyKey::Scope(scope) = key {
                            let entities = scope_entities_for_key(&game_instance.game, scope);
                            ScriptSystem::deactivate_payload_scripts(
                                &game_instance.game.ecs,
                                &mut game_instance.game.script_manager,
                                &entities,
                            );
                        }
                    }
                    if matches!(key, ResidencyKey::Asset(_)) || should_dehydrate_payload {
                        if let Some(audio_manager) = &mut audio_manager {
                            let mut driver = HydrationDriver {
                                game: &mut game_instance.game,
                                audio_manager,
                            };
                            driver.dehydrate_key(key);
                        }
                    }
                    game_instance
                        .game
                        .hydration_coordinator
                        .release(scope.clone(), key);
                    if let ResidencyKey::Asset(asset) = key {
                        events.push(TraversalAssetEvent {
                            asset,
                            hydrated: false,
                        });
                    }
                    increment_key_count(&mut bucket.evicted, key);
                }
            }
        }

        if desired_keys.contains_key(&scope) {
            if !game_instance.game.hydration_coordinator.is_active(&scope) {
                game_instance
                    .game
                    .hydration_coordinator
                    .activate_scope(scope.clone());
            }

            if !sets_identical {
                let added: Vec<ResidencyKey> = match (old_set, new_set) {
                    (Some(old), Some(new)) => new.iter().filter(|k| !old.contains(k)).copied().collect(),
                    (None, Some(new)) => new.iter().copied().collect(),
                    (Some(_), None) | (None, None) => Vec::new(),
                };
                if !added.is_empty() {
                    let bucket = outcomes.entry(desired_label.clone()).or_default();
                    for key in added {
                        game_instance
                            .game
                            .hydration_coordinator
                            .claim(scope.clone(), key);

                        let should_hydrate_payload = matches!(key, ResidencyKey::Scope(_))
                            && previous_owner_counts.get(&key).copied().unwrap_or(0) == 0
                            && is_primary_scope_for_key(&desired_keys, &scope, key);

                        if let Some(lua) = lua {
                            if let Some(audio_manager) = &mut audio_manager {
                                if !matches!(key, ResidencyKey::Asset(_)) && !should_hydrate_payload {
                                    continue;
                                }

                            let result = {
                                let mut driver = HydrationDriver {
                                    game: &mut game_instance.game,
                                    audio_manager,
                                };
                                driver.hydrate_key_runtime(key, lua)
                            };

                                match result {
                                    Ok(()) => {
                                        if should_hydrate_payload {
                                            if let ResidencyKey::Scope(scope) = key {
                                                let entities =
                                                    scope_entities_for_key(&game_instance.game, scope);
                                                let game_ctx = game_instance.game.ctx_mut();
                                                if let Err(error) = ScriptSystem::activate_payload_scripts(
                                                    lua,
                                                    game_ctx.ecs,
                                                    game_ctx.script_manager,
                                                    &entities,
                                                ) {
                                                    bucket.failures += 1;
                                                    omni_error!(
                                                        "Traversal script activation failed for {:?}: {}",
                                                        scope,
                                                        error
                                                    );
                                                    continue;
                                                }
                                            }
                                        }
                                        if let ResidencyKey::Asset(asset) = key {
                                            events.push(TraversalAssetEvent {
                                                asset,
                                                hydrated: true,
                                            });
                                        }
                                        increment_key_count(&mut bucket.hydrated, key);
                                    }
                                    Err(error) => {
                                        bucket.failures += 1;
                                        log_hydration_error(&scope, key, &error);
                                    }
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

    if let Some(audio_manager) = &mut audio_manager {
        reconcile_scoped_audio(&game_instance.game, audio_manager);
    }

    if let Some(diagnostics) = &mut game_instance.traversal_residency_diagnostics {
        for (scope, keys) in &desired_keys {
            let label = desired_scope_labels
                .get(scope)
                .cloned()
                .unwrap_or_else(|| fallback_scope_label(scope));
            let bucket = outcomes.entry(label).or_default();
            bucket.claimed = class_count_map(keys);
        }
        let snapshot = build_traversal_snapshot(&game_instance.game, &claims, &reasons, outcomes);
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
        .map(|(&room_id, keys)| WarmRoomSnapshot {
            room_id,
            reasons: reasons.room_reasons.get(&room_id).cloned().unwrap_or_default(),
            claims: class_counts_from_map(class_count_map(keys)),
        })
        .collect();
    rooms.sort_by_key(|room| room.room_id);

    let mut worlds: Vec<WarmWorldSnapshot> = claims
        .world_claims
        .iter()
        .map(|(&world_id, keys)| WarmWorldSnapshot {
            world_id,
            reasons: reasons.world_reasons.get(&world_id).cloned().unwrap_or_default(),
            claims: class_counts_from_map(class_count_map(keys)),
        })
        .collect();
    worlds.sort_by_key(|world| world.world_id);

    let mut pinned_entities: Vec<PinnedEntitySnapshot> = claims
        .pinned_entity_claims
        .iter()
        .filter_map(|(&entity, keys)| {
            let active = game.ecs.get::<Active>(entity)?;
            Some(PinnedEntitySnapshot {
                entity,
                room_id: game.ecs.get::<CurrentRoom>(entity).map(|room| room.room_id),
                pin_count: active.pin_count,
                reasons: vec![format!(
                    "pin_count={} keeps payload resident",
                    active.pin_count
                )],
                claims: class_counts_from_map(class_count_map(keys)),
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
    let current_room_id = game.current_world().current_room_id.unwrap_or_default();
    let current_world = game.current_world().id;

    let mut room_reasons: HashMap<RoomId, BTreeSet<String>> = HashMap::new();
    room_reasons
        .entry(current_room_id)
        .or_default()
        .insert("current room".to_string());

    for edge in topology.room_graph.edges_from(current_room_id) {
        let reason = match edge.kind {
            RoomEdgeKind::Adjacency => format!("adjacent to Room({})", current_room_id.0),
            RoomEdgeKind::RoomExit => format!("exit from Room({})", current_room_id.0),
            RoomEdgeKind::WorldExit => format!("portal from Room({})", current_room_id.0),
            RoomEdgeKind::ScriptedTraversal => {
                format!("scripted traversal from Room({})", current_room_id.0)
            }
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

fn desired_traversal_scope_keys(
    claims: &DerivedTraversalClaims,
) -> HashMap<HydrationScope, BTreeSet<ResidencyKey>> {
    let mut scopes = HashMap::new();

    for (&room_id, keys) in &claims.room_claims {
        scopes.insert(HydrationScope::Room(room_id), keys.clone());
    }
    for (&world_id, keys) in &claims.world_claims {
        scopes.insert(HydrationScope::World(world_id), keys.clone());
    }
    for (&entity, keys) in &claims.pinned_entity_claims {
        scopes.insert(HydrationScope::Entity(entity), keys.clone());
    }
    if !claims.global_claims.is_empty() {
        scopes.insert(HydrationScope::Global, claims.global_claims.clone());
    }

    scopes
}

fn current_traversal_scope_keys(
    coordinator: &HydrationCoordinator,
) -> HashMap<HydrationScope, BTreeSet<ResidencyKey>> {
    coordinator
        .active_scopes()
        .into_iter()
        .filter(is_traversal_scope)
        .map(|scope| {
            let keys = coordinator.claimed_keys(&scope).into_iter().collect();
            (scope, keys)
        })
        .collect()
}

fn scope_entities_for_key(game: &Game, scope: ScopeKey) -> Vec<Entity> {
    match scope {
        ScopeKey::Global => game
            .ecs
            .get_store::<Global>()
            .data
            .keys()
            .copied()
            .collect(),
        ScopeKey::World(world_id) => game
            .get_world(world_id)
            .map(|world| world.singleton)
            .into_iter()
            .collect(),
        ScopeKey::Room(room_id) => game.ecs.entities_in_room(room_id).iter().copied().collect(),
    }
}

fn key_owner_counts(
    scopes: &HashMap<HydrationScope, BTreeSet<ResidencyKey>>,
) -> HashMap<ResidencyKey, usize> {
    let mut counts = HashMap::new();
    for keys in scopes.values() {
        for &key in keys {
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
}

fn is_primary_scope_for_key(
    scopes: &HashMap<HydrationScope, BTreeSet<ResidencyKey>>,
    scope: &HydrationScope,
    key: ResidencyKey,
) -> bool {
    let mut owners = scopes
        .iter()
        .filter(|(_, keys)| keys.contains(&key))
        .map(|(scope, _)| scope.clone())
        .collect::<Vec<_>>();
    owners.sort_by_key(|scope| format!("{:?}", scope));
    owners.first().is_some_and(|owner| owner == scope)
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
    for (&entity, keys) in &claims.pinned_entity_claims {
        let scope = HydrationScope::Entity(entity);
        let reasons = vec![format!("pinned entity ({} claims)", keys.len())];
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
        HydrationScope::Room(_)
            | HydrationScope::World(_)
            | HydrationScope::Entity(_)
            | HydrationScope::Global
    )
}

fn class_counts_from_map(counts: BTreeMap<ResourceClass, usize>) -> Vec<TraversalClassCount> {
    counts
        .into_iter()
        .map(|(class, count)| TraversalClassCount { class, count })
        .collect()
}

fn class_count_map(keys: &BTreeSet<ResidencyKey>) -> BTreeMap<ResourceClass, usize> {
    let mut counts = BTreeMap::new();
    for &key in keys {
        increment_key_count(&mut counts, key);
    }
    counts
}

fn increment_key_count(counts: &mut BTreeMap<ResourceClass, usize>, key: ResidencyKey) {
    let Some(class) = ResourceClass::for_residency_key(key) else {
        return;
    };
    *counts.entry(class).or_default() += 1;
}

fn log_hydration_error(scope: &HydrationScope, key: ResidencyKey, error: &HydrationError) {
    omni_error!(
        "Traversal hydration failed for {:?} key {:?}: {:?}",
        scope,
        key,
        error
    );
}
