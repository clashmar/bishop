pub mod dialogue;
pub mod game_text_settings;
pub mod interpolation;
pub mod text_data;
pub mod text_manager;

#[cfg(test)]
mod tests;

pub use dialogue::*;
pub use game_text_settings::*;
pub use interpolation::*;
pub use text_data::*;
pub use text_manager::*;
