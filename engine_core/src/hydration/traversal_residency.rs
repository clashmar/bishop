use crate::assets::AssetKey;
use crate::ecs::{Active, AudioSource, CurrentFrame, Ecs, Entity, Glow, Script, ScriptId, Sprite};
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
}

/// Collects hydratable assets referenced by entities in a room.
pub fn collect_room_assets(ecs: &Ecs, room_id: RoomId) -> BTreeSet<AssetKey> {
    let mut assets = BTreeSet::new();
    for &entity in ecs.entities_in_room(room_id) {
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
    claims.insert(current_world, BTreeSet::new());
    for world_id in topology.world_graph.neighbors(current_world) {
        claims.entry(world_id).or_default();
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
        let mut assets = BTreeSet::new();
        if let Some(script) = ecs.get::<Script>(entity)
            && script.script_id != ScriptId(0)
        {
            assets.insert(AssetKey::Script(script.script_id));
        }
        if let Some(sprite) = ecs.get::<Sprite>(entity) {
            assets.insert(AssetKey::Sprite(sprite.sprite));
        }
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

    DerivedTraversalClaims {
        room_claims,
        world_claims,
        pinned_entity_claims,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetKey;
    use crate::ecs::{Active, Ecs, Script, ScriptId, Sprite, SpriteId};
    use crate::worlds::RoomId;

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
    fn pinned_active_entity_keeps_required_payloads_outside_warm_window() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Active::new(false))
            .with(Script {
                script_id: ScriptId(9),
                ..Default::default()
            })
            .with_current_room(RoomId(2))
            .finish();
        ecs.get_mut::<Active>(entity).unwrap().pin();

        let pinned = collect_pinned_entity_claims(&ecs);
        assert!(pinned[&entity].contains(&AssetKey::Script(ScriptId(9))));
    }
}
