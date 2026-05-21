use crate::save_system::{RestorePhase, SaveProvider, SaveProviderId, SaveProviderRegistry, SavedSection};
use std::io;

struct StubProvider {
    id: &'static str,
    phase: RestorePhase,
}

impl SaveProvider for StubProvider {
    fn id(&self) -> SaveProviderId {
        SaveProviderId::new(self.id)
    }

    fn restore_phase(&self) -> RestorePhase {
        self.phase
    }

    fn capture(&mut self) -> io::Result<SavedSection> {
        Ok(SavedSection {
            version: 1,
            data: self.id.to_string(),
        })
    }

    fn apply(&mut self, _section: &SavedSection) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn register_provider_duplicate_id_returns_error() {
    let mut registry = SaveProviderRegistry::new();
    registry
        .register(Box::new(StubProvider {
            id: "engine.resume",
            phase: RestorePhase::PreRuntime,
        }))
        .unwrap();
    let error = registry
        .register(Box::new(StubProvider {
            id: "engine.resume",
            phase: RestorePhase::PostRuntime,
        }))
        .unwrap_err();
    assert_eq!(error.provider_id, SaveProviderId::new("engine.resume"));
}

#[test]
fn iter_providers_mixed_phases_returns_phase_then_id_order() {
    let mut registry = SaveProviderRegistry::new();
    registry
        .register(Box::new(StubProvider {
            id: "game.player",
            phase: RestorePhase::PostRuntime,
        }))
        .unwrap();
    registry
        .register(Box::new(StubProvider {
            id: "engine.resume",
            phase: RestorePhase::PreRuntime,
        }))
        .unwrap();
    registry
        .register(Box::new(StubProvider {
            id: "engine.camera",
            phase: RestorePhase::PreRuntime,
        }))
        .unwrap();
    let ordered_ids = registry
        .iter()
        .map(|provider| provider.id())
        .collect::<Vec<_>>();
    assert_eq!(
        ordered_ids,
        vec![
            SaveProviderId::new("engine.camera"),
            SaveProviderId::new("engine.resume"),
            SaveProviderId::new("game.player"),
        ]
    );
}
