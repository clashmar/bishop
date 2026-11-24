use super::super::PropertyModule;
use crate::commands::room::{RoomAmbientSettings, SetRoomAmbientCmd};
use crate::editor_global::push_command;
use crate::gui::field_layout::{
    BOTTOM_GUTTER, FIELD_GAP, FIELD_HEIGHT, TOP_PADDING, draw_label, field_rect,
    normalized_slider_value, numeric_slider_rects,
};
use crate::shared::scene_ui::inspector::InspectorContext;
use bishop::prelude::{Rect, WgpuContext};
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::game::GameCtxMut;
use engine_core::worlds::room::Room;
use widgets::{ColorInput, ColorInputEdit, InputCommit, NumberInput, Slider, SliderState, WidgetId};

/// Title for the room ambient property module.
pub const ROOM_AMBIENT_MODULE_TITLE: &str = "Ambient";

const DARKNESS_MAX: f32 = 1.0;
const EXPOSURE_MAX: f32 = 4.0;

/// Authors darkness, ambient color, and exposure for a room.
#[derive(Default)]
pub struct RoomAmbientModule {
    darkness_id: WidgetId,
    darkness_slider_id: WidgetId,
    color_id: WidgetId,
    exposure_id: WidgetId,
    exposure_slider_id: WidgetId,
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
        let mut command_old_settings = old_settings;
        let mut new_settings = old_settings;
        let mut command_needed = false;
        let mut y = rect.y + TOP_PADDING;

        draw_label(ctx, "Darkness:", rect, y);
        let darkness_value = normalized_slider_value(room.darkness, 0.0, DARKNESS_MAX, 2);
        let (input_rect, slider_rect) = numeric_slider_rects(rect, y);
        let (darkness, darkness_commit) = NumberInput::new(
            self.darkness_id,
            input_rect,
            darkness_value,
        )
        .min(0.0)
        .max(DARKNESS_MAX)
        .show(ctx);
        let (darkness_slider, darkness_slider_state) = Slider::new(
            self.darkness_slider_id,
            slider_rect,
            0.0,
            DARKNESS_MAX,
            darkness_value,
        )
        .show(ctx);
        if darkness_commit == InputCommit::Committed {
            new_settings.darkness = normalized_slider_value(darkness, 0.0, DARKNESS_MAX, 2);
            command_needed = true;
        }
        match darkness_slider_state {
            SliderState::Previewing => {
                let preview = normalized_slider_value(darkness_slider, 0.0, DARKNESS_MAX, 2);
                room.darkness = preview;
                new_settings.darkness = preview;
            }
            SliderState::Committed { initial_value } => {
                command_old_settings.darkness =
                    normalized_slider_value(initial_value, 0.0, DARKNESS_MAX, 2);
                new_settings.darkness =
                    normalized_slider_value(darkness_slider, 0.0, DARKNESS_MAX, 2);
                command_needed = true;
            }
            SliderState::Unchanged => {}
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Color:", rect, y);
        let (color_response, color_commit) = ColorInput::new(
            self.color_id,
            field_rect(rect, y),
            room.ambient_color,
        )
        .show_with_commit(ctx);
        match color_response.edit {
            ColorInputEdit::Preview { initial } => {
                room.ambient_color = color_response.color;
                command_old_settings.color = initial;
                new_settings.color = color_response.color;
            }
            ColorInputEdit::Commit { initial } => {
                room.ambient_color = color_response.color;
                command_old_settings.color = initial;
                new_settings.color = color_response.color;
                command_needed = true;
            }
            ColorInputEdit::Cancel { initial } => {
                room.ambient_color = initial;
                command_old_settings.color = initial;
                new_settings.color = initial;
            }
            ColorInputEdit::None if color_commit == InputCommit::Committed => {
                new_settings.color = color_response.color;
                command_needed = true;
            }
            ColorInputEdit::None => {}
        }
        y += FIELD_HEIGHT + FIELD_GAP;

        draw_label(ctx, "Exposure:", rect, y);
        let exposure_value = normalized_slider_value(room.exposure, 0.0, EXPOSURE_MAX, 2);
        let (input_rect, slider_rect) = numeric_slider_rects(rect, y);
        let (exposure, exposure_commit) = NumberInput::new(
            self.exposure_id,
            input_rect,
            exposure_value,
        )
        .min(0.0)
        .max(EXPOSURE_MAX)
        .show(ctx);
        let (exposure_slider, exposure_slider_state) = Slider::new(
            self.exposure_slider_id,
            slider_rect,
            0.0,
            EXPOSURE_MAX,
            exposure_value,
        )
        .show(ctx);
        if exposure_commit == InputCommit::Committed {
            new_settings.exposure = normalized_slider_value(exposure, 0.0, EXPOSURE_MAX, 2);
            command_needed = true;
        }
        match exposure_slider_state {
            SliderState::Previewing => {
                let preview = normalized_slider_value(exposure_slider, 0.0, EXPOSURE_MAX, 2);
                room.exposure = preview;
                new_settings.exposure = preview;
            }
            SliderState::Committed { initial_value } => {
                command_old_settings.exposure =
                    normalized_slider_value(initial_value, 0.0, EXPOSURE_MAX, 2);
                new_settings.exposure =
                    normalized_slider_value(exposure_slider, 0.0, EXPOSURE_MAX, 2);
                command_needed = true;
            }
            SliderState::Unchanged => {}
        }

        if command_needed && new_settings != command_old_settings {
            push_command(Box::new(SetRoomAmbientCmd::new(
                world_id,
                room.id,
                command_old_settings,
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
