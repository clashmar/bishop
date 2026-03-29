use super::menu_element::MenuElement;
use super::menu_panel::PanelBackground;
use crate::menu::*;
use serde::{Deserialize, Serialize};

/// Element that arranges its children using layout rules.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LayoutGroupElement {
    pub layout: LayoutConfig,
    pub children: Vec<LayoutChild>,
    /// Optional panel background rendered behind the children.
    #[serde(default)]
    pub background: Option<PanelBackground>,
    pub nav_targets: NavTargets,
}

impl Navigable for LayoutGroupElement {
    fn nav_targets(&self) -> &NavTargets {
        &self.nav_targets
    }

    fn nav_targets_mut(&mut self) -> &mut NavTargets {
        &mut self.nav_targets
    }

    fn from_element(el: &MenuElement) -> Option<&Self> {
        match &el.kind {
            MenuElementKind::LayoutGroup(group) => Some(group),
            _ => None,
        }
    }

    fn wrap_into_element(self) -> MenuElementKind {
        MenuElementKind::LayoutGroup(self)
    }
}

/// A child element within a layout group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutChild {
    pub element: MenuElement,
    /// When true, position is computed from layout rules.
    /// When false, rect is relative to group origin but not subject to layout.
    pub managed: bool,
}
