use crate::menu::*;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};

/// Actions that can be triggered by menu items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MenuAction {
    /// Resume the game.
    Resume,
    /// Open a menu by id.
    OpenMenu(String),
    /// Close the current menu.
    CloseMenu,
    /// Quit to main menu.
    QuitToMainMenu,
    /// Quit the game.
    QuitGame,
    /// Custom action for extensibility.
    Custom(String),
}

impl MenuAction {
    /// Returns a human-readable label for this action.
    pub fn ui_label(&self) -> &'static str {
        match self {
            MenuAction::Resume => "Resume",
            MenuAction::OpenMenu(_) => "Open Menu",
            MenuAction::CloseMenu => "Close Menu",
            MenuAction::QuitToMainMenu => "Quit To Main Menu",
            MenuAction::QuitGame => "Quit Game",
            MenuAction::Custom(_) => "Custom",
        }
    }
}

/// Builder for composing menus with flexible layouts.
#[derive(Debug, Clone)]
pub struct MenuBuilder {
    id: String,
    elements: Vec<MenuElement>,
    background: MenuBackground,
    mode: MenuMode,
    screen_width: f32,
    screen_height: f32,
}

impl MenuBuilder {
    /// Creates a new menu builder with an id.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            elements: Vec::new(),
            background: MenuBackground::default(),
            mode: MenuMode::Paused,
            screen_width: 800.0,
            screen_height: 600.0,
        }
    }

    /// Sets the screen dimensions for layout calculations.
    pub fn screen_size(mut self, width: f32, height: f32) -> Self {
        self.screen_width = width;
        self.screen_height = height;
        self
    }

    /// Sets the menu mode.
    pub fn mode(mut self, mode: MenuMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the background style.
    pub fn background(mut self, bg: MenuBackground) -> Self {
        self.background = bg;
        self
    }

    /// Adds a pre-built element directly.
    pub fn element(mut self, element: MenuElement) -> Self {
        self.elements.push(element);
        self
    }

    /// Adds a layout group element with the given config and children built via closure.
    pub fn layout_group<F>(mut self, rect: Rect, layout: LayoutConfig, build_fn: F) -> Self
    where
        F: FnOnce(LayoutGroupBuilder) -> LayoutGroupBuilder,
    {
        let builder = LayoutGroupBuilder::new(layout);
        let builder = build_fn(builder);
        let group = builder.build();
        self.elements.push(MenuElement::layout_group(group, rect));
        self
    }

    /// Builds the final menu template.
    pub fn build(self) -> MenuTemplate {
        MenuTemplate {
            id: self.id,
            background: self.background,
            elements: self.elements,
            mode: self.mode,
        }
    }
}

/// Builder for constructing layout group children.
#[derive(Debug, Clone)]
pub struct LayoutGroupBuilder {
    layout: LayoutConfig,
    children: Vec<LayoutChild>,
}

impl LayoutGroupBuilder {
    /// Creates a new layout group builder.
    pub fn new(layout: LayoutConfig) -> Self {
        Self {
            layout,
            children: Vec::new(),
        }
    }

    /// Adds a managed label child.
    pub fn label(mut self, text_key: &str) -> Self {
        let element = MenuElement::label(text_key.to_string(), Rect::new(0.0, 0.0, 0.0, 0.0));
        self.children.push(LayoutChild {
            element,
            managed: true,
        });
        self
    }

    /// Adds a managed button child.
    pub fn button(mut self, text_key: &str, action: MenuAction) -> Self {
        let element =
            MenuElement::button(text_key.to_string(), action, Rect::new(0.0, 0.0, 0.0, 0.0));
        self.children.push(LayoutChild {
            element,
            managed: true,
        });
        self
    }

    /// Adds a managed slider child.
    pub fn slider(
        mut self,
        text_key: &str,
        key: &str,
        min: f32,
        max: f32,
        step: f32,
        default: f32,
    ) -> Self {
        let element = MenuElement::slider(
            text_key.to_string(),
            key.to_string(),
            min,
            max,
            step,
            default,
            Rect::new(0.0, 0.0, 0.0, 0.0),
        );
        self.children.push(LayoutChild {
            element,
            managed: true,
        });
        self
    }

    /// Adds a child element with explicit managed flag.
    pub fn child(mut self, element: MenuElement, managed: bool) -> Self {
        self.children.push(LayoutChild { element, managed });
        self
    }

    /// Builds the layout group element.
    pub fn build(self) -> LayoutGroupElement {
        LayoutGroupElement {
            layout: self.layout,
            children: self.children,
            background: None,
            nav_targets: NavTargets::default(),
        }
    }
}
