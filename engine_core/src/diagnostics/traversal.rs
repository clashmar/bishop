use crate::assets::AssetKey;
use crate::ecs::Entity;
use crate::hydration::ResourceClass;
use crate::worlds::{RoomId, WorldId};
use std::collections::{BTreeMap, VecDeque};

const THRASH_WINDOW: u64 = 6;
const MAX_THRASH_ENTRIES: usize = 6;
const MAX_RECENT_EVENTS: usize = 64;

/// Count summary for one resource class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraversalClassCount {
    pub class: ResourceClass,
    pub count: usize,
}

/// Warm room information for diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WarmRoomSnapshot {
    pub room_id: RoomId,
    pub reasons: Vec<String>,
    pub claims: Vec<TraversalClassCount>,
}

/// Warm world information for diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WarmWorldSnapshot {
    pub world_id: WorldId,
    pub reasons: Vec<String>,
    pub claims: Vec<TraversalClassCount>,
}

/// Pinned-entity residency information for diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PinnedEntitySnapshot {
    pub entity: Entity,
    pub room_id: Option<RoomId>,
    pub pin_count: u16,
    pub reasons: Vec<String>,
    pub claims: Vec<TraversalClassCount>,
}

/// Per-scope outcome summary from the latest traversal refresh.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TraversalOutcomeSnapshot {
    pub label: String,
    pub claimed: Vec<TraversalClassCount>,
    pub hydrated: Vec<TraversalClassCount>,
    pub evicted: Vec<TraversalClassCount>,
    pub failures: usize,
}

/// Repeated near-boundary churn surfaced in diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraversalThrashSnapshot {
    pub asset: AssetKey,
    pub events: usize,
    pub hydrates: usize,
    pub evictions: usize,
}

/// The latest traversal-residency diagnostics snapshot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TraversalResidencySnapshot {
    pub rooms: Vec<WarmRoomSnapshot>,
    pub worlds: Vec<WarmWorldSnapshot>,
    pub pinned_entities: Vec<PinnedEntitySnapshot>,
    pub global_claims: Vec<TraversalClassCount>,
    pub outcomes: Vec<TraversalOutcomeSnapshot>,
    pub thrash: Vec<TraversalThrashSnapshot>,
}

/// One concrete hydration or eviction event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraversalAssetEvent {
    pub asset: AssetKey,
    pub hydrated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecentTraversalAssetEvent {
    asset: AssetKey,
    hydrated: bool,
    refresh_id: u64,
}

/// Runtime tracker for traversal-residency diagnostics.
#[derive(Clone, Debug, Default)]
pub struct TraversalResidencyDiagnostics {
    pub snapshot: TraversalResidencySnapshot,
    refresh_id: u64,
    recent_events: VecDeque<RecentTraversalAssetEvent>,
}

impl TraversalResidencyDiagnostics {
    /// Records one traversal refresh snapshot plus its concrete asset events.
    pub fn record_refresh(
        &mut self,
        mut snapshot: TraversalResidencySnapshot,
        events: Vec<TraversalAssetEvent>,
    ) {
        self.refresh_id = self.refresh_id.saturating_add(1);
        let refresh_id = self.refresh_id;

        for event in events {
            self.recent_events.push_back(RecentTraversalAssetEvent {
                asset: event.asset,
                hydrated: event.hydrated,
                refresh_id,
            });
        }

        while self.recent_events.len() > MAX_RECENT_EVENTS {
            self.recent_events.pop_front();
        }

        while self
            .recent_events
            .front()
            .is_some_and(|event| refresh_id.saturating_sub(event.refresh_id) > THRASH_WINDOW)
        {
            self.recent_events.pop_front();
        }

        snapshot.thrash = self.compute_thrash();
        self.snapshot = snapshot;
    }

    fn compute_thrash(&self) -> Vec<TraversalThrashSnapshot> {
        let mut by_asset: BTreeMap<AssetKey, (usize, usize, usize)> = BTreeMap::new();

        for event in &self.recent_events {
            let entry = by_asset.entry(event.asset).or_default();
            entry.0 += 1;
            if event.hydrated {
                entry.1 += 1;
            } else {
                entry.2 += 1;
            }
        }

        let mut thrash: Vec<TraversalThrashSnapshot> = by_asset
            .into_iter()
            .filter_map(|(asset, (events, hydrates, evictions))| {
                (hydrates > 0 && evictions > 0 && events >= 3).then_some(TraversalThrashSnapshot {
                    asset,
                    events,
                    hydrates,
                    evictions,
                })
            })
            .collect();

        thrash.sort_by(|a, b| {
            b.events
                .cmp(&a.events)
                .then_with(|| b.hydrates.cmp(&a.hydrates))
                .then_with(|| b.evictions.cmp(&a.evictions))
                .then_with(|| a.asset.cmp(&b.asset))
        });
        thrash.truncate(MAX_THRASH_ENTRIES);
        thrash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetKey;
    use crate::ecs::SpriteId;

    #[test]
    fn recent_hydrate_evict_churn_surfaces_as_thrash() {
        let mut diagnostics = TraversalResidencyDiagnostics::default();
        let asset = AssetKey::Sprite(SpriteId(7));

        diagnostics.record_refresh(
            TraversalResidencySnapshot::default(),
            vec![TraversalAssetEvent {
                asset,
                hydrated: true,
            }],
        );
        diagnostics.record_refresh(
            TraversalResidencySnapshot::default(),
            vec![TraversalAssetEvent {
                asset,
                hydrated: false,
            }],
        );
        diagnostics.record_refresh(
            TraversalResidencySnapshot::default(),
            vec![TraversalAssetEvent {
                asset,
                hydrated: true,
            }],
        );

        assert_eq!(diagnostics.snapshot.thrash.len(), 1);
        assert_eq!(diagnostics.snapshot.thrash[0].asset, asset);
        assert_eq!(diagnostics.snapshot.thrash[0].events, 3);
    }
}
