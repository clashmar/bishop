-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta

--- Onscreen text display module
---@class TextApi
engine.text = {}

--- Sets the current text display language.
---@param lang string The language code (e.g. "en", "es")
function engine.text.set_language(lang) end

--- Gets the current text display language.
---@return string
function engine.text.get_language() end

--- Gets a list of available languages.
---@return string[]
function engine.text.get_languages() end

--- Gets the current text display configuration.
---@return {default_duration: number, font_size: number, max_width: number, default_offset_y: number, padding: number, show_background: boolean, default_color: number[], default_background_color: number[]}
function engine.text.get_config() end

