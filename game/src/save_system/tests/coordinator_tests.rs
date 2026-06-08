use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;

use crate::save_system::{
    coordinator::{
        apply_document, apply_document_phase, capture_document, SaveCoordinatorError,
    },
    RestorePhase, SaveProvider, SaveProviderId, SaveProviderRegistry, RuntimeSaveDocument,
    RuntimeSaveMetadata, SaveLane, SaveSlotKey, SavedSection,
};
use uuid::Uuid;

// --- Test double for capture tests ---

struct CaptureProvider {
    id: &'static str,
    result: Option<io::Result<SavedSection>>,
}

impl CaptureProvider {
    fn new(id: &'static str) -> Self {
        Self { id, result: None }
    }

    fn with_result(mut self, result: io::Result<SavedSection>) -> Self {
        self.result = Some(result);
        self
    }
}

impl SaveProvider for CaptureProvider {
    fn id(&self) -> SaveProviderId {
        SaveProviderId::new(self.id)
    }

    fn restore_phase(&self) -> RestorePhase {
        RestorePhase::PreRuntime
    }

    fn capture(&mut self) -> io::Result<SavedSection> {
        self.result.take().unwrap_or_else(|| {
            Ok(SavedSection {
                version: 1,
                data: self.id.to_string(),
            })
        })
    }

    fn apply(&mut self, _section: &SavedSection) -> io::Result<()> {
        Ok(())
    }
}

// --- Test double for apply tests ---

/// Shared log of apply invocations across multiple providers.
#[derive(Clone)]
struct ApplyOrderLog {
    entries: Rc<RefCell<Vec<String>>>,
}

impl ApplyOrderLog {
    fn new() -> Self {
        Self {
            entries: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn push(&self, entry: String) {
        self.entries.borrow_mut().push(entry);
    }

    fn snapshot(&self) -> Vec<String> {
        self.entries.borrow().clone()
    }
}

struct ApplyTestProvider {
    id: &'static str,
    phase: RestorePhase,
    should_fail: bool,
    log: Option<ApplyOrderLog>,
}

impl ApplyTestProvider {
    fn new(id: &'static str, phase: RestorePhase) -> Self {
        Self {
            id,
            phase,
            should_fail: false,
            log: None,
        }
    }

    fn with_log(mut self, log: &ApplyOrderLog) -> Self {
        self.log = Some(log.clone());
        self
    }

    fn failing(mut self) -> Self {
        self.should_fail = true;
        self
    }
}

impl SaveProvider for ApplyTestProvider {
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
        if let Some(ref log) = self.log {
            log.push(self.id.to_string());
        }
        if self.should_fail {
            Err(io::Error::other("apply failed"))
        } else {
            Ok(())
        }
    }
}

// --- Helpers ---

fn sample_metadata() -> RuntimeSaveMetadata {
    RuntimeSaveMetadata {
        schema_version: 1,
        game_id: Uuid::nil(),
        game_name: "test".to_string(),
        lane: SaveLane::Manual,
        slot: SaveSlotKey::Default,
        saved_at_unix_ms: 123,
    }
}

// --- Tests ---

#[test]
fn capture_document_captures_two_sections_with_string_keys() {
    let mut registry = SaveProviderRegistry::new();
    registry
        .register(Box::new(CaptureProvider::new("engine.resume")))
        .unwrap();
    registry
        .register(Box::new(CaptureProvider::new("game.player")))
        .unwrap();

    let document: RuntimeSaveDocument = capture_document(
        &mut registry, &sample_metadata(),
    )
        .unwrap();

    assert_eq!(document.metadata, sample_metadata());
    assert_eq!(document.sections.len(), 2);
    assert!(document.sections.contains_key("engine.resume"));
    assert!(document.sections.contains_key("game.player"));
}

#[test]
fn capture_document_returns_capture_error_on_provider_failure() {
    let mut registry = SaveProviderRegistry::new();
    let err = io::Error::other("disk full");
    registry
        .register(Box::new(
            CaptureProvider::new("engine.resume").with_result(Err(err)),
        ))
        .unwrap();

    let result = capture_document(&mut registry, &sample_metadata());

    match result {
        Err(SaveCoordinatorError::Capture {
            provider_id,
            source,
        }) => {
            assert_eq!(provider_id, SaveProviderId::new("engine.resume"));
            assert_eq!(source.kind(), io::ErrorKind::Other);
        }
        other => panic!("expected Capture error, got {:?}", other),
    }
}

#[test]
fn apply_document_mixed_phases_runs_pre_before_post() {
    let mut registry = SaveProviderRegistry::new();
    let log = ApplyOrderLog::new();

    // PostRuntime registered first but PreRuntime providers should run first
    registry
        .register(Box::new(
            ApplyTestProvider::new("game.player", RestorePhase::PostRuntime).with_log(&log),
        ))
        .unwrap();
    registry
        .register(Box::new(
            ApplyTestProvider::new("engine.resume", RestorePhase::PreRuntime).with_log(&log),
        ))
        .unwrap();

    let mut sections = HashMap::new();
    sections.insert(
        "engine.resume".to_string(),
        SavedSection {
            version: 1,
            data: "resume".to_string(),
        },
    );
    sections.insert(
        "game.player".to_string(),
        SavedSection {
            version: 1,
            data: "player".to_string(),
        },
    );

    let document = RuntimeSaveDocument {
        metadata: sample_metadata(),
        sections,
    };

    let report =
        apply_document(&mut registry, &document).unwrap();

    let order = log.snapshot();
    assert_eq!(order, vec!["engine.resume", "game.player"]);
    assert!(report.unknown_section_ids.is_empty());
}

#[test]
fn apply_document_unknown_sections_returns_sorted_report() {
    let mut registry = SaveProviderRegistry::new();
    registry
        .register(Box::new(ApplyTestProvider::new(
            "engine.resume",
            RestorePhase::PreRuntime,
        )))
        .unwrap();

    let mut sections = HashMap::new();
    sections.insert(
        "engine.resume".to_string(),
        SavedSection {
            version: 1,
            data: "resume".to_string(),
        },
    );
    sections.insert(
        "unknown.alpha".to_string(),
        SavedSection {
            version: 1,
            data: "alpha".to_string(),
        },
    );
    sections.insert(
        "unknown.beta".to_string(),
        SavedSection {
            version: 1,
            data: "beta".to_string(),
        },
    );

    let document = RuntimeSaveDocument {
        metadata: sample_metadata(),
        sections,
    };

    let report =
        apply_document(&mut registry, &document).unwrap();

    assert_eq!(
        report.unknown_section_ids,
        vec!["unknown.alpha".to_string(), "unknown.beta".to_string()]
    );
}

#[test]
fn apply_document_missing_section_skips_provider() {
    let mut registry = SaveProviderRegistry::new();
    let log = ApplyOrderLog::new();

    registry
        .register(Box::new(
            ApplyTestProvider::new("engine.resume", RestorePhase::PreRuntime)
                .with_log(&log),
        ))
        .unwrap();

    // Document has no sections at all
    let document = RuntimeSaveDocument {
        metadata: sample_metadata(),
        sections: HashMap::new(),
    };

    let report =
        apply_document(&mut registry, &document).unwrap();

    // Provider should not have been called
    assert!(log.snapshot().is_empty());
    assert!(report.unknown_section_ids.is_empty());
}

#[test]
fn apply_document_provider_apply_fails_returns_provider_id_error() {
    let mut registry = SaveProviderRegistry::new();

    registry
        .register(Box::new(
            ApplyTestProvider::new("engine.resume", RestorePhase::PreRuntime).failing(),
        ))
        .unwrap();

    let mut sections = HashMap::new();
    sections.insert(
        "engine.resume".to_string(),
        SavedSection {
            version: 1,
            data: "resume".to_string(),
        },
    );

    let document = RuntimeSaveDocument {
        metadata: sample_metadata(),
        sections,
    };

    let result =
        apply_document(&mut registry, &document);

    match result {
        Err(SaveCoordinatorError::Apply {
            provider_id,
            source,
        }) => {
            assert_eq!(provider_id, SaveProviderId::new("engine.resume"));
            assert_eq!(source.kind(), io::ErrorKind::Other);
        }
        other => panic!("expected Apply error, got {other:?}"),
    }
}

#[test]
fn apply_document_phase_only_runs_matching_phase() {
    let mut registry = SaveProviderRegistry::new();
    let log = ApplyOrderLog::new();

    registry
        .register(Box::new(
            ApplyTestProvider::new("engine.resume", RestorePhase::PreRuntime).with_log(&log),
        ))
        .unwrap();
    registry
        .register(Box::new(
            ApplyTestProvider::new("game.player", RestorePhase::PostRuntime).with_log(&log),
        ))
        .unwrap();

    let mut sections = HashMap::new();
    sections.insert(
        "engine.resume".to_string(),
        SavedSection {
            version: 1,
            data: "resume".to_string(),
        },
    );
    sections.insert(
        "game.player".to_string(),
        SavedSection {
            version: 1,
            data: "player".to_string(),
        },
    );

    let document = RuntimeSaveDocument {
        metadata: sample_metadata(),
        sections,
    };

    let report = apply_document_phase(
        &mut registry,
        &document,
        RestorePhase::PreRuntime,
    )
    .unwrap();

    let order = log.snapshot();
    assert_eq!(order, vec!["engine.resume"]);
    assert!(report.unknown_section_ids.is_empty());
}

#[test]
fn apply_document_phase_only_runs_matching_post_runtime() {
    let mut registry = SaveProviderRegistry::new();
    let log = ApplyOrderLog::new();

    registry
        .register(Box::new(
            ApplyTestProvider::new("engine.resume", RestorePhase::PreRuntime).with_log(&log),
        ))
        .unwrap();
    registry
        .register(Box::new(
            ApplyTestProvider::new("game.player", RestorePhase::PostRuntime).with_log(&log),
        ))
        .unwrap();

    let mut sections = HashMap::new();
    sections.insert(
        "engine.resume".to_string(),
        SavedSection {
            version: 1,
            data: "resume".to_string(),
        },
    );
    sections.insert(
        "game.player".to_string(),
        SavedSection {
            version: 1,
            data: "player".to_string(),
        },
    );

    let document = RuntimeSaveDocument {
        metadata: sample_metadata(),
        sections,
    };

    let report = apply_document_phase(
        &mut registry,
        &document,
        RestorePhase::PostRuntime,
    )
    .unwrap();

    let order = log.snapshot();
    assert_eq!(order, vec!["game.player"]);
    assert!(report.unknown_section_ids.is_empty());
}
