local GameManager = require("game_manager")
engine.game_manager = GameManager

local input = require("_engine.input")
require("audio_settings")

engine.update = function(dt)
    if engine.input.pressed(input.M) then
        engine.menu.open("start")
    end
end
