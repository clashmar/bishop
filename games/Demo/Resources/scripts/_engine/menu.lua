-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta

--- Menu system module
---@class MenuApi
engine.menu = {}

--- Opens a menu.
---@param menu string|Menus A menu id string or generated Menus table
function engine.menu.open(menu) end

--- Closes the current menu.
function engine.menu.close() end

--- Returns true if any menu is currently active.
---@return boolean
function engine.menu.is_open() end

--- Sets the enabled state of a named element in a menu template.
---@param menu string|Menus
---@param element_name string
---@param enabled boolean
function engine.menu.set_enabled(menu, element_name, enabled) end

--- Sets the visible state of a named element in a menu template.
---@param menu string|Menus
---@param element_name string
---@param visible boolean
function engine.menu.set_visible(menu, element_name, visible) end

