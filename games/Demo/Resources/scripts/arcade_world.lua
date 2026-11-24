-- arcade_world.lua
-- World script for the Arcade world. Runs while this world is active.

---@class Script
local ArcadeWorld = {
    public = {},

    init = function(self)
        engine.log.info("Arcade world entered")
    end,

    update = function(self, dt)
        if engine.input.is_action_pressed(InputAction.Back) then
            engine.return_from_world()
        end
    end,
}

return ArcadeWorld
