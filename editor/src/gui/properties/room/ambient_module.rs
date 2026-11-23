use super::super::PropertyModule;
use crate::commands::room::{RoomAmbientSettings, SetRoomAmbientCmd};
use crate::editor_global::push_command;
use crate::gui::field_layout::{
    BOTTOM_GUTTER, FIELD_GAP, FIELD_HEIGHT, TOP_PADDING, draw_label, field_rect,
};
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::{Rect, WgpuContext};
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::room::Room;
use widgets::{ColorInput, InputCommit, NumberInput, WidgetId};

/// Title for the room ambient property module.
pub const ROOM_AMBIENT_MODULE_TITLE: &str = "Ambient";

/// Authors darkness, ambient color, and exposure for a room.
#[derive(Default)]
pub struct RoomAmbientModule {
    darkness_id: WidgetId,
    color_id: WidgetId,
    exposure_id: WidgetId,
}

impl RoomAmbientModule {
    /// Creates a room ambient property module.
    pub fn new() -> Self {
        Self::default()
    }
}

impl PropertyModule<Room> for RoomAmbientModule {
    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        room: &mut Room,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) {
        let Some(world_id) = game_ctx.world.as_deref().map(|world| world.id) else {
            return;
        };

        let old_settings = RoomAmbientSettings {
            darkness: room.darkness,
            color: room.ambient_color,
            exposure: room.exposure,
        };
        let mut new_settings = old_settings;
        let mut y = rect.y + TOP_PADDING;

        draw_label(ctx, "Darkness:", rect, y);
        let (darkness, darkness_commit) = NumberInput::new(
            self.darkness_id,
            field_rect(rect, y),
            room.darkness,
        )
        .min(0.0)
        .max(1.0)
        .show(ctx);
        if darkness_commit == InputCommit::Committed {
            new_settings.darkness = normalized_darkness(darkness);
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Color:", rect, y);
        let (mut color, color_commit) = ColorInput::new(
            self.color_id,
            field_rect(rect, y),
            room.ambient_color,
        )
        .show_with_commit(ctx);
        color.a = room.ambient_color.a;
        if color_commit == InputCommit::Committed {
            new_settings.color = color;
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Exposure:", rect, y);
        let (exposure, exposure_commit) = NumberInput::new(
            self.exposure_id,
            field_rect(rect, y),
            room.exposure,
        )
        .min(0.0)
        .show(ctx);
        if exposure_commit == InputCommit::Committed {
            new_settings.exposure = normalized_exposure(exposure);
        }

        if new_settings != old_settings {
            push_command(Box::new(SetRoomAmbientCmd::new(
                world_id,
                room.id,
                old_settings,
                new_settings,
            )));
        }
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .top_padding(TOP_PADDING)
            .rows(3, FIELD_GAP)
            .bottom_gutter(BOTTOM_GUTTER)
    }

    fn title(&self) -> &str {
        ROOM_AMBIENT_MODULE_TITLE
    }
}

fn normalized_darkness(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn normalized_exposure(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::{normalized_darkness, normalized_exposure};

    #[test]
    fn ambient_values_use_runtime_darkness_and_exposure_bounds() {
        assert_eq!(normalized_darkness(-0.5), 0.0);
        assert_eq!(normalized_darkness(1.5), 1.0);
        assert_eq!(normalized_darkness(f32::NAN), 0.0);
        assert_eq!(normalized_exposure(-0.5), 0.0);
        assert_eq!(normalized_exposure(f32::NAN), 1.0);
    }
}
