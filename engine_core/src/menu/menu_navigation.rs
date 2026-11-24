use crate::input::{InputBinding, InputSource};
use crate::menu::{MenuElement, MenuElementKind};
use bishop::{BishopContext, KeyCode};
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
    pub up: Vec<InputBinding>,
    pub down: Vec<InputBinding>,
    pub left: Vec<InputBinding>,
    pub right: Vec<InputBinding>,
    pub confirm: Vec<InputBinding>,
    pub cancel: Vec<InputBinding>,
    pub pause: Vec<InputBinding>,
}

impl Default for MenuNavigation {
    fn default() -> Self {
        Self {
            up: vec![
                InputBinding::keyboard(KeyCode::Up),
                InputBinding::keyboard(KeyCode::W),
            ],
            down: vec![
                InputBinding::keyboard(KeyCode::Down),
                InputBinding::keyboard(KeyCode::S),
            ],
            left: vec![
                InputBinding::keyboard(KeyCode::Left),
                InputBinding::keyboard(KeyCode::A),
            ],
            right: vec![
                InputBinding::keyboard(KeyCode::Right),
                InputBinding::keyboard(KeyCode::D),
            ],
            confirm: vec![
                InputBinding::keyboard(KeyCode::Enter),
                InputBinding::keyboard(KeyCode::Space),
            ],
            cancel: vec![InputBinding::keyboard(KeyCode::Escape)],
            pause: vec![
                InputBinding::keyboard(KeyCode::P),
                InputBinding::keyboard(KeyCode::Escape),
            ],
        }
    }
}

impl MenuNavigation {
    /// Checks if up was pressed this frame.
    pub fn up_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        bindings_pressed(&self.up, ctx)
    }

    /// Checks if down was pressed this frame.
    pub fn down_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        bindings_pressed(&self.down, ctx)
    }

    /// Checks if left was pressed this frame.
    pub fn left_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        bindings_pressed(&self.left, ctx)
    }

    /// Checks if left is currently held down.
    pub fn left_down<C: BishopContext>(&self, ctx: &C) -> bool {
        bindings_down(&self.left, ctx)
    }

    /// Checks if right was pressed this frame.
    pub fn right_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        bindings_pressed(&self.right, ctx)
    }

    /// Checks if right is currently held down.
    pub fn right_down<C: BishopContext>(&self, ctx: &C) -> bool {
        bindings_down(&self.right, ctx)
    }

    /// Checks if confirm was pressed this frame.
    pub fn confirm_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        bindings_pressed(&self.confirm, ctx)
    }

    /// Checks if cancel was pressed this frame.
    pub fn cancel_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        bindings_pressed(&self.cancel, ctx)
    }

    /// Checks if pause was pressed this frame.
    pub fn pause_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        bindings_pressed(&self.pause, ctx)
    }
}

fn bindings_pressed<C: BishopContext>(bindings: &[InputBinding], ctx: &C) -> bool {
    bindings.iter().any(|binding| match binding.source {
        InputSource::Key(key) => ctx.is_key_pressed(key),
        InputSource::GamepadButton(_) | InputSource::GamepadAxis { .. } => false,
    })
}

fn bindings_down<C: BishopContext>(bindings: &[InputBinding], ctx: &C) -> bool {
    bindings.iter().any(|binding| match binding.source {
        InputSource::Key(key) => ctx.is_key_down(key),
        InputSource::GamepadButton(_) | InputSource::GamepadAxis { .. } => false,
    })
}
