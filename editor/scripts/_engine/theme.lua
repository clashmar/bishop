-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta

---@class ThemeApi
engine.theme = {}

---@enum Widget
Widget = {
    Button = "Button",
    Slider = "Slider",
    Panel = "Panel",
    Label = "Label",
}

---@class Theme
---@field primary Color -- Brand accent; interactive control fill
---@field secondary Color -- Alternate accent for secondary emphasis
---@field background Color -- Page-level background
---@field surface Color -- Elevated surfaces above background
---@field text Color -- Primary text for readability
---@field text_muted Color -- Subdued text for secondary or disabled content
---@field accent Color -- Emphasized accent for active or focused elements
---@field border Color -- Outline color for widgets and containers
---@field hover Color -- Hover or pressed overlay
---@field danger Color -- Error, destructive action, or critical warning
---@field selection Color -- Text-selection highlight background
---@field highlight Color -- Transient highlight for active or matching elements
---@field placeholder Color -- Fill for placeholder or ghost content
---@field overlay Color -- Scrim or backdrop for overlays and modals
---@field panel Color -- Large surface for panels and sidebars
---@field panel_text Color -- Text rendered on panel surfaces
---@field rule fun(self: Theme, selector: Widget|string, props: table)

---@return Theme
function engine.theme.new() end

--- Activates the given theme globally.
---@param theme Theme
function engine.theme.activate(theme) end

