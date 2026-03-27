//! Type conversions between winit and bishop types.

use crate::input::{KeyCode, MouseButton};
use crate::window::CursorIcon;
use winit::event::MouseButton as WinitMouseButton;
use winit::keyboard::KeyCode as WinitKeyCode;

/// Converts a winit KeyCode to a bishop KeyCode.
pub fn convert_keycode(key: WinitKeyCode) -> KeyCode {
    match key {
        WinitKeyCode::Space => KeyCode::Space,
        WinitKeyCode::Quote => KeyCode::Apostrophe,
        WinitKeyCode::Comma => KeyCode::Comma,
        WinitKeyCode::Minus => KeyCode::Minus,
        WinitKeyCode::Period => KeyCode::Period,
        WinitKeyCode::Slash => KeyCode::Slash,
        WinitKeyCode::Digit0 => KeyCode::Key0,
        WinitKeyCode::Digit1 => KeyCode::Key1,
        WinitKeyCode::Digit2 => KeyCode::Key2,
        WinitKeyCode::Digit3 => KeyCode::Key3,
        WinitKeyCode::Digit4 => KeyCode::Key4,
        WinitKeyCode::Digit5 => KeyCode::Key5,
        WinitKeyCode::Digit6 => KeyCode::Key6,
        WinitKeyCode::Digit7 => KeyCode::Key7,
        WinitKeyCode::Digit8 => KeyCode::Key8,
        WinitKeyCode::Digit9 => KeyCode::Key9,
        WinitKeyCode::Semicolon => KeyCode::Semicolon,
        WinitKeyCode::Equal => KeyCode::Equal,
        WinitKeyCode::KeyA => KeyCode::A,
        WinitKeyCode::KeyB => KeyCode::B,
        WinitKeyCode::KeyC => KeyCode::C,
        WinitKeyCode::KeyD => KeyCode::D,
        WinitKeyCode::KeyE => KeyCode::E,
        WinitKeyCode::KeyF => KeyCode::F,
        WinitKeyCode::KeyG => KeyCode::G,
        WinitKeyCode::KeyH => KeyCode::H,
        WinitKeyCode::KeyI => KeyCode::I,
        WinitKeyCode::KeyJ => KeyCode::J,
        WinitKeyCode::KeyK => KeyCode::K,
        WinitKeyCode::KeyL => KeyCode::L,
        WinitKeyCode::KeyM => KeyCode::M,
        WinitKeyCode::KeyN => KeyCode::N,
        WinitKeyCode::KeyO => KeyCode::O,
        WinitKeyCode::KeyP => KeyCode::P,
        WinitKeyCode::KeyQ => KeyCode::Q,
        WinitKeyCode::KeyR => KeyCode::R,
        WinitKeyCode::KeyS => KeyCode::S,
        WinitKeyCode::KeyT => KeyCode::T,
        WinitKeyCode::KeyU => KeyCode::U,
        WinitKeyCode::KeyV => KeyCode::V,
        WinitKeyCode::KeyW => KeyCode::W,
        WinitKeyCode::KeyX => KeyCode::X,
        WinitKeyCode::KeyY => KeyCode::Y,
        WinitKeyCode::KeyZ => KeyCode::Z,
        WinitKeyCode::BracketLeft => KeyCode::LeftBracket,
        WinitKeyCode::Backslash => KeyCode::Backslash,
        WinitKeyCode::BracketRight => KeyCode::RightBracket,
        WinitKeyCode::Backquote => KeyCode::GraveAccent,
        WinitKeyCode::Escape => KeyCode::Escape,
        WinitKeyCode::Enter => KeyCode::Enter,
        WinitKeyCode::Tab => KeyCode::Tab,
        WinitKeyCode::Backspace => KeyCode::Backspace,
        WinitKeyCode::Insert => KeyCode::Insert,
        WinitKeyCode::Delete => KeyCode::Delete,
        WinitKeyCode::ArrowRight => KeyCode::Right,
        WinitKeyCode::ArrowLeft => KeyCode::Left,
        WinitKeyCode::ArrowDown => KeyCode::Down,
        WinitKeyCode::ArrowUp => KeyCode::Up,
        WinitKeyCode::PageUp => KeyCode::PageUp,
        WinitKeyCode::PageDown => KeyCode::PageDown,
        WinitKeyCode::Home => KeyCode::Home,
        WinitKeyCode::End => KeyCode::End,
        WinitKeyCode::CapsLock => KeyCode::CapsLock,
        WinitKeyCode::ScrollLock => KeyCode::ScrollLock,
        WinitKeyCode::NumLock => KeyCode::NumLock,
        WinitKeyCode::PrintScreen => KeyCode::PrintScreen,
        WinitKeyCode::Pause => KeyCode::Pause,
        WinitKeyCode::F1 => KeyCode::F1,
        WinitKeyCode::F2 => KeyCode::F2,
        WinitKeyCode::F3 => KeyCode::F3,
        WinitKeyCode::F4 => KeyCode::F4,
        WinitKeyCode::F5 => KeyCode::F5,
        WinitKeyCode::F6 => KeyCode::F6,
        WinitKeyCode::F7 => KeyCode::F7,
        WinitKeyCode::F8 => KeyCode::F8,
        WinitKeyCode::F9 => KeyCode::F9,
        WinitKeyCode::F10 => KeyCode::F10,
        WinitKeyCode::F11 => KeyCode::F11,
        WinitKeyCode::F12 => KeyCode::F12,
        WinitKeyCode::F13 => KeyCode::F13,
        WinitKeyCode::F14 => KeyCode::F14,
        WinitKeyCode::F15 => KeyCode::F15,
        WinitKeyCode::F16 => KeyCode::F16,
        WinitKeyCode::F17 => KeyCode::F17,
        WinitKeyCode::F18 => KeyCode::F18,
        WinitKeyCode::F19 => KeyCode::F19,
        WinitKeyCode::F20 => KeyCode::F20,
        WinitKeyCode::F21 => KeyCode::F21,
        WinitKeyCode::F22 => KeyCode::F22,
        WinitKeyCode::F23 => KeyCode::F23,
        WinitKeyCode::F24 => KeyCode::F24,
        WinitKeyCode::Numpad0 => KeyCode::Kp0,
        WinitKeyCode::Numpad1 => KeyCode::Kp1,
        WinitKeyCode::Numpad2 => KeyCode::Kp2,
        WinitKeyCode::Numpad3 => KeyCode::Kp3,
        WinitKeyCode::Numpad4 => KeyCode::Kp4,
        WinitKeyCode::Numpad5 => KeyCode::Kp5,
        WinitKeyCode::Numpad6 => KeyCode::Kp6,
        WinitKeyCode::Numpad7 => KeyCode::Kp7,
        WinitKeyCode::Numpad8 => KeyCode::Kp8,
        WinitKeyCode::Numpad9 => KeyCode::Kp9,
        WinitKeyCode::NumpadDecimal => KeyCode::KpDecimal,
        WinitKeyCode::NumpadDivide => KeyCode::KpDivide,
        WinitKeyCode::NumpadMultiply => KeyCode::KpMultiply,
        WinitKeyCode::NumpadSubtract => KeyCode::KpSubtract,
        WinitKeyCode::NumpadAdd => KeyCode::KpAdd,
        WinitKeyCode::NumpadEnter => KeyCode::KpEnter,
        WinitKeyCode::NumpadEqual => KeyCode::KpEqual,
        WinitKeyCode::ShiftLeft => KeyCode::LeftShift,
        WinitKeyCode::ControlLeft => KeyCode::LeftControl,
        WinitKeyCode::AltLeft => KeyCode::LeftAlt,
        WinitKeyCode::SuperLeft => KeyCode::LeftSuper,
        WinitKeyCode::ShiftRight => KeyCode::RightShift,
        WinitKeyCode::ControlRight => KeyCode::RightControl,
        WinitKeyCode::AltRight => KeyCode::RightAlt,
        WinitKeyCode::SuperRight => KeyCode::RightSuper,
        WinitKeyCode::ContextMenu => KeyCode::Menu,
        _ => KeyCode::Unknown,
    }
}

/// Converts a winit MouseButton to a bishop MouseButton.
pub fn convert_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
    match button {
        WinitMouseButton::Left => Some(MouseButton::Left),
        WinitMouseButton::Right => Some(MouseButton::Right),
        WinitMouseButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

/// Converts a winit KeyCode to an ASCII character (lowercase).
/// Returns None for non-printable keys.
pub fn keycode_to_char(key: WinitKeyCode, shift: bool) -> Option<char> {
    let c = match key {
        WinitKeyCode::KeyA => 'a',
        WinitKeyCode::KeyB => 'b',
        WinitKeyCode::KeyC => 'c',
        WinitKeyCode::KeyD => 'd',
        WinitKeyCode::KeyE => 'e',
        WinitKeyCode::KeyF => 'f',
        WinitKeyCode::KeyG => 'g',
        WinitKeyCode::KeyH => 'h',
        WinitKeyCode::KeyI => 'i',
        WinitKeyCode::KeyJ => 'j',
        WinitKeyCode::KeyK => 'k',
        WinitKeyCode::KeyL => 'l',
        WinitKeyCode::KeyM => 'm',
        WinitKeyCode::KeyN => 'n',
        WinitKeyCode::KeyO => 'o',
        WinitKeyCode::KeyP => 'p',
        WinitKeyCode::KeyQ => 'q',
        WinitKeyCode::KeyR => 'r',
        WinitKeyCode::KeyS => 's',
        WinitKeyCode::KeyT => 't',
        WinitKeyCode::KeyU => 'u',
        WinitKeyCode::KeyV => 'v',
        WinitKeyCode::KeyW => 'w',
        WinitKeyCode::KeyX => 'x',
        WinitKeyCode::KeyY => 'y',
        WinitKeyCode::KeyZ => 'z',
        WinitKeyCode::Digit0 => {
            if shift {
                ')'
            } else {
                '0'
            }
        }
        WinitKeyCode::Digit1 => {
            if shift {
                '!'
            } else {
                '1'
            }
        }
        WinitKeyCode::Digit2 => {
            if shift {
                '@'
            } else {
                '2'
            }
        }
        WinitKeyCode::Digit3 => {
            if shift {
                '#'
            } else {
                '3'
            }
        }
        WinitKeyCode::Digit4 => {
            if shift {
                '$'
            } else {
                '4'
            }
        }
        WinitKeyCode::Digit5 => {
            if shift {
                '%'
            } else {
                '5'
            }
        }
        WinitKeyCode::Digit6 => {
            if shift {
                '^'
            } else {
                '6'
            }
        }
        WinitKeyCode::Digit7 => {
            if shift {
                '&'
            } else {
                '7'
            }
        }
        WinitKeyCode::Digit8 => {
            if shift {
                '*'
            } else {
                '8'
            }
        }
        WinitKeyCode::Digit9 => {
            if shift {
                '('
            } else {
                '9'
            }
        }
        WinitKeyCode::Space => ' ',
        WinitKeyCode::Comma => {
            if shift {
                '<'
            } else {
                ','
            }
        }
        WinitKeyCode::Period => {
            if shift {
                '>'
            } else {
                '.'
            }
        }
        WinitKeyCode::Slash => {
            if shift {
                '?'
            } else {
                '/'
            }
        }
        WinitKeyCode::Semicolon => {
            if shift {
                ':'
            } else {
                ';'
            }
        }
        WinitKeyCode::Quote => {
            if shift {
                '"'
            } else {
                '\''
            }
        }
        WinitKeyCode::BracketLeft => {
            if shift {
                '{'
            } else {
                '['
            }
        }
        WinitKeyCode::BracketRight => {
            if shift {
                '}'
            } else {
                ']'
            }
        }
        WinitKeyCode::Backslash => {
            if shift {
                '|'
            } else {
                '\\'
            }
        }
        WinitKeyCode::Minus => {
            if shift {
                '_'
            } else {
                '-'
            }
        }
        WinitKeyCode::Equal => {
            if shift {
                '+'
            } else {
                '='
            }
        }
        WinitKeyCode::Backquote => {
            if shift {
                '~'
            } else {
                '`'
            }
        }
        _ => return None,
    };
    if shift && c.is_ascii_lowercase() {
        Some(c.to_ascii_uppercase())
    } else {
        Some(c)
    }
}

/// Converts a bishop CursorIcon to a winit CursorIcon.
pub fn convert_cursor_icon(icon: CursorIcon) -> winit::window::CursorIcon {
    match icon {
        CursorIcon::Default => winit::window::CursorIcon::Default,
        CursorIcon::Pointer => winit::window::CursorIcon::Pointer,
        CursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
        CursorIcon::Move => winit::window::CursorIcon::Move,
        CursorIcon::Text => winit::window::CursorIcon::Text,
        CursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
        CursorIcon::EWResize => winit::window::CursorIcon::EwResize,
        CursorIcon::NSResize => winit::window::CursorIcon::NsResize,
        CursorIcon::NESWResize => winit::window::CursorIcon::NeswResize,
        CursorIcon::NWSEResize => winit::window::CursorIcon::NwseResize,
    }
}
