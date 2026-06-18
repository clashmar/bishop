use crate::hydration::scope::{HydrationScope, ResourceClaim, ResourceClass};
use crate::omni_debug;
use std::collections::{HashMap, HashSet};

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
    claims: HashMap<(HydrationScope, ResourceClass), usize>,
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
        self.claims.retain(|(s, _), _| *s != scope);
    }

    /// Returns true if the scope is currently active.
    pub fn is_active(&self, scope: &HydrationScope) -> bool {
        self.active_scopes.contains(scope)
    }

    /// Returns all currently active scopes.
    pub fn active_scopes(&self) -> Vec<HydrationScope> {
        let mut scopes: Vec<HydrationScope> = self.active_scopes.iter().cloned().collect();
        scopes.sort_by_key(|s| format!("{:?}", s));
        scopes
    }

    /// Record a resource claim for a scope.
    pub fn claim(&mut self, scope: HydrationScope, class: ResourceClass, count: usize) {
        omni_debug!(
            "hydration coordinator claim scope={:?} class={:?} count={}",
            scope,
            class,
            count
        );
        *self.claims.entry((scope, class)).or_insert(0) += count;
    }

    /// Reduce a resource claim for a scope. Saturates at zero.
    pub fn release(&mut self, scope: HydrationScope, class: ResourceClass, count: usize) {
        omni_debug!(
            "hydration coordinator release scope={:?} class={:?} count={}",
            scope,
            class,
            count
        );
        self.claims
            .entry((scope, class))
            .and_modify(|c| *c = c.saturating_sub(count));
    }

    /// Returns a point-in-time snapshot of coordinator state.
    pub fn snapshot(&self) -> CoordinatorSnapshot {
        let mut scopes: Vec<HydrationScope> = self.active_scopes.iter().cloned().collect();
        scopes.sort_by_key(|s| format!("{:?}", s));
        CoordinatorSnapshot {
            active_scopes: scopes,
            claims: self
                .claims
                .iter()
                .map(|((scope, class), count)| ResourceClaim {
                    scope: scope.clone(),
                    class: *class,
                    count: *count,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Entity;
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
        coordinator.claim(scope.clone(), ResourceClass::Texture, 3);
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
    fn claim_adds_to_snapshot() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::World(WorldId(2));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResourceClass::Script, 5);
        coordinator.claim(scope.clone(), ResourceClass::Texture, 3);

        let snapshot = coordinator.snapshot();
        assert_eq!(snapshot.active_scopes.len(), 1);
        assert_eq!(snapshot.claims.len(), 2);
        assert!(snapshot.claims.contains(&ResourceClaim {
            scope: scope.clone(),
            class: ResourceClass::Script,
            count: 5,
        }));
        assert!(snapshot.claims.contains(&ResourceClaim {
            scope: scope.clone(),
            class: ResourceClass::Texture,
            count: 3,
        }));
    }

    #[test]
    fn claim_is_additive_for_same_scope_and_class() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(1));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResourceClass::Texture, 2);
        coordinator.claim(scope.clone(), ResourceClass::Texture, 3);

        let snapshot = coordinator.snapshot();
        let texture_claim = snapshot
            .claims
            .iter()
            .find(|c| c.scope == scope && c.class == ResourceClass::Texture)
            .unwrap();
        assert_eq!(texture_claim.count, 5);
    }

    #[test]
    fn release_reduces_claim_count() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(4));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResourceClass::Audio, 8);
        coordinator.release(scope.clone(), ResourceClass::Audio, 3);

        let snapshot = coordinator.snapshot();
        let audio_claim = snapshot
            .claims
            .iter()
            .find(|c| c.scope == scope && c.class == ResourceClass::Audio)
            .unwrap();
        assert_eq!(audio_claim.count, 5);
    }

    #[test]
    fn release_saturates_at_zero() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(6));
        coordinator.activate_scope(scope.clone());
        coordinator.claim(scope.clone(), ResourceClass::Prefab, 1);
        coordinator.release(scope.clone(), ResourceClass::Prefab, 5);

        let snapshot = coordinator.snapshot();
        let prefab_claim = snapshot
            .claims
            .iter()
            .find(|c| c.scope == scope && c.class == ResourceClass::Prefab)
            .unwrap();
        assert_eq!(prefab_claim.count, 0);
    }

    #[test]
    fn release_on_nonexistent_claim_is_noop() {
        let mut coordinator = HydrationCoordinator::default();
        let scope = HydrationScope::Room(RoomId(7));
        coordinator.release(scope.clone(), ResourceClass::Texture, 3);
        let snapshot = coordinator.snapshot();
        assert!(snapshot.claims.is_empty());
    }

    #[test]
    fn snapshot_includes_all_active_scopes_and_their_claims() {
        let mut coordinator = HydrationCoordinator::default();
        let world = HydrationScope::World(WorldId(1));
        let room_a = HydrationScope::Room(RoomId(2));
        let room_b = HydrationScope::Room(RoomId(3));

        coordinator.activate_scope(world.clone());
        coordinator.claim(world.clone(), ResourceClass::Script, 2);

        coordinator.activate_scope(room_a.clone());
        coordinator.claim(room_a.clone(), ResourceClass::Texture, 4);
        coordinator.claim(room_a.clone(), ResourceClass::Audio, 1);

        coordinator.activate_scope(room_b.clone());
        coordinator.claim(room_b.clone(), ResourceClass::Texture, 6);

        let snapshot = coordinator.snapshot();
        assert_eq!(snapshot.active_scopes.len(), 3);
        assert_eq!(snapshot.claims.len(), 4);
    }
}
