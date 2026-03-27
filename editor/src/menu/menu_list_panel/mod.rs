// editor/src/menu/menu_list_panel/mod.rs
use crate::commands::menu::{CreateTemplateCmd, DeleteTemplateCmd};
use crate::editor_global::push_command;
use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::ui::widgets::*;

const MENU_ITEM_HEIGHT: f32 = 24.0;
const BUTTON_HEIGHT: f32 = 28.0;

/// Groups data for menu list panel.
pub struct MenuListPanel {
    scroll_state: ScrollState,
    new_menu_id: WidgetId,
    pending_new_menu: bool,
    new_menu_name: String,
}

impl MenuListPanel {
    /// Creates a new menu list panel.
    pub fn new() -> Self {
        Self {
            scroll_state: ScrollState::new(),
            new_menu_id: WidgetId::default(),
            pending_new_menu: false,
            new_menu_name: String::new(),
        }
    }
}

impl MenuEditor {
    /// Renders the menu panel and handles input.
    pub fn draw_menu_list_panel(&mut self, ctx: &mut WgpuContext, rect: Rect, blocked: bool) {
        let mouse: Vec2 = ctx.mouse_position().into();
        let content_height = self.calculate_menu_list_height();

        let area = ScrollableArea::new(rect, content_height)
            .scroll_speed(20.0)
            .blocked(blocked)
            .begin(ctx, &mut self.menu_list_panel.scroll_state);

        let mut y = rect.y + self.menu_list_panel.scroll_state.scroll_y + 8.0;
        let content_x = rect.x + 8.0;
        let content_w = area.content_rect().w - 16.0;

        if area.is_fully_visible(y, 24.0) {
            ctx.draw_text("Menus", content_x, y + 14.0, 14.0, Color::GREY);
        }
        y += 24.0;

        // New/Delete buttons
        if area.is_fully_visible(y, BUTTON_HEIGHT) {
            let btn_w = (content_w - 8.0) / 2.0;
            let new_btn_rect = Rect::new(content_x, y, btn_w, BUTTON_HEIGHT);
            let delete_btn_rect = Rect::new(content_x + btn_w + 8.0, y, btn_w, BUTTON_HEIGHT);

            let new_clicked = Button::new(new_btn_rect, "New").blocked(blocked).show(ctx);
            let delete_clicked = Button::new(delete_btn_rect, "Delete")
                .blocked(blocked || self.current_template_index.is_none())
                .show(ctx);

            if new_clicked {
                self.menu_list_panel.pending_new_menu = true;
                self.menu_list_panel.new_menu_name = String::new();
                text_input_reset(self.menu_list_panel.new_menu_id);
            }

            if delete_clicked {
                if let Some(index) = self.current_template_index {
                    push_command(Box::new(DeleteTemplateCmd::new(index)));
                }
            }
        }
        y += BUTTON_HEIGHT + 8.0;

        // New menu name input (if pending)
        if self.menu_list_panel.pending_new_menu {
            if area.is_fully_visible(y, 24.0) {
                let cancel_w = 24.0;
                let field_rect = Rect::new(content_x, y, content_w - cancel_w - 4.0, 24.0);
                let cancel_rect = Rect::new(content_x + content_w - cancel_w, y, cancel_w, 24.0);

                let (new_text, _) = TextInput::new(
                    self.menu_list_panel.new_menu_id,
                    field_rect,
                    &self.menu_list_panel.new_menu_name,
                )
                .focused(true)
                .blocked(blocked)
                .show(ctx);
                self.menu_list_panel.new_menu_name = new_text;

                let cancel_clicked = Button::new(cancel_rect, "×").blocked(blocked).show(ctx);

                let name_trimmed = self.menu_list_panel.new_menu_name.trim();
                let duplicate = self.templates.iter().any(|t| t.id == name_trimmed);
                if ctx.is_key_pressed(KeyCode::Enter) && !name_trimmed.is_empty() && !duplicate {
                    let name = name_trimmed.to_string();
                    push_command(Box::new(CreateTemplateCmd::new(name)));
                    self.menu_list_panel.pending_new_menu = false;
                    self.menu_list_panel.new_menu_name.clear();
                }

                if cancel_clicked || ctx.is_key_pressed(KeyCode::Escape) {
                    self.menu_list_panel.pending_new_menu = false;
                    self.menu_list_panel.new_menu_name.clear();
                }
            }

            y += 32.0;
        }

        // List of menus sorted alphabetically - collect click info first to avoid borrow issues
        let mut clicked_index = None;
        let mut sorted_indices: Vec<usize> = (0..self.templates.len()).collect();
        sorted_indices.sort_by(|a, b| {
            self.templates[*a]
                .id
                .to_lowercase()
                .cmp(&self.templates[*b].id.to_lowercase())
        });

        for index in sorted_indices {
            let template = &self.templates[index];
            if !area.is_fully_visible(y, MENU_ITEM_HEIGHT) {
                y += MENU_ITEM_HEIGHT + 4.0;
                continue;
            }

            let item_rect = Rect::new(content_x, y, content_w, MENU_ITEM_HEIGHT);
            let is_selected = self.current_template_index == Some(index);
            let hover = item_rect.contains(mouse);

            let bg_color = if is_selected {
                Color::new(0.3, 0.4, 0.6, 1.0)
            } else if hover && !blocked {
                Color::new(0.25, 0.25, 0.3, 1.0)
            } else {
                Color::new(0.2, 0.2, 0.25, 1.0)
            };

            ctx.draw_rectangle(item_rect.x, item_rect.y, item_rect.w, item_rect.h, bg_color);

            ctx.draw_text(
                &template.id,
                item_rect.x + 8.0,
                item_rect.y + 16.0,
                12.0,
                if is_selected {
                    Color::WHITE
                } else {
                    Color::new(0.8, 0.8, 0.8, 1.0)
                },
            );

            if hover && !blocked && ctx.is_mouse_button_pressed(MouseButton::Left) {
                clicked_index = Some(index);
            }

            y += MENU_ITEM_HEIGHT + 4.0;
        }

        // Apply clicked selection after iteration completes
        if let Some(index) = clicked_index {
            self.select_template(index);
        }

        area.draw_scrollbar(ctx, self.menu_list_panel.scroll_state.scroll_y);
    }

    fn calculate_menu_list_height(&self) -> f32 {
        let header_height = 24.0;
        let buttons_height = BUTTON_HEIGHT + 8.0;
        let input_height = if self.menu_list_panel.pending_new_menu {
            32.0
        } else {
            0.0
        };
        let items_height = (MENU_ITEM_HEIGHT + 4.0) * self.templates.len() as f32;

        header_height + buttons_height + input_height + items_height + 16.0
    }
}

impl Default for MenuListPanel {
    fn default() -> Self {
        Self::new()
    }
}
