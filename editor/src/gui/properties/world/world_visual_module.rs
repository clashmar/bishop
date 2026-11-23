use std::cell::Cell;

use super::super::PropertyModule;
use crate::commands::game::EditWorldCmd;
use crate::editor_global::push_command;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::storage::assets_folder;
use engine_core::worlds::{WorldVisual, world::World};
use ::widgets::constants::layout;
use ::widgets::{Button, ColorInput, WidgetId};

const ROW_HEIGHT: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorldVisualControlMode {
    Image,
    Color,
}

/// Edits the image or solid color used to represent a world in the game overview.
pub struct WorldVisualModule {
    picker_id: WidgetId,
    control_mode: Cell<WorldVisualControlMode>,
}

impl WorldVisualModule {
    /// Creates the world visual property module.
    pub fn new() -> Self {
        Self {
            picker_id: WidgetId::default(),
            control_mode: Cell::new(WorldVisualControlMode::Color),
        }
    }

    fn controls_for(visual: WorldVisual) -> WorldVisualControlMode {
        match visual {
            WorldVisual::Image(_) => WorldVisualControlMode::Image,
            WorldVisual::Color(_) => WorldVisualControlMode::Color,
        }
    }

    fn choose_image(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        label: &str,
        game_ctx: &mut GameCtxMut,
    ) -> Option<WorldVisual> {
        if !Button::new(rect, label)
            .interaction_id(self.picker_id)
            .show_native_dialog(ctx)
        {
            return None;
        }

        let path = rfd::FileDialog::new()
            .add_filter("PNG images", &["png"])
            .set_directory(assets_folder())
            .pick_file()?;
        let asset_registry = &mut *game_ctx.asset_registry;
        let sprite_manager = &mut *game_ctx.sprite_manager;
        let normalized_path = sprite_manager.normalize_path(path);
        sprite_manager
            .get_or_load(asset_registry, ctx, &normalized_path)
            .map(WorldVisual::Image)
    }
}

impl Default for WorldVisualModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldVisualModule {
    fn visible(&self, target: &World, _game_ctx: &GameCtxMut) -> bool {
        self.control_mode.set(Self::controls_for(target.meta.visual));
        true
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        world: &mut World,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let world_id = world.id;
        let gap = layout::WIDGET_SPACING;
        match Self::controls_for(world.meta.visual) {
            WorldVisualControlMode::Image => {
                let first_row = centered_rows_y(rect, 2);
                let image_rect = Rect::new(rect.x, first_row, rect.w, ROW_HEIGHT);
                if let Some(visual) =
                    self.choose_image(ctx, image_rect, "Change World Image", game_ctx)
                {
                    push_command(Box::new(EditWorldCmd::new(world_id, None, Some(visual))));
                }

                let remove_rect = Rect::new(
                    rect.x,
                    first_row + ROW_HEIGHT + gap,
                    rect.w,
                    ROW_HEIGHT,
                );
                if Button::new(remove_rect, "Remove Image").show(ctx) {
                    push_command(Box::new(EditWorldCmd::new(
                        world_id,
                        None,
                        Some(WorldVisual::default()),
                    )));
                }
            }
            WorldVisualControlMode::Color => {
                let first_row = centered_rows_y(rect, 2);
                let WorldVisual::Color(current_color) = world.meta.visual else {
                    return;
                };
                let color_rect = Rect::new(rect.x, first_row, rect.w, ROW_HEIGHT);
                let mut color =
                    ColorInput::new(self.picker_id, color_rect, current_color).show(ctx);
                color.a = current_color.a;
                let visual = WorldVisual::Color(color);
                if visual != world.meta.visual {
                    push_command(Box::new(EditWorldCmd::new(world_id, None, Some(visual))));
                }

                let image_rect = Rect::new(rect.x, first_row + ROW_HEIGHT + gap, rect.w, ROW_HEIGHT);
                if let Some(visual) =
                    self.choose_image(ctx, image_rect, "Choose World Image", game_ctx)
                {
                    push_command(Box::new(EditWorldCmd::new(world_id, None, Some(visual))));
                }
            }
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        body_layout()
    }

    fn title(&self) -> &str {
        "World Visual"
    }
}

fn body_layout() -> InspectorBodyLayout {
    InspectorBodyLayout::new()
        .top_padding(layout::WIDGET_SPACING)
        .rows(2, layout::WIDGET_SPACING)
}

fn centered_rows_y(rect: Rect, rows: usize) -> f32 {
    let gaps = rows.saturating_sub(1) as f32 * layout::WIDGET_SPACING;
    let content_height = rows as f32 * ROW_HEIGHT + gaps;
    rect.y + ((rect.h - content_height).max(0.0) / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::SpriteId;

    #[test]
    fn image_mode_exposes_only_image_visual_controls() {
        assert_eq!(
            WorldVisualModule::controls_for(WorldVisual::Image(SpriteId(1))),
            WorldVisualControlMode::Image
        );
    }

    #[test]
    fn color_mode_exposes_only_color_visual_controls() {
        assert_eq!(
            WorldVisualModule::controls_for(WorldVisual::Color(Color::new(0.3, 0.6, 0.8, 1.0))),
            WorldVisualControlMode::Color
        );
    }

    #[test]
    fn active_controls_are_vertically_centered_in_the_module_body() {
        let rect = Rect::new(0.0, 10.0, 200.0, 150.0);

        for row_count in [2, 3] {
            let top = centered_rows_y(rect, row_count);
            let content_height = row_count as f32 * ROW_HEIGHT
                + (row_count - 1) as f32 * layout::WIDGET_SPACING;
            let top_space = top - rect.y;
            let bottom_space = rect.y + rect.h - top - content_height;

            assert!((top_space - bottom_space).abs() <= f32::EPSILON);
        }
    }

    #[test]
    fn image_and_color_modes_use_two_rows_of_body_height() {
        let height = body_layout().height();
        let expected_height = InspectorBodyLayout::new()
            .top_padding(layout::WIDGET_SPACING)
            .rows(2, layout::WIDGET_SPACING)
            .height();

        assert_eq!(height, expected_height);
    }
}
