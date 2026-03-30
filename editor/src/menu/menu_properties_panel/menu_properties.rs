// editor/src/menu_editor/menu_properties_panel/menu_properties.rs
use super::{common_properties::row_visible, FIELD_HEIGHT, LABEL_WIDTH, ROW_HEIGHT};
use crate::commands::menu::{TemplateProperty, UpdateTemplateCmd};
use crate::editor_global::push_command;
use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::prelude::*;

impl MenuEditor {
    pub(super) fn draw_menu_properties(
        &mut self,
        ctx: &mut WgpuContext,
        y: &mut f32,
        x: f32,
        w: f32,
        blocked: bool,
        clip: &Rect,
    ) {
        let Some(template) = self.current_template() else {
            return;
        };
        let current_name = template.id.clone();
        let current_mode = template.mode;
        let current_bg = template.background;

        if row_visible(*y, 20.0, clip) {
            ctx.draw_text("Menu Properties", x, *y + 14.0, 12.0, Color::GREY);
        }
        *y += 20.0;

        // Name field
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Name:", x, *y + 16.0, 12.0, Color::WHITE);
            let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            let (new_name, _) = TextInput::new(
                self.properties_panel.widget_ids.menu_name_id,
                field_rect,
                &current_name,
            )
            .blocked(blocked)
            .show(ctx);

            if new_name != current_name && !new_name.is_empty() {
                let is_duplicate = self.templates.iter().any(|t| t.id == new_name);
                if !is_duplicate {
                    if let Some(idx) = self.current_template_index {
                        push_command(Box::new(UpdateTemplateCmd::new(
                            idx,
                            TemplateProperty::Name {
                                old: current_name.clone(),
                                new: new_name,
                            },
                        )));
                    }
                }
            }
        }
        *y += ROW_HEIGHT;

        // Mode dropdown
        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Mode:", x, *y + 16.0, 12.0, Color::WHITE);
            let mode_options = ["Paused", "Overlay", "FrontEnd"];
            let current_mode_str = match current_mode {
                MenuMode::Paused => "Paused",
                MenuMode::Overlay => "Overlay",
                MenuMode::FrontEnd => "FrontEnd",
            };
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.mode_id,
                dropdown_rect,
                current_mode_str,
                &mode_options,
                |s| s.to_string(),
            )
            .blocked(blocked)
            .fixed_width()
            .show(ctx)
            {
                let new_mode = match selected {
                    "Paused" => MenuMode::Paused,
                    "Overlay" => MenuMode::Overlay,
                    "FrontEnd" => MenuMode::FrontEnd,
                    _ => current_mode,
                };
                if new_mode != current_mode {
                    if let Some(idx) = self.current_template_index {
                        push_command(Box::new(UpdateTemplateCmd::new(
                            idx,
                            TemplateProperty::Mode {
                                old: current_mode,
                                new: new_mode,
                            },
                        )));
                    }
                }
            }
        }
        *y += ROW_HEIGHT;

        // Background section
        *y += 4.0;
        if row_visible(*y, 20.0, clip) {
            ctx.draw_text("Background", x, *y + 14.0, 12.0, Color::GREY);
        }
        *y += 20.0;

        if row_visible(*y, ROW_HEIGHT, clip) {
            ctx.draw_text("Type:", x, *y + 16.0, 12.0, Color::WHITE);
            let bg_options = ["None", "Solid Color", "Dimmed"];
            let current_bg_str = match current_bg {
                MenuBackground::None => "None",
                MenuBackground::SolidColor(_) => "Solid Color",
                MenuBackground::Dimmed(_) => "Dimmed",
            };
            let dropdown_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
            if let Some(selected) = Dropdown::new(
                self.properties_panel.widget_ids.bg_type_id,
                dropdown_rect,
                current_bg_str,
                &bg_options,
                |s| s.to_string(),
            )
            .blocked(blocked)
            .fixed_width()
            .show(ctx)
            {
                let new_bg = match selected {
                    "None" => MenuBackground::None,
                    "Solid Color" => MenuBackground::SolidColor(Color::BLACK),
                    "Dimmed" => MenuBackground::Dimmed(0.7),
                    _ => current_bg,
                };
                if let Some(idx) = self.current_template_index {
                    push_command(Box::new(UpdateTemplateCmd::new(
                        idx,
                        TemplateProperty::Background {
                            old: current_bg,
                            new: new_bg,
                        },
                    )));
                }
            }
        }
        *y += ROW_HEIGHT;

        // Conditional fields based on background type
        match current_bg {
            MenuBackground::SolidColor(color) => {
                if row_visible(*y, ROW_HEIGHT, clip) {
                    ctx.draw_text("Color:", x, *y + 16.0, 12.0, Color::WHITE);
                    let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
                    let new_color = ColorInput::new(
                        self.properties_panel.widget_ids.bg_color_id,
                        field_rect,
                        color,
                    )
                    .blocked(blocked)
                    .show(ctx);
                    if new_color != color {
                        if let Some(idx) = self.current_template_index {
                            push_command(Box::new(UpdateTemplateCmd::new(
                                idx,
                                TemplateProperty::Background {
                                    old: MenuBackground::SolidColor(color),
                                    new: MenuBackground::SolidColor(new_color),
                                },
                            )));
                        }
                    }
                }
                *y += ROW_HEIGHT;
            }
            MenuBackground::Dimmed(alpha) => {
                if row_visible(*y, ROW_HEIGHT, clip) {
                    ctx.draw_text("Alpha:", x, *y + 16.0, 12.0, Color::WHITE);
                    let field_rect = Rect::new(x + LABEL_WIDTH, *y, w - LABEL_WIDTH, FIELD_HEIGHT);
                    let (new_alpha, state) = gui_slider(
                        ctx,
                        self.properties_panel.widget_ids.bg_alpha_id,
                        field_rect,
                        0.0,
                        1.0,
                        alpha,
                    );
                    match state {
                        SliderState::Previewing => {
                            if let Some(idx) = self.current_template_index {
                                if let Some(template) = self.templates.get_mut(idx) {
                                    template.background = MenuBackground::Dimmed(new_alpha);
                                }
                            }
                        }
                        SliderState::Committed { initial_value } => {
                            if let Some(idx) = self.current_template_index {
                                if let Some(template) = self.templates.get_mut(idx) {
                                    template.background = MenuBackground::Dimmed(new_alpha);
                                }
                                push_command(Box::new(UpdateTemplateCmd::new(
                                    idx,
                                    TemplateProperty::Background {
                                        old: MenuBackground::Dimmed(initial_value),
                                        new: MenuBackground::Dimmed(new_alpha),
                                    },
                                )));
                            }
                        }
                        SliderState::Unchanged => {}
                    }
                }
                *y += ROW_HEIGHT;
            }
            MenuBackground::None => {}
        }

        // Elements list
        *y += 8.0;
        let element_labels: Vec<(usize, String)> = {
            let Some(template) = self.current_template() else {
                return;
            };
            template
                .elements
                .iter()
                .enumerate()
                .map(|(i, el)| {
                    let label = if !el.name.is_empty() {
                        el.name.clone()
                    } else {
                        match &el.kind {
                            MenuElementKind::Label(l) => format!("Label: {}", l.text_key),
                            MenuElementKind::Button(b) => format!("Button: {}", b.text_key),
                            MenuElementKind::Panel(_) => "Panel".to_string(),
                            MenuElementKind::LayoutGroup(_) => "Layout Group".to_string(),
                            MenuElementKind::Slider(s) => format!("Slider: {}", s.text_key),
                        }
                    };
                    (i, label)
                })
                .collect()
        };

        if row_visible(*y, 20.0, clip) {
            ctx.draw_text(
                &format!("Elements ({})", element_labels.len()),
                x,
                *y + 14.0,
                12.0,
                Color::GREY,
            );
        }
        *y += 20.0;

        let mouse: Vec2 = ctx.mouse_position().into();
        let mut clicked_index = None;

        for (index, label) in &element_labels {
            if !row_visible(*y, ROW_HEIGHT, clip) {
                *y += ROW_HEIGHT;
                continue;
            }

            let item_rect = Rect::new(x, *y, w, ROW_HEIGHT);
            let hover = item_rect.contains(mouse) && !blocked;

            let bg_color = if hover {
                Color::new(0.25, 0.25, 0.3, 1.0)
            } else {
                Color::new(0.2, 0.2, 0.25, 1.0)
            };

            ctx.draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, bg_color);
            ctx.draw_text(
                label,
                item_rect.x + 8.0,
                item_rect.y + 16.0,
                11.0,
                Color::WHITE,
            );

            if hover && ctx.is_mouse_button_pressed(MouseButton::Left) {
                clicked_index = Some(*index);
            }

            *y += ROW_HEIGHT + 4.0;
        }

        if let Some(index) = clicked_index {
            self.selected_element_indices.clear();
            self.selected_element_indices.insert(index);
            self.selected_child_index = None;
        }
    }
}
