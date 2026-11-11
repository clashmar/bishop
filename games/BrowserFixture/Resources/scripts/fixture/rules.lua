local SPEED = 140
local GOAL_X = 520
local GOAL_Y = 130
local GOAL_RADIUS = 16

return {
    initial = function()
        return 300, 130, GOAL_X, GOAL_Y
    end,
    advance = function(x, y, dx, dy, dt)
        x = math.max(0, math.min(620, x + dx * SPEED * dt))
        y = math.max(65, math.min(275, y + dy * SPEED * dt))
        local reached = (x - GOAL_X) ^ 2 + (y - GOAL_Y) ^ 2 <= GOAL_RADIUS ^ 2
        return x, y, reached
    end,
}
