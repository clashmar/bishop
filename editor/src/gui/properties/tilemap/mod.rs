pub mod details_module;
pub mod library_module;

use self::details_module::TilemapDetailsModule;
use self::library_module::TilemapLibraryModule;
use super::collapsible::CollapsiblePropertyModule;
use super::PropertyModule;
use crate::shared::scene_ui::inspector::{InspectorContent, InspectorContext, InspectorOutput};
use crate::tilemap::background_module::BackgroundModule;
use bishop::prelude::*;
use engine_core::game::GameCtxMut;
use engine_core::tiles::{TileDefId, TileRegistry};
use widgets::constants::layout;

const OUTER_PADDING: f32 = 10.0;
const BACKGROUND_SECTION_HEIGHT: f32 = 70.0;

#[derive(Clone, Debug, Default)]
pub(super) struct TilemapPaneState {
    pub(super) selected_tile_id: Option<TileDefId>,
    pub(super) brush_tile_id: Option<TileDefId>,
    pub(super) pending_select_newest: bool,
}

pub(super) fn sorted_tile_ids(tile_registry: &TileRegistry) -> Vec<TileDefId> {
    let mut tile_ids: Vec<_> = tile_registry.iter().map(|(id, _)| id).collect();
    tile_ids.sort_by_key(|id| id.0);
    tile_ids
}

impl TilemapPaneState {
    fn reconcile(&mut self, tile_registry: &TileRegistry) {
        let tile_ids = sorted_tile_ids(tile_registry);

        if tile_ids.is_empty() {
            self.selected_tile_id = None;
            self.brush_tile_id = None;
            return;
        }

        if self.pending_select_newest {
            let newest_tile_id = tile_ids.last().copied();
            self.selected_tile_id = newest_tile_id;
            self.brush_tile_id = newest_tile_id;
            self.pending_select_newest = false;
            return;
        }

        if self
            .selected_tile_id
            .map(|tile_id| tile_registry.get(tile_id).is_none())
            .unwrap_or(true)
        {
            self.selected_tile_id = tile_ids.first().copied();
        }

        if self
            .brush_tile_id
            .map(|tile_id| tile_registry.get(tile_id).is_none())
            .unwrap_or(true)
        {
            self.brush_tile_id = self.selected_tile_id;
        }
    }
}

/// Inspector content for tilemap authoring in room tile mode.
pub struct TilemapProperties {
    state: TilemapPaneState,
    library: CollapsiblePropertyModule<TilemapPaneState, TilemapLibraryModule>,
    details: CollapsiblePropertyModule<TilemapPaneState, TilemapDetailsModule>,
    background: BackgroundModule,
}

impl TilemapProperties {
    pub fn new() -> Self {
        Self {
            state: TilemapPaneState::default(),
            library: CollapsiblePropertyModule::new(TilemapLibraryModule::new()),
            details: CollapsiblePropertyModule::new(TilemapDetailsModule::new()),
            background: BackgroundModule::new(),
        }
    }

    pub fn selected_brush_id(&self) -> Option<TileDefId> {
        self.state.brush_tile_id
    }
}

impl InspectorContent for TilemapProperties {
    fn draw_modules(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        blocked: bool,
        game_ctx: &mut GameCtxMut,
        insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        self.state.reconcile(game_ctx.tile_registry);

        let mut y = rect.y + OUTER_PADDING;
        let content_x = rect.x + OUTER_PADDING;
        let content_w = rect.w - OUTER_PADDING * 2.0;

        if self.library.visible(&self.state, game_ctx) {
            let height = self.library.height();
            let module_rect = Rect::new(content_x, y, content_w, height);
            self.library
                .draw(ctx, module_rect, &mut self.state, game_ctx, insp_ctx);
            y += height + layout::WIDGET_SPACING;
        }

        if self.details.visible(&self.state, game_ctx) {
            let height = self.details.height();
            let module_rect = Rect::new(content_x, y, content_w, height);
            self.details
                .draw(ctx, module_rect, &mut self.state, game_ctx, insp_ctx);
            y += height + layout::WIDGET_SPACING;
        }

        if let Some(room) = game_ctx
            .world
            .as_deref_mut()
            .and_then(|world| world.current_room_mut())
        {
            let background_rect = Rect::new(content_x, y, content_w, BACKGROUND_SECTION_HEIGHT);
            self.background
                .draw(ctx, background_rect, &mut room.current_variant_mut().tilemap, blocked);
        }

        InspectorOutput::default()
    }

    fn total_content_height(
        &self,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> f32 {
        let mut height = OUTER_PADDING;

        if self.library.visible(&self.state, game_ctx) {
            height += self.library.height() + layout::WIDGET_SPACING;
        }

        if self.details.visible(&self.state, game_ctx) {
            height += self.details.height() + layout::WIDGET_SPACING;
        }

        if game_ctx
            .world
            .as_deref()
            .and_then(|world| world.current_room())
            .is_some()
        {
            height += BACKGROUND_SECTION_HEIGHT;
        }

        height + OUTER_PADDING
    }
}
