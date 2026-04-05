use crate::app::EditorMode;
use crate::app::SubEditor;
use crate::canvas::grid;
use crate::canvas::grid_shader::GridRenderer;
use crate::editor_global::with_lua;
use crate::gui::inspector::inspector_panel::InspectorPanel;
use crate::gui::menu_bar::draw_top_panel_full;
use crate::gui::modal::is_modal_open;
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use crate::room::entity_hitbox;
use crate::room::drawing::{draw_collider, draw_pivot_marker, highlight_selected_entity};
use crate::storage::editor_storage::load_game_by_name;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::{BTreeMap, HashSet};
use std::io;

pub const PREFAB_EDITOR_GRID_SIZE: f32 = 16.0;

pub struct PrefabStage {
    pub ecs: Ecs,
    pub asset_manager: AssetManager,
    pub script_manager: ScriptManager,
    /// Read-only prefab library loaded for linked-prefab labels.
    pub prefab_library: PrefabLibrary,
}

impl PrefabStage {
    pub fn new(game_name: &str) -> Self {
        let mut game = load_prefab_game(game_name);

        with_lua(|lua| {
            AssetManager::init_editor_metadata(&mut game.asset_manager);
            ScriptManager::init_editor_services(&mut game.script_manager, lua);
        });

        Self {
            ecs: Ecs::default(),
            asset_manager: game.asset_manager,
            script_manager: game.script_manager,
            prefab_library: game.prefab_library,
        }
    }

    pub fn ctx_mut(&mut self) -> ServicesCtxMut<'_> {
        ServicesCtxMut {
            ecs: &mut self.ecs,
            world: None,
            asset_manager: &mut self.asset_manager,
            script_manager: &mut self.script_manager,
            prefab_library: &self.prefab_library,
        }
    }
}

pub struct PrefabEditor {
    pub prefab_id: PrefabId,
    pub prefab_name: String,
    pub loaded_prefab: Option<PrefabAsset>,
    pub root_entity: Option<Entity>,
    pub selected_entities: HashSet<Entity>,
    pub inspector: InspectorPanel,
    pub active_rects: Vec<Rect>,
    pub show_grid: bool,
    create_entity_requested: bool,
}

impl PrefabEditor {
    pub fn open_existing(game_name: &str, prefab: PrefabAsset) -> (Self, PrefabStage) {
        let mut stage = PrefabStage::new(game_name);
        let root = {
            let mut game_ctx = stage.ctx_mut();
            instantiate_prefab(&mut game_ctx, &prefab, Vec2::ZERO, None)
        };

        let mut editor = Self::new(prefab.id, prefab.name.clone(), Some(prefab));
        editor.set_selected_entity(Some(root));
        editor.root_entity = Some(root);
        (editor, stage)
    }

    pub fn new(prefab_id: PrefabId, prefab_name: String, loaded_prefab: Option<PrefabAsset>) -> Self {
        Self {
            prefab_id,
            prefab_name,
            loaded_prefab,
            root_entity: None,
            selected_entities: HashSet::new(),
            inspector: InspectorPanel::new(),
            active_rects: Vec::new(),
            show_grid: true,
            create_entity_requested: false,
        }
    }

    pub fn update(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        game_ctx: &mut ServicesCtxMut,
    ) {
        self.sanitize_live_state(game_ctx.ecs);
        self.inspector
            .set_prefab_context(true, self.root_entity.or_else(|| self.single_selected_entity()));

        if ctx.is_mouse_button_pressed(MouseButton::Left) && !self.should_block_canvas(ctx) {
            self.handle_selection(ctx, camera, game_ctx.ecs, game_ctx.asset_manager);
        }

        if self.create_entity_requested {
            self.create_entity_requested = false;
            let entity = self.create_prefab_entity(game_ctx.ecs);
            self.set_selected_entity(Some(entity));
        }

        if self.selected_entities.len() == 1 {
            self.inspector.set_target(self.single_selected_entity());
        } else {
            self.inspector.set_target(None);
        }
    }

    pub fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        game_ctx: &mut ServicesCtxMut,
        grid_renderer: &GridRenderer,
    ) {
        self.active_rects.clear();

        ctx.set_camera(camera);
        ctx.clear_background(Color::BLACK);

        if self.show_grid {
            grid::draw_grid(ctx, grid_renderer, camera, PREFAB_EDITOR_GRID_SIZE);
        }

        draw_prefab_entities(ctx, game_ctx.ecs, game_ctx.asset_manager, PREFAB_EDITOR_GRID_SIZE);

        for &selected_entity in &self.selected_entities {
            highlight_selected_entity(
                ctx,
                game_ctx.ecs,
                selected_entity,
                game_ctx.asset_manager,
                Color::YELLOW,
                PREFAB_EDITOR_GRID_SIZE,
            );
            draw_pivot_marker(ctx, game_ctx.ecs, selected_entity);
        }

        if let Some(selected_entity) = self.single_selected_entity() {
            draw_collider(ctx, game_ctx.ecs, selected_entity);
        }

        ctx.set_default_camera();
        self.active_rects.push(draw_top_panel_full(ctx));

        const INSPECTOR_W: f32 = 325.0;
        let inspector_rect = Rect::new(
            ctx.screen_width() - INSPECTOR_W,
            0.0,
            INSPECTOR_W,
            ctx.screen_height(),
        );
        self.inspector.set_rect(inspector_rect);
        self.create_entity_requested =
            self.inspector
                .draw(ctx, game_ctx, EditorMode::Prefab(self.prefab_id));
    }

    pub fn save_to_disk(
        &mut self,
        game_name: &str,
        game_ctx: &mut ServicesCtxMut,
    ) -> io::Result<Option<PrefabAsset>> {
        let Some(root) = self.root_entity else {
            return Ok(None);
        };

        let prefab = capture_prefab_with_existing(
            game_ctx.ecs,
            root,
            self.prefab_id,
            self.prefab_name.clone(),
            self.loaded_prefab.as_ref(),
        );
        save_prefab(game_name, &prefab)?;
        self.prefab_name = prefab.name.clone();
        self.loaded_prefab = Some(prefab.clone());
        Ok(Some(prefab))
    }

    pub fn set_name(&mut self, name: String) {
        self.prefab_name = name;
    }

    pub fn set_selected_entity(&mut self, entity: Option<Entity>) {
        self.selected_entities.clear();
        if let Some(entity) = entity {
            self.selected_entities.insert(entity);
        }
        self.inspector.set_target(entity);
    }

    pub fn add_to_selection(&mut self, entity: Entity) {
        self.selected_entities.insert(entity);
        if self.selected_entities.len() == 1 {
            self.inspector.set_target(Some(entity));
        } else {
            self.inspector.set_target(None);
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
        self.inspector.set_target(self.single_selected_entity());
    }

    pub fn restore_deleted_root(&mut self, restored_root: Entity) {
        self.root_entity = Some(restored_root);
        self.set_selected_entity(Some(restored_root));
    }

    fn handle_selection(
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
            (true, Some(entity)) => {
                if self.selected_entities.contains(&entity) {
                    self.selected_entities.remove(&entity);
                } else {
                    self.selected_entities.insert(entity);
                }
            }
            (false, Some(entity)) => self.set_selected_entity(Some(entity)),
            (false, None) => self.set_selected_entity(None),
            (true, None) => {}
        }
    }

    fn create_prefab_entity(&mut self, ecs: &mut Ecs) -> Entity {
        let entity = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Entity".to_string()))
            .finish();

        if let Some(parent) = self
            .inspector
            .take_pending_create_parent()
            .filter(|parent| is_live_prefab_entity(ecs, *parent))
        {
            set_parent(ecs, entity, parent);
        } else if let Some(root) = self.root_entity.filter(|root| is_live_prefab_entity(ecs, *root))
        {
            set_parent(ecs, entity, root);
        } else {
            self.root_entity = Some(entity);
        }

        entity
    }

    fn sanitize_live_state(&mut self, ecs: &Ecs) {
        if self.root_entity.is_some_and(|entity| !is_live_prefab_entity(ecs, entity)) {
            self.root_entity = None;
        }

        self.selected_entities
            .retain(|entity| is_live_prefab_entity(ecs, *entity));
        self.inspector.set_target(self.single_selected_entity());
    }
}

impl SubEditor for PrefabEditor {
    fn active_rects(&self) -> &[Rect] {
        &self.active_rects
    }

    fn should_block_canvas(&self, ctx: &WgpuContext) -> bool {
        let mouse_screen: Vec2 = ctx.mouse_position().into();
        self.active_rects.iter().any(|rect| rect.contains(mouse_screen))
            || self.inspector.is_mouse_over(ctx)
            || is_dropdown_open()
            || is_modal_open()
            || is_mouse_over_panel(ctx)
    }
}

pub fn is_prefab_entity(ecs: &Ecs, entity: Entity) -> bool {
    !ecs.has::<RoomCamera>(entity)
        && !ecs.has::<PlayerProxy>(entity)
        && !ecs.has::<Player>(entity)
        && !ecs.has::<Global>(entity)
}

/// Identifies whether a linked prefab reference came from a root or child node component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefabLinkSource {
    /// The entity is the linked prefab root.
    Root,
    /// The entity is a linked prefab child node.
    Node,
}

/// Read-only display data for a linked prefab instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabLinkDisplay {
    /// The component source that identified the link.
    pub source: PrefabLinkSource,
    /// Stable prefab asset id.
    pub prefab_id: PrefabId,
    /// Human-readable label for UI display.
    pub label: String,
}

/// Returns read-only display data for a linked prefab instance entity.
pub fn linked_prefab_display(
    ecs: &Ecs,
    prefab_library: &PrefabLibrary,
    entity: Entity,
) -> Option<PrefabLinkDisplay> {
    let (source, prefab_id) = if let Some(root) = ecs.get::<PrefabInstanceRoot>(entity) {
        (PrefabLinkSource::Root, root.prefab_id)
    } else if let Some(node) = ecs.get::<PrefabInstanceNode>(entity) {
        (PrefabLinkSource::Node, node.prefab_id)
    } else {
        return None;
    };

    let prefab_label = prefab_library
        .prefabs
        .get(&prefab_id)
        .map(|prefab| prefab.name.clone())
        .unwrap_or_else(|| prefab_id.to_string());

    Some(PrefabLinkDisplay {
        source,
        prefab_id,
        label: format!("Prefab: {prefab_label}"),
    })
}

fn is_live_prefab_entity(ecs: &Ecs, entity: Entity) -> bool {
    ecs.get_store::<Transform>().contains(entity) && is_prefab_entity(ecs, entity)
}

fn load_prefab_game(game_name: &str) -> Game {
    load_game_by_name(game_name).unwrap_or_else(|_| Game {
        name: game_name.to_string(),
        ..Default::default()
    })
}

fn draw_prefab_entities<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    asset_manager: &mut AssetManager,
    grid_size: f32,
) {
    let mut layer_map: BTreeMap<i32, Vec<(Entity, Vec2)>> = BTreeMap::new();

    for (entity, transform) in ecs.get_store::<Transform>().data.iter() {
        if !transform.visible || !is_prefab_entity(ecs, *entity) {
            continue;
        }

        let z = ecs.get_store::<Layer>().get(*entity).map_or(0, |layer| layer.z);
        layer_map
            .entry(z)
            .or_default()
            .push((*entity, transform.position));
    }

    for entities in layer_map.into_values() {
        for (entity, position) in entities {
            draw_prefab_entity(ctx, ecs, asset_manager, entity, position, grid_size);
        }
    }
}

fn draw_prefab_entity<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    asset_manager: &mut AssetManager,
    entity: Entity,
    pos: Vec2,
    grid_size: f32,
) {
    let visual_entity = resolve_visual_entity(ecs, entity);
    let pivot = ecs
        .get_store::<Transform>()
        .get(entity)
        .map(|transform| transform.pivot)
        .unwrap_or(Pivot::BottomCenter);
    let params = EntityDrawParams {
        pos,
        pivot,
        grid_size,
    };

    if let Some(current_frame) = ecs.get_store::<CurrentFrame>().get(visual_entity) {
        if current_frame.draw(ctx, asset_manager, &params) {
            return;
        }
    }

    if let Some(sprite) = ecs.get_store::<Sprite>().get(visual_entity) {
        if sprite.draw(ctx, asset_manager, &params) {
            return;
        }
    }

    if ecs.has_any::<(Light, Glow)>(visual_entity) {
        return;
    }

    let draw_pos = pivot_adjusted_position(pos, Vec2::splat(grid_size), pivot);
    draw_entity_placeholder(ctx, draw_pos, grid_size);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Editor;
    use crate::app::EditorMode;
    use crate::commands::editor_command_manager::EditorCommand;
    use crate::commands::room::DeleteEntityCmd;
    use crate::editor_global::{reset_services, set_editor, with_editor, EDITOR_SERVICES};
    use crate::storage::editor_storage::{create_new_game, save_game};
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use std::path::PathBuf;

    struct EditorServicesGuard;

    impl EditorServicesGuard {
        fn install(editor: Editor) -> Self {
            reset_services();
            set_editor(editor);
            Self
        }
    }

    impl Drop for EditorServicesGuard {
        fn drop(&mut self) {
            EDITOR_SERVICES.with(|services| {
                *services.editor.borrow_mut() = None;
            });
            reset_services();
        }
    }

    #[test]
    fn prefab_stage_uses_project_sprite_paths_without_room_state() {
        let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
        let test_game = TestGameFolder::new("prefab_stage_game");

        let mut game = create_new_game(test_game.name().to_string());
        game.asset_manager.sprite_id_to_path.insert(
            SpriteId(7),
            PathBuf::from("sprites/cat.png"),
        );
        save_game(&game).unwrap();

        let mut stage = PrefabStage::new(test_game.name());
        let prefab_ctx = stage.ctx_mut();

        assert_eq!(
            prefab_ctx.asset_manager.sprite_id_to_path.get(&SpriteId(7)).cloned(),
            Some(PathBuf::from("sprites/cat.png"))
        );
        assert!(prefab_ctx.ecs.get_store::<RoomCamera>().data.is_empty());
        assert!(prefab_ctx.ecs.get_store::<CurrentRoom>().data.is_empty());
        assert!(prefab_ctx.world.is_none());
    }

    #[test]
    fn linked_prefab_display_uses_root_metadata_for_roots_and_node_metadata_for_children() {
        let mut ecs = Ecs::default();
        let root = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Root".to_string()))
            .finish();
        let child = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Child".to_string()))
            .finish();
        set_parent(&mut ecs, child, root);

        let prefab_id = PrefabId(7);
        let prefab = create_prefab(prefab_id, "Crate".to_string());
        let mut prefab_library = PrefabLibrary::default();
        prefab_library.prefabs.insert(prefab_id, prefab);

        ecs.add_component_to_entity(
            root,
            PrefabInstanceRoot {
                prefab_id,
            },
        );
        ecs.add_component_to_entity(
            root,
            PrefabInstanceNode {
                prefab_id,
                node_id: 1,
                root_entity: root,
            },
        );
        ecs.add_component_to_entity(
            child,
            PrefabInstanceNode {
                prefab_id,
                node_id: 2,
                root_entity: root,
            },
        );

        let root_display = linked_prefab_display(&ecs, &prefab_library, root).unwrap();
        let child_display = linked_prefab_display(&ecs, &prefab_library, child).unwrap();

        assert_eq!(root_display.source, PrefabLinkSource::Root);
        assert_eq!(root_display.label, "Prefab: Crate");
        assert_eq!(child_display.source, PrefabLinkSource::Node);
        assert_eq!(child_display.label, "Prefab: Crate");
    }

    #[test]
    fn linked_prefab_display_falls_back_to_prefab_id_when_asset_is_missing() {
        let mut ecs = Ecs::default();
        let entity = ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Entity".to_string()))
            .finish();

        ecs.add_component_to_entity(
            entity,
            PrefabInstanceRoot {
                prefab_id: PrefabId(42),
            },
        );

        let prefab_library = PrefabLibrary::default();
        let display = linked_prefab_display(&ecs, &prefab_library, entity).unwrap();

        assert_eq!(display.source, PrefabLinkSource::Root);
        assert_eq!(display.label, "Prefab: 42");
    }

    #[test]
    fn editor_services_guard_clears_global_editor_on_drop() {
        {
            let _guard = EditorServicesGuard::install(Editor::default());
            EDITOR_SERVICES.with(|services| {
                assert!(services.editor.borrow().is_some());
            });
        }

        EDITOR_SERVICES.with(|services| {
            assert!(services.editor.borrow().is_none());
        });
    }

    #[test]
    fn creating_entity_replaces_stale_root_with_new_root() {
        let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
        let test_game = TestGameFolder::new("prefab_stale_root");
        let mut editor = PrefabEditor::new(PrefabId(1), "Prefab".to_string(), None);
        let mut stage = PrefabStage::new(test_game.name());

        let stale_root = stage
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Old Root".to_string()))
            .finish();
        editor.root_entity = Some(stale_root);
        editor.set_selected_entity(Some(stale_root));

        {
            let mut ctx = stage.ctx_mut();
            Ecs::remove_entity(&mut ctx, stale_root);
        }

        let new_entity = editor.create_prefab_entity(&mut stage.ecs);

        assert_eq!(editor.root_entity, Some(new_entity));
        assert_eq!(get_parent(&stage.ecs, new_entity), None);
    }

    #[test]
    fn deleting_prefab_root_clears_root_and_selection_state() {
        let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
        let test_game = TestGameFolder::new("prefab_delete_root");
        let mut editor = Editor {
            mode: EditorMode::Prefab(PrefabId(9)),
            prefab_editor: Some(PrefabEditor::new(
                PrefabId(9),
                "Prefab".to_string(),
                None,
            )),
            prefab_stage: Some(PrefabStage::new(test_game.name())),
            ..Default::default()
        };

        let root = editor
            .prefab_stage
            .as_mut()
            .unwrap()
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Root".to_string()))
            .finish();
        let child = editor
            .prefab_stage
            .as_mut()
            .unwrap()
            .ecs
            .create_entity()
            .with(Transform::default())
            .with(Name("Child".to_string()))
            .finish();
        set_parent(&mut editor.prefab_stage.as_mut().unwrap().ecs, child, root);

        let prefab_editor = editor.prefab_editor.as_mut().unwrap();
        prefab_editor.root_entity = Some(root);
        prefab_editor.selected_entities.insert(root);
        prefab_editor.selected_entities.insert(child);

        let _guard = EditorServicesGuard::install(editor);

        let mut cmd = DeleteEntityCmd::new(root, EditorMode::Prefab(PrefabId(9)));
        cmd.execute();

        with_editor(|editor| {
            let prefab_editor = editor.prefab_editor.as_ref().unwrap();
            assert_eq!(prefab_editor.root_entity, None);
            assert!(prefab_editor.selected_entities.is_empty());
        });
    }
}
