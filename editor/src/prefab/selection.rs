use crate::prefab::prefab_editor::{PrefabEditor, PREFAB_EDITOR_GRID_SIZE};
use crate::room::entity_hitbox;
use bishop::prelude::*;
use engine_core::prelude::*;

impl PrefabEditor {
    fn sync_inspector_to_selection(&mut self) {
        self.inspector.set_target(self.single_selected_entity());
    }

    pub fn set_selected_entity(&mut self, entity: Option<Entity>) {
        self.selected_entities.clear();
        if let Some(entity) = entity {
            self.selected_entities.insert(entity);
        }
        self.sync_inspector_to_selection();
    }

    pub fn add_to_selection(&mut self, entity: Entity) {
        self.selected_entities.insert(entity);
        self.sync_inspector_to_selection();
    }

    pub fn toggle_entity_selection(&mut self, entity: Entity) {
        if self.selected_entities.contains(&entity) {
            self.selected_entities.remove(&entity);
            self.sync_inspector_to_selection();
        } else {
            self.add_to_selection(entity);
        }
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

        self.selected_entities
            .retain(|entity| !deleted_entities.contains(entity));
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

        if let Some(parent) = requested_parent.filter(|parent| is_live_prefab_entity(ecs, *parent)) {
            set_parent(ecs, entity, parent);
        } else if let Some(root) = self.root_entity.filter(|root| is_live_prefab_entity(ecs, *root))
        {
            set_parent(ecs, entity, root);
        } else {
            self.root_entity = Some(entity);
        }

        entity
    }

    pub(crate) fn sanitize_live_state(&mut self, ecs: &Ecs) {
        if self.root_entity.is_some_and(|entity| !is_live_prefab_entity(ecs, entity)) {
            self.root_entity = None;
        }

        self.selected_entities
            .retain(|entity| is_live_prefab_entity(ecs, *entity));
        self.sync_inspector_to_selection();
    }

    pub(crate) fn handle_selection(
        &mut self,
        ctx: &WgpuContext,
        camera: &Camera2D,
        ecs: &Ecs,
        asset_manager: &mut AssetManager,
    ) {
        let shift_held =
            ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        let mut candidates = Vec::new();

        for (entity, transform) in ecs.get_store::<Transform>().data.iter() {
            if !is_prefab_entity(ecs, *entity) {
                continue;
            }

            let hitbox = entity_hitbox(
                ctx,
                *entity,
                transform.position,
                camera,
                ecs,
                asset_manager,
                PREFAB_EDITOR_GRID_SIZE,
            );

            if hitbox.contains(mouse_screen) {
                let z = ecs.get_store::<Layer>().get(*entity).map_or(0, |layer| layer.z);
                candidates.push((*entity, z));
            }
        }

        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let clicked_entity = candidates.first().map(|(entity, _)| *entity);

        match (shift_held, clicked_entity) {
            (true, Some(entity)) => self.toggle_entity_selection(entity),
            (false, Some(entity)) => self.set_selected_entity(Some(entity)),
            (false, None) => self.set_selected_entity(None),
            (true, None) => {}
        }
    }
}

pub fn is_prefab_entity(ecs: &Ecs, entity: Entity) -> bool {
    !ecs.has::<RoomCamera>(entity)
        && !ecs.has::<PlayerProxy>(entity)
        && !ecs.has::<Player>(entity)
        && !ecs.has::<Global>(entity)
}

fn is_live_prefab_entity(ecs: &Ecs, entity: Entity) -> bool {
    ecs.get_store::<Transform>().contains(entity) && is_prefab_entity(ecs, entity)
}
