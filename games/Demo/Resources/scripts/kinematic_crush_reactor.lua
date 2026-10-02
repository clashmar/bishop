---@class Script
local CrushReactor = {
    crushed = false,
}

function CrushReactor:on_collision_squeeze(event)
    if not self.crushed then
        self.crushed = true
        self.entity:move_by({ x = 0, y = -8 })
    end

    engine.log.info("[collision_squeeze] role=" .. event.role .. " kind=" .. event.kind)
end

return CrushReactor
