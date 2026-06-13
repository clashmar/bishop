-- arcade_exit.lua
-- Attach to any entity inside the arcade world. Pressing Q activates the
-- configured world; the player is untouched, so they resume in place.
---@class Script
local ArcadeExit = {
    public = {
        name = "ArcadeExit",
        world = "Main World",
    },

    update = function(self, dt)
        if engine.input.pressed(Input.Q) then
            engine.log.info("ArcadeExit: activating " .. self.public.world)
            engine.activate_world(self.public.world)
        end
    end,
}

return ArcadeExit
