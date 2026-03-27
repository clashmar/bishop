-- Auto-generated. Do not edit.
---@meta

---@class Entity
---@field id integer
local Entity = {}

-- Component getters
---@overload fun(self: Entity, component: "Light"): Light
---@overload fun(self: Entity, component: "AudioSource"): AudioSource
---@overload fun(self: Entity, component: "SpeechBubble"): SpeechBubble
---@overload fun(self: Entity, component: "RoomCamera"): RoomCamera
---@overload fun(self: Entity, component: "CurrentFrame"): CurrentFrame
---@overload fun(self: Entity, component: "Children"): Children
---@overload fun(self: Entity, component: "Parent"): Parent
---@overload fun(self: Entity, component: "Animation"): Animation
---@overload fun(self: Entity, component: "Script"): Script
---@overload fun(self: Entity, component: "Name"): Name
---@overload fun(self: Entity, component: "PhysicsBody"): PhysicsBody
---@overload fun(self: Entity, component: "PlayerProxy"): PlayerProxy
---@overload fun(self: Entity, component: "Kinematic"): Kinematic
---@overload fun(self: Entity, component: "Global"): Global
---@overload fun(self: Entity, component: "Layer"): Layer
---@overload fun(self: Entity, component: "CurrentRoom"): CurrentRoom
---@overload fun(self: Entity, component: "Player"): Player
---@overload fun(self: Entity, component: "SubPixel"): SubPixel
---@overload fun(self: Entity, component: "Solid"): Solid
---@overload fun(self: Entity, component: "Damage"): Damage
---@overload fun(self: Entity, component: "Velocity"): Velocity
---@overload fun(self: Entity, component: "Walkable"): Walkable
---@overload fun(self: Entity, component: "Grounded"): Grounded
---@overload fun(self: Entity, component: "Collider"): Collider
---@overload fun(self: Entity, component: "Glow"): Glow
---@overload fun(self: Entity, component: "Sprite"): Sprite
---@overload fun(self: Entity, component: "Transform"): Transform
---@overload fun(self: Entity, component: "FacingDirection"): FacingDirection
---@overload fun(self: Entity, component: "Interactable"): Interactable
---@param component string
---@return table|nil
function Entity:get(component) end

-- Generic set method
---@param component string
---@see ComponentId
---@param value table
function Entity:set(component, value) end

-- Typed component setters
---@param self Entity
---@param v Light
function Entity:set_light(v) end

---@param self Entity
---@param v AudioSource
function Entity:set_audio_source(v) end

---@param self Entity
---@param v SpeechBubble
function Entity:set_speech_bubble(v) end

---@param self Entity
---@param v RoomCamera
function Entity:set_room_camera(v) end

---@param self Entity
---@param v CurrentFrame
function Entity:set_current_frame(v) end

---@param self Entity
---@param v Children
function Entity:set_children(v) end

---@param self Entity
---@param v Parent
function Entity:set_parent(v) end

---@param self Entity
---@param v Animation
function Entity:set_animation(v) end

---@param self Entity
---@param v Script
function Entity:set_script(v) end

---@param self Entity
---@param v Name
function Entity:set_name(v) end

---@param self Entity
---@param v PhysicsBody
function Entity:set_physics_body(v) end

---@param self Entity
---@param v PlayerProxy
function Entity:set_player_proxy(v) end

---@param self Entity
---@param v Kinematic
function Entity:set_kinematic(v) end

---@param self Entity
---@param v Global
function Entity:set_global(v) end

---@param self Entity
---@param v Layer
function Entity:set_layer(v) end

---@param self Entity
---@param v CurrentRoom
function Entity:set_current_room(v) end

---@param self Entity
---@param v Player
function Entity:set_player(v) end

---@param self Entity
---@param v SubPixel
function Entity:set_sub_pixel(v) end

---@param self Entity
---@param v Solid
function Entity:set_solid(v) end

---@param self Entity
---@param v Damage
function Entity:set_damage(v) end

---@param self Entity
---@param v Velocity
function Entity:set_velocity(v) end

---@param self Entity
---@param v Walkable
function Entity:set_walkable(v) end

---@param self Entity
---@param v Grounded
function Entity:set_grounded(v) end

---@param self Entity
---@param v Collider
function Entity:set_collider(v) end

---@param self Entity
---@param v Glow
function Entity:set_glow(v) end

---@param self Entity
---@param v Sprite
function Entity:set_sprite(v) end

---@param self Entity
---@param v Transform
function Entity:set_transform(v) end

---@param self Entity
---@param v FacingDirection
function Entity:set_facing_direction(v) end

---@param self Entity
---@param v Interactable
function Entity:set_interactable(v) end

---@param component string
---@see ComponentId
---@return boolean
function Entity:has(component) end

---@param ... string
---@see ComponentId
---@return boolean
function Entity:has_any(...) end

---@param ... string
---@see ComponentId
---@return boolean
function Entity:has_all(...) end

---@vararg any Arguments passed to the entity's interact function
---@return nil
function Entity:interact(...) end

---@return Entity|nil
function Entity:find_best_interactable() end

--- Sets the active animation clip.
---@param clip_name string The name of the clip (e.g. "Walk", "Idle")
function Entity:set_clip(clip_name) end

--- Gets the current animation clip name.
---@return string|nil
function Entity:get_clip() end

--- Resets the current clip to frame 0.
function Entity:reset_clip() end

--- Sets horizontal flip for the sprite.
---@param flip_x boolean Whether to flip horizontally
function Entity:set_flip_x(flip_x) end

--- Gets the horizontal flip state.
---@return boolean
function Entity:get_flip_x() end

--- Sets the facing direction (for auto-flip with mirrored clips).
---@param direction string "left" or "right"
function Entity:set_facing(direction) end

--- Sets the animation playback speed multiplier.
---@param speed number Speed multiplier (1.0 = normal)
function Entity:set_anim_speed(speed) end

--- Gets the current animation frame indices.
---@return {col: integer, row: integer}|nil
function Entity:get_current_frame() end

--- Checks if the current non-looping clip has finished.
---@return boolean
function Entity:is_clip_finished() end

--- Shows a speech bubble with text from a dialogue file.
---@param dialogue_id string The dialogue file ID (e.g. "npc_merchant")
---@param key string The dialogue key (e.g. "greeting")
---@param opts? {vars?: table<string, string>, duration?: number, color?: number[], offset?: number[], font_size?: number, max_width?: number, show_background?: boolean, background_color?: number[]}
function Entity:say(dialogue_id, key, opts) end

--- Removes any speech bubble from the entity.
function Entity:clear_speech() end

--- Checks if the entity currently has a speech bubble.
---@return boolean
function Entity:is_speaking() end

--- Plays the named sound group configured on this entity's AudioSource component.
--- If the group is looping, starts a loop tracked by the entity ID.
--- If one-shot, plays with the group's pitch and volume variation.
---@param group_name SoundGroupId
function Entity:play_sound(group_name) end

--- Stops a looping sound started by this entity's AudioSource.
function Entity:stop_sound() end

--- Sets a runtime gain multiplier on this entity's AudioSource groups (0.0–1.0).
--- Takes effect on the next play_sound() call.
---@param v number Volume in range 0.0–1.0
function Entity:set_sound_volume(v) end

return Entity
