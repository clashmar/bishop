local save_flow = require("save_flow")

---@class Script
local Checkpoint = {
    public = {
        name = "Checkpoint",
    },

    interact = function(self)
        local transform = self.entity:get(Components.Transform)
        local room_id = self.entity:current_room()
        if transform == nil or room_id == nil then
            return
        end

        save_flow.request_checkpoint({
            kind = "checkpoint",
            room_id = room_id,
            x = transform.position.x,
            y = transform.position.y,
        })
        engine.log.info("Checkpoint save requested")
    end,
}

return Checkpoint
