-- Auto-generated. Do not edit.
---@meta

--- Audio system module
---@class AudioApi
engine.audio = {}

--- Plays music by ID.
--- `opts.looping` defaults to true; `opts.fade_out`, `opts.gap`, and `opts.fade_in` default to 0.0 seconds.
---@param id string Path relative to Resources/audio/ without extension
---@param opts? {looping?: boolean, fade_out?: number, gap?: number, fade_in?: number}
function engine.audio.play_music(id, opts) end

--- Returns true while music is considered active.
---@return boolean
function engine.audio.is_playing() end

--- Stops music immediately.
function engine.audio.stop_music() end

--- Fades music out over duration seconds, then stops.
---@param duration number Fade duration in seconds
function engine.audio.fade_music(duration) end

--- Plays a sound effect fire-and-forget.
---@param id string Path relative to Resources/audio/ without extension
function engine.audio.play_sfx(id) end

--- Pre-loads a sound into the cache to prevent stutter on first play.
---@param id string Path relative to Resources/audio/ without extension
function engine.audio.preload(id) end

--- Sets master volume (0.0–1.0).
---@param volume number
function engine.audio.set_master_volume(volume) end

--- Sets music group volume (0.0–1.0).
---@param volume number
function engine.audio.set_music_volume(volume) end

--- Sets SFX group volume (0.0–1.0).
---@param volume number
function engine.audio.set_sfx_volume(volume) end

--- Unpins a preloaded sound and evicts it from the cache if no components reference it.
---@param id string Path relative to Resources/audio/ without extension
function engine.audio.unload(id) end

--- Picks one sound at random from the list and plays it as a one-shot with no variation.
---@param sounds string[] Array of sound IDs
function engine.audio.play_random_sfx(sounds) end

--- Plays a single sound with optional pitch and volume variation.
---@param id string Sound ID
---@param opts? {pitch_var?: number, volume_var?: number}
function engine.audio.play_sfx_varied(id, opts) end
