use crate::theme::{FieldMapping, Theme, WidgetTheme, WidgetType};
use strum::VariantNames;

/// Generates [`WidgetType::map_theme`] and [`WidgetType::theme_fields`].
macro_rules! impl_widget_theme_mappings {
    (
        $(
            $widget:ident: [
                $( ($r:ident, $d:literal) ),+ $(,)?
            ]
        );+ $(;)?
    ) => {
        impl WidgetType {
            /// Maps a [`Theme`] to a [`WidgetTheme`] for this widget type.
            pub fn map_theme(&self, theme: &Theme) -> WidgetTheme {
                match self {
                    $(
                        WidgetType::$widget => WidgetTheme {
                            $($r: Some(theme.$r),)+
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
                            $((stringify!($r), stringify!($r), $d),)+
                        ],
                    )+
                }
            }
        }
    };
}

impl_widget_theme_mappings! {
    Button: [
        (primary, "Button fill"),
        (text, "Label text"),
        (text_muted, "Blocked state text"),
        (border, "Outline"),
        (hover, "Hover overlay"),
    ];
    Slider: [
        (primary, "Handle fill"),
        (background, "Track and label background"),
        (surface, "Track gutter"),
        (border, "Widget outline and label divider"),
        (hover, "Focused label background"),
        (highlight, "Focused outline"),
    ];
    Checkbox: [
        (background, "Box fill"),
        (border, "Box outline"),
        (primary, "Check mark"),
    ];
    TextInput: [
        (background, "Field fill"),
        (accent, "Selection highlight"),
        (border, "Field outline"),
        (text, "Input text"),
    ];
    NumberInput: [
        (background, "Field fill"),
        (accent, "Selection highlight"),
        (border, "Field outline"),
        (text, "Input text"),
    ];
    Panel: [
        (panel, "Panel surface"),
        (border, "Panel outline"),
    ];
    Dropdown: [
        (surface, "List background"),
        (text, "Entry text"),
        (border, "List border"),
        (hover, "Entry hover"),
    ];
    ContextMenu: [
        (surface, "Menu background"),
        (text, "Menu text"),
        (border, "Menu border"),
        (hover, "Entry hover"),
    ];
    ColorInput: [
        (surface, "Field fill"),
        (border, "Field outline"),
        (accent, "Selection highlight"),
        (text, "Display text"),
    ];
    Stepper: [
        (text, "Value text"),
        (border, "Field outline"),
    ];
    ScrollableArea: [
        (surface, "Scrollbar track"),
        (text, "Scrollbar thumb active"),
        (text_muted, "Scrollbar thumb idle"),
    ];
    Label: [
        (text, "Label text"),
    ];
}

impl WidgetType {
    /// Whether this widget has a corresponding menu element and is usable from Lua.
    pub fn is_exposed_to_lua(&self) -> bool {
        matches!(
            self,
            WidgetType::Button | 
            WidgetType::Slider | 
            WidgetType::Label | 
            WidgetType::Panel
        )
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
