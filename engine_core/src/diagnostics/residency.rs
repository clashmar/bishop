use crate::assets::SpriteManager;
use crate::audio::AudioDiagnosticsSnapshot;
use crate::scripting::ScriptManager;

/// Label for texture residency summaries.
pub const TEXTURES_RESIDENCY_LABEL: &str = "Textures";
/// Label for script residency summaries.
pub const SCRIPTS_RESIDENCY_LABEL: &str = "Scripts";
/// Label for audio residency summaries.
pub const AUDIO_RESIDENCY_LABEL: &str = "Audio";

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
}

impl RuntimeResidencySnapshot {
    /// Builds a residency snapshot from the active runtime managers.
    pub fn from_sources(
        sprite_manager: &SpriteManager,
        script_manager: &ScriptManager,
        audio: &AudioDiagnosticsSnapshot,
        audio_known: usize,
        audio_active: usize,
    ) -> Self {
        Self {
            textures: ResourceResidencySnapshot::new(
                TEXTURES_RESIDENCY_LABEL,
                ResidencyCounts {
                    known: sprite_manager.registered_id_count(),
                    resident: sprite_manager.texture_count(),
                    pending: sprite_manager.pending_texture_count(),
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetRegistry;
    use crate::ecs::{Entity, ScriptId, SpriteId};
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

        let mut sprite_manager = SpriteManager::default();
        SpriteManager::init_editor_metadata(&registry, &mut sprite_manager);

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

        let snapshot = RuntimeResidencySnapshot::from_sources(
            &sprite_manager,
            &script_manager,
            &audio,
            4,
            3,
        );

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
    }
}
