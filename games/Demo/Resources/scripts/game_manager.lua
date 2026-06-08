-- game_manager.lua
---@class GameManager
local GameManager = {
    public = {
        score = 0,
        level = 1,
    },
}

function GameManager:add_score(amount)
    self.public.score = self.public.score + amount
    engine.log.info("Score increased by " .. amount .. ". Total: " .. self.public.score)
    return self.public.score
end

function GameManager:get_score()
    return self.public.score
end
function GameManager:set_level(l)
    self.public.level = l
end

return GameManager
