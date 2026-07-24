use super::{sorted_tile_ids, TilemapPaneState};
use crate::commands::tilemap::{CreateTileDefinitionCmd, DeleteTileDefinitionCmd};
use crate::editor_global::push_command;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::SpriteId;
use engine_core::game::GameCtxMut;
use engine_core::theme::with_theme;
use engine_core::tiles::TileDef;
use widgets::constants::{colors, layout};
use widgets::*;

const ACTIONS_HEIGHT: f32 = layout::DEFAULT_FIELD_HEIGHT;
const ACTIONS_GAP: f32 = layout::WIDGET_SPACING;
const ROW_HEIGHT: f32 = 40.0;
const ROW_GAP: f32 = 6.0;
const PREVIEW_SIZE: f32 = 28.0;
const BRUSH_BUTTON_WIDTH: f32 = 72.0;

/// Tile-library module for selecting, creating, and deleting tile definitions.
pub struct TilemapLibraryModule {
    row_count: usize,
}

impl TilemapLibraryModule {
    pub fn new() -> Self {
        Self { row_count: 1 }
    }
}

impl super::super::PropertyModule<TilemapPaneState> for TilemapLibraryModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        state: &mut TilemapPaneState,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let tile_ids = sorted_tile_ids(game_ctx.tile_registry);
        self.row_count = tile_ids.len().max(1);

        let create_rect = Rect::new(rect.x, rect.y, 110.0, ACTIONS_HEIGHT);
        if Button::new(create_rect, "Create Tile").show(ctx) {
            push_command(Box::new(CreateTileDefinitionCmd::new(TileDef {
                sprite_id: SpriteId(0),
                components: Vec::new(),
            })));
            state.pending_select_newest = true;
        }

        let delete_rect = Rect::new(
            create_rect.x + create_rect.w + layout::WIDGET_SPACING,
            create_rect.y,
            95.0,
            ACTIONS_HEIGHT,
        );
        if Button::new(delete_rect, "Delete")
            .blocked(state.selected_tile_id.is_none())
            .show(ctx)
        {
            if let Some(tile_id) = state.selected_tile_id {
                push_command(Box::new(DeleteTileDefinitionCmd::new(tile_id)));
                if state.brush_tile_id == Some(tile_id) {
                    state.brush_tile_id = None;
                }
                state.selected_tile_id = None;
            }
        }

        let mut y = rect.y + ACTIONS_HEIGHT + ACTIONS_GAP;
        if tile_ids.is_empty() {
            ctx.draw_text(
                "No tile definitions yet",
                rect.x,
                y + 20.0,
                layout::DEFAULT_FONT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            return;
        }

        for tile_id in tile_ids {
            let Some(sprite_id) = game_ctx.tile_registry.get(tile_id).map(|tile_def| tile_def.sprite_id) else {
                continue;
            };

            let row_rect = Rect::new(rect.x, y, rect.w, ROW_HEIGHT);
            let brush_rect = Rect::new(
                row_rect.x + row_rect.w - BRUSH_BUTTON_WIDTH,
                row_rect.y + 5.0,
                BRUSH_BUTTON_WIDTH,
                layout::DEFAULT_FIELD_HEIGHT,
            );
            let select_rect = Rect::new(
                row_rect.x,
                row_rect.y,
                row_rect.w - BRUSH_BUTTON_WIDTH - layout::WIDGET_SPACING,
                row_rect.h,
            );

            let selected = state.selected_tile_id == Some(tile_id);
            let brush_selected = state.brush_tile_id == Some(tile_id);
            let row_bg = if selected {
                with_theme(|theme| theme.panel)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.25)
            };
            let row_border = if brush_selected { Color::RED } else { Color::WHITE };

            ctx.draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h, row_bg);
            ctx.draw_rectangle_lines(row_rect.x, row_rect.y, row_rect.w, row_rect.h, 2.0, row_border);

            if Button::new(select_rect, "").plain().show(ctx) {
                state.selected_tile_id = Some(tile_id);
                state.brush_tile_id = Some(tile_id);
            }

            if Button::new(
                brush_rect,
                if brush_selected { "Using" } else { "Brush" },
            )
            .show(ctx)
            {
                state.selected_tile_id = Some(tile_id);
                state.brush_tile_id = Some(tile_id);
            }

            let preview_rect = Rect::new(row_rect.x + 6.0, row_rect.y + 6.0, PREVIEW_SIZE, PREVIEW_SIZE);
            let texture = game_ctx
                .sprite_manager
                .get_texture_from_id(ctx, sprite_id);
            ctx.draw_texture_ex(
                texture,
                preview_rect.x,
                preview_rect.y,
                Color::WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(preview_rect.w, preview_rect.h)),
                    ..Default::default()
                },
            );

            ctx.draw_text(
                &format!("Tile {}", tile_id.0),
                preview_rect.x + PREVIEW_SIZE + 8.0,
                row_rect.y + 17.0,
                layout::DEFAULT_FONT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            ctx.draw_text(
                &format!("Sprite {}", sprite_id.0),
                preview_rect.x + PREVIEW_SIZE + 8.0,
                row_rect.y + 31.0,
                layout::FIELD_TEXT_SIZE_16,
                Color::new(0.8, 0.8, 0.8, 1.0),
            );

            y += ROW_HEIGHT + ROW_GAP;
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        let list_height = if self.row_count == 0 {
            ROW_HEIGHT
        } else {
            self.row_count as f32 * ROW_HEIGHT + (self.row_count.saturating_sub(1)) as f32 * ROW_GAP
        };

        InspectorBodyLayout::new()
            .block(ACTIONS_HEIGHT)
            .gap(ACTIONS_GAP)
            .block(list_height)
    }

    fn title(&self) -> &str {
        "Library"
    }
}
