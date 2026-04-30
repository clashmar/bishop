-- npc.lua
---@class ScriptDef
local npc = {
    public = {
        name = "NPC",
        dialogue = engine.asset.toml(),
    },

    interact = function(self)
        engine.log.info("Talking")
        if self.entity:is_speaking() then
            self.entity:say(self.public.dialogue, "farewell")
        else
            local player = engine.player()
            if player then
                self.entity:say(self.public.dialogue, "greeting", {
                    vars = {
                        player_name = player.public.name
                    }
                })
            end
        end
    end,
}

return npc
