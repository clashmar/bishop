use crate::assets::AssetKey;
use crate::ecs::{Active, AudioSource, CurrentFrame, Ecs, Entity, Global, Glow, Script, ScriptId, Sprite};
use crate::game::Game;
use crate::worlds::{RoomId, TraversalTopology, WorldId};
use std::collections::{BTreeSet, HashMap};

type RoomAssetClaims = HashMap<RoomId, BTreeSet<AssetKey>>;
type WorldAssetClaims = HashMap<WorldId, BTreeSet<AssetKey>>;
type EntityAssetClaims = HashMap<Entity, BTreeSet<AssetKey>>;

/// Bundles residency claims derived from the current traversal state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DerivedTraversalClaims {
    /// Per-room asset claims for the current room frontier.
    pub room_claims: RoomAssetClaims,
    /// Per-world asset claims for the current world frontier.
    pub world_claims: WorldAssetClaims,
    /// Per-entity asset claims for pinned-active entities.
    pub pinned_entity_claims: EntityAssetClaims,
    /// Asset claims for global entities (persist across all rooms/worlds).
    pub global_claims: BTreeSet<AssetKey>,
}

/// Collects hydratable assets referenced by entities in a room.
/// Global entities are excluded — their assets are claimed under the Global scope.
pub fn collect_room_assets(ecs: &Ecs, room_id: RoomId) -> BTreeSet<AssetKey> {
    let mut assets = BTreeSet::new();
    for &entity in ecs.entities_in_room(room_id) {
        if ecs.has::<Global>(entity) {
            continue;
        }
        assets.extend(collect_entity_assets(ecs, entity));
    }
    assets
}

/// Collects hydratable assets from all global entities.
pub fn collect_global_assets(ecs: &Ecs) -> BTreeSet<AssetKey> {
    let mut assets = BTreeSet::new();
    for &entity in ecs.get_store::<Global>().data.keys() {
        assets.extend(collect_entity_assets(ecs, entity));
    }
    assets
}

/// Collects world-scoped asset claims for the current world frontier.
pub fn collect_world_assets(
    game: &Game,
    topology: &TraversalTopology,
    current_room: RoomId,
) -> HashMap<WorldId, BTreeSet<AssetKey>> {
    let mut claims = HashMap::new();
    let room_worlds = game.room_world_map();
    let Some(&current_world) = room_worlds.get(&current_room) else {
        return claims;
    };

    for world_id in topology.world_frontier(current_world) {
        let assets = game
            .get_world(world_id)
            .and_then(|world| world.singleton)
            .map(|entity| collect_entity_assets(&game.ecs, entity))
            .unwrap_or_default();
        claims.insert(world_id, assets);
    }

    claims
}

/// Collects asset claims for entities that are pinned active.
pub fn collect_pinned_entity_claims(ecs: &Ecs) -> HashMap<Entity, BTreeSet<AssetKey>> {
    let mut claims = HashMap::new();
    for (&entity, active) in ecs.get_store::<Active>().data.iter() {
        if active.pin_count == 0 {
            continue;
        }
        let assets = collect_entity_assets(ecs, entity);
        if !assets.is_empty() {
            claims.insert(entity, assets);
        }
    }
    claims
}

/// Derives all traversal residency claims from the current game state.
pub fn derive_traversal_claims(
    game: &Game,
    topology: &TraversalTopology,
) -> DerivedTraversalClaims {
    let current_room = game.current_world().current_room_id.unwrap_or_default();
    let frontier = topology.room_frontier(current_room);
    let room_claims = frontier
        .iter()
        .map(|room_id| (*room_id, collect_room_assets(&game.ecs, *room_id)))
        .collect();
    let world_claims = collect_world_assets(game, topology, current_room);
    let pinned_entity_claims = collect_pinned_entity_claims(&game.ecs);
    let global_claims = collect_global_assets(&game.ecs);

    DerivedTraversalClaims {
        room_claims,
        world_claims,
        pinned_entity_claims,
        global_claims,
    }
}

/// Collects hydratable asset keys from an entity's components.
pub(crate) fn collect_entity_assets(ecs: &Ecs, entity: Entity) -> BTreeSet<AssetKey> {
    let mut assets = BTreeSet::new();

    if let Some(sprite) = ecs.get::<Sprite>(entity) {
        assets.insert(AssetKey::Sprite(sprite.sprite));
    }
    if let Some(current_frame) = ecs.get::<CurrentFrame>(entity) {
        assets.insert(AssetKey::Sprite(current_frame.sprite_id));
    }
    if let Some(glow) = ecs.get::<Glow>(entity) {
        assets.insert(AssetKey::Sprite(glow.sprite_id));
    }
    if let Some(script) = ecs.get::<Script>(entity)
        && script.script_id != ScriptId(0)
    {
        assets.insert(AssetKey::Script(script.script_id));
    }
    if let Some(audio) = ecs.get::<AudioSource>(entity) {
        for sound_id in audio.all_sound_ids() {
            assets.insert(AssetKey::Sound(sound_id));
        }
    }

    assets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetKey;
    use crate::ecs::{
        Active, AudioGroup, AudioSource, Ecs, Global, Script, ScriptId, SoundGroupId,
        SoundId, Sprite, SpriteId, WorldExit,
    };
    use crate::worlds::{ExitDestination, Room, RoomId, World, WorldExitTrigger, WorldId};

    #[test]
    fn room_claims_collect_assets_from_entities_in_the_frontier_room() {
        let mut ecs = Ecs::default();
        let _entity = ecs
            .create_entity()
            .with(Active::default())
            .with(Sprite {
                sprite: SpriteId(7),
            })
            .with(Script {
                script_id: ScriptId(3),
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();

        let assets = collect_room_assets(&ecs, RoomId(1));
        assert!(assets.contains(&AssetKey::Sprite(SpriteId(7))));
        assert!(assets.contains(&AssetKey::Script(ScriptId(3))));
    }

    #[test]
    fn world_claims_collect_assets_from_world_singletons_in_the_frontier() {
        let mut game = Game::default();

        let mut world_a = World::new(WorldId(1), "Overworld".to_string(), 16.0);
        world_a.add_room(Room {
            id: RoomId(1),
            ..Default::default()
        });
        world_a.current_room_id = Some(RoomId(1));

        let world_a_singleton = game
            .ecs
            .create_entity()
            .with(Active::default())
            .with(Script {
                script_id: ScriptId(11),
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();
        world_a.singleton = Some(world_a_singleton);

        let mut world_b = World::new(WorldId(2), "Dungeon".to_string(), 16.0);
        world_b.add_room(Room {
            id: RoomId(2),
            ..Default::default()
        });
        let world_b_singleton = game
            .ecs
            .create_entity()
            .with(Active::default())
            .with(AudioSource {
                groups: [(
                    SoundGroupId::Custom("ambient".to_string()),
                    AudioGroup {
                        sounds: vec![SoundId(9)],
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            })
            .with_current_room(RoomId(2))
            .finish();
        world_b.singleton = Some(world_b_singleton);

        game.add_world(world_a);
        game.add_world(world_b);
        game.current_world_id = Some(WorldId(1));

        let _exit_entity = game
            .ecs
            .create_entity()
            .with(WorldExit {
                destination: Some(ExitDestination::World(WorldId(2))),
                entry: None,
                trigger: WorldExitTrigger::OnInteract,
            })
            .with_current_room(RoomId(1))
            .finish();

        let topology = crate::worlds::extract_topology(&game);
        let claims = collect_world_assets(&game, &topology, RoomId(1));

        assert!(claims[&WorldId(1)].contains(&AssetKey::Script(ScriptId(11))));
        assert!(claims[&WorldId(2)].contains(&AssetKey::Sound(SoundId(9))));
    }

    #[test]
    fn global_entities_are_excluded_from_room_claims() {
        let mut ecs = Ecs::default();
        let _global = ecs
            .create_entity()
            .with(Global {})
            .with(Script {
                script_id: ScriptId(2),
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();
        let _local = ecs
            .create_entity()
            .with(Sprite {
                sprite: SpriteId(7),
            })
            .with_current_room(RoomId(1))
            .finish();

        let assets = collect_room_assets(&ecs, RoomId(1));
        assert!(assets.contains(&AssetKey::Sprite(SpriteId(7))));
        assert!(!assets.contains(&AssetKey::Script(ScriptId(2))));
    }

    #[test]
    fn global_assets_collects_from_all_global_entities() {
        let mut ecs = Ecs::default();
        let _player = ecs
            .create_entity()
            .with(Global {})
            .with(Script {
                script_id: ScriptId(2),
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();
        let _other = ecs
            .create_entity()
            .with(Global {})
            .with(Sprite {
                sprite: SpriteId(9),
            })
            .with_current_room(RoomId(2))
            .finish();

        let assets = collect_global_assets(&ecs);
        assert!(assets.contains(&AssetKey::Script(ScriptId(2))));
        assert!(assets.contains(&AssetKey::Sprite(SpriteId(9))));
    }

    #[test]
    fn derive_traversal_claims_includes_global_claims() {
        let mut game = Game::default();
        let mut world = World::new(WorldId(1), "Test".to_string(), 16.0);
        world.add_room(Room {
            id: RoomId(1),
            ..Default::default()
        });
        world.current_room_id = Some(RoomId(1));
        game.add_world(world);
        game.current_world_id = Some(WorldId(1));

        game.ecs
            .create_entity()
            .with(Global {})
            .with(Script {
                script_id: ScriptId(5),
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();

        let topology = crate::worlds::extract_topology(&game);
        let claims = derive_traversal_claims(&game, &topology);
        assert!(claims.global_claims.contains(&AssetKey::Script(ScriptId(5))));
    }

    #[test]
    fn pinned_active_entity_keeps_required_payloads_outside_warm_window() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Active::new(false))
            .with(Script {
                script_id: ScriptId(9),
                ..Default::default()
            })
            .with(Sprite {
                sprite: SpriteId(6),
            })
            .with(AudioSource {
                groups: [(
                    SoundGroupId::Custom("growl".to_string()),
                    AudioGroup {
                        sounds: vec![SoundId(4)],
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            })
            .with_current_room(RoomId(2))
            .finish();
        ecs.get_mut::<Active>(entity).unwrap().pin();

        let pinned = collect_pinned_entity_claims(&ecs);
        assert!(pinned[&entity].contains(&AssetKey::Script(ScriptId(9))));
        assert!(pinned[&entity].contains(&AssetKey::Sprite(SpriteId(6))));
        assert!(pinned[&entity].contains(&AssetKey::Sound(SoundId(4))));
    }
}
