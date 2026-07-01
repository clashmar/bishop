use crate::assets::AssetKey;
use crate::ecs::Ecs;
use crate::ecs::Entity;
use crate::hydration::residency_key::ResidencyKey;
use crate::hydration::scope::{HydrationScope, ResourceClaim, ResourceClass};
use crate::hydration::traversal_residency::collect_entity_assets;
use crate::omni_debug;
use crate::worlds::RoomId;
use std::collections::{HashMap, HashSet};

type ClaimMap = HashMap<(HydrationScope, ResourceClass), HashSet<ResidencyKey>>;

/// Point-in-time picture of coordinator state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoordinatorSnapshot {
    /// Currently active scopes.
    pub active_scopes: Vec<HydrationScope>,
    /// Per-scope per-class resource claims.
    pub claims: Vec<ResourceClaim>,
}

/// Tracks scope activation and resource claims.
#[derive(Clone, Debug, Default)]
pub struct HydrationCoordinator {
    active_scopes: HashSet<HydrationScope>,
    claims: ClaimMap,
}

impl HydrationCoordinator {
    /// Mark a scope as active. Idempotent.
    pub fn activate_scope(&mut self, scope: HydrationScope) {
        omni_debug!("hydration coordinator activate scope={:?}", scope);
        self.active_scopes.insert(scope);
    }

    /// Mark a scope as inactive and clear all its claims.
    pub fn deactivate_scope(&mut self, scope: HydrationScope) {
        omni_debug!("hydration coordinator deactivate scope={:?}", scope);
        self.active_scopes.remove(&scope);
        self.claims.retain(|(claimed_scope, _), _| *claimed_scope != scope);
    }

    /// Returns true if the scope is currently active.
    pub fn is_active(&self, scope: &HydrationScope) -> bool {
        self.active_scopes.contains(scope)
    }

    /// Returns all currently active scopes in stable order.
    pub fn active_scopes(&self) -> Vec<HydrationScope> {
        let mut scopes: Vec<HydrationScope> = self.active_scopes.iter().cloned().collect();
        scopes.sort_by_key(|scope| format!("{:?}", scope));
        scopes
    }

    /// Record ownership of a residency key for a scope.
    pub fn claim(&mut self, scope: HydrationScope, key: ResidencyKey) {
        let Some(class) = ResourceClass::for_residency_key(key) else {
            return;
        };
        omni_debug!(
            "hydration coordinator claim scope={:?} class={:?} key={:?}",
            scope,
            class,
            key
        );
        self.claims.entry((scope, class)).or_default().insert(key);
    }

    /// Release ownership of a residency key for a scope.
    pub fn release(&mut self, scope: HydrationScope, key: ResidencyKey) {
        let Some(class) = ResourceClass::for_residency_key(key) else {
            return;
        };
        omni_debug!(
            "hydration coordinator release scope={:?} class={:?} key={:?}",
            scope,
            class,
            key
        );

        let mut remove_entry = false;
        if let Some(keys) = self.claims.get_mut(&(scope.clone(), class)) {
            keys.remove(&key);
            remove_entry = keys.is_empty();
        }
        if remove_entry {
            self.claims.remove(&(scope, class));
        }
    }

    /// Returns all claimed residency keys for a scope in stable order.
    pub fn claimed_keys(&self, scope: &HydrationScope) -> Vec<ResidencyKey> {
        let mut keys = self
            .claims
            .iter()
            .filter(|((claimed_scope, _), _)| claimed_scope == scope)
            .flat_map(|(_, keys)| keys.iter().copied())
            .collect::<Vec<_>>();
        keys.sort();
        keys
    }

    /// Returns all claimed residency keys for a scope/class pair in stable order.
    pub fn claimed_keys_by_class(
        &self,
        scope: &HydrationScope,
        class: ResourceClass,
    ) -> Vec<ResidencyKey> {
        let mut keys = self
            .claims
            .get(&(scope.clone(), class))
            .map(|keys| keys.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        keys.sort();
        keys
    }

    /// Returns all claimed assets for a scope in stable order.
    pub fn claimed_assets(&self, scope: &HydrationScope) -> Vec<AssetKey> {
        let mut assets = self
            .claimed_keys(scope)
            .into_iter()
            .filter_map(|key| match key {
                ResidencyKey::Asset(asset) => Some(asset),
                ResidencyKey::Scope(_) => None,
            })
            .collect::<Vec<_>>();
        assets.sort();
        assets
    }

    /// Returns all claimed assets for a scope/class pair in stable order.
    pub fn claimed_assets_by_class(
        &self,
        scope: &HydrationScope,
        class: ResourceClass,
    ) -> Vec<AssetKey> {
        let mut assets = self
            .claimed_keys_by_class(scope, class)
            .into_iter()
            .filter_map(|key| match key {
                ResidencyKey::Asset(asset) => Some(asset),
                ResidencyKey::Scope(_) => None,
            })
            .collect::<Vec<_>>();
        assets.sort();
        assets
    }

    /// Returns the number of claimed residency keys for a scope/class pair.
    pub fn claim_count(&self, scope: &HydrationScope, class: ResourceClass) -> usize {
        self.claims
            .get(&(scope.clone(), class))
            .map(HashSet::len)
            .unwrap_or(0)
    }

    /// Returns a point-in-time snapshot of coordinator state.
    pub fn snapshot(&self) -> CoordinatorSnapshot {
        let scopes = self.active_scopes();

        let mut claims = self
            .claims
            .iter()
            .map(|((scope, class), keys)| {
                let mut keys = keys.iter().copied().collect::<Vec<_>>();
                keys.sort();
                ResourceClaim {
                    scope: scope.clone(),
                    class: *class,
                    keys,
                }
            })
            .collect::<Vec<_>>();
        claims.sort_by_key(|claim| (format!("{:?}", claim.scope), claim.class));

        CoordinatorSnapshot {
            active_scopes: scopes,
            claims,
        }
    }

    /// Claims all hydratable assets referenced by an entity's components
    /// under the given room scope. The scope must already be active.
    pub fn claim_entity_assets(&mut self, ecs: &Ecs, entity: Entity, room_id: RoomId) {
        let scope = HydrationScope::Room(room_id);
        debug_assert!(
            self.is_active(&scope),
            "claim_entity_assets called for inactive scope {:?}",
            scope
        );
        for asset in collect_entity_assets(ecs, entity) {
            self.claim(scope.clone(), ResidencyKey::Asset(asset));
        }
    }

    /// Releases all hydratable assets previously claimed for an entity
    /// under the given room scope. The scope must already be active.
    pub fn release_entity_assets(&mut self, ecs: &Ecs, entity: Entity, room_id: RoomId) {
        let scope = HydrationScope::Room(room_id);
        debug_assert!(
            self.is_active(&scope),
            "release_entity_assets called for inactive scope {:?}",
            scope
        );
        for asset in collect_entity_assets(ecs, entity) {
            self.release(scope.clone(), ResidencyKey::Asset(asset));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{Entity, Script, ScriptId, SoundId, Sprite, SpriteId};
    use crate::hydration::ScopeKey;
    use crate::worlds::{RoomId, WorldId};

    #[test]
    fn coordinator_starts_with_no_active_scopes() {
        let coordinator = HydrationCoordinator::default();
        assert!(coordinator.active_scopes().is_empty());
    }

    #[test]
    fn activate_scope_makes_it_active() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(3));
        coordinator.activate_scope(scope.clone());
        assert!(coordinator.is_active(&scope));
        assert_eq!(coordinator.active_scopes(), vec![scope]);
    }

    #[test]
    fn double_activate_is_idempotent() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::World(WorldId(1));
        coordinator.activate_scope(scope.clone());
        coordinator.activate_scope(scope.clone());
        assert!(coordinator.is_active(&scope));
        assert_eq!(coordinator.active_scopes().len(), 1);
    }

    #[test]
    fn deactivate_scope_removes_it_and_clears_claims() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(5));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(3))));
        coordinator.deactivate_scope(scope.clone());
        assert!(!coordinator.is_active(&scope));
        assert!(coordinator.active_scopes().is_empty());
        let snapshot = coordinator.snapshot();
        assert!(snapshot.claims.is_empty());
    }

    #[test]
    fn deactivate_inactive_scope_is_noop() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Entity(Entity(9));
        coordinator.deactivate_scope(scope.clone());
        assert!(!coordinator.is_active(&scope));
        assert!(coordinator.active_scopes().is_empty());
    }

    #[test]
    fn claim_tracks_specific_assets() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(3));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(1))));
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(2))));

        assert_eq!(coordinator.claim_count(&scope, ResourceClass::Texture), 2);
        assert_eq!(
            coordinator.claimed_assets_by_class(&scope, ResourceClass::Texture),
            vec![AssetKey::Sprite(SpriteId(1)), AssetKey::Sprite(SpriteId(2))]
        );
    }

    #[test]
    fn claim_is_idempotent_for_duplicates() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Boot;
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Script(ScriptId(5))));
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Script(ScriptId(5))));

        assert_eq!(coordinator.claim_count(&scope, ResourceClass::Script), 1);
    }

    #[test]
    fn scope_claims_accumulate_by_scope_and_class() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(2));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Scope(ScopeKey::Room(RoomId(2))));

        assert_eq!(coordinator.claim_count(&scope, ResourceClass::RoomPayload), 1);
    }

    #[test]
    fn release_removes_only_requested_asset() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(4));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sound(SoundId(2))));
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sound(SoundId(3))));
        coordinator.release(scope.clone(), ResidencyKey::Asset(AssetKey::Sound(SoundId(2))));

        assert_eq!(coordinator.claim_count(&scope, ResourceClass::Audio), 1);
        assert_eq!(
            coordinator.claimed_assets_by_class(&scope, ResourceClass::Audio),
            vec![AssetKey::Sound(SoundId(3))]
        );
    }

    #[test]
    fn release_on_nonexistent_claim_is_noop() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(7));
        coordinator.release(scope.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(3))));
        let snapshot = coordinator.snapshot();
        assert!(snapshot.claims.is_empty());
    }

    #[test]
    fn claimed_assets_merges_classes_for_scope() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::World(WorldId(2));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Script(ScriptId(5))));
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(3))));

        assert_eq!(
            coordinator.claimed_assets(&scope),
            vec![AssetKey::Sprite(SpriteId(3)), AssetKey::Script(ScriptId(5))]
        );
    }

    #[test]
    fn snapshot_includes_all_active_scopes_and_their_claims() {
        let mut coordinator = HydrationCoordinator::default();
        let world = HydrationScope::World(WorldId(1));
        let room_a = HydrationScope::Room(RoomId(2));
        let room_b = HydrationScope::Room(RoomId(3));

        coordinator.activate_scope(world.clone());
        coordinator.claim(world.clone(), ResidencyKey::Asset(AssetKey::Script(ScriptId(2))));

        coordinator.activate_scope(room_a.clone());
        coordinator.claim(room_a.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(4))));
        coordinator.claim(room_a.clone(), ResidencyKey::Asset(AssetKey::Sound(SoundId(1))));

        coordinator.activate_scope(room_b.clone());
        coordinator.claim(room_b.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(6))));

        let snapshot = coordinator.snapshot();
        assert_eq!(snapshot.active_scopes.len(), 3);
        assert_eq!(snapshot.claims.len(), 4);
        assert_eq!(snapshot.active_scopes[0], HydrationScope::Room(RoomId(2)));
        assert_eq!(snapshot.active_scopes[1], HydrationScope::Room(RoomId(3)));
        assert_eq!(snapshot.active_scopes[2], HydrationScope::World(WorldId(1)));
    }

    #[test]
    fn claim_entity_assets_claims_all_asset_referencing_components() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Sprite { sprite: SpriteId(5) })
            .with(Script {
                script_id: ScriptId(3),
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();

        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(1));
        coordinator.activate_scope(scope.clone());

        coordinator.claim_entity_assets(&ecs, entity, RoomId(1));

        let claims = coordinator.claimed_assets(&scope);
        assert!(claims.contains(&AssetKey::Sprite(SpriteId(5))));
        assert!(claims.contains(&AssetKey::Script(ScriptId(3))));
    }

    #[test]
    fn release_entity_assets_releases_all_asset_claims() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Sprite { sprite: SpriteId(5) })
            .with_current_room(RoomId(1))
            .finish();

        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(1));
        coordinator.activate_scope(scope.clone());
        coordinator.claim_entity_assets(&ecs, entity, RoomId(1));

        coordinator.release_entity_assets(&ecs, entity, RoomId(1));

        let claims = coordinator.claimed_assets(&scope);
        assert!(!claims.contains(&AssetKey::Sprite(SpriteId(5))));
    }

    #[test]
    fn cross_scope_claims_accumulate_for_same_entity() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Sprite { sprite: SpriteId(5) })
            .with_current_room(RoomId(1))
            .finish();

        let mut coordinator = HydrationCoordinator::default();
        let scope_a = HydrationScope::Room(RoomId(1));
        let scope_b = HydrationScope::Room(RoomId(2));
        coordinator.activate_scope(scope_a.clone());
        coordinator.activate_scope(scope_b.clone());

        coordinator.claim_entity_assets(&ecs, entity, RoomId(1));
        coordinator.claim_entity_assets(&ecs, entity, RoomId(2));

        let claims_a = coordinator.claimed_assets(&scope_a);
        let claims_b = coordinator.claimed_assets(&scope_b);
        assert!(claims_a.contains(&AssetKey::Sprite(SpriteId(5))));
        assert!(claims_b.contains(&AssetKey::Sprite(SpriteId(5))));
    }

    #[test]
    fn snapshot_includes_claims_by_resource_class() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(1));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(1))));
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Sprite(SpriteId(2))));
        coordinator.claim(scope.clone(), ResidencyKey::Asset(AssetKey::Script(ScriptId(3))));

        let snapshot = coordinator.snapshot();
        let room_claims = &snapshot.claims;
        let sprite_count = room_claims
            .iter()
            .filter(|c| c.class == ResourceClass::Texture)
            .map(|c| c.keys.len())
            .sum::<usize>();
        let script_count = room_claims
            .iter()
            .filter(|c| c.class == ResourceClass::Script)
            .map(|c| c.keys.len())
            .sum::<usize>();

        assert_eq!(sprite_count, 2);
        assert_eq!(script_count, 1);
    }
}
