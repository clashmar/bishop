use crate::app::{
    Editor, EditorMode, PendingPrefabRequest, PendingPrefabTransition, PrefabTransitionPrompt,
};
use crate::commands::scene::{
    ApplyInstanceToPrefabCmd, RevertPrefabInstanceCmd, UnlinkPrefabInstanceCmd,
};
use crate::editor_assets::write_prefabs_lua;
use crate::editor_global::push_command;
use crate::gui::prompts::{DirtyPrefabExitPromptResult, EmptyPrefabExitPromptResult};
use crate::prefab::prefab_editor::{PrefabRoomSyncState, StagedPrefabState};
use crate::shared::scene_ui::inspector::{ScenePrefabAction, ScenePrefabActionRequest};
use crate::storage::editor_storage::{collect_prefab_names, save_game};
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PrefabEditorLaunch {
    OpenExisting(PrefabId),
    CaptureSelection(Entity),
    OpenPicker,
}

impl Editor {
    pub(super) fn prefab_editor_launch(&self) -> PrefabEditorLaunch {
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
                self.pending_prefab_request = Some(PendingPrefabRequest::CaptureSelection(entity));
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
        let Some(prefab) = self.game.prefab_library.prefabs.get(&prefab_id).cloned() else {
            self.toast = Some(Toast::new("Prefab not found.", 2.5));
            return;
        };

        let last_room_synced_state =
            capture_prefab_room_sync_state(&mut self.game.ecs, prefab_id, prefab.clone());
        let (prefab_editor, prefab_stage) = super::PrefabEditor::open_existing_from_game(
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

        let prefab_id = self.game.prefab_library.allocate_prefab_id();
        let prefab = capture_prefab(&mut self.game.ecs, entity, prefab_id, name);
        if let Err(error) = save_prefab(&self.game.name, &prefab) {
            onscreen_error!("Could not save prefab: {error}");
            return;
        }

        self.game
            .prefab_library
            .prefabs
            .insert(prefab.id, prefab.clone());
        if let Err(error) = sync_prefabs_lua_file(&self.game) {
            onscreen_error!("Could not write prefabs.lua: {error}");
            return;
        }
        let _ = self.reconcile_prefab_palette_after_library_change();
        let Some(linked_root) = relink_room_subtree_to_prefab(&mut self.game, entity, &prefab)
        else {
            self.toast = Some(Toast::new("Could not link selected entity to prefab.", 2.5));
            return;
        };

        self.open_prefab_editor_for_id(prefab.id);
        self.room_editor.set_selected_entity(Some(linked_root));
    }

    fn create_blank_prefab_impl(&mut self, name: String) {
        let prefab_id = self.game.prefab_library.allocate_prefab_id();
        let prefab = create_prefab(prefab_id, name);
        if let Err(error) = save_prefab(&self.game.name, &prefab) {
            onscreen_error!("Could not save prefab: {error}");
            return;
        }

        self.game
            .prefab_library
            .prefabs
            .insert(prefab.id, prefab.clone());
        if let Err(error) = sync_prefabs_lua_file(&self.game) {
            onscreen_error!("Could not write prefabs.lua: {error}");
            return;
        }
        let _ = self.reconcile_prefab_palette_after_library_change();
        self.open_prefab_editor_for_id(prefab_id);
    }

    pub(crate) fn request_blank_prefab_transition(&mut self, ctx: &WgpuContext, name: String) {
        match self.request_prefab_transition(PendingPrefabTransition::CreateBlank(name)) {
            PrefabTransitionPrompt::None => {}
            PrefabTransitionPrompt::Dirty => self.open_dirty_prefab_exit_modal(ctx),
            PrefabTransitionPrompt::Empty => self.open_empty_prefab_exit_modal(ctx),
        }
    }

    pub fn save_active_prefab(&mut self) {
        let Some(staged_state) = self.active_prefab_staged_state() else {
            return;
        };

        match staged_state {
            StagedPrefabState::PrefabAsset(prefab) => {
                self.commit_prefab_asset_save(prefab);
            }
            StagedPrefabState::Empty => {
                self.toast = Some(Toast::new("Prefab is empty", 2.5));
            }
        }
    }

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

    pub(crate) fn reconcile_active_prefab_room_preview(&mut self) {
        let Some(staged_state) = self.active_prefab_staged_state() else {
            return;
        };

        let needs_sync = self.prefab_editor.as_ref().is_some_and(|prefab_editor| {
            prefab_editor.last_room_synced_state.staged_prefab != staged_state
        });
        if !needs_sync {
            return;
        }

        self.reconcile_prefab_room_state(staged_state);
    }

    pub(crate) fn confirm_empty_prefab_save_delete(&mut self) {
        self.commit_prefab_delete();
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

        let Some(staged_state) = self.active_prefab_staged_state() else {
            self.execute_prefab_transition(transition);
            return PrefabTransitionPrompt::None;
        };

        if self.active_prefab_is_clean() {
            self.execute_prefab_transition(transition);
            return PrefabTransitionPrompt::None;
        }

        self.pending_prefab_transition = Some(transition);
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
        let prefab = load_prefab_asset_from_path(path)?;
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
                self.pending_prefab_transition = None;
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
                self.pending_prefab_transition = None;
            }
        }
    }

    pub(crate) fn close_active_prefab_editor(&mut self) {
        self.prefab_editor = None;
        self.prefab_stage = None;
        self.mode = self.return_mode.unwrap_or(EditorMode::Game);
        self.return_mode = None;
        self.pending_prefab_transition = None;
        match self.mode {
            EditorMode::Room(_) | EditorMode::World(_) | EditorMode::Game => {
                self.pending_camera_reset = true;
            }
            _ => {}
        }
    }

    pub(crate) fn commit_prefab_asset_save(&mut self, prefab: PrefabAsset) -> bool {
        if self.prefab_editor.is_none() {
            return false;
        }
        let root_entity = self
            .prefab_editor
            .as_ref()
            .and_then(|prefab_editor| prefab_editor.root_entity);

        if let (Some(root_entity), Some(prefab_stage)) = (root_entity, self.prefab_stage.as_mut()) {
            sync_prefab_stage_instance_metadata(&mut prefab_stage.ecs, root_entity, &prefab);
        }

        if let Some(prefab_stage) = self.prefab_stage.as_ref() {
            if let Err(error) = prefab_stage.sync_editor_services(&mut self.game) {
                onscreen_error!("Could not save prefab: {error}");
                return false;
            }
        }

        self.game
            .prefab_library
            .prefabs
            .insert(prefab.id, prefab.clone());
        if let Err(error) = save_game(&self.game) {
            onscreen_error!("Could not save prefab metadata: {error}");
            return false;
        }

        let Some(prefab_editor) = self.prefab_editor.as_mut() else {
            return false;
        };
        if let Err(error) = prefab_editor.save_prefab_asset(&self.game.name, &prefab) {
            onscreen_error!("Could not save prefab: {error}");
            return false;
        }

        if let Some(prefab_stage) = self.prefab_stage.as_mut() {
            prefab_stage
                .prefab_library
                .prefabs
                .insert(prefab.id, prefab.clone());
        }
        self.reconcile_prefab_room_state(StagedPrefabState::PrefabAsset(prefab.clone()));

        if !self.promote_prefab_in_palette(prefab.id) {
            return false;
        }

        self.toast = Some(Toast::new("Prefab saved", 2.5));
        true
    }

    pub(crate) fn commit_prefab_delete(&mut self) {
        let Some(prefab_id) = self
            .prefab_editor
            .as_ref()
            .map(|prefab_editor| prefab_editor.prefab_id)
        else {
            return;
        };

        if let Err(error) = delete_prefab(&self.game.name, prefab_id) {
            onscreen_error!("Could not delete prefab: {error}");
            return;
        }

        self.game.prefab_library.prefabs.remove(&prefab_id);
        if let Err(error) = sync_prefabs_lua_file(&self.game) {
            onscreen_error!("Could not write prefabs.lua: {error}");
            return;
        }
        if let Some(prefab_stage) = self.prefab_stage.as_mut() {
            prefab_stage.prefab_library.prefabs.remove(&prefab_id);
        }
        if let Some(prefab_editor) = self.prefab_editor.as_mut() {
            prefab_editor.last_committed_prefab = StagedPrefabState::Empty;
        }
        self.reconcile_prefab_room_state(StagedPrefabState::Empty);
        if !self.remove_prefab_from_palette(prefab_id) {
            return;
        }
        self.toast = Some(Toast::new("Prefab deleted", 2.5));
    }

    pub(crate) fn reconcile_prefab_room_state(&mut self, target_state: StagedPrefabState) {
        let Some(prefab_editor) = self.prefab_editor.as_mut() else {
            return;
        };

        let preserved_snapshots = prefab_editor
            .last_room_synced_state
            .linked_instance_snapshots
            .clone();
        let prefab_id = prefab_editor.prefab_id;

        match &target_state {
            StagedPrefabState::PrefabAsset(prefab) => {
                restore_prefab_instance_snapshots(&mut self.game, prefab_id, &preserved_snapshots);
                refresh_linked_prefab_instances(&mut self.game, prefab);
                prefab_editor.last_room_synced_state =
                    capture_prefab_room_sync_state(&mut self.game.ecs, prefab_id, prefab.clone());
            }
            StagedPrefabState::Empty => {
                let snapshots = remove_prefab_and_linked_instances(
                    &mut self.game,
                    &mut self.room_editor,
                    prefab_id,
                );
                prefab_editor.last_room_synced_state = PrefabRoomSyncState {
                    staged_prefab: StagedPrefabState::Empty,
                    linked_instance_snapshots: if snapshots.is_empty() {
                        preserved_snapshots
                    } else {
                        snapshots
                    },
                };
            }
        }
    }

    fn finish_pending_prefab_transition(&mut self) {
        let transition = self
            .pending_prefab_transition
            .take()
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

fn sync_prefabs_lua_file(game: &Game) -> io::Result<()> {
    let prefab_names = collect_prefab_names(&game.prefab_library)?;
    write_prefabs_lua(&scripts_folder(), &prefab_names)
}

fn load_prefab_asset_from_path(path: &Path) -> io::Result<PrefabAsset> {
    let ron = fs::read_to_string(path)?;
    let prefab = ron::from_str(&ron).map_err(|error| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Could not parse prefab '{}': {error}", path.display()),
        )
    })?;
    validate_prefab(&prefab)?;
    Ok(prefab)
}

fn relink_room_subtree_to_prefab(
    game: &mut Game,
    root_entity: Entity,
    prefab: &PrefabAsset,
) -> Option<Entity> {
    let root_position = game
        .ecs
        .get::<Transform>(root_entity)
        .map(|transform| transform.position)
        .unwrap_or_default();
    let parent_entity = get_parent(&game.ecs, root_entity);
    let room_id = game.ecs.get::<CurrentRoom>(root_entity).map(|room| room.0);

    let replacement_root = {
        let mut ctx = game.ctx_mut();
        let mut services_ctx = ctx.services_ctx_mut();
        instantiate_prefab(&mut services_ctx, prefab, root_position, room_id)
    };

    if replacement_root == Entity::null() {
        return None;
    }

    if let Some(parent_entity) = parent_entity {
        set_parent(&mut game.ecs, replacement_root, parent_entity);
    }

    let mut ctx = game.ctx_mut();
    let mut services_ctx = ctx.services_ctx_mut();
    Ecs::remove_entity(&mut services_ctx, root_entity);
    Some(replacement_root)
}

fn refresh_linked_prefab_instances(game: &mut Game, prefab: &PrefabAsset) {
    let roots = linked_prefab_instance_roots(&game.ecs, prefab.id);

    for root_entity in roots {
        let room_id = game.ecs.get::<CurrentRoom>(root_entity).map(|room| room.0);
        let mut ctx = game.ctx_mut();
        let mut services_ctx = ctx.services_ctx_mut();
        refresh_prefab_instance(&mut services_ctx, root_entity, prefab, room_id);
    }
}

fn linked_prefab_instance_roots(ecs: &Ecs, prefab_id: PrefabId) -> Vec<Entity> {
    ecs.get_store::<PrefabInstanceRoot>()
        .data
        .iter()
        .filter_map(|(&entity, root)| (root.prefab_id == prefab_id).then_some(entity))
        .collect()
}

fn capture_prefab_room_sync_state(
    ecs: &mut Ecs,
    prefab_id: PrefabId,
    prefab: PrefabAsset,
) -> PrefabRoomSyncState {
    PrefabRoomSyncState {
        staged_prefab: StagedPrefabState::PrefabAsset(prefab),
        linked_instance_snapshots: capture_linked_prefab_instance_snapshots(ecs, prefab_id),
    }
}

fn sync_prefab_stage_instance_metadata(ecs: &mut Ecs, root_entity: Entity, prefab: &PrefabAsset) {
    let subtree = capture_subtree(ecs, root_entity);
    if subtree.len() != prefab.nodes.len() {
        return;
    }

    for (snapshot, node) in subtree.into_iter().zip(prefab.nodes.iter()) {
        ecs.add_component_to_entity(
            snapshot.entity,
            PrefabInstanceNode {
                prefab_id: prefab.id,
                node_id: node.node_id,
                root_entity,
            },
        );
    }

    ecs.add_component_to_entity(
        root_entity,
        PrefabInstanceRoot {
            prefab_id: prefab.id,
        },
    );
}

fn capture_linked_prefab_instance_snapshots(
    ecs: &mut Ecs,
    prefab_id: PrefabId,
) -> Vec<GroupSnapshot> {
    linked_prefab_instance_roots(ecs, prefab_id)
        .into_iter()
        .map(|root| capture_subtree(ecs, root))
        .collect()
}

fn restore_prefab_instance_snapshots(
    game: &mut Game,
    prefab_id: PrefabId,
    snapshots: &[GroupSnapshot],
) {
    let existing_roots = linked_prefab_instance_roots(&game.ecs, prefab_id)
        .into_iter()
        .collect::<HashSet<_>>();

    for snapshot in snapshots {
        let Some(root_entity) = snapshot.first().map(|entity| entity.entity) else {
            continue;
        };
        if existing_roots.contains(&root_entity) {
            continue;
        }

        let mut ctx = game.ctx_mut();
        let mut services_ctx = ctx.services_ctx_mut();
        restore_subtree(&mut services_ctx, snapshot);
    }
}

fn remove_prefab_and_linked_instances(
    game: &mut Game,
    room_editor: &mut crate::room::room_editor::RoomEditor,
    prefab_id: PrefabId,
) -> Vec<GroupSnapshot> {
    let roots = linked_prefab_instance_roots(&game.ecs, prefab_id);
    let mut removed_entities = HashSet::new();
    let mut snapshots = Vec::with_capacity(roots.len());

    for root_entity in roots {
        let snapshot = capture_subtree(&mut game.ecs, root_entity);
        removed_entities.extend(snapshot.iter().map(|entity| entity.entity));
        snapshots.push(snapshot);

        let mut ctx = game.ctx_mut();
        let mut services_ctx = ctx.services_ctx_mut();
        Ecs::remove_entity(&mut services_ctx, root_entity);
    }

    room_editor
        .selected_entities
        .retain(|entity| !removed_entities.contains(entity));
    if !room_editor
        .inspector
        .target
        .is_some_and(|entity| removed_entities.contains(&entity))
    {
        return snapshots;
    }

    room_editor.inspector.set_target(None);
    snapshots
}
