#[cfg(not(target_arch = "wasm32"))]
use arboard::Clipboard;
use std::cell::RefCell;

thread_local! {
    static INTERNAL_CLIPBOARD: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Gets text from the system clipboard, falling back to internal buffer.
pub fn clipboard_get_text() -> Option<String> {
    system_clipboard_get_text()
        .filter(|text| !text.is_empty())
        .or_else(|| {
            let internal = INTERNAL_CLIPBOARD.with(|cb| cb.borrow().clone());
            (!internal.is_empty()).then_some(internal)
        })
}

/// Sets text to the system clipboard and internal buffer.
pub fn clipboard_set_text(text: &str) -> bool {
    INTERNAL_CLIPBOARD.with(|cb| *cb.borrow_mut() = text.to_string());

    system_clipboard_set_text(text);

    true
}

#[cfg(not(target_arch = "wasm32"))]
fn system_clipboard_get_text() -> Option<String> {
    Clipboard::new()
        .ok()
        .and_then(|mut clipboard| clipboard.get_text().ok())
}

#[cfg(target_arch = "wasm32")]
fn system_clipboard_get_text() -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn system_clipboard_set_text(text: &str) {
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(text.to_string());
    }
}

#[cfg(target_arch = "wasm32")]
fn system_clipboard_set_text(_text: &str) {}
