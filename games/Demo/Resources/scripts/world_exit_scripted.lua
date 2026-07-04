---@class Script
local WorldExitScripted = {
    interact = function(self)
        local player = engine.player().entity
        if player == nil then
            return
        end

        local current_world = engine.current_world()
        if current_world.name == Worlds.SecondWorld.Name then
            player:move_to_entry(Entries.SecondWorld.MainEntry)
        else
            player:move_to_entry(Entries.SecondWorld.Start)
        end
    end,
}

return WorldExitScripted
