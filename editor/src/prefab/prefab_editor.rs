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
use crate::shared::scene_ui::inspector::{
    SceneCreateRequest, SceneEmptyInspectorBehavior, SceneInspectorContext,
};
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
    create_request: Option<SceneCreateRequest>,
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
            create_request: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        game_ctx: &mut ServicesCtxMut,
    ) {
        self.sanitize_live_state(game_ctx.ecs);

        if ctx.is_mouse_button_pressed(MouseButton::Left) && !self.should_block_canvas(ctx) {
            self.handle_selection(ctx, camera, game_ctx.ecs, game_ctx.asset_manager);
        }

        if let Some(create_request) = self.create_request.take() {
            let entity = self.create_prefab_entity(game_ctx.ecs, create_request.parent);
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
        let inspector_ctx = SceneInspectorContext {
            command_mode: EditorMode::Prefab(self.prefab_id),
            show_linked_prefab_metadata: false,
            hide_room_only_components: true,
            selected_create_parent: self.single_selected_entity(),
            empty_state: SceneEmptyInspectorBehavior::Prefab {
                fallback_parent: self.root_entity,
            },
        };
        self.create_request = self.inspector.draw(ctx, game_ctx, &inspector_ctx).create_request;
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
