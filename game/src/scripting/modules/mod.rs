pub mod audio_module;
pub mod color_module;
pub mod engine_module;
pub mod entity_module;
pub mod grid_module;
pub mod input_module;
pub mod logging_module;
pub mod menu_module;
pub mod particle_module;
pub mod prefab_module;
pub mod save_module;
pub mod spatial_module;
pub mod text_module;
pub mod theme_module;

#[cfg(test)]
#[path = "tests/entity_module_tests.rs"]
mod entity_module_tests;

#[cfg(test)]
#[path = "tests/entity_hierarchy_tests.rs"]
mod entity_hierarchy_tests;

#[cfg(test)]
#[path = "tests/engine_module_tests.rs"]
mod engine_module_tests;

#[cfg(test)]
#[path = "tests/grid_module_tests.rs"]
mod grid_module_tests;

#[cfg(test)]
#[path = "tests/save_module_tests.rs"]
mod save_module_tests;

#[cfg(test)]
#[path = "tests/spatial_module_tests.rs"]
mod spatial_module_tests;

#[cfg(test)]
#[path = "tests/demo_pathing_tests.rs"]
mod demo_pathing_tests;

#[cfg(test)]
#[path = "tests/demo_patrol_tests.rs"]
mod demo_patrol_tests;

#[cfg(test)]
#[path = "tests/demo_save_flow_tests.rs"]
mod demo_save_flow_tests;

#[cfg(test)]
#[path = "tests/demo_save_manager_tests.rs"]
mod demo_save_manager_tests;

#[cfg(test)]
#[path = "tests/menu_module_tests.rs"]
mod menu_module_tests;

#[cfg(test)]
#[path = "tests/toml_asset_tests.rs"]
mod toml_asset_tests;

#[cfg(test)]
#[path = "tests/particle_module_test_support.rs"]
mod particle_module_test_support;

#[cfg(test)]
#[path = "tests/particle_module_config_tests.rs"]
mod particle_module_config_tests;

#[cfg(test)]
#[path = "tests/particle_module_lifecycle_tests.rs"]
mod particle_module_lifecycle_tests;

#[cfg(test)]
#[path = "tests/prefab_module_tests.rs"]
mod prefab_module_tests;
