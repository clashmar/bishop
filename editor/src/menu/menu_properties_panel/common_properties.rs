// editor/src/menu_editor/menu_properties_panel/common_properties.rs
use super::{FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

impl MenuEditor {
    pub(super) fn draw_common_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        _w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (current_name, rect_val, z_order, type_label) = {
            let Some(element) = self.selected_element() else {
                return;
            };
            let type_label = match &element.kind {
                MenuElementKind::Label(_) => "Label",
                MenuElementKind::Button(_) => "Button",
                MenuElementKind::Panel(_) => "Panel",
                MenuElementKind::LayoutGroup(_) => "Layout Group",
                MenuElementKind::Slider(_) => "Slider",
            };
            (
                element.name.clone(),
                element.rect,
                element.z_order,
                type_label,
            )
        };
        let child_is_managed = self.is_selected_child_managed();

        // Type (read-only)
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Type:", x, *y + 16.0, 12.0, Color::WHITE);
            ctx.draw_text(
                type_label,
                x + LABEL_WIDTH,
                *y + 16.0,
                12.0,
                Color::new(0.7, 0.7, 0.7, 1.0),
            );
        }
        *y += ROW_HEIGHT;

        // Name field
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Name:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, _w - LABEL_WIDTH, FIELD_HEIGHT);
            let (new_name, _) = TextInput::new(
                self.properties_panel.widget_ids.name_id,
                field_rect,
                &current_name,
            )
            .blocked(blocked)
            .show(ctx);
            if new_name != current_name {
                self.push_element_update(|el| el.name = new_name);
            }
        }
        *y += ROW_HEIGHT;

        // Z Order (only for top-level elements, children inherit from parent)
        if self.selected_child_index.is_none() {
            if row_visible(*y, ROW_HEIGHT, clip) {
                ctx.draw_text("Z Order:", x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
                let new_z = NumberInput::new(
                    self.properties_panel.widget_ids.z_order_id,
                    field_rect,
                    z_order as f32,
                )
                .blocked(blocked)
                .show(ctx);
                let new_z = new_z as i32;
                if new_z != z_order {
                    self.push_element_update(|el| el.z_order = new_z);
                }
            }
            *y += ROW_HEIGHT;
        }

        if !child_is_managed {
            if row_visible(*y, 20.0, clip) {
                ctx.draw_text("Position (normalized)", x, *y + 14.0, 12.0, Color::GREY);
            }
            *y += 20.0;

            if row_visible(*y, ROW_HEIGHT, clip) {
                // Position X
                ctx.draw_text("X:", x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + 24.0, *y, 60.0, FIELD_HEIGHT);
                let new_x = NumberInput::new(
                    self.properties_panel.widget_ids.pos_x_id,
                    field_rect,
                    rect_val.x,
                )
                .blocked(blocked)
                .show(ctx);
                let px_x = format!("{}px", (new_x * DESIGN_RESOLUTION_WIDTH) as i32);
                ctx.draw_text(&px_x, x + 88.0, *y + 16.0, 10.0, Color::GREY);

                // Position Y
                ctx.draw_text("Y:", x + 130.0, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + 154.0, *y, 60.0, FIELD_HEIGHT);
                let new_y = NumberInput::new(
                    self.properties_panel.widget_ids.pos_y_id,
                    field_rect,
                    rect_val.y,
                )
                .blocked(blocked)
                .show(ctx);

                if (new_x - rect_val.x).abs() > 0.001 || (new_y - rect_val.y).abs() > 0.001 {
                    self.push_element_update(|el| {
                        el.rect.x = new_x;
                        el.rect.y = new_y;
                    });
                }
            }
            *y += ROW_HEIGHT;

            if row_visible(*y, ROW_HEIGHT, clip) {
                // Size W
                ctx.draw_text("W:", x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + 24.0, *y, 60.0, FIELD_HEIGHT);
                let new_w = NumberInput::new(
                    self.properties_panel.widget_ids.size_w_id,
                    field_rect,
                    rect_val.w,
                )
                .blocked(blocked)
                .min(0.005)
                .show(ctx);
                let px_w = format!("{}px", (new_w * DESIGN_RESOLUTION_WIDTH) as i32);
                ctx.draw_text(&px_w, x + 88.0, *y + 16.0, 10.0, Color::GREY);

                // Size H
                ctx.draw_text("H:", x + 130.0, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + 154.0, *y, 60.0, FIELD_HEIGHT);
                let new_h = NumberInput::new(
                    self.properties_panel.widget_ids.size_h_id,
                    field_rect,
                    rect_val.h,
                )
                .blocked(blocked)
                .min(0.005)
                .show(ctx);

                if (new_w - rect_val.w).abs() > 0.001 || (new_h - rect_val.h).abs() > 0.001 {
                    self.push_element_update(|el| {
                        el.rect.w = new_w;
                        el.rect.h = new_h;
                    });
                }
            }
            *y += ROW_HEIGHT + 8.0;
        } else {
            if row_visible(*y, 20.0, clip) {
                ctx.draw_text(
                    "Position/size managed by layout",
                    x,
                    *y + 14.0,
                    10.0,
                    Color::new(0.5, 0.5, 0.5, 1.0),
                );
            }
            *y += 20.0;
        }
    }
}

/// Returns true if a row is fully visible within the clip rect.
pub(super) fn row_visible(y: f32, h: f32, clip: &Rect) -> bool {
    y >= clip.y && y + h <= clip.y + clip.h
}
