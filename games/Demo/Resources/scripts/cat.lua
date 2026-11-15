local Awareness = require("ai.awareness")
local Patrol = require("ai.patrol")

local Cat = {
    public = {
        speed = 24,
        flee_speed = 44,
        patrol_min_x = 48,
        patrol_max_x = 112,
    },

    _state = nil,
}

local function state_for(script)
    script._state = script._state or {
        direction = -1,
        wait_remaining = 0,
    }
    return script._state
end

local function blocked_ahead(entity, direction)
    local transform = engine.get_transform(entity)
    if not transform then
        return false
    end

    local cell = engine.grid.cell_at(transform.position)
    return engine.grid.is_blocked(entity, {
        x = cell.x + direction,
        y = cell.y,
    })
end

local function flee_intent(script, dt)
    local transform = engine.get_transform(script.entity)
    local player = engine.player()
    local player_transform = engine.get_transform(player and player.entity)
    if not transform or not player_transform then
        return { x = 0, y = 0 }
    end

    local direction = transform.position.x < player_transform.position.x and -1 or 1
    if
        Patrol.would_exceed_bounds(
            transform.position.x,
            direction,
            script.public.flee_speed,
            dt,
            script.public.patrol_min_x,
            script.public.patrol_max_x
        )
    then
        return { x = 0, y = 0 }
    end

    return { x = direction * script.public.flee_speed, y = 0 }
end

function Cat.update(self, dt)
    local state = state_for(self)
    local visible = Awareness.player_visible(self.entity)
    if visible then
        local intent = flee_intent(self, dt)
        self.entity:set_velocity({ x = intent.x or 0, y = intent.y or 0 })
        return
    end

    local direction = state.direction or -1
    local transform = engine.get_transform(self.entity)
    local intent = Patrol.choose_step(state, {
        speed = self.public.speed,
        wait_time = 0.8,
        position_x = transform and transform.position.x,
        min_x = self.public.patrol_min_x,
        max_x = self.public.patrol_max_x,
        blocked = blocked_ahead(self.entity, direction),
    }, dt)
    self.entity:set_velocity({ x = intent.x or 0, y = intent.y or 0 })
end

return Cat
