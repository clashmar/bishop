-- world_door.lua
-- Attach to an entity with an Interactable component. On interact (I key),
-- transports the player to the configured world. Set `entry` to a WorldEntry
-- name in that world, or leave empty to arrive at the world's start.
---@class Script
local WorldDoor = {
    public = {
        name = "WorldDoor",
        world = "Second World",
        entry = "",
    },

    interact = function(self)
        local player = engine.player()
        if player == nil then
            return
        end

        local entry = self.public.entry
        if entry == "" then
            entry = nil
        end

        engine.log.info("WorldDoor: transporting player to " .. self.public.world)
        player.entity:move_to_world(self.public.world, entry)
    end,
}

return WorldDoor
