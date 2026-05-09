use crate::theme::{Theme, WidgetTheme, WidgetType};

/// Common widget fields shared by all widgets.
#[derive(Default)]
pub struct WidgetBase {
    pub class_name: Option<String>,
    pub style_id: Option<String>,
    pub blocked: bool,
    pub overrides: WidgetTheme,
}

/// Common API shared by all widgets.
pub trait Widget: Sized {
    fn widget_type() -> WidgetType;
    fn base_mut(&mut self) -> &mut WidgetBase;

    /// Maps the active theme into this widget's color slots.
    fn map_theme(theme: &Theme) -> WidgetTheme {
        Self::widget_type().map_theme(theme)
    }

    fn apply_selectors(mut self, class: Option<&str>, id: Option<&str>) -> Self {
        if let Some(c) = class {
            self.base_mut().class_name = Some(c.to_string());
        }
        if let Some(i) = id {
            self.base_mut().style_id = Some(i.to_string());
        }
        self
    }

    fn class(mut self, cls: impl Into<String>) -> Self {
        self.base_mut().class_name = Some(cls.into());
        self
    }

    fn style_id(mut self, id: impl Into<String>) -> Self {
        self.base_mut().style_id = Some(id.into());
        self
    }

    fn blocked(mut self, blocked: bool) -> Self {
        self.base_mut().blocked = blocked;
        self
    }

    fn overrides(mut self, overrides: WidgetTheme) -> Self {
        self.base_mut().overrides = overrides;
        self
    }
}
