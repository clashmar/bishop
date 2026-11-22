-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta

--- Constructors for asset-backed script values.
engine.asset = {}

--- A TOML dialogue asset value.
---@class TomlId
local TomlId = {}

--- Returns a toml asset field.
---@return TomlId
function engine.asset.toml() end

--- Built-in tag constants.
engine.tags = {}
engine.tags.Autosave = "Autosave"

--- Built-in event name constants.
engine.events = {}
engine.events.room_entered = "room:entered"
engine.events.world_entered = "world:entered"
engine.events.save_succeeded = "save:succeeded"
engine.events.save_failed = "save:failed"
engine.events.collision_enter = "collision:enter"
engine.events.collision_exit = "collision:exit"
engine.events.collision_squeeze = "collision:squeeze"
engine.events.sensor_enter = "sensor:enter"
engine.events.sensor_stay = "sensor:stay"
engine.events.sensor_exit = "sensor:exit"

--- Get an entity's transform, or nil when the entity or component is absent
---@param entity Entity|nil
---@return Transform|nil
function engine.get_transform(entity) end

--- Get the player entity's script instance table
--- @return table|nil -- The player's script instance, or nil if not found
function engine.player() end

--- Call a named entity script in the active room or among globals
--- @param name string The name of the entity
--- @param method string The method name to call
--- @param ... any Additional arguments to pass to the method
--- @return any Returns whatever the method returns
function engine.call(name, method, ...) end

--- Register an event handler
--- @param event string The name of the event to listen for
--- @param handler function The Lua function that will be called
--- @return nil
function engine.on(event, handler) end

--- Emit an event to all registered handlers
--- @param event string The name of the event to emit
--- @param ... any Arguments that will be passed to each handler
--- @return nil
function engine.emit(event, ...) end

--- Quit to the title screen.
---@return nil
function engine.quit_to_title() end

--- Overlays another world without moving any entity.
--- The world resumes at the named entry's room, or its start when omitted.
---@param world_name string
---@param entry_name string|nil
---@return nil
function engine.overlay_world(world_name, entry_name) end

--- Overlays another world at a generated entry handle destination.
---@param entry table
---@return nil
function engine.overlay_entry(entry) end

--- Returns from the current overlay world.
---@return nil
function engine.return_from_world() end

--- Returns the active world.
---@return { id: integer, name: string }
function engine.current_world() end

---@class RestoreLocation
---@field world_id integer
---@field room_id integer
---@field x number
---@field y number

--- Restores the player to a specific world, room, and position.
---@param location RestoreLocation
---@return nil
function engine.restore_location(location) end

---@param input string
---@return boolean
function engine.input.is_down(input) end

---@param input string
---@return boolean
function engine.input.pressed(input) end

---@param input string
---@return boolean
function engine.input.released(input) end

---@param name string
---@param priority number
---@return nil
function engine.input.take_control(name, priority) end

---@param name string
---@return nil
function engine.input.release_control(name) end

---@param name string
---@return boolean
function engine.input.in_control(name) end

---@param msg string
function engine.log.info(msg) end

---@param msg string
function engine.log.warn(msg) end

---@param msg string
function engine.log.error(msg) end

---@param msg string
function engine.log.debug(msg) end

---@class ParticleFrame
---@field clip_id string
---@field col integer
---@field row integer
---@field offset vec2
---@field sprite_id integer
---@field frame_size vec2
---@field flip_x boolean

---@class ParticleSpriteVisual
---@field sprite ParticleSpriteVisualData

---@class ParticleSpriteVisualData
---@field frame ParticleFrame

---@class ParticleEmitterConfig
---@field mode? 'burst'|'continuous'
---@field visual? 'pixel'|'line'|ParticleSpriteVisual
---@field area_size? vec2
---@field line_length? integer
---@field lifetime_min? number
---@field lifetime_max? number
---@field velocity_min? vec2
---@field velocity_max? vec2
---@field scale_start? number
---@field scale_end? number
---@field color? Color
---@field opacity_start? number
---@field opacity_end? number
---@field burst_count? integer
---@field emission_rate? number
---@field max_particles? integer
---@field seed? integer

engine.particles = {}

---@param position vec2
---@param config? ParticleEmitterConfig
---@return Entity
function engine.particles.create(position, config) end

engine.prefab = {}

---@param prefab_name PrefabId
---@param position vec2
---@param init? table
---@return Entity
function engine.prefab.spawn(prefab_name, position, init) end

