//! Backend-agnostic input event types.
//!
//! These mirror the key and mouse model exposed by common terminal libraries,
//! but are owned by iocraft so that rendering/input backends do not have to
//! depend on any particular one. When the `crossterm` feature is enabled,
//! `From` conversions from the corresponding `crossterm::event` types are
//! provided.

use bitflags::bitflags;
use std::fmt;

/// Represents a key on the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub enum KeyCode {
    /// Backspace key (Delete on macOS, Backspace on other platforms).
    Backspace,
    /// Enter key.
    Enter,
    /// Left arrow key.
    Left,
    /// Right arrow key.
    Right,
    /// Up arrow key.
    Up,
    /// Down arrow key.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page up key.
    PageUp,
    /// Page down key.
    PageDown,
    /// Tab key.
    Tab,
    /// Shift + Tab key.
    BackTab,
    /// Delete key.
    Delete,
    /// Insert key.
    Insert,
    /// Function key, e.g. `KeyCode::F(1)` for F1.
    F(u8),
    /// A character key.
    Char(char),
    /// Null.
    Null,
    /// Escape key.
    Esc,
    /// Caps Lock key (requires keyboard enhancement).
    CapsLock,
    /// Scroll Lock key (requires keyboard enhancement).
    ScrollLock,
    /// Num Lock key (requires keyboard enhancement).
    NumLock,
    /// Print Screen key (requires keyboard enhancement).
    PrintScreen,
    /// Pause key (requires keyboard enhancement).
    Pause,
    /// Menu key (requires keyboard enhancement).
    Menu,
    /// The "Begin" key, often the keypad 5 with Num Lock on (requires keyboard enhancement).
    KeypadBegin,
    /// A media key (requires keyboard enhancement).
    Media(MediaKeyCode),
    /// A modifier key (requires keyboard enhancement).
    Modifier(ModifierKeyCode),
}

/// Represents a media key (as part of [`KeyCode::Media`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub enum MediaKeyCode {
    /// Play media key.
    Play,
    /// Pause media key.
    Pause,
    /// Play/Pause media key.
    PlayPause,
    /// Reverse media key.
    Reverse,
    /// Stop media key.
    Stop,
    /// Fast-forward media key.
    FastForward,
    /// Rewind media key.
    Rewind,
    /// Next-track media key.
    TrackNext,
    /// Previous-track media key.
    TrackPrevious,
    /// Record media key.
    Record,
    /// Lower-volume media key.
    LowerVolume,
    /// Raise-volume media key.
    RaiseVolume,
    /// Mute media key.
    MuteVolume,
}

/// Represents a modifier key (as part of [`KeyCode::Modifier`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub enum ModifierKeyCode {
    /// Left Shift key.
    LeftShift,
    /// Left Control key.
    LeftControl,
    /// Left Alt key.
    LeftAlt,
    /// Left Super key.
    LeftSuper,
    /// Left Hyper key.
    LeftHyper,
    /// Left Meta key.
    LeftMeta,
    /// Right Shift key.
    RightShift,
    /// Right Control key.
    RightControl,
    /// Right Alt key.
    RightAlt,
    /// Right Super key.
    RightSuper,
    /// Right Hyper key.
    RightHyper,
    /// Right Meta key.
    RightMeta,
    /// Iso Level3 Shift key.
    IsoLevel3Shift,
    /// Iso Level5 Shift key.
    IsoLevel5Shift,
}

bitflags! {
    /// Represents key modifiers (shift, control, alt, etc.).
    #[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
    pub struct KeyModifiers: u8 {
        /// The shift key.
        const SHIFT = 0b0000_0001;
        /// The control key.
        const CONTROL = 0b0000_0010;
        /// The alt key.
        const ALT = 0b0000_0100;
        /// The super key.
        const SUPER = 0b0000_1000;
        /// The hyper key.
        const HYPER = 0b0001_0000;
        /// The meta key.
        const META = 0b0010_0000;
        /// No modifiers.
        const NONE = 0b0000_0000;
    }
}

/// Represents the kind of a key event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub enum KeyEventKind {
    /// The key was pressed.
    Press,
    /// The key is being held down and repeating.
    Repeat,
    /// The key was released.
    Release,
}

/// Represents a mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
}

/// Represents the kind of a mouse event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub enum MouseEventKind {
    /// A mouse button was pressed.
    Down(MouseButton),
    /// A mouse button was released.
    Up(MouseButton),
    /// The mouse was moved with a button held down.
    Drag(MouseButton),
    /// The mouse was moved with no button held down.
    Moved,
    /// The mouse wheel was scrolled down (towards the user).
    ScrollDown,
    /// The mouse wheel was scrolled up (away from the user).
    ScrollUp,
    /// The mouse wheel was scrolled left.
    ScrollLeft,
    /// The mouse wheel was scrolled right.
    ScrollRight,
}

impl fmt::Display for KeyCode {
    /// Formats the `KeyCode` using the given formatter. The output matches
    /// crossterm's, including its platform-specific key names (e.g. the
    /// Backspace key is displayed as "Delete" on macOS).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(target_os = "macos")]
            KeyCode::Backspace => write!(f, "Delete"),
            #[cfg(target_os = "macos")]
            KeyCode::Delete => write!(f, "Fwd Del"),
            #[cfg(not(target_os = "macos"))]
            KeyCode::Backspace => write!(f, "Backspace"),
            #[cfg(not(target_os = "macos"))]
            KeyCode::Delete => write!(f, "Del"),
            #[cfg(target_os = "macos")]
            KeyCode::Enter => write!(f, "Return"),
            #[cfg(not(target_os = "macos"))]
            KeyCode::Enter => write!(f, "Enter"),
            KeyCode::Left => write!(f, "Left"),
            KeyCode::Right => write!(f, "Right"),
            KeyCode::Up => write!(f, "Up"),
            KeyCode::Down => write!(f, "Down"),
            KeyCode::Home => write!(f, "Home"),
            KeyCode::End => write!(f, "End"),
            KeyCode::PageUp => write!(f, "Page Up"),
            KeyCode::PageDown => write!(f, "Page Down"),
            KeyCode::Tab => write!(f, "Tab"),
            KeyCode::BackTab => write!(f, "Back Tab"),
            KeyCode::Insert => write!(f, "Insert"),
            KeyCode::F(n) => write!(f, "F{}", n),
            KeyCode::Char(c) => match c {
                // special case for non-visible characters
                ' ' => write!(f, "Space"),
                c => write!(f, "{}", c),
            },
            KeyCode::Null => write!(f, "Null"),
            KeyCode::Esc => write!(f, "Esc"),
            KeyCode::CapsLock => write!(f, "Caps Lock"),
            KeyCode::ScrollLock => write!(f, "Scroll Lock"),
            KeyCode::NumLock => write!(f, "Num Lock"),
            KeyCode::PrintScreen => write!(f, "Print Screen"),
            KeyCode::Pause => write!(f, "Pause"),
            KeyCode::Menu => write!(f, "Menu"),
            KeyCode::KeypadBegin => write!(f, "Begin"),
            KeyCode::Media(media) => write!(f, "{}", media),
            KeyCode::Modifier(modifier) => write!(f, "{}", modifier),
        }
    }
}

impl fmt::Display for KeyModifiers {
    /// Formats the key modifiers joined by a `+` character, matching
    /// crossterm's output, including its platform-specific modifier names
    /// (e.g. the super key is displayed as "Command" on macOS).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for modifier in self.iter() {
            if !first {
                f.write_str("+")?;
            }
            first = false;
            match modifier {
                KeyModifiers::SHIFT => f.write_str("Shift")?,
                #[cfg(unix)]
                KeyModifiers::CONTROL => f.write_str("Control")?,
                #[cfg(windows)]
                KeyModifiers::CONTROL => f.write_str("Ctrl")?,
                #[cfg(target_os = "macos")]
                KeyModifiers::ALT => f.write_str("Option")?,
                #[cfg(not(target_os = "macos"))]
                KeyModifiers::ALT => f.write_str("Alt")?,
                #[cfg(target_os = "macos")]
                KeyModifiers::SUPER => f.write_str("Command")?,
                #[cfg(target_os = "windows")]
                KeyModifiers::SUPER => f.write_str("Windows")?,
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                KeyModifiers::SUPER => f.write_str("Super")?,
                KeyModifiers::HYPER => f.write_str("Hyper")?,
                KeyModifiers::META => f.write_str("Meta")?,
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}

impl fmt::Display for MediaKeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaKeyCode::Play => write!(f, "Play"),
            MediaKeyCode::Pause => write!(f, "Pause"),
            MediaKeyCode::PlayPause => write!(f, "Play/Pause"),
            MediaKeyCode::Reverse => write!(f, "Reverse"),
            MediaKeyCode::Stop => write!(f, "Stop"),
            MediaKeyCode::FastForward => write!(f, "Fast Forward"),
            MediaKeyCode::Rewind => write!(f, "Rewind"),
            MediaKeyCode::TrackNext => write!(f, "Next Track"),
            MediaKeyCode::TrackPrevious => write!(f, "Previous Track"),
            MediaKeyCode::Record => write!(f, "Record"),
            MediaKeyCode::LowerVolume => write!(f, "Lower Volume"),
            MediaKeyCode::RaiseVolume => write!(f, "Raise Volume"),
            MediaKeyCode::MuteVolume => write!(f, "Mute Volume"),
        }
    }
}

impl fmt::Display for ModifierKeyCode {
    /// Formats the modifier key using the given formatter. The output matches
    /// crossterm's, including its platform-specific key names.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModifierKeyCode::LeftShift => write!(f, "Left Shift"),
            ModifierKeyCode::LeftHyper => write!(f, "Left Hyper"),
            ModifierKeyCode::LeftMeta => write!(f, "Left Meta"),
            ModifierKeyCode::RightShift => write!(f, "Right Shift"),
            ModifierKeyCode::RightHyper => write!(f, "Right Hyper"),
            ModifierKeyCode::RightMeta => write!(f, "Right Meta"),
            ModifierKeyCode::IsoLevel3Shift => write!(f, "Iso Level 3 Shift"),
            ModifierKeyCode::IsoLevel5Shift => write!(f, "Iso Level 5 Shift"),
            #[cfg(target_os = "macos")]
            ModifierKeyCode::LeftControl => write!(f, "Left Control"),
            #[cfg(not(target_os = "macos"))]
            ModifierKeyCode::LeftControl => write!(f, "Left Ctrl"),
            #[cfg(target_os = "macos")]
            ModifierKeyCode::LeftAlt => write!(f, "Left Option"),
            #[cfg(not(target_os = "macos"))]
            ModifierKeyCode::LeftAlt => write!(f, "Left Alt"),
            #[cfg(target_os = "macos")]
            ModifierKeyCode::LeftSuper => write!(f, "Left Command"),
            #[cfg(target_os = "windows")]
            ModifierKeyCode::LeftSuper => write!(f, "Left Windows"),
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            ModifierKeyCode::LeftSuper => write!(f, "Left Super"),
            #[cfg(target_os = "macos")]
            ModifierKeyCode::RightControl => write!(f, "Right Control"),
            #[cfg(not(target_os = "macos"))]
            ModifierKeyCode::RightControl => write!(f, "Right Ctrl"),
            #[cfg(target_os = "macos")]
            ModifierKeyCode::RightAlt => write!(f, "Right Option"),
            #[cfg(not(target_os = "macos"))]
            ModifierKeyCode::RightAlt => write!(f, "Right Alt"),
            #[cfg(target_os = "macos")]
            ModifierKeyCode::RightSuper => write!(f, "Right Command"),
            #[cfg(target_os = "windows")]
            ModifierKeyCode::RightSuper => write!(f, "Right Windows"),
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            ModifierKeyCode::RightSuper => write!(f, "Right Super"),
        }
    }
}

#[cfg(feature = "crossterm")]
mod crossterm_conv {
    use super::*;
    use crossterm::event as ct;

    impl From<ct::MediaKeyCode> for MediaKeyCode {
        fn from(c: ct::MediaKeyCode) -> Self {
            match c {
                ct::MediaKeyCode::Play => Self::Play,
                ct::MediaKeyCode::Pause => Self::Pause,
                ct::MediaKeyCode::PlayPause => Self::PlayPause,
                ct::MediaKeyCode::Reverse => Self::Reverse,
                ct::MediaKeyCode::Stop => Self::Stop,
                ct::MediaKeyCode::FastForward => Self::FastForward,
                ct::MediaKeyCode::Rewind => Self::Rewind,
                ct::MediaKeyCode::TrackNext => Self::TrackNext,
                ct::MediaKeyCode::TrackPrevious => Self::TrackPrevious,
                ct::MediaKeyCode::Record => Self::Record,
                ct::MediaKeyCode::LowerVolume => Self::LowerVolume,
                ct::MediaKeyCode::RaiseVolume => Self::RaiseVolume,
                ct::MediaKeyCode::MuteVolume => Self::MuteVolume,
            }
        }
    }

    impl From<ct::ModifierKeyCode> for ModifierKeyCode {
        fn from(c: ct::ModifierKeyCode) -> Self {
            match c {
                ct::ModifierKeyCode::LeftShift => Self::LeftShift,
                ct::ModifierKeyCode::LeftControl => Self::LeftControl,
                ct::ModifierKeyCode::LeftAlt => Self::LeftAlt,
                ct::ModifierKeyCode::LeftSuper => Self::LeftSuper,
                ct::ModifierKeyCode::LeftHyper => Self::LeftHyper,
                ct::ModifierKeyCode::LeftMeta => Self::LeftMeta,
                ct::ModifierKeyCode::RightShift => Self::RightShift,
                ct::ModifierKeyCode::RightControl => Self::RightControl,
                ct::ModifierKeyCode::RightAlt => Self::RightAlt,
                ct::ModifierKeyCode::RightSuper => Self::RightSuper,
                ct::ModifierKeyCode::RightHyper => Self::RightHyper,
                ct::ModifierKeyCode::RightMeta => Self::RightMeta,
                ct::ModifierKeyCode::IsoLevel3Shift => Self::IsoLevel3Shift,
                ct::ModifierKeyCode::IsoLevel5Shift => Self::IsoLevel5Shift,
            }
        }
    }

    impl From<ct::KeyCode> for KeyCode {
        fn from(c: ct::KeyCode) -> Self {
            match c {
                ct::KeyCode::Backspace => Self::Backspace,
                ct::KeyCode::Enter => Self::Enter,
                ct::KeyCode::Left => Self::Left,
                ct::KeyCode::Right => Self::Right,
                ct::KeyCode::Up => Self::Up,
                ct::KeyCode::Down => Self::Down,
                ct::KeyCode::Home => Self::Home,
                ct::KeyCode::End => Self::End,
                ct::KeyCode::PageUp => Self::PageUp,
                ct::KeyCode::PageDown => Self::PageDown,
                ct::KeyCode::Tab => Self::Tab,
                ct::KeyCode::BackTab => Self::BackTab,
                ct::KeyCode::Delete => Self::Delete,
                ct::KeyCode::Insert => Self::Insert,
                ct::KeyCode::F(n) => Self::F(n),
                ct::KeyCode::Char(c) => Self::Char(c),
                ct::KeyCode::Null => Self::Null,
                ct::KeyCode::Esc => Self::Esc,
                ct::KeyCode::CapsLock => Self::CapsLock,
                ct::KeyCode::ScrollLock => Self::ScrollLock,
                ct::KeyCode::NumLock => Self::NumLock,
                ct::KeyCode::PrintScreen => Self::PrintScreen,
                ct::KeyCode::Pause => Self::Pause,
                ct::KeyCode::Menu => Self::Menu,
                ct::KeyCode::KeypadBegin => Self::KeypadBegin,
                ct::KeyCode::Media(m) => Self::Media(m.into()),
                ct::KeyCode::Modifier(m) => Self::Modifier(m.into()),
            }
        }
    }

    impl From<ct::KeyModifiers> for KeyModifiers {
        fn from(m: ct::KeyModifiers) -> Self {
            // Bit layout is identical, so bits round-trip directly.
            Self::from_bits_truncate(m.bits())
        }
    }

    impl From<ct::KeyEventKind> for KeyEventKind {
        fn from(k: ct::KeyEventKind) -> Self {
            match k {
                ct::KeyEventKind::Press => Self::Press,
                ct::KeyEventKind::Repeat => Self::Repeat,
                ct::KeyEventKind::Release => Self::Release,
            }
        }
    }

    impl From<ct::MouseButton> for MouseButton {
        fn from(b: ct::MouseButton) -> Self {
            match b {
                ct::MouseButton::Left => Self::Left,
                ct::MouseButton::Right => Self::Right,
                ct::MouseButton::Middle => Self::Middle,
            }
        }
    }

    impl From<ct::MouseEventKind> for MouseEventKind {
        fn from(k: ct::MouseEventKind) -> Self {
            match k {
                ct::MouseEventKind::Down(b) => Self::Down(b.into()),
                ct::MouseEventKind::Up(b) => Self::Up(b.into()),
                ct::MouseEventKind::Drag(b) => Self::Drag(b.into()),
                ct::MouseEventKind::Moved => Self::Moved,
                ct::MouseEventKind::ScrollDown => Self::ScrollDown,
                ct::MouseEventKind::ScrollUp => Self::ScrollUp,
                ct::MouseEventKind::ScrollLeft => Self::ScrollLeft,
                ct::MouseEventKind::ScrollRight => Self::ScrollRight,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn display_matches_crossterm() {
            let codes = [
                ct::KeyCode::Backspace,
                ct::KeyCode::Enter,
                ct::KeyCode::Delete,
                ct::KeyCode::F(12),
                ct::KeyCode::Char(' '),
                ct::KeyCode::Char('q'),
                ct::KeyCode::PageUp,
                ct::KeyCode::Media(ct::MediaKeyCode::PlayPause),
                ct::KeyCode::Modifier(ct::ModifierKeyCode::LeftSuper),
            ];
            for code in codes {
                assert_eq!(
                    KeyCode::from(code).to_string(),
                    code.to_string(),
                    "display mismatch for {code:?}"
                );
            }
            let modifiers =
                ct::KeyModifiers::SHIFT | ct::KeyModifiers::CONTROL | ct::KeyModifiers::ALT;
            assert_eq!(
                KeyModifiers::from(modifiers).to_string(),
                modifiers.to_string()
            );
        }

        #[test]
        fn key_modifiers_bits_match_crossterm() {
            assert_eq!(KeyModifiers::SHIFT.bits(), ct::KeyModifiers::SHIFT.bits());
            assert_eq!(
                KeyModifiers::CONTROL.bits(),
                ct::KeyModifiers::CONTROL.bits()
            );
            assert_eq!(KeyModifiers::ALT.bits(), ct::KeyModifiers::ALT.bits());
            assert_eq!(KeyModifiers::SUPER.bits(), ct::KeyModifiers::SUPER.bits());
            assert_eq!(KeyModifiers::HYPER.bits(), ct::KeyModifiers::HYPER.bits());
            assert_eq!(KeyModifiers::META.bits(), ct::KeyModifiers::META.bits());
        }
    }
}
