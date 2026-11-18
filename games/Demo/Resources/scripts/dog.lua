local Awareness = require("ai.awareness")
local Pathing = require("ai.pathing")
local Patrol = require("ai.patrol")

local FOLLOW_PATH_MAX_STEPS = 2048

local Dog = {
    public = {
        speed = 32,
        follow_speed = 48,
        patrol_min_x = 144,
        patrol_max_x = 224,
    },

    _state = nil,
}

local function state_for(script)
    script._state = script._state or {
        direction = 1,
        wait_remaining = 0,
    }
    return script._state
end

local function player_cell()
    local player = engine.player()
    return engine.grid.cell_for_entity(player and player.entity)
end

local function blocked_ahead(entity, direction)
    local cell = engine.grid.cell_for_entity(entity)
    if not cell then
        return false
    end

    return engine.grid.is_blocked(entity, {
        x = cell.x + direction,
        y = cell.y,
    })
end

function Dog.update(self, dt)
    local state = state_for(self)
    local transform = engine.get_transform(self.entity)
    if Awareness.player_visible(self.entity) then
        local target = player_cell()
        if target and transform then
            local entity_cell = engine.grid.cell_for_entity(self.entity)
            if not entity_cell then
                self.entity:set_velocity({ x = 0, y = 0 })
                return
            end
            local follow_target = Pathing.target_on_actor_row(entity_cell, target)
            self.entity:set_velocity(Pathing.next_velocity(self.entity, follow_target, {
                speed = self.public.follow_speed,
                max_steps = FOLLOW_PATH_MAX_STEPS,
            }))
            return
        end
    end

    local direction = state.direction or 1
    local intent = Patrol.choose_step(state, {
        speed = self.public.speed,
        wait_time = 0.3,
        position_x = transform and transform.position.x,
        min_x = self.public.patrol_min_x,
        max_x = self.public.patrol_max_x,
        blocked = blocked_ahead(self.entity, direction),
    }, dt)
    self.entity:set_velocity({ x = intent.x or 0, y = intent.y or 0 })
end

return Dog
