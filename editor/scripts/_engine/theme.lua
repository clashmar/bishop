-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta

---@class ThemeApi
engine.theme = {}

---@enum Widget
Widget = {
    Button = "Button",
    Slider = "Slider",
    Checkbox = "Checkbox",
    TextInput = "TextInput",
    NumberInput = "NumberInput",
    Dropdown = "Dropdown",
    ContextMenu = "ContextMenu",
    ColorInput = "ColorInput",
    Stepper = "Stepper",
    ScrollableArea = "ScrollableArea",
}

---@class Theme
---@field primary Color
---@field secondary Color
---@field background Color
---@field surface Color
---@field text Color
---@field text_muted Color
---@field accent Color
---@field border Color
---@field hover Color
---@field danger Color
---@field selection Color
---@field highlight Color
---@field placeholder Color
---@field card Color
---@field overlay Color
---@field panel Color
---@field panel_text Color
---@field rule fun(self: Theme, selector: Widget|string, props: table)

---@return Theme
function engine.theme.new() end

--- Activates the given theme globally.
---@param theme Theme
function engine.theme.activate(theme) end

