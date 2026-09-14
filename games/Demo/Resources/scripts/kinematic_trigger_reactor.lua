---@class Script
local TriggerReactor = {
    triggered = false,
}

function TriggerReactor:on_kinematic_contact(event)
    if not self.triggered then
        self.triggered = true
        self.entity:move_by({ x = 0, y = -8 })
    end

    engine.log.info("[kinematic_trigger] role=" .. event.role .. " kind=" .. event.kind)
end

return TriggerReactor
