local GameManager = require("game_manager")
engine.game_manager = GameManager

local input = require("_engine.input")

local AudioSettings = require("audio_settings")
local audio_initialized = false

engine.update = function(dt)
    if not audio_initialized then
        AudioSettings.init()
        audio_initialized = true
    end
    if engine.input.pressed(input.M) then
        engine.menu.open("start")
    end
end