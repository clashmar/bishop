local Pathing = {}

local function velocity_from_step(entity, step, speed)
    local transform = engine.get_transform(entity)
    if not transform then
        return { x = 0, y = 0 }
    end

    local current = engine.grid.cell_at(transform.position)
    local dx = step.x - current.x
    local dy = step.y - current.y
    return {
        x = dx * speed,
        y = dy * speed,
    }
end

---@param entity Entity
---@param target table
---@param config { speed: number?, max_steps: integer? }?
---@return { x: number, y: number }
function Pathing.next_velocity(entity, target, config)
    config = config or {}
    local step = engine.grid.next_step_towards(entity, target, {
        max_steps = config.max_steps or 40,
    })
    if not step then
        return { x = 0, y = 0 }
    end

    return velocity_from_step(entity, step, config.speed or 30)
end

---@param actor_cell table
---@param target_cell table
---@return table
function Pathing.target_on_actor_row(actor_cell, target_cell)
    return { x = target_cell.x, y = actor_cell.y }
end

return Pathing
