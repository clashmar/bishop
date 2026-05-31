-- player.lua
local primary_music_track = "music/Egobyte_CalmessPersonified"
local secondary_music_track = "music/Across the Sea"

---@return Velocity
local function current_velocity(entity)
    return entity:get(Components.Velocity) or { x = 0, y = 0 }
end

---@class Script
local Player = {
    public = {
        speed = 100,
        run_speed = 180,
        jump_speed = 200,
        name = "Player",
        health = 100,
    },

    _state = nil,

    init = function(self)
        self._state = {
            facing = Direction.Right,
            clip = nil,
            spawned_bullets = {},
        }
    end,

    update = function(self, dt)
        if engine.menu.is_open() then
            local cur_vel = current_velocity(self.entity)
            self.entity:set_velocity({ x = 0, y = cur_vel.y })
            return
        end

        local state = self._state
            or {
                facing = Direction.Right,
                clip = nil,
                spawned_bullets = {},
            }
        self._state = state

        local horiz = 0
        if engine.input.is_down(Input.Right) then
            horiz = horiz + 1
        end
        if engine.input.is_down(Input.Left) then
            horiz = horiz - 1
        end

        -- Update facing direction based on movement
        if horiz > 0 then
            self.entity:set_facing(Direction.Right)
            state.facing = Direction.Right
        elseif horiz < 0 then
            self.entity:set_facing(Direction.Left)
            state.facing = Direction.Left
        end

        -- Check if running
        local is_running = engine.input.is_down(Input.LeftShift)
        local move_speed = is_running and self.public.run_speed or self.public.speed

        -- Get current velocity
        local cur_vel = current_velocity(self.entity)

        -- Check grounded state (use Grounded component with velocity fallback)
        local is_grounded = self.entity:get(Components.Grounded)
        if is_grounded == nil then
            is_grounded = cur_vel.y == 0
        end

        ---@type Velocity
        local new_vel = {
            x = horiz * move_speed,
            y = cur_vel.y,
        }

        -- Jump if grounded and space pressed
        if engine.input.pressed(Input.Space) and is_grounded then
            new_vel.y = -self.public.jump_speed
            -- engine.audio.play_sfx("sfx/jump")
            self.entity:play_sound(Sounds.Jump)
        end

        self.entity:set_velocity(new_vel)

        -- Determine new state
        local new_state = self:determine_state(horiz, is_grounded, new_vel, is_running)

        -- Only change clip when state changes
        if new_state ~= state.clip then
            state.clip = new_state
            self.entity:set_clip(new_state)
        end

        -- Interaction
        if engine.input.pressed(Input.I) then
            local entity = self.entity:find_best_interactable()
            if entity then
                entity:interact()
            end
        end

        -- Debug score
        if engine.input.pressed(Input.P) then
            local new_score = engine.game_manager:add_score(10)
            engine.log.info("New score: " .. new_score)
        end

        -- Debug event
        if engine.input.pressed(Input.F) then
            engine.call("EventTest", "fire")
        end

        if engine.input.pressed(Input.Enter) then
            engine.audio.play_music(primary_music_track, {
                looping = true,
            })
        end

        if engine.input.pressed(Input.C) then
            engine.audio.play_music(secondary_music_track, {
                looping = true,
                fade_out = 6.0,
                gap = 5.0,
                fade_in = 5.0,
            })
        end

        if engine.input.pressed(Input.Q) and engine.audio.is_playing() then
            engine.audio.fade_music(2.0)
        end

        if engine.input.pressed(Input.S) and engine.audio.is_playing() then
            engine.audio.stop_music()
        end

        if engine.input.pressed(Input.K) then
            local transform = self.entity:get(Components.Transform)
            if transform ~= nil then
                local pos = transform.position
                local x_offset = state.facing == Direction.Left and -12 or 12
                local bullet = engine.prefab.spawn(Prefabs.Bullet, {
                    x = pos.x + x_offset,
                    y = pos.y,
                }, {
                    direction = state.facing,
                })

                state.spawned_bullets[#state.spawned_bullets + 1] = bullet
            end
        end

        if engine.input.pressed(Input.L) then
            for index = 1, #state.spawned_bullets do
                local bullet = state.spawned_bullets[index]
                bullet:despawn()
            end
            state.spawned_bullets = {}
        end

        if engine.input.pressed(Input.T) then
            local transform = self.entity:get(Components.Transform)
            if transform ~= nil then
                local x_offset = state.facing == Direction.Left and -32 or 32
                self.entity:teleport({
                    x = transform.position.x + x_offset,
                    y = transform.position.y,
                })
            end
        end

        if engine.input.pressed(Input.Y) then
            self.entity:move_by({ x = 0, y = -16 })
        end
    end,

    determine_state = function(self, horiz, is_grounded, vel, is_running)
        -- Airborne states take priority
        if not is_grounded then
            if vel.y < 0 then
                return Animations.Jump
            else
                return Animations.Fall
            end
        end
        -- Test custom Fidget animation - press G while idle
        if horiz == 0 then
            if engine.input.is_down(Input.G) then
                return Animations.Fidget
            end
            return Animations.Idle
        end

        if is_running then
            return Animations.Run
        end
        return Animations.Walk
    end,
}

return Player
