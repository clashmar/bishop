-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta

--- Runtime save/load system.
engine.save = {}

--- Save the current game state to the manual save lane.
function engine.save.manual() end

--- Save the current game state to the autosave lane.
function engine.save.auto() end

--- Save a checkpoint (stored in the autosave lane).
function engine.save.checkpoint() end

--- Request loading the latest available runtime save.
function engine.save.load_latest() end

--- Register a save provider.
---@param def table A table with `id`, `version`, `capture`, and `apply` fields.
function engine.save.register_provider(def) end

