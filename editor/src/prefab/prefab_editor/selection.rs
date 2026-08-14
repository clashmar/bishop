use super::PrefabEditor;
use crate::gui::inspector::collider_module::edit::clear_active_collider_edit;
use crate::gui::inspector::interactable_module::edit::clear_active_interactable_edit;
use crate::room::collider_drag::ColliderHandleDragState;
use crate::room::interactable_drag::InteractableHandleDragState;
use engine_core::ecs::*;

impl PrefabEditor {
    pub(crate) fn sync_inspector_to_selection(&mut self) {
        if let Some(entity) = self.single_selected_entity() {
            self.inspector.select_entity(entity);
        } else {
            self.inspector.clear_target();
        }
    }

    pub fn set_selected_entity(&mut self, entity: Option<Entity>) {
        let selection_changed = match entity {
            Some(entity) => {
                self.selected_entities.len() != 1 || !self.selected_entities.contains(&entity)
            }
            None => !self.selected_entities.is_empty(),
        };
        if selection_changed {
            self.disable_active_edit_modes();
        }

        self.selected_entities.clear();
        if let Some(entity) = entity {
            self.selected_entities.insert(entity);
        }
        self.sync_inspector_to_selection();
    }

    pub fn add_to_selection(&mut self, entity: Entity) {
        if self.selected_entities.insert(entity) {
            self.disable_active_edit_modes();
        }
        self.sync_inspector_to_selection();
    }

    pub fn toggle_entity_selection(&mut self, entity: Entity) {
        if self.selected_entities.contains(&entity) {
            self.disable_active_edit_modes();
            self.selected_entities.remove(&entity);
            self.sync_inspector_to_selection();
        } else {
            self.add_to_selection(entity);
        }
    }

    pub fn clear_selection(&mut self) {
        if !self.selected_entities.is_empty() {
            self.disable_active_edit_modes();
        }
        self.selected_entities.clear();
        self.sync_inspector_to_selection();
    }

    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }

    pub fn single_selected_entity(&self) -> Option<Entity> {
        (self.selected_entities.len() == 1)
            .then(|| self.selected_entities.iter().next().copied())
            .flatten()
    }

    pub fn clear_deleted_entities(&mut self, deleted_entities: &[Entity]) {
        if self
            .root_entity
            .is_some_and(|entity| deleted_entities.contains(&entity))
        {
            self.root_entity = None;
        }

        let selected_count = self.selected_entities.len();
        self.selected_entities
            .retain(|entity| !deleted_entities.contains(entity));
        if self.selected_entities.len() != selected_count {
            self.disable_active_edit_modes();
        }
        self.sync_inspector_to_selection();
    }

    pub fn restore_deleted_root(&mut self, restored_root: Entity) {
        self.root_entity = Some(restored_root);
        self.set_selected_entity(Some(restored_root));
    }

    pub(crate) fn create_prefab_entity(
        &mut self,
        ecs: &mut Ecs,
        requested_parent: Option<Entity>,
    ) -> Entity {
        let entity = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Entity".to_string()))
            .finish();

        if let Some(parent) = requested_parent.filter(|parent| is_live_prefab_entity(ecs, *parent))
        {
            set_parent(ecs, entity, parent);
        } else if let Some(root) = self
            .root_entity
            .filter(|root| is_live_prefab_entity(ecs, *root))
        {
            set_parent(ecs, entity, root);
        } else {
            self.root_entity = Some(entity);
        }

        entity
    }

    pub(crate) fn sanitize_live_state(&mut self, ecs: &Ecs) {
        let root_entity = self.root_entity;
        if self
            .root_entity
            .is_some_and(|entity| !is_live_prefab_entity(ecs, entity))
        {
            self.root_entity = None;
        }

        let selected_count = self.selected_entities.len();
        self.selected_entities
            .retain(|entity| is_live_prefab_entity(ecs, *entity));
        if self.selected_entities.len() != selected_count {
            self.disable_active_edit_modes();
        }
        if self.root_entity != root_entity || self.selected_entities.len() != selected_count {
            self.sync_inspector_to_selection();
        }
    }

    pub(crate) fn disable_active_edit_modes(&mut self) {
        clear_active_collider_edit();
        clear_active_interactable_edit();
        self.drag_state.collider_drag = ColliderHandleDragState::default();
        self.drag_state.interactable_drag = InteractableHandleDragState::default();
    }
}

pub fn is_prefab_entity(ecs: &Ecs, entity: Entity) -> bool {
    !ecs.has::<PlayerProxy>(entity)
        && !ecs.has::<Player>(entity)
        && !ecs.has::<Global>(entity)
}

fn is_live_prefab_entity(ecs: &Ecs, entity: Entity) -> bool {
    ecs.get_store::<Transform>().contains(entity) && is_prefab_entity(ecs, entity)
}
