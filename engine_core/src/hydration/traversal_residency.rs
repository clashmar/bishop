use crate::assets::AssetKey;
use crate::ecs::{
    Active, AudioSource, CurrentFrame, Ecs, Entity, Global, Glow, Script, ScriptId, Sprite,
};
use crate::game::Game;
use crate::hydration::{ScopeKey, ResidencyKey};
use crate::worlds::{RoomId, TraversalTopology, WorldId};
use std::collections::{BTreeSet, HashMap};

type RoomResidencyClaims = HashMap<RoomId, BTreeSet<ResidencyKey>>;
type WorldResidencyClaims = HashMap<WorldId, BTreeSet<ResidencyKey>>;
type EntityResidencyClaims = HashMap<Entity, BTreeSet<ResidencyKey>>;

/// Bundles residency claims derived from the current traversal state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DerivedTraversalClaims {
    /// Per-room residency claims for the current room frontier.
    pub room_claims: RoomResidencyClaims,
    /// Per-world residency claims implied by the current room frontier.
    pub world_claims: WorldResidencyClaims,
    /// Per-entity residency claims for pinned-active entities.
    pub pinned_entity_claims: EntityResidencyClaims,
    /// Residency claims for global entities (persist across all rooms/worlds).
    pub global_claims: BTreeSet<ResidencyKey>,
}

/// Collects traversal residency claims referenced by entities in a room.
/// Global entities are excluded — their assets are claimed under the Global scope.
pub fn collect_room_claims(game: &Game, room_id: RoomId) -> BTreeSet<ResidencyKey> {
    let mut claims = BTreeSet::new();
    claims.insert(ResidencyKey::Scope(ScopeKey::Room(room_id)));

    for &entity in game.ecs.entities_in_room(room_id) {
        if game.ecs.has::<Global>(entity) {
            continue;
        }
        claims.extend(
            collect_entity_assets(&game.ecs, entity)
                .into_iter()
                .map(ResidencyKey::Asset),
        );
    }
    claims
}

/// Collects traversal residency claims from all global entities.
pub fn collect_global_claims(ecs: &Ecs) -> BTreeSet<ResidencyKey> {
    let mut claims = BTreeSet::new();
    for &entity in ecs.get_store::<Global>().data.keys() {
        claims.extend(
            collect_entity_assets(ecs, entity)
                .into_iter()
                .map(ResidencyKey::Asset),
        );
    }
    claims
}

/// Collects world-scoped residency claims for the current world frontier.
pub fn collect_world_claims(
    game: &Game,
    topology: &TraversalTopology,
    current_room: RoomId,
) -> HashMap<WorldId, BTreeSet<ResidencyKey>> {
    let room_worlds = game.room_world_map();
    let Some(&current_world) = room_worlds.get(&current_room) else {
        return HashMap::new();
    };

    collect_world_claims_for_ids(game, topology.world_frontier(current_world))
}

/// Collects residency claims for entities that are pinned active.
pub fn collect_pinned_entity_claims(game: &Game) -> HashMap<Entity, BTreeSet<ResidencyKey>> {
    let mut claims = HashMap::new();
    for (&entity, active) in game.ecs.get_store::<Active>().data.iter() {
        if active.pin_count == 0 {
            continue;
        }

        let mut entity_claims = collect_entity_assets(&game.ecs, entity)
            .into_iter()
            .map(ResidencyKey::Asset)
            .collect::<BTreeSet<_>>();

        if let Some(room_id) = game.ecs.get::<crate::ecs::CurrentRoom>(entity).map(|room| room.0)
        {
            entity_claims.insert(ResidencyKey::Scope(ScopeKey::Room(room_id)));
        }

        if !entity_claims.is_empty() {
            claims.insert(entity, entity_claims);
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
        .map(|room_id| (*room_id, collect_room_claims(game, *room_id)))
        .collect();
    let world_ids = frontier
        .iter()
        .filter_map(|room_id| topology.world_for_room(*room_id))
        .collect::<BTreeSet<_>>();
    let world_claims = collect_world_claims_for_ids(game, world_ids);
    let pinned_entity_claims = collect_pinned_entity_claims(game);
    let global_claims = collect_global_claims(&game.ecs);

    DerivedTraversalClaims {
        room_claims,
        world_claims,
        pinned_entity_claims,
        global_claims,
    }
}

fn collect_world_claims_for_ids(
    game: &Game,
    world_ids: BTreeSet<WorldId>,
) -> HashMap<WorldId, BTreeSet<ResidencyKey>> {
    let mut claims = HashMap::new();

    for world_id in world_ids {
        let mut world_claims = BTreeSet::new();
        if let Some(world) = game.get_world(world_id) {
            world_claims.insert(ResidencyKey::Scope(ScopeKey::World(world_id)));
            world_claims.extend(
                collect_entity_assets(&game.ecs, world.singleton)
                    .into_iter()
                    .map(ResidencyKey::Asset),
            );
        }
        claims.insert(world_id, world_claims);
    }

    claims
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
        Active, AudioGroup, AudioSource, Ecs, Global, Script, ScriptId, SoundGroupId, SoundId,
        Sprite, SpriteId, WorldEntry, WorldExit,
    };
    use crate::worlds::{ExitDestination, Room, RoomId, World, WorldExitTrigger, WorldId};

    #[test]
    fn room_claims_include_room_payload_and_entity_assets_in_the_frontier() {
        let mut game = Game::default();
        let mut world = World::new(WorldId(1), "Test".to_string(), 16.0);
        world.current_room_id = Some(RoomId(1));
        world.add_room(Room {
            id: RoomId(1),
            ..Default::default()
        });
        game.add_world(world);

        let _entity = game
            .ecs
            .create_entity()
            .with(Active::default())
            .with(Sprite { sprite: SpriteId(7) })
            .with(Script {
                script_id: ScriptId(3),
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();

        let claims = collect_room_claims(&game, RoomId(1));
        assert!(claims.contains(&ResidencyKey::Scope(ScopeKey::Room(RoomId(1)))));
        assert!(claims.contains(&ResidencyKey::Asset(AssetKey::Sprite(SpriteId(7)))));
        assert!(claims.contains(&ResidencyKey::Asset(AssetKey::Script(ScriptId(3)))));
    }

    #[test]
    fn world_claims_include_scopes_and_world_singleton_assets_in_the_frontier() {
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
        world_a.singleton = world_a_singleton;

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
        world_b.singleton = world_b_singleton;

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
        let claims = collect_world_claims(&game, &topology, RoomId(1));

        assert!(claims[&WorldId(1)].contains(&ResidencyKey::Scope(ScopeKey::World(WorldId(1)))));
        assert!(claims[&WorldId(1)].contains(&ResidencyKey::Asset(AssetKey::Script(ScriptId(11)))));
        assert!(claims[&WorldId(2)].contains(&ResidencyKey::Scope(ScopeKey::World(WorldId(2)))));
        assert!(claims[&WorldId(2)].contains(&ResidencyKey::Asset(AssetKey::Sound(SoundId(9)))));
    }

    #[test]
    fn global_entities_are_excluded_from_room_claims() {
        let mut game = Game::default();
        let mut world = World::new(WorldId(1), "Test".to_string(), 16.0);
        world.add_room(Room {
            id: RoomId(1),
            ..Default::default()
        });
        game.add_world(world);

        let _global = game
            .ecs
            .create_entity()
            .with(Global {})
            .with(Script {
                script_id: ScriptId(2),
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();
        let _local = game
            .ecs
            .create_entity()
            .with(Sprite { sprite: SpriteId(7) })
            .with_current_room(RoomId(1))
            .finish();

        let claims = collect_room_claims(&game, RoomId(1));
        assert!(claims.contains(&ResidencyKey::Asset(AssetKey::Sprite(SpriteId(7)))));
        assert!(!claims.contains(&ResidencyKey::Asset(AssetKey::Script(ScriptId(2)))));
    }

    #[test]
    fn global_claims_collects_from_all_global_entities() {
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
            .with(Sprite { sprite: SpriteId(9) })
            .with_current_room(RoomId(2))
            .finish();

        let claims = collect_global_claims(&ecs);
        assert!(claims.contains(&ResidencyKey::Asset(AssetKey::Script(ScriptId(2)))));
        assert!(claims.contains(&ResidencyKey::Asset(AssetKey::Sprite(SpriteId(9)))));
    }

    fn game_with_cross_world_world_exit() -> Game {
        let mut game = Game::default();

        let mut world_a = World::new(WorldId(1), "Overworld".to_string(), 16.0);
        world_a.add_room(Room {
            id: RoomId(1),
            ..Default::default()
        });
        world_a.current_room_id = Some(RoomId(1));

        let mut world_b = World::new(WorldId(2), "Dungeon".to_string(), 16.0);
        world_b.add_room(Room {
            id: RoomId(2),
            ..Default::default()
        });

        game.add_world(world_a);
        game.add_world(world_b);
        game.current_world_id = Some(WorldId(1));

        game.ecs
            .create_entity()
            .with(WorldEntry {
                name: WorldEntry::START.to_string(),
            })
            .with_current_room(RoomId(2))
            .finish();

        game.ecs
            .create_entity()
            .with(WorldExit {
                destination: Some(ExitDestination::World(WorldId(2))),
                entry: None,
                trigger: WorldExitTrigger::OnInteract,
            })
            .with_current_room(RoomId(1))
            .finish();

        game
    }

    #[test]
    fn derive_traversal_claims_when_cross_world_world_exit_exists_includes_destination_room_scope() {
        let game = game_with_cross_world_world_exit();
        let topology = crate::worlds::extract_topology(&game);
        let claims = derive_traversal_claims(&game, &topology);

        assert!(claims.room_claims.contains_key(&RoomId(1)));
        assert!(claims.room_claims.contains_key(&RoomId(2)));
        assert!(claims.world_claims.contains_key(&WorldId(1)));
        assert!(claims.world_claims.contains_key(&WorldId(2)));
        assert!(claims.world_claims[&WorldId(2)]
            .contains(&ResidencyKey::Scope(ScopeKey::World(WorldId(2)))));
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
        assert!(claims.global_claims.contains(&ResidencyKey::Asset(AssetKey::Script(ScriptId(5)))));
    }

    #[test]
    fn pinned_active_entity_keeps_required_scopes_outside_warm_window() {
        let mut game = Game::default();
        let mut world = World::new(WorldId(1), "Test".to_string(), 16.0);
        world.add_room(Room {
            id: RoomId(2),
            ..Default::default()
        });
        game.add_world(world);

        let entity = game
            .ecs
            .create_entity()
            .with(Active::new(false))
            .with(Script {
                script_id: ScriptId(9),
                ..Default::default()
            })
            .with(Sprite { sprite: SpriteId(6) })
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
        game.ecs.get_mut::<Active>(entity).unwrap().pin();

        let pinned = collect_pinned_entity_claims(&game);
        assert!(pinned[&entity].contains(&ResidencyKey::Scope(ScopeKey::Room(RoomId(2)))));
        assert!(pinned[&entity].contains(&ResidencyKey::Asset(AssetKey::Script(ScriptId(9)))));
        assert!(pinned[&entity].contains(&ResidencyKey::Asset(AssetKey::Sprite(SpriteId(6)))));
        assert!(pinned[&entity].contains(&ResidencyKey::Asset(AssetKey::Sound(SoundId(4)))));
    }
}
