-- bullet.lua
local comp = require("_engine.components")
local direction = require("_engine.direction")

---@class ScriptDef
local bullet = {
    public = {
        speed = 260,
        lifetime = 1.5,
    },

    _state = nil,

    init = function(self, spawn_args)
        local launch_direction = (spawn_args and spawn_args.direction) or direction.Right

        self._state = {
            age = 0,
            direction = launch_direction,
        }
    end,

    update = function(self, dt)
        if self._state == nil then
            return
        end

        if self._state.dead then
            return
        end

        self._state.age = self._state.age + dt
        if self._state.age >= self.public.lifetime then
            self._state.dead = true
            self.entity:despawn()
            return
        end

        local transform = self.entity:get(comp.Transform)
        if transform == nil then
            return
        end

        local position = transform.position
        local step = self.public.speed * dt
        local launch_direction = self._state.direction

        if launch_direction == direction.Left then
            position.x = position.x - step
        elseif launch_direction == direction.Up then
            position.y = position.y - step
        elseif launch_direction == direction.Down then
            position.y = position.y + step
        else
            position.x = position.x + step
        end

        self.entity:set_transform(transform)
    end,
}

return bullet
