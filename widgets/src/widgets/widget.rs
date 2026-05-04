use crate::theme::{Theme, WidgetType, WidgetTheme};

/// Common widget fields shared by all widgets.
#[derive(Default)]
pub struct WidgetBase {
    pub class_name: Option<String>,
    pub style_id: Option<String>,
    pub blocked: bool,
    pub visuals: WidgetTheme,
}

/// Common API shared by all widgets.
pub trait Widget: Sized {
    fn widget_type() -> WidgetType;
    fn base_mut(&mut self) -> &mut WidgetBase;
    fn map_theme(theme: &Theme) -> WidgetTheme;

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

    fn visuals(mut self, visuals: WidgetTheme) -> Self {
        self.base_mut().visuals = visuals;
        self
    }
}
