local GameManager = require("game_manager")
engine.game_manager = GameManager

require("save_manager")

-- Activates listeners for audio setting sliders
local AudioSettings = require("audio_settings")
AudioSettings:init()

-- Load and activate the game theme at startup
engine.theme.activate(require("bishop_theme"))

engine.update = function(dt)
    if engine.input.pressed(Input.M) then
        engine.menu.open(Menus.Pause)
    end
end
