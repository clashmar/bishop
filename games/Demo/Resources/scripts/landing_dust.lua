local function is_grounded(entity)
    return entity ~= nil and entity:get(Components.Grounded) == true
end

---@class Script
local LandingDust = {}

function LandingDust:init()
    self._was_grounded = is_grounded(self.entity:parent())
end

function LandingDust:update()
    local parent = self.entity:parent()
    if parent == nil then
        self._was_grounded = false
        return
    end

    local grounded = is_grounded(parent)
    if grounded and not self._was_grounded then
        self.entity:burst_particles()
    end
    self._was_grounded = grounded
end

return LandingDust
