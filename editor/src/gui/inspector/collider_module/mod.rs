use bishop::prelude::*;
use engine_core::ecs::inspector::factory::ModuleFactoryEntry;
use engine_core::ecs::inspector::layout::InspectorBodyLayout;
use engine_core::ecs::inspector::module::CollapsibleComponentModule;
use engine_core::ecs::{Collider, ColliderShape, CurrentFrame, Ecs, Entity, InspectorModule, Sprite};
use engine_core::game::GameCtxMut;
use engine_core::physics::collider_system;
use engine_core::ui::measure_text;
use strum::IntoEnumIterator;
use widgets::constants::colors;
use widgets::constants::layout as layout_constants;
use widgets::{Button, Color, Dropdown, NumberInput, Rect, Widget, WidgetId};

use crate::editor_assets::assets::{move_icon, refresh_icon};
use crate::gui::inspector::interactable_module::edit::clear_interactable_edit;

pub mod edit;

#[cfg(test)]
mod tests;

const ROW_H: f32 = 30.0;
const LABEL_Y_OFFSET: f32 = 20.0;
const COLON_GAP: f32 = 8.0;
const NUM_FIELD_W: f32 = 64.0;
const COLUMN_GAP: f32 = 8.0;
const ROW_LABEL_INPUT_GAP: f32 = 10.0;
const EDIT_BTN_SIZE: f32 = 24.0;
const EDIT_BTN_GAP: f32 = 8.0;
const RESET_BTN_GAP: f32 = 4.0;

/// Inspector module for the `Collider` component.
pub struct ColliderModule {
    shape_dropdown_id: WidgetId,
    width_id: WidgetId,
    height_id: WidgetId,
    radius_id: WidgetId,
    capsule_height_id: WidgetId,
    offset_x_id: WidgetId,
    offset_y_id: WidgetId,
    shape_options: Vec<ColliderShape>,
}

impl Default for ColliderModule {
    fn default() -> Self {
        Self {
            shape_dropdown_id: WidgetId::default(),
            width_id: WidgetId::default(),
            height_id: WidgetId::default(),
            radius_id: WidgetId::default(),
            capsule_height_id: WidgetId::default(),
            offset_x_id: WidgetId::default(),
            offset_y_id: WidgetId::default(),
            shape_options: ColliderShape::iter().collect(),
        }
    }
}

impl InspectorModule for ColliderModule {
    fn undo_component_type(&self) -> Option<&'static str> {
        Some(Collider::TYPE_NAME)
    }

    fn visible(&self, ecs: &Ecs, entity: Entity) -> bool {
        ecs.get::<Collider>(entity).is_some()
    }

    fn removable(&self) -> bool {
        true
    }

    fn remove(&mut self, game_ctx: &mut GameCtxMut, entity: Entity) {
        clear_interactable_edit(entity);
        edit::clear_collider_edit(entity);
        Ecs::remove_component::<Collider>(game_ctx, entity);
    }

    fn body_layout(&self) -> InspectorBodyLayout {
        body_layout()
    }

    fn draw(
        &mut self,
        ctx: &mut WgpuContext,
        blocked: bool,
        rect: Rect,
        game_ctx: &mut GameCtxMut,
        entity: Entity,
    ) {
        let edit_mode_active = edit::is_collider_edit_active_for(entity);
        let mut y = rect.y + layout_constants::WIDGET_SPACING;
        let full_w = rect.w - 2.0 * layout_constants::WIDGET_PADDING;

        let shape_label_w =
            measure_text(ctx, "Shape:", layout_constants::FIELD_TEXT_SIZE_16).width + COLON_GAP;
        let edit_btn_x = rect.x + full_w - EDIT_BTN_SIZE * 2.0 - RESET_BTN_GAP + layout_constants::WIDGET_PADDING;
        let reset_btn_x = edit_btn_x + EDIT_BTN_SIZE + RESET_BTN_GAP;
        let dropdown_w = edit_btn_x
            - rect.x
            - shape_label_w
            - layout_constants::WIDGET_PADDING
            - EDIT_BTN_GAP;

        let reset_btn_rect = Rect::new(
            reset_btn_x,
            y + (ROW_H - EDIT_BTN_SIZE) / 2.0,
            EDIT_BTN_SIZE,
            EDIT_BTN_SIZE,
        );
        let reset_clicked = Button::icon(reset_btn_rect, refresh_icon(), "Reset Collider")
            .suppressed(blocked)
            .show(ctx);
        if reset_clicked {
            let default_collider = {
                let current_frame_store = game_ctx.ecs.get_store::<CurrentFrame>();
                if let Some(col) = collider_system::collider_from_animation_component(
                    current_frame_store,
                    entity,
                    game_ctx.sprite_manager,
                ) {
                    col
                } else if let Some(sprite) = game_ctx.ecs.get_store::<Sprite>().get(entity) {
                    collider_system::collider_from_sprite(
                        game_ctx.sprite_manager,
                        sprite.sprite,
                    )
                    .unwrap_or_default()
                } else {
                    Collider::default()
                }
            };
            if let Some(collider) = game_ctx.ecs.get_mut::<Collider>(entity) {
                reset_collider_to_default(collider, &default_collider);
            }
            return;
        }

        let collider = match game_ctx.ecs.get_mut::<Collider>(entity) {
            Some(collider) => collider,
            None => return,
        };
        let current_shape_label = collider.shape.ui_label();

        ctx.draw_text(
            "Shape:",
            rect.x + layout_constants::WIDGET_PADDING,
            y + LABEL_Y_OFFSET,
            layout_constants::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );

        let dropdown_rect = Rect::new(
            rect.x + shape_label_w + layout_constants::WIDGET_PADDING,
            y,
            dropdown_w,
            ROW_H,
        );
        if let Some(selected) = Dropdown::new(
            self.shape_dropdown_id,
            dropdown_rect,
            current_shape_label,
            self.shape_options.as_slice(),
            |shape| shape.ui_label().to_string(),
        )
        .suppressed(blocked)
        .show(ctx)
        {
            collider.shape = collider.shape.convert_to(selected);
        }

        let edit_btn_rect = Rect::new(
            edit_btn_x,
            y + (ROW_H - EDIT_BTN_SIZE) / 2.0,
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
        if Button::icon(edit_btn_rect, move_icon(), "Edit Collider")
            .suppressed(blocked)
            .show(ctx)
        {
            edit::toggle_collider_edit(entity);
        }

        y += ROW_H + layout_constants::WIDGET_SPACING;

        match &mut collider.shape {
            ColliderShape::Aabb { width, height } => {
                draw_pair_labels(ctx, "Width:", "Height:", y, rect);
                let (width_rect, height_rect) = pair_input_rects(y, rect);
                let (new_w, _) = NumberInput::new(self.width_id, width_rect, *width)
                    .blocked(blocked)
                    .show(ctx);
                let (new_h, _) = NumberInput::new(self.height_id, height_rect, *height)
                    .blocked(blocked)
                    .show(ctx);
                *width = new_w;
                *height = new_h;
                y += ROW_H + layout_constants::WIDGET_SPACING;
            }
            ColliderShape::Circle { radius } => {
                draw_single_label(ctx, "Radius:", y, rect);
                let input_rect = single_input_rect(ctx, "Radius:", y, rect);
                let (new_r, _) = NumberInput::new(self.radius_id, input_rect, *radius)
                    .blocked(blocked)
                    .show(ctx);
                *radius = new_r;
                y += ROW_H + layout_constants::WIDGET_SPACING;
            }
            ColliderShape::Capsule { radius, height } => {
                draw_pair_labels(ctx, "Radius:", "Height:", y, rect);
                let (radius_rect, height_rect) = pair_input_rects(y, rect);
                let (new_r, _) = NumberInput::new(self.radius_id, radius_rect, *radius)
                    .blocked(blocked)
                    .show(ctx);
                let (new_h, _) = NumberInput::new(self.capsule_height_id, height_rect, *height)
                    .blocked(blocked)
                    .show(ctx);
                *radius = new_r;
                *height = new_h;
                y += ROW_H + layout_constants::WIDGET_SPACING;
            }
            ColliderShape::Point => {}
        }

        let axis_layout = axis_row_layout(ctx, "Offset X:", "Y:", y, rect);
        ctx.draw_text(
            "Offset X:",
            rect.x + layout_constants::WIDGET_PADDING,
            y + LABEL_Y_OFFSET,
            layout_constants::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        ctx.draw_text(
            "Y:",
            axis_layout.label_b_x,
            y + LABEL_Y_OFFSET,
            layout_constants::FIELD_TEXT_SIZE_16,
            colors::DEFAULT_TEXT_COLOR,
        );
        let (new_ox, _) = NumberInput::new(self.offset_x_id, axis_layout.input_a, collider.offset.x)
            .blocked(blocked)
            .show(ctx);
        let (new_oy, _) = NumberInput::new(self.offset_y_id, axis_layout.input_b, collider.offset.y)
            .blocked(blocked)
            .show(ctx);
        collider.offset.x = new_ox;
        collider.offset.y = new_oy;
    }
}

/// Resets a collider to default dimensions while preserving the current shape variant.
pub fn reset_collider_to_default(collider: &mut Collider, default_collider: &Collider) {
    let current_shape = collider.shape;
    collider.shape = default_collider.shape.convert_to(current_shape);
    collider.offset = default_collider.offset;
}

fn body_layout() -> InspectorBodyLayout {
    InspectorBodyLayout::new()
        .top_padding(layout_constants::WIDGET_SPACING)
        .rows(3, layout_constants::WIDGET_SPACING)
}

fn single_input_rect(
    ctx: &WgpuContext,
    label: &str,
    y: f32,
    rect: Rect,
) -> Rect {
    let label_w = measure_text(ctx, label, layout_constants::FIELD_TEXT_SIZE_16).width + COLON_GAP;
    let x = rect.x + layout_constants::WIDGET_PADDING + label_w + ROW_LABEL_INPUT_GAP;
    let width = rect.x + rect.w - layout_constants::WIDGET_PADDING - x;
    Rect::new(x, y, width.max(NUM_FIELD_W), ROW_H)
}

fn pair_input_rects(y: f32, rect: Rect) -> (Rect, Rect) {
    let column_width =
        (rect.w - 2.0 * layout_constants::WIDGET_PADDING - COLUMN_GAP) / 2.0;
    let first_x = rect.x + layout_constants::WIDGET_PADDING + column_width - NUM_FIELD_W;
    let second_base_x = rect.x + layout_constants::WIDGET_PADDING + column_width + COLUMN_GAP;
    let second_x = second_base_x + column_width - NUM_FIELD_W;

    (
        Rect::new(first_x, y, NUM_FIELD_W, ROW_H),
        Rect::new(second_x, y, NUM_FIELD_W, ROW_H),
    )
}

struct AxisRowLayout {
    label_b_x: f32,
    input_a: Rect,
    input_b: Rect,
}

fn axis_row_layout(
    ctx: &WgpuContext,
    label_a: &str,
    label_b: &str,
    y: f32,
    rect: Rect,
) -> AxisRowLayout {
    let label_a_w =
        measure_text(ctx, label_a, layout_constants::FIELD_TEXT_SIZE_16).width + COLON_GAP;
    let label_b_w =
        measure_text(ctx, label_b, layout_constants::FIELD_TEXT_SIZE_16).width + COLON_GAP;
    let input_a_x = rect.x + layout_constants::WIDGET_PADDING + label_a_w + ROW_LABEL_INPUT_GAP;
    let input_a = Rect::new(input_a_x, y, NUM_FIELD_W, ROW_H);
    let label_b_x = input_a.x + input_a.w + COLUMN_GAP;
    let input_b_x = label_b_x + label_b_w + ROW_LABEL_INPUT_GAP;
    let input_b_w = rect.x + rect.w - layout_constants::WIDGET_PADDING - input_b_x;

    AxisRowLayout {
        label_b_x,
        input_a,
        input_b: Rect::new(input_b_x, y, input_b_w.max(NUM_FIELD_W), ROW_H),
    }
}

fn draw_single_label(ctx: &mut WgpuContext, label: &str, y: f32, rect: Rect) {
    ctx.draw_text(
        label,
        rect.x + layout_constants::WIDGET_PADDING,
        y + LABEL_Y_OFFSET,
        layout_constants::FIELD_TEXT_SIZE_16,
        colors::DEFAULT_TEXT_COLOR,
    );
}

fn draw_pair_labels(
    ctx: &mut WgpuContext,
    label_a: &str,
    label_b: &str,
    y: f32,
    rect: Rect,
) {
    ctx.draw_text(
        label_a,
        rect.x + layout_constants::WIDGET_PADDING,
        y + LABEL_Y_OFFSET,
        layout_constants::FIELD_TEXT_SIZE_16,
        colors::DEFAULT_TEXT_COLOR,
    );
    ctx.draw_text(
        label_b,
        rect.x + rect.w / 2.0 + COLUMN_GAP / 2.0,
        y + LABEL_Y_OFFSET,
        layout_constants::FIELD_TEXT_SIZE_16,
        colors::DEFAULT_TEXT_COLOR,
    );
}

inventory::submit! {
    ModuleFactoryEntry {
        type_name: <Collider>::TYPE_NAME,
        title: "Collider",
        factory: || Box::new(
            CollapsibleComponentModule::new(
                ColliderModule::default()
            ).with_title("Collider")
        ),
        allowed_for: None,
    }
}
