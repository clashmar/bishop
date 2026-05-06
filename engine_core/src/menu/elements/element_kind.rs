use super::menu_element::MenuElement;
use bishop::prelude::*;

pub trait ElementKind: std::fmt::Debug + Clone {
    fn kind_name(&self) -> &'static str;
    fn default_rect(&self) -> Rect;
    fn is_focusable(&self) -> bool {
        false
    }
    fn wrap(&self, name: String) -> MenuElement;
}
