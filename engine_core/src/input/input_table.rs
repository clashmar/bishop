// engine_core/src/input/key_table.rs
use crate::input::input_constants::*;
use bishop::prelude::*;

pub fn lua_key_name(s: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;

    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize {
                out.push(ch.to_ascii_uppercase());
                capitalize = false;
            } else {
                out.push(ch);
            }
        } else {
            // Separators start a new word
            capitalize = true;
        }
    }

    // Fallback
    if out.is_empty() {
        s.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    } else {
        out
    }
}

/// All mouse codes
pub static MOUSE_TABLE: &[(&str, MouseButton)] = &[
    (MOUSE_LEFT, MouseButton::Left),
    (MOUSE_MIDDLE, MouseButton::Middle),
    (MOUSE_RIGHT, MouseButton::Right),
];

/// All key codes.
pub static KEY_TABLE: &[(&str, KeyCode)] = &[
    // printable characters
    (SPACE, KeyCode::Space),
    (APOSTROPHE, KeyCode::Apostrophe),
    (COMMA, KeyCode::Comma),
    (MINUS, KeyCode::Minus),
    (PERIOD, KeyCode::Period),
    (SLASH, KeyCode::Slash),
    (KEY0, KeyCode::Key0),
    (KEY1, KeyCode::Key1),
    (KEY2, KeyCode::Key2),
    (KEY3, KeyCode::Key3),
    (KEY4, KeyCode::Key4),
    (KEY5, KeyCode::Key5),
    (KEY6, KeyCode::Key6),
    (KEY7, KeyCode::Key7),
    (KEY8, KeyCode::Key8),
    (KEY9, KeyCode::Key9),
    (SEMICOLON, KeyCode::Semicolon),
    (EQUAL, KeyCode::Equal),
    (A, KeyCode::A),
    (B, KeyCode::B),
    (C, KeyCode::C),
    (D, KeyCode::D),
    (E, KeyCode::E),
    (F, KeyCode::F),
    (G, KeyCode::G),
    (H, KeyCode::H),
    (I, KeyCode::I),
    (J, KeyCode::J),
    (K, KeyCode::K),
    (L, KeyCode::L),
    (M, KeyCode::M),
    (N, KeyCode::N),
    (O, KeyCode::O),
    (P, KeyCode::P),
    (Q, KeyCode::Q),
    (R, KeyCode::R),
    (S, KeyCode::S),
    (T, KeyCode::T),
    (U, KeyCode::U),
    (V, KeyCode::V),
    (W, KeyCode::W),
    (X, KeyCode::X),
    (Y, KeyCode::Y),
    (Z, KeyCode::Z),
    (LEFT_BRACKET, KeyCode::LeftBracket),
    (BACKSLASH, KeyCode::Backslash),
    (RIGHT_BRACKET, KeyCode::RightBracket),
    (GRAVE_ACCENT, KeyCode::GraveAccent),
    (WORLD1, KeyCode::World1),
    (WORLD2, KeyCode::World2),
    // navigation / editing
    (ESCAPE, KeyCode::Escape),
    (ENTER, KeyCode::Enter),
    (TAB, KeyCode::Tab),
    (BACKSPACE, KeyCode::Backspace),
    (INSERT, KeyCode::Insert),
    (DELETE, KeyCode::Delete),
    (RIGHT, KeyCode::Right),
    (LEFT, KeyCode::Left),
    (DOWN, KeyCode::Down),
    (UP, KeyCode::Up),
    (PAGE_UP, KeyCode::PageUp),
    (PAGE_DOWN, KeyCode::PageDown),
    (HOME, KeyCode::Home),
    (END, KeyCode::End),
    // lock / system keys
    (CAPS_LOCK, KeyCode::CapsLock),
    (SCROLL_LOCK, KeyCode::ScrollLock),
    (NUM_LOCK, KeyCode::NumLock),
    (PRINT_SCREEN, KeyCode::PrintScreen),
    (PAUSE, KeyCode::Pause),
    // function keys
    (F1, KeyCode::F1),
    (F2, KeyCode::F2),
    (F3, KeyCode::F3),
    (F4, KeyCode::F4),
    (F5, KeyCode::F5),
    (F6, KeyCode::F6),
    (F7, KeyCode::F7),
    (F8, KeyCode::F8),
    (F9, KeyCode::F9),
    (F10, KeyCode::F10),
    (F11, KeyCode::F11),
    (F12, KeyCode::F12),
    (F13, KeyCode::F13),
    (F14, KeyCode::F14),
    (F15, KeyCode::F15),
    (F16, KeyCode::F16),
    (F17, KeyCode::F17),
    (F18, KeyCode::F18),
    (F19, KeyCode::F19),
    (F20, KeyCode::F20),
    (F21, KeyCode::F21),
    (F22, KeyCode::F22),
    (F23, KeyCode::F23),
    (F24, KeyCode::F24),
    (F25, KeyCode::F25),
    // keypad
    (KP0, KeyCode::Kp0),
    (KP1, KeyCode::Kp1),
    (KP2, KeyCode::Kp2),
    (KP3, KeyCode::Kp3),
    (KP4, KeyCode::Kp4),
    (KP5, KeyCode::Kp5),
    (KP6, KeyCode::Kp6),
    (KP7, KeyCode::Kp7),
    (KP8, KeyCode::Kp8),
    (KP9, KeyCode::Kp9),
    (KP_DECIMAL, KeyCode::KpDecimal),
    (KP_DIVIDE, KeyCode::KpDivide),
    (KP_MULTIPLY, KeyCode::KpMultiply),
    (KP_SUBTRACT, KeyCode::KpSubtract),
    (KP_ADD, KeyCode::KpAdd),
    (KP_ENTER, KeyCode::KpEnter),
    (KP_EQUAL, KeyCode::KpEqual),
    // modifiers
    (LEFT_SHIFT, KeyCode::LeftShift),
    (LEFT_CONTROL, KeyCode::LeftControl),
    (LEFT_ALT, KeyCode::LeftAlt),
    (LEFT_SUPER, KeyCode::LeftSuper),
    (RIGHT_SHIFT, KeyCode::RightShift),
    (RIGHT_CONTROL, KeyCode::RightControl),
    (RIGHT_ALT, KeyCode::RightAlt),
    (RIGHT_SUPER, KeyCode::RightSuper),
    // misc
    (MENU, KeyCode::Menu),
    (BACK, KeyCode::Back),
    (UNKNOWN, KeyCode::Unknown),
];
