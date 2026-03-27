use crate::storage::path_utils::menus_folder;
use crate::{onscreen_error, onscreen_log};
use crate::text::TextManager;
use crate::menu::runtime::*;
use crate::menu::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use widgets::MouseButton;
use bishop::prelude::*;
use std::fs;

/// Manages menu templates, active menu stack, and navigation.
pub struct MenuManager {
    /// Registered menu templates by id.
    templates: HashMap<String, MenuTemplate>,
    /// Stack of active menu ids (top = current).
    menu_stack: Vec<String>,
    /// Navigation input bindings.
    navigation: MenuNavigation,
    /// Current focus state for keyboard navigation.
    focus: MenuFocus,
    /// Action handler for custom menu actions.
    action_handler: Box<dyn MenuActionHandler>,
    /// The game viewport rect used to transform normalized menu coordinates to screen space.
    viewport: Rect,
    /// Current values for slider elements, keyed by slider key.
    slider_values: HashMap<String, f32>,
    /// Hold-to-repeat state for the currently focused slider.
    slider_repeat: SliderRepeatState,
}

impl Default for MenuManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the menu mode for a given menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MenuMode {
    #[default]
    /// Game paused, visible in background.
    Paused,
    /// Overlay menu, game continues (except player movement).
    Overlay,
}

impl MenuMode {
    /// Returns true if the game logic should be pause.
    pub fn is_paused(&self) -> bool {
        matches!(self, MenuMode::Paused)
    }
}

impl MenuManager {
    /// Creates a new menu manager with default settings.
    pub fn new() -> Self {
        let mut manager = Self {
            templates: HashMap::new(),
            menu_stack: Vec::new(),
            navigation: MenuNavigation::default(),
            focus: MenuFocus::new(0),
            action_handler: Box::new(NoOpActionHandler),
            viewport: Rect::new(0.0, 0.0, 1.0, 1.0),
            slider_values: HashMap::new(),
            slider_repeat: SliderRepeatState::default(),
        };
        for template in default_menus() {
            manager.register_template(template);
        }
        manager
    }

    /// Sets the game viewport rect so menu coordinates are correctly mapped to screen space.
    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = viewport;
    }

    /// Sets the custom action handler.
    pub fn set_action_handler<H: MenuActionHandler + 'static>(&mut self, handler: H) {
        self.action_handler = Box::new(handler);
    }

    /// Registers a menu template.
    pub fn register_template(&mut self, template: MenuTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    /// Opens a menu by id.
    pub fn open_menu(&mut self, id: &str) {
        if let Some(template) = self.templates.get(id) {
            self.focus.reset(template);
            self.slider_repeat.reset();
            self.menu_stack.push(id.to_string());
        }
    }

    /// Closes the current menu and returns to previous menu if any.
    pub fn close_menu(&mut self) {
        self.menu_stack.pop();
        if let Some(parent_id) = self.menu_stack.last() 
        && let Some(template) = self.templates.get(parent_id) {
            self.focus.reset(template);
            self.slider_repeat.reset();
            return;
        }
        self.focus = MenuFocus::new(0);
        self.slider_repeat.reset();
    }

    /// Closes all menus and returns to game.
    pub fn close_all(&mut self) {
        self.menu_stack.clear();
        self.focus = MenuFocus::new(0);
        self.slider_repeat.reset();
    }

    /// Returns the current menu mode based on active menu.
    pub fn mode(&self) -> Option<MenuMode> {
        if let Some(menu_id) = self.menu_stack.last() 
        && let Some(template) = self.templates.get(menu_id) {
            return Some(template.mode);
        }
        None
    }

    /// Returns true if the menu is blocking game updates.
    pub fn is_pausing_game(&self) -> bool {
        self.mode().is_some_and(|m| m.is_paused())
    }

    /// Returns true if the bottom menu's background fully obscures the game.
    pub fn is_hiding_game(&self) -> bool {
        self.menu_stack.first()
            .and_then(|id| self.templates.get(id))
            .is_some_and(|t| t.background.is_opaque())
    }

    /// Returns true if any menu is active.
    pub fn has_active_menu(&self) -> bool {
        !self.menu_stack.is_empty()
    }

    /// Handles input for menu toggling and navigation.
    pub fn handle_input<C: BishopContext>(&mut self, ctx: &mut C) {
        if self.navigation.pause_pressed(ctx) {
            if self.has_active_menu() {
                self.close_menu();
            } else {
                self.open_menu("pause");
            }
            return;
        }

        if !self.has_active_menu() {
            return;
        }

        if let Some(menu_id) = self.menu_stack.last().cloned()
        && let Some(template) = self.templates.get(&menu_id).cloned() {
            let focus_before_input = self.focus.clone();

            if ctx.is_mouse_button_pressed(MouseButton::Left) {
                let mouse = ctx.mouse_position();
                let mouse = Vec2::new(mouse.0, mouse.1);
                if let Some(focus) = focus_target_at(&template, self.viewport, mouse) {
                    self.focus = focus;
                    self.slider_repeat.reset();
                }
            }

            let up_pressed = self.navigation.up_pressed(ctx);
            let down_pressed = self.navigation.down_pressed(ctx);
            let left_pressed = self.navigation.left_pressed(ctx);
            let left_down = self.navigation.left_down(ctx);
            let right_pressed = self.navigation.right_pressed(ctx);
            let right_down = self.navigation.right_down(ctx);

            if up_pressed {
                self.focus.navigate(NavDirection::Up, &template);
            }
            if down_pressed {
                self.focus.navigate(NavDirection::Down, &template);
            }

            if self.focus != focus_before_input {
                self.slider_repeat.reset();
            }

            let focused_slider = template
                .get_element_at_focus(&self.focus)
                .and_then(|el| {
                    if let MenuElementKind::Slider(slider) = &el.kind {
                        Some((slider.key.clone(), slider.step, slider.min, slider.max, slider.default_value))
                    } else {
                        None
                    }
                });

            if let Some((key, step, min, max, default_value)) = focused_slider {
                if let Some(direction) = self.slider_repeat.next_adjustment(
                    ctx.get_time(),
                    left_pressed,
                    left_down,
                    right_pressed,
                    right_down,
                ) {
                    let current = self
                        .slider_values
                        .get(&key)
                        .copied()
                        .unwrap_or(default_value);
                    if let Some(new_value) =
                        adjust_slider_value(current, step, min, max, direction)
                    {
                        self.slider_values.insert(key.clone(), new_value);
                        push_slider_event(key, new_value);
                    }
                }
            } else if left_pressed {
                self.slider_repeat.reset();
                self.focus.navigate(NavDirection::Left, &template);
            } else if right_pressed {
                self.slider_repeat.reset();
                self.focus.navigate(NavDirection::Right, &template);
            } else {
                self.slider_repeat.reset();
            }

            let cancel_pressed = self.navigation.cancel_pressed(ctx);
            let confirm_pressed = self.navigation.confirm_pressed(ctx);
            let action_to_handle = if confirm_pressed {
                template.get_element_at_focus(&self.focus)
                    .and_then(|element| {
                        if let MenuElementKind::Button(button) = &element.kind {
                            Some(button.action.clone())
                        } else {
                            None
                        }
                    })
            } else {
                None
            };

            if cancel_pressed {
                self.close_menu();
            }

            if let Some(action) = action_to_handle {
                self.handle_action(action);
            }
        }
    }

    /// Renders the active menu if any.
    pub fn render<C: BishopContext>(&mut self, ctx: &mut C, text_manager: &TextManager) {
        if !self.has_active_menu() {
            return;
        }

        if let Some(menu_id) = self.menu_stack.last()
            && let Some(template) = self.templates.get(menu_id)
            && let Some(action) = render_active_menu(
                ctx,
                template,
                menu_id,
                self.viewport,
                &self.focus,
                &mut self.slider_values,
                text_manager,
            )
        {
            self.handle_action(action);
        }
    }

    fn handle_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Resume => self.close_all(),
            MenuAction::CloseMenu => self.close_menu(),
            MenuAction::OpenMenu(menu_id) => self.open_menu(&menu_id),
            MenuAction::QuitToMainMenu => {
                self.close_all();
            }
            MenuAction::QuitGame => {
                self.close_all();
            }
            MenuAction::Custom(action_name) => {
                self.action_handler.handle_action(&action_name);
            }
        }
    }

    /// Closes any active menu and resumes the game.
    pub fn close(&mut self) {
        self.close_all();
    }

    /// Loads all .ron menu templates from the menus folder and registers them.
    pub fn load_templates_from_disk(&mut self) {
        let dir = menus_folder();
        if !dir.exists() {
            return;
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            return;
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "ron") {
                continue;
            }

            let ron_str = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    onscreen_error!("Failed to read menu file {:?}: {}", path, e);
                    continue;
                }
            };

            match ron::de::from_str::<MenuTemplate>(&ron_str) {
                Ok(template) => self.register_template(template),
                Err(e) => onscreen_error!("Failed to parse menu file {:?}: {}", path, e),
            }
        }
    }
}
