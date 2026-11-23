use super::super::PropertyModule;
use crate::commands::game::EditWorldCmd;
use crate::editor_global::push_command;
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::*;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::storage::assets_folder;
use engine_core::worlds::world::World;
use ::widgets::constants::layout;
use ::widgets::{Button, WidgetId};

const ROW_HEIGHT: f32 = 30.0;

/// Edits the image used to represent a world in the game overview.
pub struct WorldVisualModule {
    picker_id: WidgetId,
}

impl WorldVisualModule {
    /// Creates the world image property module.
    pub fn new() -> Self {
        Self {
            picker_id: WidgetId::default(),
        }
    }
}

impl Default for WorldVisualModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyModule<World> for WorldVisualModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        world: &mut World,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let world_id = world.id;
        let pick_rect = Rect::new(rect.x, rect.y, rect.w, ROW_HEIGHT);
        let pick_label = if world.meta.sprite_id.is_some() {
            "Change World Image"
        } else {
            "Choose World Image"
        };
        if Button::new(pick_rect, pick_label)
            .interaction_id(self.picker_id)
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
                    if world.meta.sprite_id != Some(sprite_id) {
                        push_command(Box::new(EditWorldCmd::new(
                            world_id,
                            None,
                            Some(Some(sprite_id)),
                        )));
                    }
                }
            }
        }

        let clear_rect = Rect::new(
            rect.x,
            rect.y + ROW_HEIGHT + layout::WIDGET_SPACING,
            rect.w,
            ROW_HEIGHT,
        );
        if Button::new(clear_rect, "Use Default Circle").show(ctx)
            && world.meta.sprite_id.is_some()
        {
            push_command(Box::new(EditWorldCmd::new(world_id, None, Some(None))));
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .top_padding(layout::WIDGET_SPACING)
            .rows(2, layout::WIDGET_SPACING)
    }

    fn title(&self) -> &str {
        "World Visual"
    }
}
