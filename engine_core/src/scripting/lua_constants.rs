// engine_core/src/scripting/lua_constants.rs

/// Generated filenames
pub const MAIN_FILE: &str = "main.lua";
pub const ENGINE_FILE: &str = "engine.lua";
pub const ENTITY_FILE: &str = "entity.lua";
pub const ANIMATIONS_FILE: &str = "animations.lua";
pub const SOUNDS_FILE: &str = "sounds.lua";

/// GLOBALS
pub const LUA_GAME_CTX: &str = "lua_game_ctx";
pub const LUA_EVENT_BUS: &str = "lua_event_bus";
pub const ENTITY: &str = "entity";

// Directories
pub const ENGINE_DIR: &str = "_engine";
pub const SCRIPTS_DIR: &str = "scripts";

/// Field name for script public fields.
pub const PUBLIC: &str = "public";

// _engine APIS
pub const ENGINE: &str = "engine";
pub const GLOBAL: &str = "global";
pub const ENGINE_CALL: &str = "call";
pub const ENGINE_ON: &str = "on";
pub const ENGINE_EMIT: &str = "emit";
pub const INPUT: &str = "input";
pub const LOG: &str = "log";

// Entity methods
pub const UPDATE: &str = "update";
pub const INIT: &str = "init";
pub const GET: &str = "get";
pub const SET: &str = "set";
pub const HAS: &str = "has";
pub const HAS_ANY: &str = "has_any";
pub const HAS_ALL: &str = "has_all";
pub const INTERACT: &str = "interact";
pub const FIND_BEST_INTERACTABLE: &str = "find_best_interactable";

// Animation methods
pub const SET_CLIP: &str = "set_clip";
pub const GET_CLIP: &str = "get_clip";
pub const RESET_CLIP: &str = "reset_clip";
pub const SET_FLIP_X: &str = "set_flip_x";
pub const GET_FLIP_X: &str = "get_flip_x";
pub const SET_FACING: &str = "set_facing";
pub const SET_ANIM_SPEED: &str = "set_anim_speed";
pub const GET_CURRENT_FRAME: &str = "get_current_frame";
pub const IS_CLIP_FINISHED: &str = "is_clip_finished";
pub const ON_CLIP_FINISHED: &str = "on_clip_finished";

// Entity fields
pub const ID: &str = "id";

// Dialogue methods
pub const SAY: &str = "say";
pub const SAY_DIALOGUE: &str = "say_dialogue";
pub const CLEAR_SPEECH: &str = "clear_speech";
pub const IS_SPEAKING: &str = "is_speaking";

// Text module
pub const TEXT: &str = "text";
pub const TEXT_FILE: &str = "text.lua";
pub const GET_LANGUAGE: &str = "get_language";
pub const GET_LANGUAGES: &str = "get_languages";
pub const SET_LANGUAGE: &str = "set_language";
pub const GET_CONFIG: &str = "get_config";

// Menu module
pub const LUA_MENU: &str = "menu";
pub const MENU_FILE: &str = "menu.lua";
pub const OPEN_MENU: &str = "open";
pub const CLOSE_MENU: &str = "close";
pub const IS_MENU_OPEN: &str = "is_open";

// Audio module
pub const LUA_AUDIO: &str = "audio";
pub const AUDIO_FILE: &str = "audio.lua";
pub const AUDIO_PLAY_MUSIC: &str = "play_music";
pub const AUDIO_IS_PLAYING: &str = "is_playing";
pub const AUDIO_STOP_MUSIC: &str = "stop_music";
pub const AUDIO_FADE_MUSIC: &str = "fade_music";
pub const AUDIO_PLAY_SFX: &str = "play_sfx";
pub const AUDIO_PRELOAD: &str = "preload";
pub const AUDIO_SET_MASTER_VOLUME: &str = "set_master_volume";
pub const AUDIO_SET_MUSIC_VOLUME: &str = "set_music_volume";
pub const AUDIO_SET_SFX_VOLUME: &str = "set_sfx_volume";
pub const AUDIO_UNLOAD: &str = "unload";
pub const AUDIO_PLAY_RANDOM_SFX: &str = "play_random_sfx";
pub const AUDIO_PLAY_SFX_VARIED: &str = "play_sfx_varied";

// Entity audio methods
pub const ENTITY_PLAY_SOUND: &str = "play_sound";
pub const ENTITY_STOP_SOUND: &str = "stop_sound";
pub const ENTITY_SET_SOUND_VOLUME: &str = "set_sound_volume";
