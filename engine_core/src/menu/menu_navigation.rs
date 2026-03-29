use crate::menu::*;
use bishop::prelude::*;
use serde::{Deserialize, Serialize};

/// Navigation targets for each direction.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NavTargets {
    pub up: Option<usize>,
    pub down: Option<usize>,
    pub left: Option<usize>,
    pub right: Option<usize>,
}

/// Trait for navigable menu elements to implement.
pub trait Navigable {
    fn nav_targets(&self) -> &NavTargets;
    fn nav_targets_mut(&mut self) -> &mut NavTargets;
    // How to get this type from a MenuElement.
    fn from_element(el: &MenuElement) -> Option<&Self>;
    // How to wrap this type back into a MenuElementKind.
    fn wrap_into_element(self) -> MenuElementKind;
}

/// Configurable navigation bindings for menu interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuNavigation {
    pub up: InputBinding,
    pub down: InputBinding,
    pub left: InputBinding,
    pub right: InputBinding,
    pub confirm: InputBinding,
    pub cancel: InputBinding,
    pub pause: InputBinding,
}

impl Default for MenuNavigation {
    fn default() -> Self {
        Self {
            up: InputBinding::keyboard_with_alt(KeyCode::Up, KeyCode::W),
            down: InputBinding::keyboard_with_alt(KeyCode::Down, KeyCode::S),
            left: InputBinding::keyboard_with_alt(KeyCode::Left, KeyCode::A),
            right: InputBinding::keyboard_with_alt(KeyCode::Right, KeyCode::D),
            confirm: InputBinding::keyboard_with_alt(KeyCode::Enter, KeyCode::Space),
            cancel: InputBinding::keyboard(KeyCode::Escape),
            pause: InputBinding::keyboard_with_alt(KeyCode::P, KeyCode::Escape),
        }
    }
}

impl MenuNavigation {
    /// Checks if up was pressed this frame.
    pub fn up_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        self.up.is_pressed(ctx)
    }

    /// Checks if down was pressed this frame.
    pub fn down_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        self.down.is_pressed(ctx)
    }

    /// Checks if left was pressed this frame.
    pub fn left_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        self.left.is_pressed(ctx)
    }

    /// Checks if left is currently held down.
    pub fn left_down<C: BishopContext>(&self, ctx: &C) -> bool {
        self.left.is_down(ctx)
    }

    /// Checks if right was pressed this frame.
    pub fn right_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        self.right.is_pressed(ctx)
    }

    /// Checks if right is currently held down.
    pub fn right_down<C: BishopContext>(&self, ctx: &C) -> bool {
        self.right.is_down(ctx)
    }

    /// Checks if confirm was pressed this frame.
    pub fn confirm_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        self.confirm.is_pressed(ctx)
    }

    /// Checks if cancel was pressed this frame.
    pub fn cancel_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        self.cancel.is_pressed(ctx)
    }

    /// Checks if pause was pressed this frame.
    pub fn pause_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        self.pause.is_pressed(ctx)
    }
}
