use crate::menu::menu_canvas::drawing::MenuCanvasFrame;
use crate::menu::menu_properties_panel::common_properties::row_visible;
use crate::menu::menu_properties_panel::{FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

impl MenuEditor {
    pub(crate) fn draw_slider(
        &self,
        frame: &mut MenuCanvasFrame<'_>,
        element: &MenuElement,
        element_rect: Rect,
        is_selected: bool,
    ) {
        let MenuElementKind::Slider(slider) = &element.kind else {
            return;
        };
        Slider::new(
            slider.widget_id,
            element_rect,
            slider.min,
            slider.max,
            slider.default_value,
        )
        .label(&slider.text_key)
        .blocked(true)
        .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
        .show(frame.ctx);
        if !frame.preview {
            let outline_color = if is_selected {
                with_theme(|theme| theme.highlight)
            } else {
                Color::new(0., 0., 0., 0.)
            };
            let thickness = if is_selected { 2.0 } else { 1.0 };
            frame.ctx.draw_rectangle_lines(
                element_rect.x,
                element_rect.y,
                element_rect.w,
                element_rect.h,
                thickness,
                outline_color,
            );
        }
    }

    pub(crate) fn draw_slider_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (text_key, key, min, max, step, default_value) = {
            let Some(element) = self.selected_element() else {
                return;
            };
            let MenuElementKind::Slider(slider) = &element.kind else {
                return;
            };
            (
                slider.text_key.clone(),
                slider.key.clone(),
                slider.min,
                slider.max,
                slider.step,
                slider.default_value,
            )
        };

        // Text key
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Label:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let (new_val, commit) = TextInput::new(
                self.properties_panel.widget_ids.slider_text_id,
                field_rect,
                &text_key,
            )
            .blocked(blocked)
            .show(ctx);
            self.push_input_update(commit, |el| {
                if let MenuElementKind::Slider(s) = &mut el.kind {
                    s.text_key = new_val;
                }
            });
        }
        *y += ROW_HEIGHT;

        // Key
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Key:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let (new_val, commit) = TextInput::new(
                self.properties_panel.widget_ids.slider_key_id,
                field_rect,
                &key,
            )
            .blocked(blocked)
            .show(ctx);
            self.push_input_update(commit, |el| {
                if let MenuElementKind::Slider(s) = &mut el.kind {
                    s.key = new_val;
                }
            });
        }
        *y += ROW_HEIGHT;

        // Min
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Min:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 80.0, FIELD_HEIGHT);
            let (new_val, commit) = NumberInput::new(
                self.properties_panel.widget_ids.slider_min_id,
                field_rect,
                min,
            )
            .blocked(blocked)
            .show(ctx);
            self.push_input_update(commit, |el| {
                if let MenuElementKind::Slider(s) = &mut el.kind {
                    s.min = new_val;
                }
            });
        }
        *y += ROW_HEIGHT;

        // Max
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Max:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 80.0, FIELD_HEIGHT);
            let (new_val, commit) = NumberInput::new(
                self.properties_panel.widget_ids.slider_max_id,
                field_rect,
                max,
            )
            .blocked(blocked)
            .show(ctx);
            self.push_input_update(commit, |el| {
                if let MenuElementKind::Slider(s) = &mut el.kind {
                    s.max = new_val;
                }
            });
        }
        *y += ROW_HEIGHT;

        // Step
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Step:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 80.0, FIELD_HEIGHT);
            let (new_val, commit) = NumberInput::new(
                self.properties_panel.widget_ids.slider_step_id,
                field_rect,
                step,
            )
            .blocked(blocked)
            .min(0.001)
            .show(ctx);
            self.push_input_update(commit, |el| {
                if let MenuElementKind::Slider(s) = &mut el.kind {
                    s.step = new_val;
                }
            });
        }
        *y += ROW_HEIGHT;

        // Default value
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Default:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 80.0, FIELD_HEIGHT);
            let (new_val, commit) = NumberInput::new(
                self.properties_panel.widget_ids.slider_default_id,
                field_rect,
                default_value,
            )
            .blocked(blocked)
            .show(ctx);
            self.push_input_update(commit, |el| {
                if let MenuElementKind::Slider(s) = &mut el.kind {
                    s.default_value = new_val;
                }
            });
        }
        *y += ROW_HEIGHT;
    }
}
