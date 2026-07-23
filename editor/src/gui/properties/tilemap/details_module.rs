use super::TilemapPaneState;
use crate::commands::tilemap::UpdateTileDefinitionCmd;
use crate::editor_global::push_command;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::storage::assets_folder;
use engine_core::tiles::{TileComponent, TileDef, TileDefId};
use widgets::constants::{colors, layout};
use widgets::{Button, Checkbox, InputCommit, NumberInput, WidgetId};

const TOP_LABEL_HEIGHT: f32 = 22.0;
const ROW_GAP: f32 = layout::WIDGET_SPACING;
const PREVIEW_SIZE: f32 = 40.0;

/// Inline tile-definition editor for the selected tile definition.
pub struct TilemapDetailsModule {
    sprite_picker_id: WidgetId,
    damage_input_id: WidgetId,
    has_selection: bool,
}

impl TilemapDetailsModule {
    pub fn new() -> Self {
        Self {
            sprite_picker_id: WidgetId::default(),
            damage_input_id: WidgetId::default(),
            has_selection: false,
        }
    }

    fn walkable(tile_def: &TileDef) -> bool {
        tile_def
            .components
            .iter()
            .find_map(|component| match component {
                TileComponent::Walkable(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(false)
    }

    fn solid(tile_def: &TileDef) -> bool {
        tile_def
            .components
            .iter()
            .find_map(|component| match component {
                TileComponent::Solid(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(false)
    }

    fn damage(tile_def: &TileDef) -> f32 {
        tile_def
            .components
            .iter()
            .find_map(|component| match component {
                TileComponent::Damage(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(0.0)
    }

    fn build_tile_def(before: &TileDef, walkable: bool, solid: bool, damage: f32) -> TileDef {
        let mut after = before.clone();
        let mut has_walkable = false;
        let mut has_solid = false;
        let mut has_damage = false;

        after.components.retain(|component| {
            !matches!(component, TileComponent::Damage(_)) || damage > 0.0
        });

        for component in &mut after.components {
            match component {
                TileComponent::Walkable(value) => {
                    *value = walkable;
                    has_walkable = true;
                }
                TileComponent::Solid(value) => {
                    *value = solid;
                    has_solid = true;
                }
                TileComponent::Damage(value) => {
                    *value = damage;
                    has_damage = true;
                }
            }
        }

        if !has_walkable {
            after.components.push(TileComponent::Walkable(walkable));
        }
        if !has_solid {
            after.components.push(TileComponent::Solid(solid));
        }
        if damage > 0.0 && !has_damage {
            after.components.push(TileComponent::Damage(damage));
        }

        after
    }

    fn push_update_if_changed(tile_id: TileDefId, before: TileDef, after: TileDef) {
        if before != after {
            push_command(Box::new(UpdateTileDefinitionCmd::new(tile_id, before, after)));
        }
    }
}

impl super::super::PropertyModule<TilemapPaneState> for TilemapDetailsModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        state: &mut TilemapPaneState,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let Some(tile_id) = state.selected_tile_id else {
            self.has_selection = false;
            ctx.draw_text(
                "Select a tile definition to edit",
                rect.x,
                rect.y + 20.0,
                layout::DEFAULT_FONT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            return;
        };

        let Some(before) = game_ctx.tile_registry.get(tile_id).cloned() else {
            self.has_selection = false;
            ctx.draw_text(
                "Selected tile definition no longer exists",
                rect.x,
                rect.y + 20.0,
                layout::DEFAULT_FONT_SIZE_16,
                colors::DEFAULT_TEXT_COLOR,
            );
            return;
        };

        self.has_selection = true;

        let walkable = Self::walkable(&before);
        let solid = Self::solid(&before);
        let damage = Self::damage(&before);

        ctx.draw_text(
            &format!("Editing Tile {}", tile_id.0),
            rect.x,
            rect.y + 18.0,
            layout::DEFAULT_FONT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

        let sprite_row_y = rect.y + TOP_LABEL_HEIGHT + ROW_GAP;
        let sprite_button_rect = Rect::new(rect.x, sprite_row_y, rect.w - PREVIEW_SIZE - ROW_GAP, layout::DEFAULT_FIELD_HEIGHT);
        if Button::new(sprite_button_rect, "Pick Sprite")
            .interaction_id(self.sprite_picker_id)
            .show_native_dialog(ctx)
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG images", &["png"])
                .set_directory(assets_folder())
                .pick_file()
            {
                let asset_registry = &mut *game_ctx.asset_registry;
                let sprite_manager = &mut *game_ctx.sprite_manager;
                let normalized_path = sprite_manager.normalize_path(path);
                if let Some(sprite_id) =
                    sprite_manager.get_or_load(asset_registry, ctx, &normalized_path)
                {
                    let mut after = before.clone();
                    after.sprite_id = sprite_id;
                    Self::push_update_if_changed(tile_id, before.clone(), after);
                }
            }
        }

        let preview_rect = Rect::new(
            rect.x + rect.w - PREVIEW_SIZE,
            sprite_row_y,
            PREVIEW_SIZE,
            PREVIEW_SIZE,
        );
        let texture = game_ctx
            .sprite_manager
            .get_texture_from_id(ctx, before.sprite_id);
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
        ctx.draw_rectangle_lines(
            preview_rect.x,
            preview_rect.y,
            preview_rect.w,
            preview_rect.h,
            2.0,
            Color::WHITE,
        );

        let walkable_rect = Rect::new(rect.x, sprite_row_y + PREVIEW_SIZE + ROW_GAP, layout::DEFAULT_CHECKBOX_DIMS, layout::DEFAULT_CHECKBOX_DIMS);
        let mut walkable_value = walkable;
        if Checkbox::new(walkable_rect, &mut walkable_value).show(ctx) {
            let after = Self::build_tile_def(&before, walkable_value, solid, damage);
            Self::push_update_if_changed(tile_id, before.clone(), after);
        }
        ctx.draw_text(
            "Walkable",
            walkable_rect.x + layout::DEFAULT_CHECKBOX_DIMS + 6.0,
            walkable_rect.y + 15.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

        let solid_rect = Rect::new(rect.x, walkable_rect.y + layout::DEFAULT_FIELD_HEIGHT, layout::DEFAULT_CHECKBOX_DIMS, layout::DEFAULT_CHECKBOX_DIMS);
        let mut solid_value = solid;
        if Checkbox::new(solid_rect, &mut solid_value).show(ctx) {
            let after = Self::build_tile_def(&before, walkable, solid_value, damage);
            Self::push_update_if_changed(tile_id, before.clone(), after);
        }
        ctx.draw_text(
            "Solid",
            solid_rect.x + layout::DEFAULT_CHECKBOX_DIMS + 6.0,
            solid_rect.y + 15.0,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

        let damage_label_y = solid_rect.y + layout::DEFAULT_FIELD_HEIGHT + 20.0;
        ctx.draw_text(
            "Damage",
            rect.x,
            damage_label_y,
            layout::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        let damage_rect = Rect::new(
            rect.x + 70.0,
            damage_label_y - 16.0,
            90.0,
            layout::DEFAULT_FIELD_HEIGHT,
        );
        let (damage_value, commit) = NumberInput::new(self.damage_input_id, damage_rect, damage)
            .min(0.0)
            .show(ctx);
        if commit == InputCommit::Committed && (damage_value - damage).abs() > f32::EPSILON {
            let after = Self::build_tile_def(&before, walkable, solid, damage_value);
            Self::push_update_if_changed(tile_id, before, after);
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        if !self.has_selection {
            return InspectorBodyLayout::new().block(layout::DEFAULT_FIELD_HEIGHT);
        }

        InspectorBodyLayout::new().block(164.0)
    }

    fn title(&self) -> &str {
        "Details"
    }
}
