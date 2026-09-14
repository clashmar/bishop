---@class Script
local CrushReactor = {
    crushed = false,
}

function CrushReactor:on_kinematic_crushed(event)
    if not self.crushed then
        self.crushed = true
        self.entity:move_by({ x = 0, y = -8 })
    end

    engine.log.info("[kinematic_crush] role=" .. event.role .. " kind=" .. event.kind)
end

return CrushReactor
