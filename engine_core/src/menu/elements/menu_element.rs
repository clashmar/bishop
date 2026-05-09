use super::button::ButtonElement;
use super::element_kind::ElementKind;
use super::label::LabelElement;
use super::layout_group::LayoutGroupElement;
use super::panel::PanelElement;
use super::slider::SliderElement;
use crate::menu::menu_builder::MenuAction;
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

impl ElementKind for MenuElementKind {
    fn kind_name(&self) -> &'static str {
        match self {
            MenuElementKind::Label(l) => l.kind_name(),
            MenuElementKind::Button(b) => b.kind_name(),
            MenuElementKind::Panel(p) => p.kind_name(),
            MenuElementKind::LayoutGroup(g) => g.kind_name(),
            MenuElementKind::Slider(s) => s.kind_name(),
        }
    }

    fn default_rect(&self) -> Rect {
        match self {
            MenuElementKind::Label(l) => l.default_rect(),
            MenuElementKind::Button(b) => b.default_rect(),
            MenuElementKind::Panel(p) => p.default_rect(),
            MenuElementKind::LayoutGroup(g) => g.default_rect(),
            MenuElementKind::Slider(s) => s.default_rect(),
        }
    }

    fn is_focusable(&self) -> bool {
        match self {
            MenuElementKind::Button(b) => b.is_focusable(),
            MenuElementKind::Slider(s) => s.is_focusable(),
            MenuElementKind::LayoutGroup(g) => g.is_focusable(),
            _ => false,
        }
    }

    fn wrap(&self, name: String) -> MenuElement {
        match self {
            MenuElementKind::Label(l) => l.wrap(name),
            MenuElementKind::Button(b) => b.wrap(name),
            MenuElementKind::Panel(p) => p.wrap(name),
            MenuElementKind::LayoutGroup(g) => g.wrap(name),
            MenuElementKind::Slider(s) => s.wrap(name),
        }
    }
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
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default)]
    pub style_id: Option<String>,
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
            class: None,
            style_id: None,
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
    pub fn panel(rect: Rect) -> Self {
        Self::new(MenuElementKind::Panel(PanelElement), rect)
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
