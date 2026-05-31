pub mod audio_module;
pub mod color_module;
pub mod engine_module;
pub mod entity_module;
pub mod input_module;
pub mod logging_module;
pub mod menu_module;
pub mod prefab_module;
pub mod save_module;
pub mod text_module;
pub mod theme_module;

#[cfg(test)]
#[path = "tests/entity_module_tests.rs"]
mod entity_module_tests;

#[cfg(test)]
#[path = "tests/save_module_tests.rs"]
mod save_module_tests;

#[cfg(test)]
#[path = "tests/menu_module_tests.rs"]
mod menu_module_tests;

#[cfg(test)]
#[path = "tests/toml_asset_tests.rs"]
mod toml_asset_tests;
