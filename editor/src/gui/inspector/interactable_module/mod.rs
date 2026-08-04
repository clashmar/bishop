use bishop::{
    Draw,
    Text,
    prelude::{Rect, Vec2, WgpuContext, vec2},
};
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::inspector::module::{CollapsibleComponentModule, InspectorModule};
use engine_core::ecs::{Ecs, Entity, Interactable, InteractableShape};
use engine_core::game::GameCtxMut;
use engine_core::ui::measure_text;
use widgets::constants::{colors, layout};
use widgets::{Button, Color, Dropdown, InputCommit, NumberInput, Widget, WidgetId};

use crate::editor_assets::assets::move_icon;

pub mod edit;

#[cfg(test)]
mod tests;

const TITLE: &str = "Interactable";
const TOP_PADDING: f32 = layout::WIDGET_SPACING;
const BOTTOM_GUTTER: f32 = 10.0;
const ROW_H: f32 = 30.0;
const GAP: f32 = layout::WIDGET_SPACING;
const LABEL_Y_OFFSET: f32 = 20.0;
const COLON_GAP: f32 = 8.0;
const EDIT_BTN_SIZE: f32 = 24.0;
const EDIT_BTN_GAP: f32 = 8.0;

#[derive(Default)]
pub struct InteractableModule {
    shape_id: WidgetId,
    offset_x_id: WidgetId,
    offset_y_id: WidgetId,
    radius_id: WidgetId,
    rect_w_id: WidgetId,
    rect_h_id: WidgetId,
    input_active: bool,
}

impl InspectorModule for InteractableModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(Interactable::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<Interactable>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        edit::clear_interactable_edit(entity);
        Ecs::remove_component::<Interactable>(game_ctx, entity);
    }

    fn was_input_active(&self) -> bool {
        self.input_active
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        InspectorBodyLayout::new()
            .top_padding(TOP_PADDING)
            .rows(3, GAP)
            .bottom_gutter(BOTTOM_GUTTER)
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        self.input_active = false;
        let Some(interactable) = game_ctx.ecs.get_mut::<Interactable>(entity) else {
            return;
        };

        let edit_mode_active = edit::is_interactable_edit_active_for(entity);
        let mut y = rect.y + TOP_PADDING;
        let half_w = (rect.w - GAP) * 0.5;
        let value_font = layout::FIELD_TEXT_SIZE_16;
        let full_w = rect.w - 2.0 * layout::WIDGET_PADDING;
        let shape_label_w = measure_text(ctx, "Shape:", value_font).width + COLON_GAP;
        let edit_btn_x = rect.x + full_w - EDIT_BTN_SIZE + layout::WIDGET_PADDING;
        let dropdown_w = edit_btn_x
            - rect.x
            - shape_label_w
            - layout::WIDGET_PADDING
            - EDIT_BTN_GAP;

        ctx.draw_text(
            "Shape:",
            rect.x + layout::WIDGET_PADDING,
            y + LABEL_Y_OFFSET,
            value_font,
            colors::DEFAULT_TEXT_COLOR,
        );
        if let Some(shape) = Dropdown::new(
            self.shape_id,
            Rect::new(
                rect.x + shape_label_w + layout::WIDGET_PADDING,
                y,
                dropdown_w,
                ROW_H,
            ),
            interactable.shape().ui_label(),
            &InteractableShape::ALL,
            |shape| shape.ui_label().to_string(),
        )
        .suppressed(blocked)
        .show(ctx)
        {
            interactable.use_rect = shape == InteractableShape::Rect;
        }

        let edit_btn_rect = Rect::new(
            edit_btn_x,
            y + (ROW_H - EDIT_BTN_SIZE) * 0.5,
            EDIT_BTN_SIZE,
            EDIT_BTN_SIZE,
        );
        if edit_mode_active {
            ctx.draw_rectangle(
                edit_btn_rect.x - 2.0,
                edit_btn_rect.y - 2.0,
                edit_btn_rect.w + 4.0,
                edit_btn_rect.h + 4.0,
                Color::new(0.39, 0.78, 1.0, 0.31),
            );
        }
        if Button::icon(edit_btn_rect, move_icon(), "Edit Interactable")
            .suppressed(blocked)
            .show(ctx)
        {
            edit::toggle_interactable_edit(entity);
        }
        y += ROW_H + GAP;

        ctx.draw_text("X", rect.x, y + 20.0, value_font, colors::DEFAULT_TEXT_COLOR);
        ctx.draw_text(
            "Y",
            rect.x + half_w + GAP,
            y + 20.0,
            value_font,
            colors::DEFAULT_TEXT_COLOR,
        );
        let offset_label_w = 18.0;
        let x_rect = Rect::new(rect.x + offset_label_w, y, half_w - offset_label_w, ROW_H);
        let y_rect = Rect::new(
            rect.x + half_w + GAP + offset_label_w,
            y,
            half_w - offset_label_w,
            ROW_H,
        );
        let (offset_x, commit_x) = NumberInput::new(self.offset_x_id, x_rect, interactable.offset.x)
            .blocked(blocked)
            .show(ctx);
        let (offset_y, commit_y) = NumberInput::new(self.offset_y_id, y_rect, interactable.offset.y)
            .blocked(blocked)
            .show(ctx);
        interactable.offset = vec2(offset_x, offset_y);
        self.input_active |= is_input_active(commit_x) || is_input_active(commit_y);
        y += ROW_H + GAP;

        match interactable.shape() {
            InteractableShape::Circle => {
                let label_w = 58.0;
                ctx.draw_text(
                    "Radius",
                    rect.x,
                    y + 20.0,
                    value_font,
                    colors::DEFAULT_TEXT_COLOR,
                );
                let radius_rect = Rect::new(rect.x + label_w, y, rect.w - label_w, ROW_H);
                let (radius, commit) = NumberInput::new(self.radius_id, radius_rect, interactable.radius)
                    .min(1.0)
                    .blocked(blocked)
                    .show(ctx);
                interactable.radius = radius.max(1.0);
                self.input_active |= is_input_active(commit);
            }
            InteractableShape::Rect => {
                ctx.draw_text("W", rect.x, y + 20.0, value_font, colors::DEFAULT_TEXT_COLOR);
                ctx.draw_text(
                    "H",
                    rect.x + half_w + GAP,
                    y + 20.0,
                    value_font,
                    colors::DEFAULT_TEXT_COLOR,
                );
                let size_label_w = 18.0;
                let w_rect = Rect::new(rect.x + size_label_w, y, half_w - size_label_w, ROW_H);
                let h_rect = Rect::new(
                    rect.x + half_w + GAP + size_label_w,
                    y,
                    half_w - size_label_w,
                    ROW_H,
                );
                let (rect_w, commit_w) = NumberInput::new(self.rect_w_id, w_rect, interactable.rect_size.x)
                    .min(1.0)
                    .blocked(blocked)
                    .show(ctx);
                let (rect_h, commit_h) = NumberInput::new(self.rect_h_id, h_rect, interactable.rect_size.y)
                    .min(1.0)
                    .blocked(blocked)
                    .show(ctx);
                interactable.rect_size = Vec2::new(rect_w.max(1.0), rect_h.max(1.0));
                self.input_active |= is_input_active(commit_w) || is_input_active(commit_h);
            }
        }
    }
}

fn is_input_active(commit: InputCommit) -> bool {
    matches!(commit, InputCommit::Previewing | InputCommit::Committed)
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: Interactable::TYPE_NAME,
        title: TITLE,
        factory: || Box::new(
            CollapsibleComponentModule::new(
                crate::gui::inspector::interactable_module::InteractableModule::default()
            ).with_title(TITLE)
        ),
        allowed_for: None,
    }
}
