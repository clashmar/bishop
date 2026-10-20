use bishop::input::{KeyCode, MouseButton};

pub(crate) fn button_from_dom(button: i32) -> Option<MouseButton> {
    match button {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Middle),
        2 => Some(MouseButton::Right),
        _ => None,
    }
}

pub(crate) fn key_from_code(code: &str) -> Option<KeyCode> {
    use KeyCode::*;

    const LETTERS: [KeyCode; 26] = [
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    ];
    const DIGITS: [KeyCode; 10] = [
        Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    ];
    const KEYPAD: [KeyCode; 10] = [
        Kp0, Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7, Kp8, Kp9,
    ];
    const FUNCTIONS: [KeyCode; 25] = [
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18,
        F19, F20, F21, F22, F23, F24, F25,
    ];

    if let Some(letter) = code.strip_prefix("Key").filter(|letter| letter.len() == 1) {
        let character = letter.as_bytes()[0];
        if character.is_ascii_uppercase() {
            return Some(LETTERS[usize::from(character - b'A')]);
        }
    }
    if let Some(index) = code
        .strip_prefix("Digit")
        .filter(|suffix| suffix.len() == 1)
        .and_then(|suffix| suffix.parse::<usize>().ok())
    {
        return Some(DIGITS[index]);
    }
    if let Some(index) = code
        .strip_prefix("Numpad")
        .filter(|suffix| suffix.len() == 1)
        .and_then(|suffix| suffix.parse::<usize>().ok())
    {
        return Some(KEYPAD[index]);
    }
    if let Some(index) = code
        .strip_prefix('F')
        .filter(|suffix| !suffix.starts_with('0'))
        .and_then(|suffix| suffix.parse::<usize>().ok())
        .filter(|index| (1..=FUNCTIONS.len()).contains(index))
    {
        return Some(FUNCTIONS[index - 1]);
    }

    Some(match code {
        "Space" => Space,
        "Quote" => Apostrophe,
        "Comma" => Comma,
        "Minus" => Minus,
        "Period" => Period,
        "Slash" => Slash,
        "Semicolon" => Semicolon,
        "Equal" => Equal,
        "BracketLeft" => LeftBracket,
        "Backslash" => Backslash,
        "BracketRight" => RightBracket,
        "Backquote" => GraveAccent,
        "Escape" => Escape,
        "Enter" => Enter,
        "Tab" => Tab,
        "Backspace" => Backspace,
        "Insert" => Insert,
        "Delete" => Delete,
        "ArrowRight" => Right,
        "ArrowLeft" => Left,
        "ArrowDown" => Down,
        "ArrowUp" => Up,
        "PageUp" => PageUp,
        "PageDown" => PageDown,
        "Home" => Home,
        "End" => End,
        "CapsLock" => CapsLock,
        "ScrollLock" => ScrollLock,
        "NumLock" => NumLock,
        "PrintScreen" => PrintScreen,
        "Pause" => Pause,
        "NumpadDecimal" => KpDecimal,
        "NumpadDivide" => KpDivide,
        "NumpadMultiply" => KpMultiply,
        "NumpadSubtract" => KpSubtract,
        "NumpadAdd" => KpAdd,
        "NumpadEnter" => KpEnter,
        "NumpadEqual" => KpEqual,
        "ShiftLeft" => LeftShift,
        "ShiftRight" => RightShift,
        "ControlLeft" => LeftControl,
        "ControlRight" => RightControl,
        "AltLeft" => LeftAlt,
        "AltRight" => RightAlt,
        "MetaLeft" => LeftSuper,
        "MetaRight" => RightSuper,
        "ContextMenu" => Menu,
        _ => return None,
    })
}

#[cfg(target_os = "emscripten")]
mod callbacks {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    use bishop::input::MouseWheelKind;

    use super::{button_from_dom, key_from_code};
    use crate::browser_loop;

    fn code_from_ptr(ptr: *const c_char) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
    }

    #[unsafe(no_mangle)]
    extern "C" fn bishop_focus_changed(active: i32) -> i32 {
        browser_loop::focus_changed(active != 0);
        1
    }

    #[unsafe(no_mangle)]
    extern "C" fn bishop_key_down(code: *const c_char, key: *const c_char, repeat: i32) -> i32 {
        let Some(mapped) = code_from_ptr(code).as_deref().and_then(key_from_code) else {
            return 0;
        };
        browser_loop::with_input(|input| {
            input.on_key_down(mapped);
            if repeat == 0
                && let Some(text) = code_from_ptr(key)
                && text.chars().count() == 1
                && let Some(character) = text.chars().next()
                && !character.is_control()
            {
                input.on_char(character);
            }
        });
        1
    }

    #[unsafe(no_mangle)]
    extern "C" fn bishop_key_up(code: *const c_char) -> i32 {
        if let Some(mapped) = code_from_ptr(code).as_deref().and_then(key_from_code) {
            browser_loop::with_input(|input| input.on_key_up(mapped));
        }
        1
    }

    #[unsafe(no_mangle)]
    extern "C" fn bishop_mouse_down(button: i32, x: f64, y: f64, time_ms: f64) -> i32 {
        if let Some(button) = button_from_dom(button) {
            browser_loop::with_input(|input| {
                input.on_mouse_move(x as f32, y as f32);
                input.on_mouse_down(button, time_ms / 1000.0, (x as f32, y as f32));
            });
        }
        1
    }

    #[unsafe(no_mangle)]
    extern "C" fn bishop_mouse_up(button: i32) -> i32 {
        if let Some(button) = button_from_dom(button) {
            browser_loop::with_input(|input| input.on_mouse_up(button));
        }
        1
    }

    #[unsafe(no_mangle)]
    extern "C" fn bishop_mouse_move(x: f64, y: f64) -> i32 {
        browser_loop::with_input(|input| input.on_mouse_move(x as f32, y as f32));
        1
    }

    #[unsafe(no_mangle)]
    extern "C" fn bishop_wheel(x: f64, y: f64, mode: i32) -> i32 {
        let kind = match mode {
            0 => MouseWheelKind::Pixel,
            1 => MouseWheelKind::Line,
            _ => return 0,
        };
        browser_loop::with_input(|input| input.on_mouse_wheel(x as f32, y as f32, kind));
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_physical_codes_map_to_existing_bishop_keys() {
        assert_eq!(key_from_code("KeyA"), Some(KeyCode::A));
        assert_eq!(key_from_code("Digit7"), Some(KeyCode::Key7));
        assert_eq!(key_from_code("Numpad7"), Some(KeyCode::Kp7));
        assert_eq!(key_from_code("ArrowLeft"), Some(KeyCode::Left));
        assert_eq!(key_from_code("ShiftRight"), Some(KeyCode::RightShift));
        assert_eq!(key_from_code("F25"), Some(KeyCode::F25));
        assert_eq!(key_from_code("F26"), None);
        assert_eq!(key_from_code("Digit10"), None);
        assert_eq!(key_from_code("Unidentified"), None);
        assert_eq!(key_from_code(""), None);
    }

    #[test]
    fn browser_buttons_map_only_supported_buttons() {
        assert_eq!(button_from_dom(0), Some(MouseButton::Left));
        assert_eq!(button_from_dom(1), Some(MouseButton::Middle));
        assert_eq!(button_from_dom(2), Some(MouseButton::Right));
        assert_eq!(button_from_dom(3), None);
    }
}
