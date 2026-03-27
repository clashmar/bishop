use crate::menu::*;
use crate::storage::path_utils::menus_folder;
use crate::text::TextManager;
use crate::{onscreen_error, onscreen_log};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bishop::prelude::*;
use widgets::*;
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
        manager.register_default_menus();
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

            self.update_click_focus(ctx, &template);

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
                    Self::adjust_slider_value(
                        &mut self.slider_values,
                        key,
                        step,
                        min,
                        max,
                        default_value,
                        direction,
                    );
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

        widgets_frame_start(ctx);

        let canvas_origin = Vec2::new(self.viewport.x, self.viewport.y);
        let canvas_size = Vec2::new(self.viewport.w, self.viewport.h);
        let mut triggered_action = None;

        if let Some(menu_id) = self.menu_stack.last() {
            let text_id = format!("ui/{}", menu_id);
            if let Some(template) = self.templates.get(menu_id) {
                template.render_background(ctx, self.viewport);

                for i in template.sorted_element_indices() {
                    let element = &template.elements[i];
                    if !element.visible {
                        continue;
                    }
                    match &element.kind {
                        MenuElementKind::Label(label) => {
                            let display_text = text_manager.resolve_ui_text(&text_id, &label.text_key);
                            let screen_rect = normalized_rect_to_screen(element.rect, canvas_origin, canvas_size);
                            MenuTemplate::render_label(ctx, label, screen_rect, &display_text);
                        }
                        MenuElementKind::Button(button) => {
                            let display_text = text_manager.resolve_ui_text(&text_id, &button.text_key);
                            let is_focused = self.focus.node == i && self.focus.child.is_none();
                            let screen_rect = normalized_rect_to_screen(element.rect, canvas_origin, canvas_size);
                            let btn = Button::new(screen_rect, &display_text)
                                .blocked(!element.enabled)
                                .focused(is_focused);
                            if btn.show(ctx) {
                                triggered_action = Some(button.action.clone());
                            }
                        }
                        MenuElementKind::Panel(panel) => {
                            let screen_rect = normalized_rect_to_screen(element.rect, canvas_origin, canvas_size);
                            ctx.draw_rectangle(
                                screen_rect.x,
                                screen_rect.y,
                                screen_rect.w,
                                screen_rect.h,
                                panel.background.render_color(),
                            );
                        }
                        MenuElementKind::LayoutGroup(group) => {
                            if let Some(bg) = &group.background {
                                let screen_rect = normalized_rect_to_screen(element.rect, canvas_origin, canvas_size);
                                ctx.draw_rectangle(
                                    screen_rect.x,
                                    screen_rect.y,
                                    screen_rect.w,
                                    screen_rect.h,
                                    bg.render_color(),
                                );
                            }
                            let resolved = resolve_layout(group, element.rect);
                            let mut focusable_idx = 0;
                            for (child, rect) in group.children.iter().zip(resolved.iter()) {
                                if !child.element.visible {
                                    continue;
                                }
                                let screen_rect = normalized_rect_to_screen(*rect, canvas_origin, canvas_size);
                                match &child.element.kind {
                                    MenuElementKind::Label(label) => {
                                        let display_text = text_manager.resolve_ui_text(&text_id, &label.text_key);
                                        MenuTemplate::render_label(ctx, label, screen_rect, &display_text);
                                    }
                                    MenuElementKind::Button(button) => {
                                        let display_text = text_manager.resolve_ui_text(&text_id, &button.text_key);
                                        let is_focused = self.focus.node == i
                                            && self.focus.child == Some(focusable_idx);
                                        let btn = Button::new(screen_rect, &display_text)
                                            .blocked(!child.element.enabled)
                                            .focused(is_focused);
                                        if btn.show(ctx) {
                                            triggered_action = Some(button.action.clone());
                                        }
                                        if child.element.enabled {
                                            focusable_idx += 1;
                                        }
                                    }
                                    MenuElementKind::Slider(slider) => {
                                        let is_focused = self.focus.node == i && self.focus.child == Some(focusable_idx);
                                        Self::render_slider(
                                            ctx,
                                            slider,
                                            screen_rect,
                                            text_manager,
                                            &text_id,
                                            &mut self.slider_values,
                                            is_focused,
                                        );
                                        if child.element.enabled {
                                            focusable_idx += 1;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        MenuElementKind::Slider(slider) => {
                            let screen_rect = normalized_rect_to_screen(element.rect, canvas_origin, canvas_size);
                            let is_focused = self.focus.node == i && self.focus.child.is_none();
                            Self::render_slider(
                                ctx,
                                slider,
                                screen_rect,
                                text_manager,
                                &text_id,
                                &mut self.slider_values,
                                is_focused,
                            );
                        }
                    }
                }
            }
        }

        widgets_frame_end(ctx);

        if let Some(action) = triggered_action {
            self.handle_action(action);
        }
    }

    /// Renders a slider element with its label, updating slider state and drawing a focus outline.
    fn render_slider<C: BishopContext>(
        ctx: &mut C,
        slider: &SliderElement,
        screen_rect: Rect,
        text_manager: &TextManager,
        text_id: &str,
        slider_values: &mut HashMap<String, f32>,
        is_focused: bool,
    ) {
        let value = slider_values.get(&slider.key).copied().unwrap_or(slider.default_value);
        let split = screen_rect.w * 0.4;
        let label_rect = Rect::new(screen_rect.x, screen_rect.y, split, screen_rect.h);
        let slider_rect = Rect::new(screen_rect.x + split, screen_rect.y, screen_rect.w - split, screen_rect.h);
        let label_bg = if is_focused { HOVER_COLOR } else { FIELD_BACKGROUND_COLOR };
        ctx.draw_rectangle(label_rect.x, label_rect.y, label_rect.w, label_rect.h, label_bg);
        // SliderElement doesn't embed label styling; defaults are used for now
        let display_text = text_manager.resolve_ui_text(text_id, &slider.text_key);
        let label = LabelElement::default();
        MenuTemplate::render_label(ctx, &label, label_rect, &display_text);
        // slider.step is used for keyboard input (Task 1D); gui_slider handles continuous drag
        let (new_value, state) = gui_slider(ctx, slider.widget_id, slider_rect, slider.min, slider.max, value);
        if !matches!(state, SliderState::Unchanged) {
            slider_values.insert(slider.key.clone(), new_value);
            push_slider_event(slider.key.clone(), new_value);
        }
        let outline_color = if is_focused { Color::WHITE } else { Color::new(0.5, 0.5, 0.5, 1.0) };
        ctx.draw_rectangle_lines(screen_rect.x, screen_rect.y, screen_rect.w, screen_rect.h, 2.0, outline_color);
    }

    fn update_click_focus<C: BishopContext>(&mut self, ctx: &C, template: &MenuTemplate) {
        if !ctx.is_mouse_button_pressed(MouseButton::Left) {
            return;
        }

        let mouse = ctx.mouse_position();
        let mouse = Vec2::new(mouse.0, mouse.1);
        let canvas_origin = Vec2::new(self.viewport.x, self.viewport.y);
        let canvas_size = Vec2::new(self.viewport.w, self.viewport.h);

        for target in collect_focus_targets(template, canvas_origin, canvas_size)
            .into_iter()
            .rev()
        {
            if target.rect.contains(mouse) {
                self.focus = target.focus;
                self.slider_repeat.reset();
                return;
            }
        }
    }

    fn adjust_slider_value(
        slider_values: &mut HashMap<String, f32>,
        key: String,
        step: f32,
        min: f32,
        max: f32,
        default_value: f32,
        direction: SliderAdjustmentDirection,
    ) {
        let current = slider_values.get(&key).copied().unwrap_or(default_value);
        let new_value = match direction {
            SliderAdjustmentDirection::Decrease => (current - step).max(min),
            SliderAdjustmentDirection::Increase => (current + step).min(max),
        };

        if (new_value - current).abs() <= f32::EPSILON {
            return;
        }

        slider_values.insert(key.clone(), new_value);
        push_slider_event(key, new_value);
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

    fn register_default_menus(&mut self) {
        let layout = LayoutConfig::vertical()
            .with_item_size(200.0, 40.0)
            .with_spacing(16.0)
            .with_padding(Padding::uniform(32.0))
            .with_alignment(Alignment::center());

        let pause_menu = MenuBuilder::new("pause")
            .background(MenuBackground::Dimmed(0.7))
            .layout_group(
                Rect::new(0.0, 0.0, 1.0, 1.0),
                layout,
                |group| {
                    group
                        .label("Paused")
                        .button("Resume", MenuAction::Resume)
                },
            )
            .build();

        self.register_template(pause_menu);

        let settings_layout = LayoutConfig::vertical()
            .with_item_size(300.0, 40.0)
            .with_spacing(16.0)
            .with_padding(Padding::uniform(32.0))
            .with_alignment(Alignment::center());

        let settings_menu = MenuBuilder::new("settings")
            .background(MenuBackground::Dimmed(0.7))
            .layout_group(
                Rect::new(0.0, 0.0, 1.0, 1.0),
                settings_layout,
                |group| {
                    group
                        .label("Settings")
                        .slider("Master Volume", "master_volume", 0.0, 1.0, 0.05, 1.0)
                        .slider("Music Volume", "music_volume", 0.0, 1.0, 0.05, 1.0)
                        .slider("SFX Volume", "sfx_volume", 0.0, 1.0, 0.05, 1.0)
                        .button("Back", MenuAction::CloseMenu)
                },
            )
            .build();

        self.register_template(settings_menu);
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
