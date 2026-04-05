use crate::app::{Editor, EditorMode, PendingPrefabRequest};
use bishop::prelude::*;
use engine_core::prelude::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

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
                self.enter_prefab_mode(ctx, prefab_id);
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

    pub(crate) fn enter_prefab_mode(&mut self, _ctx: &WgpuContext, prefab_id: PrefabId) {
        self.open_prefab_editor_for_id(prefab_id);
    }

    fn open_prefab_editor_for_id(&mut self, prefab_id: PrefabId) {
        let Some(prefab) = self.game.prefab_library.prefabs.get(&prefab_id).cloned() else {
            self.toast = Some(Toast::new("Prefab not found.", 2.5));
            return;
        };

        let (prefab_editor, prefab_stage) =
            super::PrefabEditor::open_existing(&self.game.name, prefab.clone());
        self.prefab_editor = Some(prefab_editor);
        self.prefab_stage = Some(prefab_stage);
        self.return_mode = Some(self.mode);
        self.mode = EditorMode::Prefab(prefab.id);
        self.toast = Some(Toast::new(format!("Opened prefab '{}'", prefab.name), 2.5));
    }

    pub(crate) fn create_prefab_from_selection<C>(&mut self, _ctx: &C, entity: Entity, name: String) {
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

        self.game.prefab_library.prefabs.insert(prefab.id, prefab.clone());
        let Some(linked_root) = relink_room_subtree_to_prefab(&mut self.game, entity, &prefab) else {
            self.toast = Some(Toast::new("Could not link selected entity to prefab.", 2.5));
            return;
        };

        self.room_editor.set_selected_entity(Some(linked_root));
        self.open_prefab_editor_for_id(prefab.id);
    }

    pub(crate) fn create_blank_prefab(&mut self, _ctx: &WgpuContext, name: String) {
        let prefab_id = self.game.prefab_library.allocate_prefab_id();
        let prefab = create_prefab(prefab_id, name);
        if let Err(error) = save_prefab(&self.game.name, &prefab) {
            onscreen_error!("Could not save prefab: {error}");
            return;
        }

        self.game.prefab_library.prefabs.insert(prefab.id, prefab.clone());
        self.open_prefab_editor_for_id(prefab_id);
    }

    /// Saves the currently active prefab to disk and refreshes linked instances.
    pub fn save_active_prefab(&mut self) {
        let (Some(prefab_editor), Some(prefab_stage)) =
            (self.prefab_editor.as_mut(), self.prefab_stage.as_mut())
        else {
            return;
        };

        let mut prefab_ctx = prefab_stage.ctx_mut();
        match prefab_editor.save_to_disk(&self.game.name, &mut prefab_ctx) {
            Ok(Some(prefab)) => {
                self.game.prefab_library.prefabs.insert(prefab.id, prefab.clone());
                refresh_linked_prefab_instances(&mut self.game, &prefab);
                self.toast = Some(Toast::new("Prefab saved", 2.5));
            }
            Ok(None) => {
                self.toast = Some(Toast::new("Prefab is empty", 2.5));
            }
            Err(error) => {
                onscreen_error!("Could not save prefab: {error}");
            }
        }
    }
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
    let roots = game
        .ecs
        .get_store::<PrefabInstanceRoot>()
        .data
        .iter()
        .filter_map(|(&entity, root)| (root.prefab_id == prefab.id).then_some(entity))
        .collect::<Vec<_>>();

    for root_entity in roots {
        let room_id = game.ecs.get::<CurrentRoom>(root_entity).map(|room| room.0);
        let mut ctx = game.ctx_mut();
        let mut services_ctx = ctx.services_ctx_mut();
        refresh_prefab_instance(&mut services_ctx, root_entity, prefab, room_id);
    }
}
