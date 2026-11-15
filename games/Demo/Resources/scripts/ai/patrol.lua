local Patrol = {}

local function direction_or_default(state)
    state.direction = state.direction or 1
    return state.direction
end

---@class PatrolConfig
---@field speed number?
---@field wait_time number?
---@field blocked boolean?
---@field position_x number?
---@field min_x number?
---@field max_x number?
---@param state table
---@param config PatrolConfig
---@param dt number
---@return { x: number, y: number }
function Patrol.choose_step(state, config, dt)
    local speed = config.speed or 30
    local wait_time = config.wait_time or 0
    state.wait_remaining = state.wait_remaining or 0

    if state.wait_remaining > 0 then
        state.wait_remaining = math.max(0, state.wait_remaining - dt)
        return { x = 0, y = 0 }
    end

    local direction = direction_or_default(state)
    local outside_patrol = Patrol.would_exceed_bounds(
        config.position_x,
        direction,
        speed,
        dt,
        config.min_x,
        config.max_x
    )

    if config.blocked or outside_patrol then
        direction = -direction
        state.direction = direction
        state.wait_remaining = wait_time
        return { x = 0, y = 0 }
    end

    return { x = speed * direction, y = 0 }
end

---@param position_x number?
---@param direction number
---@param speed number
---@param dt number
---@param min_x number?
---@param max_x number?
---@return boolean
function Patrol.would_exceed_bounds(position_x, direction, speed, dt, min_x, max_x)
    if not position_x then
        return false
    end

    local next_x = position_x + speed * dt * direction
    return (min_x and next_x < min_x) or (max_x and next_x > max_x) or false
end

return Patrol
