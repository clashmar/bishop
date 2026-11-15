-- This game has its own authored update action. Its rules do not live in the player.
---@diagnostic disable-next-line: duplicate-set-field -- runtime callback overrides API stub
engine.update = function(_dt)
    if engine.input.pressed("t") then
        engine.log.info("WEB_SMOKE_LANTERN_LIT")
    end
end
