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

/// Command to set the enabled state of a named element in a menu template.
pub struct SetElementEnabledCmd {
    pub menu_id: String,
    pub element_name: String,
    pub enabled: bool,
}

impl LuaCommand for SetElementEnabledCmd {
    fn execute(&mut self, engine: &mut Engine) {
        engine
            .menu_manager
            .set_element_enabled(&self.menu_id, &self.element_name, self.enabled);
    }
}

/// Command to set the visible state of a named element in a menu template.
pub struct SetElementVisibleCmd {
    pub menu_id: String,
    pub element_name: String,
    pub visible: bool,
}

impl LuaCommand for SetElementVisibleCmd {
    fn execute(&mut self, engine: &mut Engine) {
        engine
            .menu_manager
            .set_element_visible(&self.menu_id, &self.element_name, self.visible);
    }
}
