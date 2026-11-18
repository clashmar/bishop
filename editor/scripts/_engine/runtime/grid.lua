-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta

engine.grid = {}

---@param position vec2
---@return table
function engine.grid.cell_at(position) end

---@param entity Entity?
---@return table|nil
function engine.grid.cell_for_entity(entity) end

---@param actor Entity
---@param cell table
---@return boolean
function engine.grid.is_blocked(actor, cell) end

---@param cell table
---@return table
function engine.grid.neighbors(cell) end

---@param actor Entity
---@param from_cell table
---@param to_cell table
---@return boolean
function engine.grid.can_step(actor, from_cell, to_cell) end

---@param actor Entity
---@param target_cell table
---@param options? table
---@return table|nil
function engine.grid.next_step_towards(actor, target_cell, options) end

---@param options table
---@return table
function engine.grid.find_path(options) end
