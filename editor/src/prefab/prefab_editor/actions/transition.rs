use crate::app::{Editor, EditorMode};
use crate::gui::prompts::{DirtyPrefabExitPromptResult, EmptyPrefabExitPromptResult};
use crate::prefab::prefab_editor::{
    PrefabEditor, PrefabRoomSyncState, PrefabStage, StagedPrefabState,
};
use crate::prefab::{PendingPrefabTransition, PrefabTransitionPrompt, BLANK_PREFAB_ID};
use bishop::prelude::*;
use engine_core::prelude::*;
use std::io;
use std::path::Path;

impl Editor {
    pub(crate) fn active_prefab_staged_state(&mut self) -> Option<StagedPrefabState> {
        let (Some(prefab_editor), Some(prefab_stage)) =
            (self.prefab_editor.as_mut(), self.prefab_stage.as_mut())
        else {
            return None;
        };

        let mut prefab_ctx = prefab_stage.ctx_mut();
        Some(prefab_editor.staged_prefab_state(&mut prefab_ctx))
    }

    pub(crate) fn active_prefab_is_clean(&mut self) -> bool {
        let Some(staged_state) = self.active_prefab_staged_state() else {
            return true;
        };

        self.prefab_editor
            .as_ref()
            .is_some_and(|prefab_editor| prefab_editor.is_clean(&staged_state))
    }

    fn discard_active_prefab_changes(&mut self) {
        let Some(committed_state) = self
            .prefab_editor
            .as_ref()
            .map(|prefab_editor| prefab_editor.last_committed_prefab.clone())
        else {
            return;
        };

        self.reconcile_prefab_room_state(committed_state);
    }

    pub(crate) fn request_prefab_transition(
        &mut self,
        transition: PendingPrefabTransition,
    ) -> PrefabTransitionPrompt {
        if matches!(&transition, PendingPrefabTransition::OpenExisting(prefab_id) if Some(*prefab_id)
            == self.prefab_editor.as_ref().map(|prefab_editor| prefab_editor.prefab_id))
        {
            return PrefabTransitionPrompt::None;
        }

        if self.is_blank_prefab_mode() {
            self.execute_prefab_transition(transition);
            return PrefabTransitionPrompt::None;
        }

        let Some(staged_state) = self.active_prefab_staged_state() else {
            self.execute_prefab_transition(transition);
            return PrefabTransitionPrompt::None;
        };

        if self.active_prefab_is_clean() {
            self.execute_prefab_transition(transition);
            return PrefabTransitionPrompt::None;
        }

        self.prefab_state.set_pending_transition(transition);
        match staged_state {
            StagedPrefabState::PrefabAsset(_) => PrefabTransitionPrompt::Dirty,
            StagedPrefabState::Empty => PrefabTransitionPrompt::Empty,
        }
    }

    pub(crate) fn request_prefab_transition_to_asset(
        &mut self,
        prefab: PrefabAsset,
    ) -> PrefabTransitionPrompt {
        self.game
            .prefab_library
            .prefabs
            .insert(prefab.id, prefab.clone());
        let _ = self.reconcile_prefab_palette_after_library_change();
        self.request_prefab_transition(PendingPrefabTransition::OpenExisting(prefab.id))
    }

    pub(crate) fn request_prefab_transition_to_path(
        &mut self,
        path: &Path,
    ) -> io::Result<PrefabTransitionPrompt> {
        let prefab = super::save::load_prefab_asset_from_path(path)?;
        Ok(self.request_prefab_transition_to_asset(prefab))
    }

    pub(crate) fn present_prefab_transition_prompt(
        &mut self,
        ctx: &WgpuContext,
        prompt: PrefabTransitionPrompt,
    ) {
        match prompt {
            PrefabTransitionPrompt::None => {}
            PrefabTransitionPrompt::Dirty => self.open_dirty_prefab_exit_modal(ctx),
            PrefabTransitionPrompt::Empty => self.open_empty_prefab_exit_modal(ctx),
        }
    }

    pub(crate) fn confirm_dirty_prefab_transition(&mut self, result: DirtyPrefabExitPromptResult) {
        match result {
            DirtyPrefabExitPromptResult::SaveAndSync => {
                if let Some(StagedPrefabState::PrefabAsset(prefab)) =
                    self.active_prefab_staged_state()
                {
                    if self.commit_prefab_asset_save(prefab) {
                        self.finish_pending_prefab_transition();
                    }
                }
            }
            DirtyPrefabExitPromptResult::DiscardChanges => {
                self.discard_active_prefab_changes();
                self.finish_pending_prefab_transition();
            }
            DirtyPrefabExitPromptResult::Cancel => {
                self.prefab_state.clear_pending_transition();
            }
        }
    }

    pub(crate) fn confirm_empty_prefab_transition(&mut self, result: EmptyPrefabExitPromptResult) {
        match result {
            EmptyPrefabExitPromptResult::DeletePrefab => {
                self.commit_prefab_delete();
                self.finish_pending_prefab_transition();
            }
            EmptyPrefabExitPromptResult::DiscardChanges => {
                self.discard_active_prefab_changes();
                self.finish_pending_prefab_transition();
            }
            EmptyPrefabExitPromptResult::Cancel => {
                self.prefab_state.clear_pending_transition();
            }
        }
    }

    pub(crate) fn close_active_prefab_editor(&mut self) {
        self.prefab_editor = None;
        self.prefab_stage = None;
        self.mode = self.return_mode.unwrap_or(EditorMode::Game);
        self.return_mode = None;
        self.prefab_state.clear_pending_transition();
        self.prefab_state.set_require_picker(false);
        match self.mode {
            EditorMode::Room(_) | EditorMode::World(_) | EditorMode::Game => {
                self.pending_camera_reset = true;
            }
            _ => {}
        }
    }

    pub(crate) fn open_blank_prefab_editor(&mut self) {
        self.room_editor.reset();
        self.prefab_stage = Some(PrefabStage::from_editor_services(&self.game));
        self.prefab_editor = Some(PrefabEditor::new(
            BLANK_PREFAB_ID,
            "Prefab".to_string(),
            StagedPrefabState::Empty,
            PrefabRoomSyncState {
                staged_prefab: StagedPrefabState::Empty,
                linked_instance_snapshots: Vec::new(),
            },
        ));
        self.mode = EditorMode::Prefab(BLANK_PREFAB_ID);
        self.prefab_state.clear_pending_transition();
        self.prefab_state.set_require_picker(false);
    }

    pub(crate) fn enter_required_blank_prefab_mode(&mut self) {
        self.open_blank_prefab_editor();
        self.prefab_state.set_require_picker(true);
    }

    fn finish_pending_prefab_transition(&mut self) {
        let transition = self
            .prefab_state
            .take_pending_transition()
            .unwrap_or(PendingPrefabTransition::Exit);
        self.execute_prefab_transition(transition);
    }

    fn execute_prefab_transition(&mut self, transition: PendingPrefabTransition) {
        match transition {
            PendingPrefabTransition::Exit => self.close_active_prefab_editor(),
            PendingPrefabTransition::OpenExisting(prefab_id) => {
                self.open_prefab_editor_for_id(prefab_id)
            }
            PendingPrefabTransition::CreateBlank(name) => self.create_blank_prefab_impl(name),
        }
    }
}
