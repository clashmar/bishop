// editor/src/commands/asset/mod.rs
mod delete_asset_cmd;
mod remap_asset_path_cmd;
mod rename_asset_cmd;

pub use delete_asset_cmd::*;
pub use remap_asset_path_cmd::*;
pub use rename_asset_cmd::*;

#[cfg(test)]
mod tests;