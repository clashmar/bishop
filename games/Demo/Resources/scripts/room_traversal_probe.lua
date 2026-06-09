---@class Script
local Probe = {
    public = {
        speed = 80,
        facing = Direction.Right,
    },

    update = function(self, dt)
        local current = self.entity:get(Components.Velocity) or { x = 0, y = 0 }
        local horizontal = self.public.facing == Direction.Left
            and -self.public.speed
            or self.public.speed

        self.entity:set_velocity({
            x = horizontal,
            y = current.y,
        })
    end,
}

return Probe
