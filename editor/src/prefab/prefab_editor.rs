use crate::app::EditorMode;
use crate::app::SubEditor;
use crate::canvas::grid;
use crate::canvas::grid_shader::GridRenderer;
use crate::gui::inspector::inspector_panel::InspectorPanel;
use crate::gui::menu_bar::draw_top_panel_full;
use crate::gui::modal::is_modal_open;
use crate::gui::panels::panel_manager::is_mouse_over_panel;
use crate::room::drawing::{draw_collider, draw_pivot_marker, highlight_selected_entity};
use crate::shared::scene_ui::inspector::{
    SceneCreateRequest, SceneEmptyInspectorBehavior, SceneInspectorContext,
};
use crate::prefab::canvas::draw_prefab_entities;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::HashSet;

pub const PREFAB_EDITOR_GRID_SIZE: f32 = 16.0;

pub struct PrefabStage {
    pub ecs: Ecs,
    pub asset_manager: AssetManager,
    pub script_manager: ScriptManager,
    /// Read-only prefab library loaded for linked-prefab labels.
    pub prefab_library: PrefabLibrary,
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
