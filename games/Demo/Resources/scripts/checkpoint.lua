---@class Script
local Checkpoint = {
    public = {
        name = "Checkpoint",
    },

    interact = function(self)
        engine.save.checkpoint()
        engine.log.info("Checkpoint saved")
    end,
}

return Checkpoint
