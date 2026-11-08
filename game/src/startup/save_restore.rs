use crate::engine::SaveRuntime;
use crate::save_system::{apply_document_phase, RestorePhase, RuntimeSaveDocument};
use uuid::Uuid;

pub(super) fn load_restore_document(
    save_runtime: &SaveRuntime,
    game_id: Uuid,
) -> Result<RuntimeSaveDocument, String> {
    save_runtime.load_latest_document(game_id)
        .map_err(|error| format!("Failed to read save document: {error}"))?
        .ok_or_else(|| "No save data found for explicit load".to_string())
}

pub(super) fn apply_restore_phase(
    save_runtime: &SaveRuntime,
    document: &RuntimeSaveDocument,
    phase: RestorePhase,
) -> Result<(), String> {
    apply_document_phase(&mut save_runtime.providers().borrow_mut(), document, phase)
        .map_err(|error| format!("Failed to apply {phase:?} restore: {error}"))?;
    Ok(())
}
