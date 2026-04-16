-- bullet.lua
local comp = require("_engine.components")
local direction = require("_engine.direction")

---@class BulletSpawnArgs
---@field direction Direction?

---@class BulletInstance : Script
---@field age number?
---@field direction Direction?
---@field dead boolean?

---@class ScriptDef
local bullet = {
    public = {
        speed = 260,
        lifetime = 1.5,
    },
    ---@param self BulletInstance
    ---@param spawn_args BulletSpawnArgs?
    init = function(self, spawn_args)
        local launch_direction = (spawn_args and spawn_args.direction) or direction.Right

        self.age = 0
        self.direction = launch_direction
        self.dead = false
    end,

    ---@param self BulletInstance
    ---@param dt number
    update = function(self, dt)
        local age = self.age
        local launch_direction = self.direction
        if age == nil or launch_direction == nil then
            return
        end

        if self.dead then
            return
        end

        age = age + dt
        self.age = age
        if age >= self.public.lifetime then
            self.dead = true
            self.entity:despawn()
            return
        end

        local transform = self.entity:get(comp.Transform)
        if transform == nil then
            return
        end

        local position = transform.position
        local step = self.public.speed * dt

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
