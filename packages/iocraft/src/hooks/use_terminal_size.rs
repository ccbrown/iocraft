use crate::{
    hooks::{UseState, UseTerminalEvents},
    Hooks, TerminalEvent,
};
use crossterm::terminal;

/// `UseTerminalSize` is a hook that returns the current terminal size.
pub trait UseTerminalSize {
    /// Returns the current terminal size as a tuple of `(width, height)`.
    fn use_terminal_size(&mut self) -> (u16, u16);
}

impl UseTerminalSize for Hooks<'_, '_> {
    fn use_terminal_size(&mut self) -> (u16, u16) {
        let size = self.use_state(|| terminal::size().unwrap_or((0, 0)));
        self.use_terminal_events(move |event| {
            if let TerminalEvent::Resize(width, height) = event {
                size.set((width, height));
            }
        });
        size.get()
    }
}
