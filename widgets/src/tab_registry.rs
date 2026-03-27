use crate::*;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct TabTarget {
    pub id: WidgetId,
    pub rect: Rect,
    pub is_text_input: bool,
}

thread_local! {
    static TAB_REGISTRY: RefCell<HashMap<WidgetId, TabTarget>> = RefCell::new(HashMap::new());
    static PENDING_TAB: RefCell<Option<PendingTab>> = const { RefCell::new(None) };
}

/// A pending Tab request stored until the end of the frame.
#[derive(Clone, Copy, Debug)]
struct PendingTab {
    from_id: WidgetId,
    shift: bool,
}

pub fn tab_registry_add(id: WidgetId, rect: Rect, is_text_input: bool) {
    TAB_REGISTRY.with(|r| {
        r.borrow_mut().insert(
            id,
            TabTarget {
                id,
                rect,
                is_text_input,
            },
        );
    });
}

pub fn tab_registry_get_sorted() -> Vec<TabTarget> {
    TAB_REGISTRY.with(|r| {
        let mut v: Vec<TabTarget> = r.borrow().values().cloned().collect();
        v.sort_by(|a, b| {
            a.rect
                .y
                .partial_cmp(&b.rect.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.rect
                        .x
                        .partial_cmp(&b.rect.x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        v
    })
}

pub fn tab_registry_clear() {
    TAB_REGISTRY.with(|r| r.borrow_mut().clear());
    PENDING_TAB.with(|p| *p.borrow_mut() = None);
}

pub fn tab_request_pending(from: WidgetId, shift: bool) {
    PENDING_TAB.with(|p| {
        *p.borrow_mut() = Some(PendingTab {
            from_id: from,
            shift,
        })
    });
}

pub fn resolve_pending_tab() {
    let pending = PENDING_TAB.with(|p| p.borrow_mut().take());

    if let Some(PendingTab { from_id, shift }) = pending {
        let sorted = tab_registry_get_sorted();

        let src_idx = sorted.iter().position(|t| t.id == from_id);
        if let Some(idx) = src_idx {
            let len = sorted.len();
            let dest_idx = if shift {
                (idx + len - 1) % len
            } else {
                (idx + 1) % len
            };

            let target = sorted[dest_idx];
            request_focus(target.id, target.is_text_input);
        } else {
            debug_assert!(
                false,
                "Widget {:?} pressed Tab but is not in the tab registry",
                from_id
            );
        }
    }
}
