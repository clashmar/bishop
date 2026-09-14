---@class Script
local KinematicSwitch = {
    public = {
        target_name = "",
    },
}

function KinematicSwitch:toggle()
    local state = self.entity:get_kinematic_state()

    if state.running then
        self.entity:stop_kinematic()
    else
        self.entity:start_kinematic()
    end
end

function KinematicSwitch:interact()
    if self.public.target_name ~= "" then
        engine.call(self.public.target_name, "toggle")
        engine.log.info("[kinematic_switch] toggled " .. self.public.target_name)
        return
    end

    self:toggle()
    engine.log.info("[kinematic_switch] toggled self")
end

return KinematicSwitch
