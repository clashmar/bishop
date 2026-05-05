local GameManager = require("game_manager")
engine.game_manager = GameManager

local input = require("_engine.input")

-- Activates listeners for audio setting sliders
require("audio_settings")

-- Load and activate the game theme at startup
engine.theme.activate(require("bishop_theme"))

engine.update = function(dt)
    if engine.input.pressed(input.M) then
        engine.menu.open("start")
    end
end