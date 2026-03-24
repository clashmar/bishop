// editor/src/menu_editor/menu_properties_panel/element_properties.rs
use crate::menu::MenuEditor;
use super::{ROW_HEIGHT, LABEL_WIDTH, FIELD_HEIGHT, common_properties::row_visible};
use engine_core::prelude::*;
use bishop::prelude::*;

impl MenuEditor {
    pub(super) fn draw_label_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (current_text_key, current_font_size, current_alignment) = {
            let Some(element) = self.selected_element() else { return };
            let MenuElementKind::Label(label) = &element.kind else { return };
            (label.text_key.clone(), label.font_size, label.alignment)
        };

        // Text key field
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Text Key:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);

            let (new_text_key, _) = TextInput::new(
                self.properties_panel.widget_ids.text_id,
                field_rect,
                &current_text_key
            )
            .blocked(blocked)
            .show(ctx);

            if new_text_key != current_text_key {
                self.push_element_update(|el| {
                    if let MenuElementKind::Label(label) = &mut el.kind {
                        label.text_key = new_text_key;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Font size
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Font Size:", x, *y + 16.0, 12.0, Color::WHITE);

            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);

            let new_font_size = NumberInput::new(
                self.properties_panel.widget_ids.font_size_id,
                field_rect,
                current_font_size
            )
            .blocked(blocked)
            .min(8.0)
            .max(72.0)
            .show(ctx);

            if (new_font_size - current_font_size).abs() > 0.01 {
                self.push_element_update(|el| {
                    if let MenuElementKind::Label(label) = &mut el.kind {
                        label.font_size = new_font_size;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Horizontal alignment
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Align:", x, *y + 16.0, 12.0, Color::WHITE);
            let h_options = ["Left", "Center", "Right"];
            let current_h = match current_alignment {
                HorizontalAlign::Left => "Left",
                HorizontalAlign::Center => "Center",
                HorizontalAlign::Right => "Right",
            };
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.label_h_align_id,
                dropdown_rect,
                current_h,
                &h_options,
                |s| s.to_string(),
            )
            .blocked(blocked)
            .fixed_width()
            .show(ctx)
            {
                let new_align = match selected {
                    "Left" => HorizontalAlign::Left,
                    "Center" => HorizontalAlign::Center,
                    "Right" => HorizontalAlign::Right,
                    _ => current_alignment,
                };
                self.push_element_update(|el| {
                    if let MenuElementKind::Label(label) = &mut el.kind {
                        label.alignment = new_align;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;
    }

    pub(super) fn draw_button_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (current_text_key, current_font_size, current_action) = {
            let Some(element) = self.selected_element() else { return };
            let MenuElementKind::Button(button) = &element.kind else { return };
            (
                button.text_key.clone(),
                button.font_size,
                button.action.clone(),
            )
        };

        // Text key field
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Text Key:", x, *y + 16.0, 12.0, Color::WHITE);

            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);

            let (new_text_key, _) = TextInput::new(
                self.properties_panel.widget_ids.text_id,
                field_rect,
                &current_text_key
            )
            .blocked(blocked)
            .show(ctx);

            if new_text_key != current_text_key {
                self.push_element_update(|el| {
                    if let MenuElementKind::Button(button) = &mut el.kind {
                        button.text_key = new_text_key;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Font size
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Font Size:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
            let new_font_size = NumberInput::new(self.properties_panel.widget_ids.font_size_id, field_rect, current_font_size)
            .blocked(blocked)
            .min(8.0)
            .max(72.0)
            .show(ctx);

            if (new_font_size - current_font_size).abs() > 0.01 {
                self.push_element_update(|el| {
                    if let MenuElementKind::Button(button) = &mut el.kind {
                        button.font_size = new_font_size;
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Action dropdown
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Action:", x, *y + 16.0, 12.0, Color::WHITE);
            let action_variants = [
                MenuAction::Resume,
                MenuAction::CloseMenu,
                MenuAction::QuitToMainMenu,
                MenuAction::QuitGame,
                MenuAction::OpenMenu(String::new()),
                MenuAction::Custom(String::new()),
            ];
            let action_options: Vec<&str> = action_variants.iter().map(|a| a.ui_label()).collect();
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.action_id,
                dropdown_rect,
                current_action.ui_label(),
                &action_options,
                |s| s.to_string(),
            )
            .blocked(blocked)
            .fixed_width()
            .show(ctx)
            {
                if let Some(new_action) = action_variants.into_iter()
                    .find(|a| a.ui_label() == selected)
                {
                    self.push_element_update(|el| {
                        if let MenuElementKind::Button(button) = &mut el.kind {
                            button.action = new_action;
                        }
                    });
                }
            }
        }
        *y += ROW_HEIGHT;

        // Action parameter (for OpenMenu/Custom)
        let needs_param = matches!(current_action, MenuAction::OpenMenu(_) | MenuAction::Custom(_));
        if needs_param {
            if row_visible(*y, ROW_HEIGHT, clip) {
                let param_value = match &current_action {
                    MenuAction::OpenMenu(s) | MenuAction::Custom(s) => s.clone(),
                    _ => String::new(),
                };

                ctx.draw_text("Param:", x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
                let (new_param, _) = TextInput::new(self.properties_panel.widget_ids.action_param_id, field_rect, &param_value)
                    .blocked(blocked)
                    .show(ctx);

                if new_param != param_value {
                    self.push_element_update(|el| {
                        if let MenuElementKind::Button(button) = &mut el.kind {
                            button.action = match &button.action {
                                MenuAction::OpenMenu(_) => MenuAction::OpenMenu(new_param),
                                MenuAction::Custom(_) => MenuAction::Custom(new_param),
                                other => other.clone(),
                            };
                        }
                    });
                }
            }
            *y += ROW_HEIGHT;
        }

        // Navigation section (only for top-level buttons, not children of layout groups)
        if self.selected_child_index.is_none() {
            *y += 8.0;
            if row_visible(*y, 20.0, clip) {
                ctx.draw_text("Navigation", x, *y + 14.0, 12.0, Color::GREY);
            }
            *y += 20.0;

            let nav_ids = self.properties_panel.widget_ids.button_nav_ids;

            self.draw_nav_section::<ButtonElement>(
                ctx,
                y,
                x,
                w,
                blocked,
                clip,
                &nav_ids,
            );
        }
    }

    pub(super) fn draw_slider_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (text_key, key, min, max, step, default_value) = {
            let Some(element) = self.selected_element() else { return };
            let MenuElementKind::Slider(slider) = &element.kind else { return };
            (slider.text_key.clone(), slider.key.clone(), slider.min, slider.max, slider.step, slider.default_value)
        };

        // Text key
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Label:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let (new_val, _) = TextInput::new(self.properties_panel.widget_ids.slider_text_id, field_rect, &text_key).blocked(blocked).show(ctx);
            if new_val != text_key {
                self.push_element_update(|el| {
                    if let MenuElementKind::Slider(s) = &mut el.kind { s.text_key = new_val; }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Key
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Key:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let (new_val, _) = TextInput::new(self.properties_panel.widget_ids.slider_key_id, field_rect, &key).blocked(blocked).show(ctx);
            if new_val != key {
                self.push_element_update(|el| {
                    if let MenuElementKind::Slider(s) = &mut el.kind { s.key = new_val; }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Min
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Min:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 80.0, FIELD_HEIGHT);
            let new_val = NumberInput::new(self.properties_panel.widget_ids.slider_min_id, field_rect, min).blocked(blocked).show(ctx);
            if (new_val - min).abs() > f32::EPSILON {
                self.push_element_update(|el| {
                    if let MenuElementKind::Slider(s) = &mut el.kind { s.min = new_val; }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Max
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Max:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 80.0, FIELD_HEIGHT);
            let new_val = NumberInput::new(self.properties_panel.widget_ids.slider_max_id, field_rect, max).blocked(blocked).show(ctx);
            if (new_val - max).abs() > f32::EPSILON {
                self.push_element_update(|el| {
                    if let MenuElementKind::Slider(s) = &mut el.kind { s.max = new_val; }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Step
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Step:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 80.0, FIELD_HEIGHT);
            let new_val = NumberInput::new(self.properties_panel.widget_ids.slider_step_id, field_rect, step).blocked(blocked).min(0.001).show(ctx);
            if (new_val - step).abs() > f32::EPSILON {
                self.push_element_update(|el| {
                    if let MenuElementKind::Slider(s) = &mut el.kind { s.step = new_val; }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Default value
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Default:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 80.0, FIELD_HEIGHT);
            let new_val = NumberInput::new(self.properties_panel.widget_ids.slider_default_id, field_rect, default_value).blocked(blocked).show(ctx);
            if (new_val - default_value).abs() > f32::EPSILON {
                self.push_element_update(|el| {
                    if let MenuElementKind::Slider(s) = &mut el.kind { s.default_value = new_val; }
                });
            }
        }
        *y += ROW_HEIGHT;
    }

    pub(super) fn draw_panel_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (current_color, current_opacity) = {
            let Some(element) = self.selected_element() else { return };
            let MenuElementKind::Panel(panel) = &element.kind else { return };
            let PanelFill::SolidColor(color) = panel.background.fill;
            (color, panel.background.opacity)
        };

        if row_visible(*y, 20.0, clip) {
            ctx.draw_text("Background", x, *y + 14.0, 12.0, Color::GREY);
        }
        *y += 20.0;

        // Color
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Color:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let new_color = ColorInput::new(
                self.properties_panel.widget_ids.panel_color_id,
                field_rect,
                current_color,
            )
            .blocked(blocked)
            .show(ctx);
            if new_color != current_color {
                self.push_element_update(|el| {
                    if let MenuElementKind::Panel(panel) = &mut el.kind {
                        panel.background.fill = PanelFill::SolidColor(new_color);
                    }
                });
            }
        }
        *y += ROW_HEIGHT;

        // Opacity
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Opacity:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let (new_opacity, state) = gui_slider(
                ctx,
                self.properties_panel.widget_ids.panel_opacity_id,
                field_rect,
                0.0,
                1.0,
                current_opacity,
            );
            match state {
                SliderState::Previewing => {
                    self.preview_element_update(|el| {
                        if let MenuElementKind::Panel(panel) = &mut el.kind {
                            panel.background.opacity = new_opacity;
                        }
                    });
                }
                SliderState::Committed { .. } => {
                    self.preview_element_update(|el| {
                        if let MenuElementKind::Panel(panel) = &mut el.kind {
                            panel.background.opacity = new_opacity;
                        }
                    });
                    self.commit_element_update();
                }
                SliderState::Unchanged => {}
            }
        }
        *y += ROW_HEIGHT;
    }
}
