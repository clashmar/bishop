-- player.lua
local comp = require("_engine.components")
local input = require("_engine.input")
local clip = require("_engine.animations")
local sound = require("_engine.sounds")

local primary_music_track = "music/Egobyte_CalmessPersonified"
local secondary_music_track = "music/Across the Sea"

---@class ScriptDef
local Player = {
    public = {
        speed = 100,
        run_speed = 180,
        jump_speed = 200,
        name = "Player",
        health = 100,
    },

    _state = nil,

    update = function(self, dt)
        if engine.menu.is_open() then
            local cur_vel = self.entity:get(comp.Velocity)
            self.entity:set_velocity({ x = 0, y = cur_vel.y })
            return
        end

        local horiz = 0
        if engine.input.is_down(input.Right) then
            horiz = horiz + 1
        end
        if engine.input.is_down(input.Left) then
            horiz = horiz - 1
        end

        -- Update facing direction based on movement
        if horiz > 0 then
            self.entity:set_facing("right")
        elseif horiz < 0 then
            self.entity:set_facing("left")
        end

        -- Check if running
        local is_running = engine.input.is_down(input.LeftShift)
        local move_speed = is_running and self.public.run_speed or self.public.speed

        -- Get current velocity
        local cur_vel = self.entity:get(comp.Velocity)

        -- Check grounded state (use Grounded component with velocity fallback)
        local is_grounded = self.entity:get(comp.Grounded)
        if is_grounded == nil then
            is_grounded = cur_vel.y == 0
        end

        ---@type Velocity
        local new_vel = {
            x = horiz * move_speed,
            y = cur_vel.y
        }

        -- Jump if grounded and space pressed
        if engine.input.pressed(input.Space) and is_grounded then
            new_vel.y = -self.public.jump_speed
            -- engine.audio.play_sfx("sfx/jump")
            self.entity:play_sound(sound.Jump)
        end

        self.entity:set_velocity(new_vel)

        -- Determine new state
        local new_state = self:determine_state(horiz, is_grounded, new_vel, is_running)

        -- Only change clip when state changes
        if new_state ~= self._state then
            self._state = new_state
            self.entity:set_clip(new_state)
        end

        -- Interaction
        if engine.input.pressed(input.I) then
            local entity = self.entity:find_best_interactable()
            if entity then
                entity:interact()
            end
        end

        -- Debug score
        if engine.input.pressed(input.P) then
            local new_score = engine.game_manager:add_score(10)
            engine.log.info("New score: " .. new_score)
        end

        -- Debug event
        if engine.input.pressed(input.F) then
            engine.call("EventTest", "fire")
        end

        if engine.input.pressed(input.Enter) then
            engine.audio.play_music(primary_music_track, {
                looping = true,
            })
        end

        if engine.input.pressed(input.C) then
            engine.audio.play_music(secondary_music_track, {
                looping = true,
                fade_out = 6.0,
                gap = 5.0,
                fade_in = 5.0,
            })
        end

        if engine.input.pressed(input.Q) and engine.audio.is_playing() then
            engine.audio.fade_music(2.0)
        end

        if engine.input.pressed(input.S) and engine.audio.is_playing() then
            engine.audio.stop_music()
        end
    end,

    determine_state = function(self, horiz, is_grounded, vel, is_running)
        -- Airborne states take priority
        if not is_grounded then
            if vel.y < 0 then
                return clip.Jump
            else
                return clip.Fall
            end
        end
        
        -- Test custom Fidget animation - press G while idle
        if horiz == 0 then
            if engine.input.is_down(input.G) then
                return clip.Fidget
            end
            return clip.Idle
        end

        if is_running then
            return clip.Run
        end
        return clip.Walk
    end,
}

return Player
