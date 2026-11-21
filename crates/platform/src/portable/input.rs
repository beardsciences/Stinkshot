//! Input event types and key labels shared by every backend.

use stinkshot_core::PxPoint;

#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    MouseDown {
        pos: PxPoint,
        button: MouseButton,
    },
    MouseUp {
        pos: PxPoint,
        button: MouseButton,
    },
    /// Key down with a display label, e.g. "Ctrl", "Shift", "P", "Enter", "F5".
    KeyDown {
        label: String,
        vk: u32,
    },
    KeyUp {
        vk: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Layout-independent label for a virtual key (Windows VK codes, as in `InputEvent`):
/// "Ctrl", "Shift", "Enter", "F5", "P", "1", "Num 7", ... None for keys whose name depends
/// on the keyboard layout.
pub(crate) fn fixed_key_label(vk: u32) -> Option<String> {
    let fixed = match vk {
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0D => "Enter",
        0x10 | 0xA0 | 0xA1 => "Shift",
        0x11 | 0xA2 | 0xA3 => "Ctrl",
        0x12 | 0xA4 | 0xA5 => "Alt",
        0x13 => "Pause",
        0x14 => "Caps Lock",
        0x1B => "Esc",
        0x20 => "Space",
        0x21 => "Page Up",
        0x22 => "Page Down",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2C => "Print Screen",
        0x2D => "Insert",
        0x2E => "Delete",
        0x5B | 0x5C => "Win",
        0x5D => "Menu",
        0x6A => "Num *",
        0x6B => "Num +",
        0x6D => "Num -",
        0x6E => "Num .",
        0x6F => "Num /",
        0x90 => "Num Lock",
        0x91 => "Scroll Lock",
        0xBA => ";",
        0xBB => "=",
        0xBC => ",",
        0xBD => "-",
        0xBE => ".",
        0xBF => "/",
        0xC0 => "`",
        0xDB => "[",
        0xDC => "\\",
        0xDD => "]",
        0xDE => "'",
        _ => "",
    };
    if !fixed.is_empty() {
        return Some(fixed.to_string());
    }
    match vk {
        0x30..=0x39 | 0x41..=0x5A => {
            return Some(char::from_u32(vk).map(String::from).unwrap_or_default());
        }
        0x60..=0x69 => return Some(format!("Num {}", vk - 0x60)),
        0x70..=0x87 => return Some(format!("F{}", vk - 0x6F)),
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::input::key_label;

    #[test]
    fn labels() {
        assert_eq!(key_label(0xA2, 0, false), "Ctrl");
        assert_eq!(key_label(0xA1, 0, false), "Shift");
        assert_eq!(key_label(0x50, 0, false), "P");
        assert_eq!(key_label(0x0D, 0, false), "Enter");
        assert_eq!(key_label(0x74, 0, false), "F5");
        assert_eq!(key_label(0x31, 0, false), "1");
        assert_eq!(key_label(0x67, 0, false), "Num 7");
        assert_eq!(key_label(0x5B, 0, false), "Win");
    }
}
