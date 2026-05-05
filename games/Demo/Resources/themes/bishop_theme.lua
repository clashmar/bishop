-- Sample theme for the Bishop engine.
local t = engine.theme.new()

t.primary     = Color.from_hex("#22b1d0")
t.secondary   = Color.rgba(0.133, 0.125, 0.204, 1.0)
t.background  = Color.from_hex("#000000")
t.surface     = Color.from_hex("#1a1a2e")
t.text        = Color.from_hex("#E0E8EA")
t.text_muted  = Color.from_hex("#7A9AA3")
t.accent      = Color.from_hex("#D95763")
t.border      = Color.from_hex("#3a3a5c")
t.hover       = Color.from_hex("#22b1d0", 0.25)
t.danger      = Color.from_hex("#C0392B")
t.selection   = Color.from_hex("#22b1d0", 0.3)
t.highlight   = Color.from_hex("#E8FBFF")
t.placeholder = Color.from_hex("#22b1d0", 0.22)
t.overlay     = Color.from_hex("#000000", 0.6)
t.panel       = Color.from_hex("#1a1a2e")
t.panel_text  = Color.from_hex("#E0E8EA")

t:rule(Widget.Button, { primary = t.danger })
t:rule(Widget.Slider, { background = t.panel })
t:rule(Widget.Panel,  { panel = t.danger })

t:rule(".danger",  { text = t.danger })
t:rule("#confirm", { panel = t.highlight })

return t
