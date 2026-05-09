/// Generated Lua filenames.
pub mod lua_files {
    pub const MAIN: &str = "main.lua";
    pub const ENGINE: &str = "engine.lua";
    pub const COMPONENTS: &str = "components.lua";
    pub const ENTITY: &str = "entity.lua";
    pub const ANIMATIONS: &str = "animations.lua";
    pub const SOUNDS: &str = "sounds.lua";
    pub const PREFABS: &str = "prefabs.lua";
    pub const TEXT: &str = "text.lua";
    pub const MENU: &str = "menu.lua";
    pub const AUDIO: &str = "audio.lua";
    pub const THEME: &str = "theme.lua";
    pub const COLOR: &str = "color.lua";
    pub const BISHOP_THEME: &str = "bishop_theme.lua";
}

/// Ownership markers for generated Lua files.
pub mod lua_ownership {
    pub const LUA_OWNER_SHARED_ENGINE: &str = "-- bishop-owner: shared-engine";
    pub const LUA_OWNER_GAME_GENERATED: &str = "-- bishop-owner: game-generated";
}

/// Global Lua names.
pub mod lua_globals {
    pub const LUA_GAME_CTX: &str = "lua_game_ctx";
    pub const LUA_EVENT_BUS: &str = "lua_event_bus";
    pub const ENTITY: &str = "entity";
}

/// Lua script directories.
pub mod lua_dirs {
    pub const ENGINE: &str = "_engine";
    pub const SCRIPTS: &str = "scripts";
}

/// Shared Lua field names.
pub mod lua_fields {
    pub const ID: &str = "id";
    pub const PUBLIC: &str = "public";
    pub const POSITION: &str = "position";
    pub const X: &str = "x";
    pub const Y: &str = "y";
    pub const Z: &str = "z";
}

/// Root engine Lua table names.
pub mod lua_engine {
    pub const ENGINE: &str = "engine";
    pub const ASSET: &str = "asset";
    pub const GLOBAL: &str = "global";
    pub const CALL: &str = "call";
    pub const ON: &str = "on";
    pub const EMIT: &str = "emit";
    pub const INPUT: &str = "input";
    pub const LOG: &str = "log";
    pub const PREFAB: &str = "prefab";
    pub const SPAWN: &str = "spawn";
    pub const TOML: &str = "toml";
    pub const THEME: &str = "theme";
}

/// Entity Lua method names.
pub mod lua_entity {
    pub const UPDATE: &str = "update";
    pub const INIT: &str = "init";
    pub const DESPAWN: &str = "despawn";
    pub const GET: &str = "get";
    pub const SET: &str = "set";
    pub const HAS: &str = "has";
    pub const HAS_ANY: &str = "has_any";
    pub const HAS_ALL: &str = "has_all";
    pub const INTERACT: &str = "interact";
    pub const FIND_BEST_INTERACTABLE: &str = "find_best_interactable";
    pub const TELEPORT: &str = "teleport";
    pub const MOVE_BY: &str = "move_by";
}

/// Animation Lua method names.
pub mod lua_animation {
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
}

/// Text Lua names.
pub mod lua_text {
    pub const SAY: &str = "say";
    pub const SAY_DIALOGUE: &str = "say_dialogue";
    pub const CLEAR_SPEECH: &str = "clear_speech";
    pub const IS_SPEAKING: &str = "is_speaking";
    pub const TEXT: &str = "text";
    pub const GET_LANGUAGE: &str = "get_language";
    pub const GET_LANGUAGES: &str = "get_languages";
    pub const SET_LANGUAGE: &str = "set_language";
    pub const GET_CONFIG: &str = "get_config";
}

/// Menu Lua names.
pub mod lua_menu {
    pub const MENU: &str = "menu";
    pub const OPEN: &str = "open";
    pub const CLOSE: &str = "close";
    pub const IS_OPEN: &str = "is_open";
}

/// Audio Lua names.
pub mod lua_audio {
    pub const AUDIO: &str = "audio";
    pub const PLAY_MUSIC: &str = "play_music";
    pub const IS_PLAYING: &str = "is_playing";
    pub const STOP_MUSIC: &str = "stop_music";
    pub const FADE_MUSIC: &str = "fade_music";
    pub const PLAY_SFX: &str = "play_sfx";
    pub const PRELOAD: &str = "preload";
    pub const SET_MASTER_VOLUME: &str = "set_master_volume";
    pub const SET_MUSIC_VOLUME: &str = "set_music_volume";
    pub const SET_SFX_VOLUME: &str = "set_sfx_volume";
    pub const UNLOAD: &str = "unload";
    pub const PLAY_RANDOM_SFX: &str = "play_random_sfx";
    pub const PLAY_SFX_VARIED: &str = "play_sfx_varied";
    pub const ENTITY_PLAY_SOUND: &str = "play_sound";
    pub const ENTITY_STOP_SOUND: &str = "stop_sound";
    pub const ENTITY_SET_SOUND_VOLUME: &str = "set_sound_volume";
}

/// Theme Lua names.
pub mod lua_theme {
    pub const THEME: &str = "theme";
    pub const NEW: &str = "new";
    pub const ACTIVATE: &str = "activate";
    pub const RULE: &str = "rule";
    pub const CLASS_THEME: &str = "Theme";
    pub const CLASS_THEME_API: &str = "ThemeApi";
    pub const CLASS_COLOR: &str = "Color";
    pub const CLASS_WIDGET_TYPE: &str = "Widget";
    pub const SELECTOR: &str = "selector";
    pub const PROPS: &str = "props";
    pub const RULES_TABLE: &str = "_rules";
}

/// Color Lua names.
pub mod lua_color {
    pub const COLOR: &str = "Color";
    pub const FROM_HEX: &str = "from_hex";
    pub const RGBA: &str = "rgba";
    pub const R: &str = "r";
    pub const G: &str = "g";
    pub const B: &str = "b";
    pub const A: &str = "a";
}

/// Auto-generated documentation paths.
pub mod lua_docs {
    pub const DOCS_DIR: &str = "docs";
    pub const THEME_REFERENCE: &str = "THEME_REFERENCE.md";
}
