-- npc.lua
---@class ScriptDef
local npc = {
    public = {
        name = "NPC",
        dialogue_id = "dialogue/npcs/npc",
    },

    interact = function(self)
        engine.log.info("Talking")
        if self.entity:is_speaking() then
            self.entity:say(self.public.dialogue_id, "farewell")
        else
            local player = engine.player()
            if player then
                self.entity:say(self.public.dialogue_id, "greeting", {
                    vars = {
                        player_name = player.public.name
                    }
                })
            end
        end
    end,
}

return npc