-- bullet.lua

---@class BulletSpawnArgs
---@field direction Direction?

---@class BulletInstance : Script
---@field age number?
---@field direction Direction?
---@field dead boolean?

---@class Script
local Bullet = {
    public = {
        speed = 260,
        lifetime = 1.5,
    },
    ---@param self BulletInstance
    ---@param spawn_args BulletSpawnArgs?
    init = function(self, spawn_args)
        local launch_direction = (spawn_args and spawn_args.direction) or Direction.Right
        local velocity = { x = 0, y = 0 }

        if launch_direction == Direction.Left then
            velocity.x = -self.public.speed
        elseif launch_direction == Direction.Up then
            velocity.y = -self.public.speed
        elseif launch_direction == Direction.Down then
            velocity.y = self.public.speed
        else
            velocity.x = self.public.speed
        end

        self.age = 0
        self.direction = launch_direction
        self.dead = false
        self.entity:set_velocity(velocity)
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
    end,
}

return Bullet
