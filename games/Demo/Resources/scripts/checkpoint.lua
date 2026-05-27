---@class ScriptDef
local checkpoint = {
    public = {
        name = "Checkpoint",
    },

    interact = function(self)
        engine.save.checkpoint()
        engine.log.info("Checkpoint saved")
    end,
}

return checkpoint
