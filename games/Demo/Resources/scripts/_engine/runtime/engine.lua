-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta

--- Constructors for asset-backed script values.
engine.asset = {}

--- A TOML dialogue asset value.
---@class TomlId
local TomlId = {}

--- Returns a toml asset field.
---@return TomlId
function engine.asset.toml() end

--- Built-in tag constants.
engine.tags = {}
engine.tags.Autosave = "Autosave"

--- Built-in event name constants.
engine.events = {}
engine.events.room_entered = "room:entered"
engine.events.world_entered = "world:entered"
engine.events.save_succeeded = "save:succeeded"
engine.events.save_failed = "save:failed"

--- Get the player entity's script instance table
--- @return table|nil The player's script instance, or nil if not found
function engine.player() end

--- Call a method on a global entity script
--- @param name string The name of the global entity
--- @param method string The method name to call
--- @param ... any Additional arguments to pass to the method
--- @return any Returns whatever the method returns
function engine.call(name, method, ...) end

--- Register an event handler
--- @param event string The name of the event to listen for
--- @param handler function The Lua function that will be called
--- @return nil
function engine.on(event, handler) end

--- Emit an event to all registered handlers
--- @param event string The name of the event to emit
--- @param ... any Arguments that will be passed to each handler
--- @return nil
function engine.emit(event, ...) end

--- Quit to the title screen.
---@return nil
function engine.quit_to_title() end

--- Overlays another world without moving any entity.
--- The world resumes at the named entry's room, or its start when omitted.
---@param world_name string
---@param entry_name string|nil
---@return nil
function engine.overlay_world(world_name, entry_name) end

--- Overlays another world at a generated entry handle destination.
---@param entry table
---@return nil
function engine.overlay_entry(entry) end

--- Returns from the current overlay world.
---@return nil
function engine.return_from_world() end

--- Returns the active world.
---@return { id: integer, name: string }
function engine.current_world() end

---@class RestoreLocation
---@field world_id integer
---@field room_id integer
---@field x number
---@field y number

--- Restores the player to a specific world, room, and position.
---@param location RestoreLocation
---@return nil
function engine.restore_location(location) end

---@param msg string
function engine.log.info(msg) end

---@param msg string
function engine.log.warn(msg) end

---@param msg string
function engine.log.error(msg) end

---@param msg string
function engine.log.debug(msg) end

engine.prefab = {}

---@param prefab_name PrefabId
---@param position vec2
---@param init? table
---@return Entity
function engine.prefab.spawn(prefab_name, position, init) end

---@param input string
---@return boolean
function engine.input.is_down(input) end

---@param input string
---@return boolean
function engine.input.pressed(input) end

---@param input string
---@return boolean
function engine.input.released(input) end

---@param name string
---@param priority number
---@return nil
function engine.input.take_control(name, priority) end

---@param name string
---@return nil
function engine.input.release_control(name) end

---@param name string
---@return boolean
function engine.input.in_control(name) end

