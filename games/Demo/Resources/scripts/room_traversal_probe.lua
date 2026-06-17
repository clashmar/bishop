---@class Script
local Probe = {
    public = {
        speed = 80,
        facing = Direction.Right,
    },

    update = function(self, dt)
        local current = self.entity:get(Components.Velocity) or { x = 0, y = 0 }
        local horizontal
        if self.public.facing == Direction.Left then
            horizontal = -self.public.speed
        else
            horizontal = self.public.speed
        end

        self.entity:set_velocity({
            x = horizontal,
            y = current.y,
        })
    end,
}

return Probe
