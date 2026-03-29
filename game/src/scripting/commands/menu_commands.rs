// game/src/scripting/commands/menu_commands.rs
use crate::engine::Engine;
use crate::scripting::commands::lua_command::LuaCommand;

/// Command to open a menu by id.
pub struct OpenMenuCmd {
    pub menu_id: String,
}

impl LuaCommand for OpenMenuCmd {
    fn execute(&mut self, engine: &mut Engine) {
        engine.menu_manager.open_menu(&self.menu_id);
    }
}

/// Command to close the current menu.
pub struct CloseMenuCmd;

impl LuaCommand for CloseMenuCmd {
    fn execute(&mut self, engine: &mut Engine) {
        engine.menu_manager.close_menu();
    }
}
