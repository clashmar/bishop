use super::layout_group::LayoutGroupElement;
use super::menu_panel::PanelBackground;
use super::menu_slider::SliderElement;
use crate::menu::layout::HorizontalAlign;
use crate::menu::menu_builder::MenuAction;
use crate::menu::{NavTargets, Navigable};
use bishop::prelude::*;
use serde::{Deserialize, Serialize};

/// Different kinds of menu elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MenuElementKind {
    Label(LabelElement),
    Button(ButtonElement),
    Panel(PanelElement),
    LayoutGroup(LayoutGroupElement),
    Slider(SliderElement),
}

/// Label element displaying text resolved from a text key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelElement {
    #[serde(alias = "text")]
    pub text_key: String,
    pub font_size: f32,
    pub color: Color,
    #[serde(default)]
    pub alignment: HorizontalAlign,
}

impl Default for LabelElement {
    fn default() -> Self {
        Self {
            text_key: String::new(),
            font_size: 20.0,
            color: Color::WHITE,
            alignment: HorizontalAlign::default(),
        }
    }
}

/// Button element that triggers an action when clicked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonElement {
    #[serde(alias = "text")]
    pub text_key: String,
    pub action: MenuAction,
    pub font_size: f32,
    pub nav_targets: NavTargets,
}

impl Default for ButtonElement {
    fn default() -> Self {
        Self {
            text_key: String::new(),
            action: MenuAction::CloseMenu,
            font_size: 20.0,
            nav_targets: NavTargets::default(),
        }
    }
}

impl Navigable for ButtonElement {
    fn nav_targets(&self) -> &NavTargets {
        &self.nav_targets
    }

    fn nav_targets_mut(&mut self) -> &mut NavTargets {
        &mut self.nav_targets
    }

    fn from_element(el: &MenuElement) -> Option<&Self> {
        match &el.kind {
            MenuElementKind::Button(b) => Some(b),
            _ => None,
        }
    }

    fn wrap_into_element(self) -> MenuElementKind {
        MenuElementKind::Button(self)
    }
}

/// Decorative panel element that renders a background fill.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PanelElement {
    pub background: PanelBackground,
}

/// Menu element variants with positional data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuElement {
    pub name: String,
    pub kind: MenuElementKind,
    pub rect: Rect,
    pub enabled: bool,
    pub visible: bool,
    pub z_order: i32,
}

impl MenuElement {
    /// Creates a new menu element.
    pub fn new(kind: MenuElementKind, rect: Rect) -> Self {
        Self {
            name: String::new(),
            kind,
            rect,
            enabled: true,
            visible: true,
            z_order: 0,
        }
    }

    /// Creates a label element.
    pub fn label(text_key: String, rect: Rect) -> Self {
        Self::new(
            MenuElementKind::Label(LabelElement {
                text_key,
                ..Default::default()
            }),
            rect,
        )
    }

    /// Creates a button element.
    pub fn button(text_key: String, action: MenuAction, rect: Rect) -> Self {
        Self::new(
            MenuElementKind::Button(ButtonElement {
                text_key,
                action,
                ..Default::default()
            }),
            rect,
        )
    }

    /// Creates a panel element.
    pub fn panel(background: PanelBackground, rect: Rect) -> Self {
        Self::new(MenuElementKind::Panel(PanelElement { background }), rect)
    }

    /// Creates a layout group element.
    pub fn layout_group(group: LayoutGroupElement, rect: Rect) -> Self {
        Self::new(MenuElementKind::LayoutGroup(group), rect)
    }

    /// Creates a slider element for adjusting a bounded numeric setting.
    pub fn slider(
        text_key: String,
        key: String,
        min: f32,
        max: f32,
        step: f32,
        default_value: f32,
        rect: Rect,
    ) -> Self {
        Self::new(
            MenuElementKind::Slider(SliderElement {
                text_key,
                key,
                min,
                max,
                step,
                default_value,
                ..Default::default()
            }),
            rect,
        )
    }
}
