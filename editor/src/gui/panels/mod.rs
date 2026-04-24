pub mod console_panel;
pub mod diagnostics_panel;
pub mod generic_panel;
pub mod hierarchy_panel;
pub mod panel_manager;
pub mod prefab_browser_panel;
pub mod prefab_palette_panel;
pub mod resources_panel;

pub use console_panel::*;
pub use diagnostics_panel::*;
pub use generic_panel::*;
pub use hierarchy_panel::*;
pub use prefab_browser_panel::*;
pub use prefab_palette_panel::*;
pub use prefab_palette_panel::*;
pub use resources_panel::*;

#[cfg(test)]
mod tests;
