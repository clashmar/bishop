use crate::menu::runtime::*;
use crate::menu::*;
use crate::onscreen_error;
use crate::storage::path_utils::menus_folder;
use crate::text::TextManager;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use widgets::MouseButton;

/// Manages menu templates, active menu stack, and navigation.
pub struct MenuManager {
    /// Registered menu templates by id.
    templates: HashMap<String, MenuTemplate>,
    /// Stack of active menu ids (top = current).
    menu_stack: Vec<String>,
    /// Policy that determines how global menu shortcuts behave.
    input_policy: MenuInputPolicy,
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
    /// Tracks the last focusable element the mouse was over, for hover detection.
    last_hovered_focus: Option<MenuFocus>,
    /// Pending on_open script name, set by open_menu, consumed by invoke_on_open.
    pending_on_open: Option<String>,
}

impl Default for MenuManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Controls how menu shortcuts interact with the active stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuInputPolicy {
    /// Gameplay pause toggles the pause menu with the configured pause shortcut.
    GameplayPause { pause_menu_id: String },
    /// Front-end menus ignore the pause shortcut and keep the root menu open until an authored action closes it.
    FrontEnd,
}

impl Default for MenuInputPolicy {
    fn default() -> Self {
        Self::GameplayPause {
            pause_menu_id: "pause".to_string(),
        }
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
    /// Full-screen front-end menu shown before gameplay begins.
    FrontEnd,
}

impl MenuMode {
    /// Returns true if this mode blocks gameplay updates.
    pub fn blocks_gameplay(&self) -> bool {
        matches!(self, MenuMode::Paused | MenuMode::FrontEnd)
    }
}

impl MenuManager {
    /// Creates a new menu manager with default settings.
    pub fn new() -> Self {
        let mut manager = Self {
            templates: HashMap::new(),
            menu_stack: Vec::new(),
            input_policy: MenuInputPolicy::default(),
            navigation: MenuNavigation::default(),
            focus: MenuFocus::new(0),
            action_handler: Box::new(NoOpActionHandler),
            viewport: Rect::new(0.0, 0.0, 1.0, 1.0),
            slider_values: HashMap::new(),
            slider_repeat: SliderRepeatState::default(),
            last_hovered_focus: None,
            pending_on_open: None,
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

    /// Sets the policy that governs global menu shortcuts.
    pub fn set_input_policy(&mut self, input_policy: MenuInputPolicy) {
        self.input_policy = input_policy;
    }

    /// Returns the current global menu shortcut policy.
    pub fn input_policy(&self) -> &MenuInputPolicy {
        &self.input_policy
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
            self.last_hovered_focus = None;
            self.pending_on_open = match template.on_open.trim() {
                "" => None,
                callback => Some(callback.to_string()),
            };
        }
    }

    /// Returns a reference to a registered template by id.
    pub fn get_template(&self, id: &str) -> Option<&MenuTemplate> {
        self.templates.get(id)
    }

    /// Sets the enabled state of a named element in a menu template.
    /// Returns true if the element was found and updated.
    pub fn set_element_enabled(&mut self, menu_id: &str, name: &str, enabled: bool) -> bool {
        self.templates
            .get_mut(menu_id)
            .and_then(|template| find_named_element_mut(&mut template.elements, name))
            .map(|element| {
                element.enabled = enabled;
            })
            .is_some()
    }

    /// Sets the visible state of a named element in a menu template.
    /// Returns true if the element was found and updated.
    pub fn set_element_visible(&mut self, menu_id: &str, name: &str, visible: bool) -> bool {
        self.templates
            .get_mut(menu_id)
            .and_then(|template| find_named_element_mut(&mut template.elements, name))
            .map(|element| {
                element.visible = visible;
            })
            .is_some()
    }

    /// Returns the next pending on_open callback path, if any.
    pub fn take_pending_on_open(&mut self) -> Option<String> {
        self.pending_on_open.take()
    }

    /// Closes the current menu and returns to previous menu if any.
    pub fn close_menu(&mut self) {
        self.menu_stack.pop();
        self.last_hovered_focus = None;
        if let Some(parent_id) = self.menu_stack.last()
            && let Some(template) = self.templates.get(parent_id)
        {
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
        self.last_hovered_focus = None;
        self.slider_repeat.reset();
    }

    /// Returns the current menu mode based on active menu.
    pub fn mode(&self) -> Option<MenuMode> {
        if let Some(menu_id) = self.menu_stack.last()
            && let Some(template) = self.templates.get(menu_id)
        {
            return Some(template.mode);
        }
        None
    }

    /// Returns true if the menu is blocking game updates.
    pub fn is_pausing_game(&self) -> bool {
        self.mode().is_some_and(|m| m.blocks_gameplay())
    }

    /// Returns true if the bottom menu's background fully obscures the game.
    pub fn is_hiding_game(&self) -> bool {
        self.menu_stack
            .first()
            .and_then(|id| self.templates.get(id))
            .is_some_and(|template| {
                template.mode == MenuMode::FrontEnd || template.background.is_opaque()
            })
    }

    /// Returns true if any menu is active.
    pub fn has_active_menu(&self) -> bool {
        !self.menu_stack.is_empty()
    }

    /// Returns the id of the active menu, if any.
    pub fn active_menu_id(&self) -> Option<&str> {
        self.menu_stack.last().map(String::as_str)
    }

    fn apply_pause_shortcut(&mut self, pause_pressed: bool) -> bool {
        if pause_pressed && let MenuInputPolicy::GameplayPause { pause_menu_id } = &self.input_policy {
            let pause_menu_id = pause_menu_id.clone();
            if self.has_active_menu() {
                self.close_menu();
            } else {
                self.open_menu(&pause_menu_id);
            }
            return true;
        }
        false
    }

    fn apply_cancel_shortcut(&mut self, cancel_pressed: bool) {
        if cancel_pressed {
            match &self.input_policy {
                MenuInputPolicy::GameplayPause { .. } => self.close_menu(),
                MenuInputPolicy::FrontEnd => {
                    if self.menu_stack.len() > 1 {
                        self.close_menu();
                    }
                }
            }
        }
    }

    #[cfg(test)]
    fn apply_input_shortcuts(&mut self, pause_pressed: bool, cancel_pressed: bool) -> bool {
        let pause_consumed = self.apply_pause_shortcut(pause_pressed);
        if !pause_consumed {
            self.apply_cancel_shortcut(cancel_pressed);
        }
        pause_consumed
    }

    /// Handles input for menu toggling and navigation.
    pub fn handle_input<C: BishopContext>(&mut self, ctx: &mut C) {
        let pause_pressed = self.navigation.pause_pressed(ctx);
        let cancel_pressed = self.navigation.cancel_pressed(ctx);
        if self.apply_pause_shortcut(pause_pressed) {
            return;
        }

        if !self.has_active_menu() {
            return;
        }

        if let Some(menu_id) = self.menu_stack.last().cloned()
            && let Some(template) = self.templates.get(&menu_id).cloned()
        {
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

            let focused_slider = template.get_element_at_focus(&self.focus).and_then(|el| {
                if let MenuElementKind::Slider(slider) = &el.kind {
                    Some((
                        slider.key.clone(),
                        slider.step,
                        slider.min,
                        slider.max,
                        slider.default_value,
                    ))
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
                    if let Some(new_value) = adjust_slider_value(current, step, min, max, direction)
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

            // Mouse hover overrides keyboard focus
            let mouse = ctx.mouse_position();
            self.handle_mouse_hover(&template, Vec2::new(mouse.0, mouse.1));

            let confirm_pressed = self.navigation.confirm_pressed(ctx);
            let action_to_handle = if confirm_pressed {
                template
                    .get_element_at_focus(&self.focus)
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

            self.apply_cancel_shortcut(cancel_pressed);

            if let Some(action) = action_to_handle {
                self.handle_action(action);
            }
        }
    }

    /// Applies mouse hover to focus: only changes focus when the mouse enters a new element.
    fn handle_mouse_hover(&mut self, template: &MenuTemplate, mouse: Vec2) {
        let hovered_focus = focus_target_at(template, self.viewport, mouse);

        if hovered_focus != self.last_hovered_focus
            && let Some(f) = hovered_focus.clone() {
            self.focus = f;
            self.slider_repeat.reset();
        }
        self.last_hovered_focus = hovered_focus;
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

fn find_named_element_mut<'a>(elements: &'a mut [MenuElement], name: &str) -> Option<&'a mut MenuElement> {
    for element in elements {
        if element.name == name {
            return Some(element);
        }
        if let MenuElementKind::LayoutGroup(group) = &mut element.kind
            && let Some(found) = find_named_child_element_mut(&mut group.children, name)
        {
            return Some(found);
        }
    }
    None
}

fn find_named_child_element_mut<'a>(children: &'a mut [LayoutChild], name: &str) -> Option<&'a mut MenuElement> {
    for child in children {
        if child.element.name == name {
            return Some(&mut child.element);
        }
        if let MenuElementKind::LayoutGroup(group) = &mut child.element.kind
            && let Some(found) = find_named_child_element_mut(&mut group.children, name)
        {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
#[path = "tests/menu_manager_tests.rs"]
mod tests;
