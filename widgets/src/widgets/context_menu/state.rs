use crate::{Rect, WidgetId};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContextMenuState {
    pub open: bool,
    pub rect: Rect,
    pub just_opened: bool,
}

thread_local! {
    pub static STATE: RefCell<HashMap<WidgetId, ContextMenuState>> =
        RefCell::new(HashMap::new());
}

pub fn get(key: WidgetId) -> ContextMenuState {
    STATE.with(|s| *s.borrow().get(&key).unwrap_or(&ContextMenuState::default()))
}

pub fn set(key: WidgetId, value: ContextMenuState) {
    STATE.with(|s| {
        s.borrow_mut().insert(key, value);
    })
}

pub fn any_open() -> bool {
    STATE.with(|s| s.borrow().values().any(|st| st.open))
}

pub fn close_all() {
    STATE.with(|s| {
        for state in s.borrow_mut().values_mut() {
            state.open = false;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_default_for_unknown_id() {
        let id = WidgetId(9999);
        assert_eq!(get(id), ContextMenuState::default());
        STATE.with(|s| s.borrow_mut().remove(&id));
    }

    #[test]
    fn set_and_get_roundtrip() {
        let id = WidgetId(100);
        let state = ContextMenuState {
            open: true,
            rect: Rect::new(10.0, 20.0, 150.0, 90.0),
            just_opened: true,
        };
        set(id, state);
        assert_eq!(get(id), state);
        STATE.with(|s| s.borrow_mut().remove(&id));
    }

    #[test]
    fn close_all_sets_all_open_to_false() {
        let id1 = WidgetId(200);
        let id2 = WidgetId(201);
        set(
            id1,
            ContextMenuState {
                open: true,
                ..Default::default()
            },
        );
        set(
            id2,
            ContextMenuState {
                open: true,
                ..Default::default()
            },
        );
        close_all();
        assert!(!get(id1).open);
        assert!(!get(id2).open);
        STATE.with(|s| s.borrow_mut().remove(&id1));
        STATE.with(|s| s.borrow_mut().remove(&id2));
    }

    #[test]
    fn any_open_returns_false_when_none_open() {
        let id = WidgetId(300);
        set(
            id,
            ContextMenuState {
                open: false,
                ..Default::default()
            },
        );
        assert!(!any_open());
        STATE.with(|s| s.borrow_mut().remove(&id));
    }

    #[test]
    fn any_open_returns_true_when_one_open() {
        let id = WidgetId(301);
        set(
            id,
            ContextMenuState {
                open: true,
                ..Default::default()
            },
        );
        assert!(any_open());
        STATE.with(|s| s.borrow_mut().remove(&id));
    }
}
