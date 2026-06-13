-- pacman_world.lua
-- World script for the Pacman world. Runs while this world is active.

---@class Script
local PacmanWorld = {
    public = {},

    init = function(self)
        engine.log("Pacman world script initialised")
    end,

    update = function(self, dt)
    end,
}

return PacmanWorld
