pub mod room_name_module;
pub mod room_tags_module;

use bishop::prelude::*;
use engine_core::prelude::*;
use widgets::constants::layout;

/// Draw a label + text input row. Returns the committed value.
pub(crate) fn draw_labeled_text_input(
    ctx: &mut WgpuContext,
    rect: Rect,
    label: &str,
    value: &str,
    widget_id: WidgetId,
) -> (String, InputCommit) {
    let label_measure = measure_text(ctx, label, layout::DEFAULT_FONT_SIZE_16);
    ctx.draw_text(
        label,
        rect.x,
        rect.y + 20.0,
        layout::DEFAULT_FONT_SIZE_16,
        Color::WHITE,
    );
    let input_rect = Rect::new(
        rect.x + label_measure.width + layout::WIDGET_SPACING,
        rect.y,
        rect.w - label_measure.width - layout::WIDGET_SPACING,
        layout::DEFAULT_FIELD_HEIGHT,
    );
    TextInput::new(widget_id, input_rect, value).show(ctx)
}

use super::collapsible::CollapsiblePropertyModule;
use super::PropertyModule;
use crate::gui::gui_constants::{BTN_HEIGHT, INSPECTOR_HEADER_BUTTON_Y, INSPECTOR_HEADER_HEIGHT};
use crate::gui::menu_bar::menu_button;
use crate::shared::scene_ui::inspector::InspectorContent;
use crate::shared::scene_ui::inspector::{
    SceneCreateRequest, SceneInspectorContext, SceneInspectorOutput,
};
use engine_core::game::GameCtxMut;
use engine_core::worlds::room::Room;

pub struct RoomProperties {
    pub modules: Vec<Box<dyn PropertyModule<Room>>>,
}

impl RoomProperties {
    pub fn new() -> Self {
        Self {
            modules: vec![
                Box::new(CollapsiblePropertyModule::new(room_name_module::RoomNameModule::new())),
                Box::new(CollapsiblePropertyModule::new(room_tags_module::RoomTagsModule::new())),
            ],
        }
    }
}

impl InspectorContent for RoomProperties {
    fn header_height(&self) -> f32 {
        INSPECTOR_HEADER_HEIGHT
    }

    fn draw_header(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        blocked: bool,
        game_ctx: &mut GameCtxMut,
        _scene_ctx: &SceneInspectorContext,
    ) -> SceneInspectorOutput {
        let mut output = SceneInspectorOutput::default();

        let create_label = "+ Entity";
        let cam_label = "+ Camera";

        let txt_create = measure_text(ctx, create_label, layout::HEADER_FONT_SIZE_20);
        let txt_cam = measure_text(ctx, cam_label, layout::HEADER_FONT_SIZE_20);
        let create_btn_w = txt_create.width + layout::WIDGET_PADDING * 2.0;
        let cam_btn_w = txt_cam.width + layout::WIDGET_PADDING * 2.0;

        const BTN_MARGIN: f32 = 10.0;
        let create_btn = Rect::new(
            rect.x + rect.w - create_btn_w - BTN_MARGIN,
            rect.y + INSPECTOR_HEADER_BUTTON_Y,
            create_btn_w,
            BTN_HEIGHT,
        );

        let cam_btn = Rect::new(
            create_btn.x - layout::WIDGET_SPACING - cam_btn_w,
            create_btn.y,
            cam_btn_w,
            30.0,
        );

        if menu_button(ctx, cam_btn, cam_label, blocked) {
            if let Some(world) = game_ctx.world.as_deref() {
                if let Some(room) = world.current_room() {
                    room.create_room_camera(&mut game_ctx.ecs, room.id, world.grid_size);
                }
            }
        }

        if menu_button(ctx, create_btn, create_label, blocked) {
            output.create_request = Some(SceneCreateRequest { parent: None });
        }

        output
    }

    fn draw_modules(
        &mut self,
        ctx: &mut WgpuContext,
        rect: Rect,
        _blocked: bool,
        game_ctx: &mut GameCtxMut,
        _scene_ctx: &SceneInspectorContext,
    ) -> SceneInspectorOutput {
        let Some(room) = game_ctx
            .world
            .as_deref()
            .and_then(|world| world.current_room())
            .cloned()
        else {
            return SceneInspectorOutput::default();
        };

        let mut edited_room = room;
        let mut y = rect.y + 10.0;
        for module in &mut self.modules {
            if module.visible(&edited_room, game_ctx) {
                let h = module.height();
                let sub_rect = Rect::new(rect.x + 10.0, y, rect.w - 20.0, h);
                module.draw(ctx, sub_rect, &mut edited_room, game_ctx);
                y += h + layout::WIDGET_SPACING;
            }
        }

        if let Some(room) = game_ctx
            .world
            .as_deref_mut()
            .and_then(|world| world.current_room_mut())
        {
            *room = edited_room;
        }

        SceneInspectorOutput::default()
    }

    fn total_content_height(
        &self,
        game_ctx: &mut GameCtxMut,
        _scene_ctx: &SceneInspectorContext,
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

    fn was_input_active(&self) -> bool {
        self.modules.iter().any(|m| m.was_input_active())
    }
}
