use crate::menu::menu_canvas::drawing::MenuCanvasFrame;
use crate::menu::menu_properties_panel::common_properties::row_visible;
use crate::menu::menu_properties_panel::nav_section::NavSectionStyle;
use crate::menu::menu_properties_panel::{FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::menu::MenuEditor;
use crate::shared::input::canvas_blocked_by_global_ui;
use bishop::prelude::*;
use engine_core::prelude::*;

impl MenuEditor {
    pub(crate) fn draw_button(
        &self,
        frame: &mut MenuCanvasFrame<'_>,
        element: &MenuElement,
        element_rect: Rect,
        is_selected: bool,
    ) {
        let MenuElementKind::Button(button) = &element.kind else {
            return;
        };
        let display_text = button.text_key.to_string();
        Button::new(element_rect, &display_text)
            .font_size(button.font_size)
            .mouse_position(frame.world_mouse)
            .suppressed(canvas_blocked_by_global_ui(frame.ctx))
            .apply_selectors(element.class.as_deref(), element.style_id.as_deref())
            .show(frame.ctx);
        if is_selected {
            frame.ctx.draw_rectangle_lines(
                element_rect.x,
                element_rect.y,
                element_rect.w,
                element_rect.h,
                2.0,
                with_theme(|theme| theme.highlight),
            );
        }
    }

    pub(crate) fn draw_button_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let (current_text_key, current_font_size, current_action) = {
            let Some(element) = self.selected_element() else {
                return;
            };
            let MenuElementKind::Button(button) = &element.kind else {
                return;
            };
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

            let (new_text_key, commit) = TextInput::new(
                self.properties_panel.widget_ids.text_id,
                field_rect,
                &current_text_key,
            )
            .blocked(blocked)
            .show(ctx);

            self.push_input_update(commit, |el| {
                if let MenuElementKind::Button(button) = &mut el.kind {
                    button.text_key = new_text_key;
                }
            });
        }
        *y += ROW_HEIGHT;

        // Font size
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Font Size:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, 60.0, FIELD_HEIGHT);
            let (new_font_size, commit) = NumberInput::new(
                self.properties_panel.widget_ids.font_size_id,
                field_rect,
                current_font_size,
            )
            .blocked(blocked)
            .min(8.0)
            .max(72.0)
            .show(ctx);

            self.push_input_update(commit, |el| {
                if let MenuElementKind::Button(button) = &mut el.kind {
                    button.font_size = new_font_size;
                }
            });
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
            .suppressed(blocked)
            .fixed_width()
            .show(ctx)
            {
                if let Some(new_action) = action_variants
                    .into_iter()
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
        let needs_param = matches!(
            current_action,
            MenuAction::OpenMenu(_) | MenuAction::Custom(_)
        );
        if needs_param {
            if row_visible(*y, ROW_HEIGHT, clip) {
                let param_value = match &current_action {
                    MenuAction::OpenMenu(s) | MenuAction::Custom(s) => s.clone(),
                    _ => String::new(),
                };

                ctx.draw_text("Param:", x, *y + 16.0, 12.0, Color::WHITE);
                let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
                let (new_param, commit) = TextInput::new(
                    self.properties_panel.widget_ids.action_param_id,
                    field_rect,
                    &param_value,
                )
                .blocked(blocked)
                .show(ctx);

                self.push_input_update(commit, |el| {
                    if let MenuElementKind::Button(button) = &mut el.kind {
                        button.action = match &button.action {
                            MenuAction::OpenMenu(_) => MenuAction::OpenMenu(new_param),
                            MenuAction::Custom(_) => MenuAction::Custom(new_param),
                            other => other.clone(),
                        };
                    }
                });
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
                NavSectionStyle {
                    x,
                    w,
                    blocked,
                    clip,
                },
                &nav_ids,
            );
        }
    }
}
