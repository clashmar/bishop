-- bullet.lua
local comp = require("_engine.components")
local direction = require("_engine.direction")

local function vec2_x(v)
    if v == nil then
        return nil
    end
    return v.x or v[1]
end

local function vec2_y(v)
    if v == nil then
        return nil
    end
    return v.y or v[2]
end

local function set_vec2_x(v, value)
    if v == nil then
        return
    end
    if v.x ~= nil then
        v.x = value
    else
        v[1] = value
    end
end

local function set_vec2_y(v, value)
    if v == nil then
        return
    end
    if v.y ~= nil then
        v.y = value
    else
        v[2] = value
    end
end

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
        local pos_x = vec2_x(position)
        local pos_y = vec2_y(position)
        if pos_x == nil or pos_y == nil then
            return
        end

        local step = self.public.speed * dt
        local launch_direction = self._state.direction

        if launch_direction == direction.Left then
            set_vec2_x(position, pos_x - step)
        elseif launch_direction == direction.Up then
            set_vec2_y(position, pos_y - step)
        elseif launch_direction == direction.Down then
            set_vec2_y(position, pos_y + step)
        else
            set_vec2_x(position, pos_x + step)
        end

        self.entity:set_transform(transform)
    end,
}

return bullet
