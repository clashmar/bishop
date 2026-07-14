pub mod room_audio_source_module;
pub mod room_name_module;
pub mod room_script_module;
pub mod room_tags_module;

use bishop::prelude::*;
use engine_core::ui::{measure_text};
use ::widgets::constants::layout;

use crate::commands::room::EditRoomTagsCmd;
use crate::editor_global::push_command;
use super::collapsible::CollapsiblePropertyModule;
use super::PropertyModule;
use crate::gui::gui_constants::{self, BTN_HEIGHT};
use crate::gui::menu_bar::menu_button;
use crate::shared::scene_ui::inspector::InspectorContent;
use crate::shared::scene_ui::inspector::{
    CreateRequest, InspectorContext, InspectorOutput,
};
use engine_core::game::GameCtxMut;
use engine_core::worlds::room::Room;

/// Editable properties for the current room.
pub struct RoomProperties {
    pub modules: Vec<Box<dyn PropertyModule<Room>>>,
}

impl RoomProperties {
    /// Creates a new room properties pane.
    pub fn new() -> Self {
        Self {
            modules: vec![
                Box::new(CollapsiblePropertyModule::new(room_name_module::RoomNameModule::new())),
                Box::new(CollapsiblePropertyModule::new(room_script_module::RoomScriptModule::new())),
                Box::new(CollapsiblePropertyModule::new(room_audio_source_module::RoomAudioSourceModule::new())),
                Box::new(CollapsiblePropertyModule::new(room_tags_module::RoomTagsModule::for_room())),
            ],
        }
    }
}

impl InspectorContent for RoomProperties {
    fn header_height(&self) -> f32 {
        gui_constants::inspector::HEADER_HEIGHT
    }

    fn draw_header(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        blocked: bool,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        let mut output = InspectorOutput::default();

        let create_label = "+Entity";
        let cam_label = "+Cam";

        let txt_create = measure_text(ctx, create_label, layout::HEADER_FONT_SIZE_20);
        let txt_cam = measure_text(ctx, cam_label, layout::HEADER_FONT_SIZE_20);
        let create_btn_w = txt_create.width + layout::WIDGET_PADDING * 2.0;
        let cam_btn_w = txt_cam.width + layout::WIDGET_PADDING * 2.0;

        const BTN_MARGIN: f32 = 10.0;
        let create_btn = Rect::new(
            rect.x + rect.w - create_btn_w - BTN_MARGIN,
            rect.y + gui_constants::inspector::HEADER_BUTTON_Y,
            create_btn_w,
            BTN_HEIGHT,
        );

        let cam_btn = Rect::new(
            create_btn.x - layout::WIDGET_SPACING - cam_btn_w,
            create_btn.y,
            cam_btn_w,
            30.0,
        );

        if menu_button(ctx, cam_btn, cam_label, false, blocked) {
            if let Some(world) = game_ctx.world.as_deref() {
                output.create_camera_request = Some(world.grid_size);
            }
        }

        if menu_button(ctx, create_btn, create_label, false, blocked) {
            output.create_request = Some(CreateRequest { parent: None });
        }

        output
    }

    fn draw_modules(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _blocked: bool,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> InspectorOutput {
        let Some((world_id, room)) = game_ctx
            .world
            .as_deref()
            .and_then(|world| world.current_room().map(|r| (world.id, r.clone())))
        else {
            return InspectorOutput::default();
        };

        let original_tags = room.tags.clone();
        let mut edited_room = room;
        let mut y = rect.y + 10.0;
        for module in &mut self.modules {
            if module.visible(&edited_room, game_ctx) {
                let h = module.height();
                let sub_rect = Rect::new(rect.x + 10.0, y, rect.w - 20.0, h);
                module.draw(ctx, sub_rect, &mut edited_room, game_ctx, _insp_ctx);
                y += h + layout::WIDGET_SPACING;
            }
        }

        let tags_changed = original_tags != edited_room.tags;

        if let Some(room) = game_ctx
            .world
            .as_deref_mut()
            .and_then(|world| world.current_room_mut())
        {
            room.name = edited_room.name.clone();
        }

        if tags_changed {
            let room_id = edited_room.id;
            push_command(Box::new(EditRoomTagsCmd::new(
                world_id,
                room_id,
                original_tags,
                edited_room.tags,
            )));
        }

        InspectorOutput {
            refresh_event_tags: tags_changed,
            ..InspectorOutput::default()
        }
    }

    fn total_content_height(
        &self,
        game_ctx: &mut GameCtxMut,
        _insp_ctx: &InspectorContext,
    ) -> f32 {
        let Some(world) = game_ctx.world.as_deref() else {
            return 0.0;
        };
        let Some(room) = world.current_room() else {
            return 0.0;
        };

        let mut h = 0.0;
        for module in &self.modules {
            if module.visible(room, game_ctx) {
                h += module.height() + layout::WIDGET_SPACING;
            }
        }
        if h > 0.0 {
            h -= layout::WIDGET_SPACING;
        }
        h + 20.0
    }
}
