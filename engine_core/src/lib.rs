pub mod animation;
pub mod assets;
pub mod audio;
pub mod camera;
pub mod constants;
pub mod controls;
pub mod diagnostics;
pub mod ecs;
pub mod engine_global;
pub mod game;
pub mod input;
pub mod lighting;
pub mod logging;
pub mod menu;
pub mod physics;
pub mod rendering;
pub mod scripting;
pub mod shaders;
pub mod storage;
pub mod task;
pub mod text;
pub mod tiles;
pub mod ui;
pub mod worlds;

/// Prelude module for convenient imports.
#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use crate::animation::*;
    pub use crate::assets::*;
    #[allow(ambiguous_glob_reexports)]
    pub use crate::audio::*;
    pub use crate::camera::*;
    pub use crate::constants::*;
    pub use crate::controls::*;
    pub use crate::diagnostics::*;
    pub use crate::ecs::*;
    pub use crate::engine_global::*;
    pub use crate::game::*;
    pub use crate::input::*;
    pub use crate::lighting::*;
    pub use crate::logging::*;
    #[allow(ambiguous_glob_reexports)]
    pub use crate::menu::*;
    pub use crate::physics::*;
    pub use crate::register_lua_api;
    pub use crate::register_lua_module;
    pub use crate::rendering::*;
    pub use crate::scripting::*;
    pub use crate::shaders::*;
    pub use crate::storage::*;
    pub use crate::task::*;
    pub use crate::text::*;
    pub use crate::tiles::*;
    pub use crate::ui::*;
    pub use crate::worlds::*;
}
