mod animation;
mod canvas;
mod document;
pub(crate) mod instance_sync;
mod movement;
pub mod prefab_actions;
pub mod prefab_editor;
mod selection;
mod shortcuts;

pub use prefab_editor::*;
pub use selection::is_prefab_entity;

#[cfg(test)]
mod tests;
