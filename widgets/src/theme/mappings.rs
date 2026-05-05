use crate::theme::{FieldMapping, Theme, WidgetTheme, WidgetType};
use strum::VariantNames;

/// Generates [`WidgetType::map_theme`] and [`WidgetType::theme_fields`].
macro_rules! impl_widget_theme_mappings {
    (
        $(
            $widget:ident: [
                $( ($f:ident, $r:ident, $d:literal) ),+ $(,)?
            ]
        );+ $(;)?
    ) => {
        impl WidgetType {
            /// Maps a [`Theme`] to a [`WidgetTheme`] for this widget type.
            pub fn map_theme(&self, theme: &Theme) -> WidgetTheme {
                match self {
                    $(
                        WidgetType::$widget => WidgetTheme {
                            $($f: Some(theme.$r),)+
                            ..Default::default()
                        },
                    )+
                }
            }

            /// Returns the per-widget theme field mappings for auto-generating docs.
            pub fn theme_fields(&self) -> FieldMapping {
                match self {
                    $(
                        WidgetType::$widget => &[
                            $((stringify!($f), stringify!($r), $d),)+
                        ],
                    )+
                }
            }
        }
    };
}

impl_widget_theme_mappings! {
    Button: [
        (primary, primary, "Button fill"),
        (text, text, "Label text"),
        (text_muted, text_muted, "Blocked state text"),
        (border, border, "Outline"),
        (hover, hover, "Hover overlay"),
    ];
    Slider: [
        (primary, primary, "Handle fill"),
        (background, background, "Track fill"),
        (surface, surface, "Track gutter"),
        (border, border, "Handle outline"),
        (hover, hover, "Handle when dragging"),
    ];
    Checkbox: [
        (background, background, "Box fill"),
        (border, border, "Box outline"),
        (primary, primary, "Check mark"),
    ];
    TextInput: [
        (background, background, "Field fill"),
        (accent, accent, "Selection highlight"),
        (border, border, "Field outline"),
        (text, text, "Input text"),
    ];
    NumberInput: [
        (background, background, "Field fill"),
        (accent, accent, "Selection highlight"),
        (border, border, "Field outline"),
        (text, text, "Input text"),
    ];
    Dropdown: [
        (background, surface, "List background"),
        (text, text, "Entry text"),
        (border, border, "List border"),
        (hover, hover, "Entry hover"),
    ];
    ContextMenu: [
        (background, surface, "Menu background"),
        (text, text, "Menu text"),
        (border, border, "Menu border"),
        (hover, hover, "Entry hover"),
    ];
    ColorInput: [
        (background, surface, "Field fill"),
        (border, border, "Field outline"),
        (accent, accent, "Selection highlight"),
        (text, text, "Display text"),
    ];
    Stepper: [
        (text, text, "Value text"),
        (border, border, "Field outline"),
    ];
    ScrollableArea: [
        (surface, surface, "Scrollbar track"),
        (text, text, "Scrollbar thumb active"),
        (text_muted, text_muted, "Scrollbar thumb idle"),
    ];
}

impl WidgetType {
    /// Whether this widget has a corresponding menu element and is usable from Lua.
    pub fn is_exposed_to_lua(&self) -> bool {
        matches!(self, WidgetType::Button | WidgetType::Slider)
    }
}

/// Generates a markdown reference documenting every widget's theme color-role mappings.
pub fn generate_theme_reference_markdown() -> String {
    use std::fmt::Write;

    let mut out = String::new();

    writeln!(out, "# Theme Color Reference").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "Set theme colors globally, then override per-widget with `t:rule()`."
    )
    .unwrap();
    writeln!(out).unwrap();

    writeln!(out, "## Global color roles").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "| Role | What it colors |").unwrap();
    writeln!(out, "|------|---------------|").unwrap();
    macro_rules! role_row {
        ($role:ident, $desc:literal) => {
            writeln!(out, "| `{}` | {} |", stringify!($role), $desc).unwrap();
        };
    }
    crate::each_color_field_desc!(role_row);
    writeln!(out).unwrap();

    writeln!(out, "## Per-widget rule fields").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "These are the fields you can set in `t:rule(Widget.X, {{ ... }})`."
    )
    .unwrap();
    writeln!(out).unwrap();

    for &name in WidgetType::VARIANTS {
        let wt: WidgetType = name.parse().unwrap();
        if !wt.is_exposed_to_lua() {
            continue;
        }
        let fields = wt.theme_fields();
        if fields.is_empty() {
            continue;
        }
        writeln!(out, "### {}", name).unwrap();
        writeln!(out).unwrap();
        writeln!(out, "| Field | What it colors |").unwrap();
        writeln!(out, "|-------|---------------|").unwrap();
        for (field, _role, desc) in fields {
            writeln!(out, "| `{}` | {} |", field, desc).unwrap();
        }
        writeln!(out).unwrap();
    }

    out
}
