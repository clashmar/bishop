use crate::app::{Editor, EditorMode};
use crate::commands::scene::{
    ApplyInstanceToPrefabCmd, RevertPrefabInstanceCmd, UnlinkPrefabInstanceCmd,
};
use crate::editor_global::push_command;
use crate::prefab::{
    PendingPrefabRequest, PendingPrefabTransition, PrefabTransitionPrompt, BLANK_PREFAB_ID,
};
use crate::shared::scene_ui::inspector::{ScenePrefabAction, ScenePrefabActionRequest};
use bishop::prelude::*;
use engine_core::prelude::*;

mod room_sync;
mod save;
mod transition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrefabEditorLaunch {
    OpenExisting(PrefabId),
    CaptureSelection(Entity),
    OpenPicker,
}

impl Editor {
    pub(crate) fn is_blank_prefab_mode(&self) -> bool {
        matches!(self.mode, EditorMode::Prefab(BLANK_PREFAB_ID))
    }

    pub(crate) fn active_persisted_prefab_id(&self) -> Option<PrefabId> {
        self.prefab_editor
            .as_ref()
            .map(|prefab_editor| prefab_editor.prefab_id)
            .filter(|prefab_id| *prefab_id != BLANK_PREFAB_ID)
    }

    pub(in crate::prefab) fn prefab_editor_launch(&self) -> PrefabEditorLaunch {
        if let EditorMode::Room(_) = self.mode {
            if let Some(entity) = self.room_editor.single_selected_entity() {
                if let Some(instance) = self.game.ecs.get::<PrefabInstanceRoot>(entity) {
                    return PrefabEditorLaunch::OpenExisting(instance.prefab_id);
                }

                if let Some(instance) = self.game.ecs.get::<PrefabInstanceNode>(entity) {
                    return PrefabEditorLaunch::OpenExisting(instance.prefab_id);
                }

                return PrefabEditorLaunch::CaptureSelection(entity);
            }
        }

        PrefabEditorLaunch::OpenPicker
    }

    pub(crate) fn open_prefab_editor(&mut self, ctx: &mut WgpuContext) {
        match self.prefab_editor_launch() {
            PrefabEditorLaunch::OpenExisting(prefab_id) => {
                self.enter_prefab_transition(ctx, prefab_id);
            }
            PrefabEditorLaunch::CaptureSelection(entity) => {
                self.prefab_state
                    .set_pending_request(PendingPrefabRequest::CaptureSelection(entity));
                self.open_prefab_name_modal(ctx);
            }
            PrefabEditorLaunch::OpenPicker => {
                self.open_prefab_picker_modal(ctx);
            }
        }
    }

    pub(crate) fn handle_room_prefab_action(
        &mut self,
        ctx: &WgpuContext,
        request: ScenePrefabActionRequest,
        room_id: RoomId,
    ) {
        match request.action {
            ScenePrefabAction::OpenPrefabEditor => {
                self.enter_prefab_transition(ctx, request.prefab_id);
            }
            ScenePrefabAction::UnlinkInstance => {
                push_command(Box::new(UnlinkPrefabInstanceCmd::new(
                    request.selected_entity,
                    EditorMode::Room(room_id),
                )));
            }
            ScenePrefabAction::ApplyInstanceToPrefab => {
                push_command(Box::new(ApplyInstanceToPrefabCmd::new(
                    request.selected_entity,
                    EditorMode::Room(room_id),
                )));
            }
            ScenePrefabAction::RevertInstanceToPrefab => {
                push_command(Box::new(RevertPrefabInstanceCmd::new(
                    request.selected_entity,
                    EditorMode::Room(room_id),
                )));
            }
        }
    }

    pub(crate) fn enter_prefab_transition(&mut self, ctx: &WgpuContext, prefab_id: PrefabId) {
        let prompt =
            self.request_prefab_transition(PendingPrefabTransition::OpenExisting(prefab_id));
        self.present_prefab_transition_prompt(ctx, prompt);
    }

    pub(crate) fn open_prefab_editor_for_id(&mut self, prefab_id: PrefabId) {
        let Some(prefab) = self.game.prefab_manager.prefabs.get(&prefab_id).cloned() else {
            self.toast = Some(Toast::new("Prefab not found.", 2.5));
            return;
        };

        let last_room_synced_state = room_sync::capture_prefab_room_sync_state(
            &mut self.game.ecs,
            prefab_id,
            prefab.clone(),
        );
        let (prefab_editor, prefab_stage) =
            crate::prefab::prefab_editor::PrefabEditor::open_existing_from_game(
                &self.game,
                prefab.clone(),
                last_room_synced_state,
            );
        self.room_editor.reset();
        self.prefab_editor = Some(prefab_editor);
        self.prefab_stage = Some(prefab_stage);
        if !matches!(self.mode, EditorMode::Prefab(_)) {
            self.return_mode = Some(self.mode);
        }
        self.mode = EditorMode::Prefab(prefab.id);
        self.prefab_state.set_require_picker(false);
        self.toast = Some(Toast::new(format!("Opened prefab '{}'", prefab.name), 2.5));
    }

    pub(crate) fn create_prefab_from_selection<C>(
        &mut self,
        _ctx: &C,
        entity: Entity,
        name: String,
    ) {
        self.create_prefab_from_selection_impl(entity, name);
    }

    pub(super) fn create_prefab_from_selection_impl(&mut self, entity: Entity, name: String) {
        if !self.game.ecs.has::<Transform>(entity) {
            self.toast = Some(Toast::new("Selected entity no longer exists.", 2.5));
            return;
        }

        let prefab_id = self.game.prefab_manager.allocate_prefab_id();
        let prefab = capture_prefab(&mut self.game.ecs, entity, prefab_id, name);
        let prefab = match self.game.prefab_manager.save_prefab(
            &self.game.name,
            &mut self.game.asset_registry,
            &prefab,
        ) {
            Ok(prefab) => prefab,
            Err(error) => {
                onscreen_error!("Could not save prefab: {error}");
                return;
            }
        };

        if let Err(error) = save::sync_prefabs_lua_file(&self.game) {
            onscreen_error!("Could not write prefabs.lua: {error}");
            return;
        }
        let _ = self.reconcile_prefab_palette_after_library_change();
        let Some(linked_root) =
            room_sync::relink_room_subtree_to_prefab(&mut self.game, entity, &prefab)
        else {
            self.toast = Some(Toast::new("Could not link selected entity to prefab.", 2.5));
            return;
        };

        self.open_prefab_editor_for_id(prefab.id);
        self.room_editor.set_selected_entity(Some(linked_root));
    }

    pub(super) fn create_blank_prefab_impl(&mut self, name: String) {
        let prefab_id = self.game.prefab_manager.allocate_prefab_id();
        let prefab = create_prefab(prefab_id, name);
        if let Err(error) = self.game.prefab_manager.save_prefab(
            &self.game.name,
            &mut self.game.asset_registry,
            &prefab,
        ) {
            onscreen_error!("Could not save prefab: {error}");
            return;
        }

        if let Err(error) = save::sync_prefabs_lua_file(&self.game) {
            onscreen_error!("Could not write prefabs.lua: {error}");
            return;
        }
        let _ = self.reconcile_prefab_palette_after_library_change();
        self.open_prefab_editor_for_id(prefab_id);
    }

    pub(crate) fn request_blank_prefab_transition(
        &mut self,
        name: String,
    ) -> PrefabTransitionPrompt {
        self.request_prefab_transition(PendingPrefabTransition::CreateBlank(name))
    }
}
