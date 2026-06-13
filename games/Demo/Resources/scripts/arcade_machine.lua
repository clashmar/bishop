-- arcade_machine.lua
-- Attach to an entity with an Interactable component. On interact (I key),
-- activates the configured world without moving the player — the player
-- resumes exactly in place when the arcade world activates back.
---@class Script
local ArcadeMachine = {
    public = {
        name = "ArcadeMachine",
        world = "Pacman",
    },

    interact = function(self)
        engine.log.info("ArcadeMachine: activating " .. self.public.world)
        engine.activate_world(self.public.world)
    end,
}

return ArcadeMachine
