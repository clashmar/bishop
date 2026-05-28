-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta

--- Runtime save/load system.
engine.save = {}

--- Save the current game state to the manual save lane.
---@return nil
function engine.save.manual() end

--- Save the current game state to the autosave lane.
---@return nil
function engine.save.auto() end

--- Save a checkpoint (stored in the autosave lane).
---@return nil
function engine.save.checkpoint() end

--- Request loading the latest available runtime save.
---@return nil
function engine.save.load_latest() end

--- Register a save provider.
---@return nil
---@param def table A table with `id`, `version`, `capture`, and `apply` fields.
function engine.save.register_provider(def) end

--- Serialize a Lua value to a string.
---@return string
function engine.save.to_string(value) end

--- Deserialize a string to a Lua value.
---@return table|nil
function engine.save.from_string(json) end

--- Returns true if a latest save exists on disk.
---@return boolean
function engine.save.has_latest() end

