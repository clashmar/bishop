use crate::audio::AudioDiagnosticsSnapshot;
use crate::game::Game;
use crate::hydration::{HydrationScope, ResourceClass, ResidencyKey};
use crate::scripting::ScriptManager;

/// Label for texture residency summaries.
pub const TEXTURES_RESIDENCY_LABEL: &str = "Textures";
/// Label for script residency summaries.
pub const SCRIPTS_RESIDENCY_LABEL: &str = "Scripts";
/// Label for audio residency summaries.
pub const AUDIO_RESIDENCY_LABEL: &str = "Audio";
/// Label for global payload residency summaries.
pub const GLOBAL_PAYLOADS_RESIDENCY_LABEL: &str = "Global Payloads";
/// Label for world residency summaries.
pub const WORLD_RESIDENCY_LABEL: &str = "World Residency";
/// Label for room payload residency summaries.
pub const ROOM_PAYLOADS_RESIDENCY_LABEL: &str = "Room Payloads";

/// Counts residency state for one resource class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResidencyCounts {
    pub known: usize,
    pub resident: usize,
    pub pending: usize,
    pub pinned: usize,
    pub active: usize,
}

impl ResidencyCounts {
    /// Returns the not-yet-loaded count for this resource class.
    pub fn cold(&self) -> usize {
        self.known
            .saturating_sub(self.resident.saturating_add(self.pending))
    }
}

/// Captures residency counts for one labeled resource class.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceResidencySnapshot {
    pub label: &'static str,
    pub counts: ResidencyCounts,
}

impl ResourceResidencySnapshot {
    /// Creates a residency snapshot for one resource class.
    pub fn new(label: &'static str, counts: ResidencyCounts) -> Self {
        Self { label, counts }
    }
}

/// Combines residency snapshots across runtime resource classes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeResidencySnapshot {
    pub textures: ResourceResidencySnapshot,
    pub scripts: ResourceResidencySnapshot,
    pub audio: ResourceResidencySnapshot,
    pub global_payloads: ResourceResidencySnapshot,
    pub world_payloads: ResourceResidencySnapshot,
    pub room_payloads: ResourceResidencySnapshot,
}

impl RuntimeResidencySnapshot {
    /// Builds a residency snapshot from the active runtime managers.
    pub fn from_sources(
        game: &Game,
        script_manager: &ScriptManager,
        audio: &AudioDiagnosticsSnapshot,
        audio_known: usize,
        audio_active: usize,
    ) -> Self {
        Self {
            textures: ResourceResidencySnapshot::new(
                TEXTURES_RESIDENCY_LABEL,
                ResidencyCounts {
                    known: game.sprite_manager.registered_id_count(),
                    resident: game.sprite_manager.texture_count(),
                    pending: game.sprite_manager.pending_texture_count(),
                    pinned: 0,
                    active: 0,
                },
            ),
            scripts: ResourceResidencySnapshot::new(
                SCRIPTS_RESIDENCY_LABEL,
                ResidencyCounts {
                    known: script_manager.registered_id_count(),
                    resident: script_manager.loaded_script_count(),
                    pending: script_manager.pending_init_count(),
                    pinned: 0,
                    active: script_manager.instance_count(),
                },
            ),
            audio: ResourceResidencySnapshot::new(
                AUDIO_RESIDENCY_LABEL,
                ResidencyCounts {
                    known: audio_known,
                    resident: audio.cached_sound_count,
                    pending: audio.loading_sound_count,
                    pinned: 0,
                    active: audio_active,
                },
            ),
            global_payloads: ResourceResidencySnapshot::new(
                GLOBAL_PAYLOADS_RESIDENCY_LABEL,
                scope_counts(game, ResourceClass::GlobalPayload),
            ),
            world_payloads: ResourceResidencySnapshot::new(
                WORLD_RESIDENCY_LABEL,
                scope_counts(game, ResourceClass::World),
            ),
            room_payloads: ResourceResidencySnapshot::new(
                ROOM_PAYLOADS_RESIDENCY_LABEL,
                scope_counts(game, ResourceClass::RoomPayload),
            ),
        }
    }
}

fn scope_counts(game: &Game, class: ResourceClass) -> ResidencyCounts {
    let known = match class {
        ResourceClass::GlobalPayload => 0,
        ResourceClass::World => game.worlds().len(),
        ResourceClass::RoomPayload => game
            .worlds()
            .iter()
            .map(|world| world.rooms().len())
            .sum(),
        ResourceClass::Texture
        | ResourceClass::Script
        | ResourceClass::Audio
        | ResourceClass::Prefab => 0,
    };

    let snapshot = game.hydration_coordinator.snapshot();
    let mut resident = std::collections::BTreeSet::<ResidencyKey>::new();
    let mut pinned = std::collections::BTreeSet::<ResidencyKey>::new();
    for claim in snapshot.claims.into_iter().filter(|claim| claim.class == class) {
        for key in claim.keys {
            resident.insert(key);
            if matches!(claim.scope, HydrationScope::Entity(_)) {
                pinned.insert(key);
            }
        }
    }

    ResidencyCounts {
        known,
        resident: resident.len(),
        pending: 0,
        pinned: pinned.len(),
        active: resident.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetRegistry;
    use crate::ecs::{Entity, ScriptId, SpriteId};
    use crate::hydration::{HydrationScope, ScopeKey};
    use crate::worlds::{Room, RoomId, World, WorldId};
    use std::path::PathBuf;

    #[test]
    fn residency_counts_calculate_cold_without_underflow() {
        let counts = ResidencyCounts {
            known: 6,
            resident: 2,
            pending: 1,
            pinned: 0,
            active: 0,
        };

        assert_eq!(counts.cold(), 3);
    }

    #[test]
    fn residency_counts_saturate_when_known_is_smaller_than_loaded_work() {
        let counts = ResidencyCounts {
            known: 1,
            resident: 1,
            pending: 1,
            pinned: 0,
            active: 0,
        };

        assert_eq!(counts.cold(), 0);
    }

    #[test]
    fn runtime_residency_snapshot_collects_texture_script_and_audio_counts() {
        let mut registry = AssetRegistry::default();
        registry
            .register_asset_relative_path(SpriteId(1), "sprites/player.png")
            .unwrap();

        let mut game = Game::default();
        game.asset_registry = registry;
        crate::assets::SpriteManager::init_editor_metadata(
            &game.asset_registry,
            &mut game.sprite_manager,
        );

        let mut script_manager = ScriptManager::default();
        script_manager
            .script_id_to_path
            .insert(ScriptId(5), PathBuf::from("actors/player.lua"));
        script_manager.pending_inits.push((Entity(1), ScriptId(5)));

        let audio = AudioDiagnosticsSnapshot {
            cached_sound_count: 2,
            loading_sound_count: 1,
            ref_count_entry_count: 0,
            entries: Vec::new(),
        };

        let snapshot = RuntimeResidencySnapshot::from_sources(&game, &script_manager, &audio, 4, 3);

        assert_eq!(snapshot.textures.label, TEXTURES_RESIDENCY_LABEL);
        assert_eq!(snapshot.textures.counts.known, 1);
        assert_eq!(snapshot.textures.counts.resident, 0);
        assert_eq!(snapshot.textures.counts.pending, 0);
        assert_eq!(snapshot.scripts.label, SCRIPTS_RESIDENCY_LABEL);
        assert_eq!(snapshot.scripts.counts.known, 1);
        assert_eq!(snapshot.scripts.counts.resident, 0);
        assert_eq!(snapshot.scripts.counts.pending, 1);
        assert_eq!(snapshot.scripts.counts.active, 0);
        assert_eq!(snapshot.audio.label, AUDIO_RESIDENCY_LABEL);
        assert_eq!(snapshot.audio.counts.known, 4);
        assert_eq!(snapshot.audio.counts.resident, 2);
        assert_eq!(snapshot.audio.counts.pending, 1);
        assert_eq!(snapshot.audio.counts.pinned, 0);
        assert_eq!(snapshot.audio.counts.active, 3);
        assert_eq!(snapshot.global_payloads.counts.known, 0);
        assert_eq!(snapshot.world_payloads.counts.known, 0);
        assert_eq!(snapshot.room_payloads.counts.known, 0);
    }

    #[test]
    fn runtime_residency_snapshot_reports_scope_counts() {
        let mut game = Game::default();
        let mut world = World::new(WorldId(1), "Demo".to_string(), 16.0);
        world.add_room(Room {
            id: RoomId(2),
            ..Default::default()
        });
        game.add_world(world);
        game.hydration_coordinator
            .activate_scope(HydrationScope::World(WorldId(1)));
        game.hydration_coordinator.claim(
            HydrationScope::World(WorldId(1)),
            ResidencyKey::Scope(ScopeKey::World(WorldId(1))),
        );
        game.hydration_coordinator
            .activate_scope(HydrationScope::Room(RoomId(2)));
        game.hydration_coordinator.claim(
            HydrationScope::Room(RoomId(2)),
            ResidencyKey::Scope(ScopeKey::Room(RoomId(2))),
        );

        let snapshot = RuntimeResidencySnapshot::from_sources(
            &game,
            &ScriptManager::default(),
            &AudioDiagnosticsSnapshot::default(),
            0,
            0,
        );

        assert_eq!(snapshot.room_payloads.counts.known, 1);
        assert_eq!(snapshot.room_payloads.counts.resident, 1);
        assert_eq!(snapshot.world_payloads.counts.known, 1);
        assert_eq!(snapshot.world_payloads.counts.resident, 1);
    }
}
