//! The [`TerminalBackend`] trait: iocraft's seam between the renderer and a
//! concrete terminal.
//!
//! All terminal interaction during a render loop flows through this trait. The
//! built-in `CrosstermBackend` renders ANSI to stdout/stderr; alternative
//! backends (a GUI grid, a test harness, a different terminal library) can
//! implement the same surface without pulling in crossterm.
//!
//! This seam is crate-internal for now; it becomes public API once there is a
//! public entry point that accepts a custom backend.

use crate::{canvas::Canvas, element::Output, terminal::TerminalEvent};
use futures::stream::BoxStream;
use std::{borrow::Cow, io};

/// Returns the current terminal size as `(columns, rows)`, if it can be
/// determined outside of an active render loop.
///
/// Backed by crossterm when the `crossterm` feature is enabled; otherwise
/// reports the size as unavailable.
pub(crate) fn terminal_size() -> io::Result<(u16, u16)> {
    #[cfg(feature = "crossterm")]
    {
        crossterm::terminal::size()
    }
    #[cfg(not(feature = "crossterm"))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "terminal size is unavailable without a terminal backend",
        ))
    }
}

/// A single chunk of passthrough output to be emitted *above* the rendered UI
/// via [`use_output`](crate::hooks::UseOutput).
///
/// The backend decides how to place this relative to the rendered canvas. ANSI
/// backends move the cursor above the canvas, write the text, and reflow; a
/// grid backend might insert scrollback lines. The backend also chooses the
/// line terminator (e.g. `\r\n` in raw mode) and must translate newlines
/// embedded in the content to it.
// Only the crossterm backend's `print_above` reads the fields today.
#[cfg_attr(not(feature = "crossterm"), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) struct Passthrough<'a> {
    /// Which standard stream this text should be written to.
    pub stream: Output,
    /// The text to write. May contain embedded newlines. When
    /// [`newline`](Self::newline) is `false` it never ends with one:
    /// `use_output` normalizes a trailing newline into the flag so that the
    /// backend picks the line terminator.
    pub content: Cow<'a, str>,
    /// Whether to terminate the text with a newline.
    pub newline: bool,
}

/// A rendering and input backend for iocraft's terminal render loop.
///
/// Implementations must be [`Send`] so the render loop can move across threads.
/// The trait is object-safe and used as `Box<dyn TerminalBackend>`.
///
/// Most methods mirror a single terminal capability. Rendering is expressed in
/// terms of [`Canvas`] rather than raw bytes, so a backend is free to diff and
/// emit ANSI (as `CrosstermBackend` does) or to push cells directly into a
/// grid.
pub(crate) trait TerminalBackend: Send {
    /// Re-samples the terminal size. Called once per frame before rendering.
    ///
    /// The default implementation does nothing, for backends with a fixed or
    /// externally-driven size.
    fn refresh_size(&mut self) {}

    /// The most recently sampled size as `(columns, rows)`, or `None` if
    /// unknown.
    fn size(&self) -> Option<(u16, u16)> {
        None
    }

    /// Enables or disables mouse event reporting. Idempotent.
    ///
    /// The default implementation is a no-op, for backends without mouse
    /// support.
    fn set_mouse_capture(&mut self, enabled: bool) -> io::Result<()> {
        let _ = enabled;
        Ok(())
    }

    /// Brackets the start of a frame so partial updates aren't shown (e.g. a
    /// DEC 2026 synchronized update on ANSI terminals).
    ///
    /// Called once per frame, paired with [`end_frame`](Self::end_frame).
    /// Backends that never tear may leave this as the default no-op.
    fn begin_frame(&mut self) -> io::Result<()> {
        Ok(())
    }

    /// Brackets the end of a frame. See [`begin_frame`](Self::begin_frame).
    fn end_frame(&mut self) -> io::Result<()> {
        Ok(())
    }

    /// Erases the previously rendered canvas from the display.
    fn clear_canvas(&mut self) -> io::Result<()>;

    /// Renders `canvas`, diffing against `prev` when it is supplied.
    fn write_canvas(&mut self, prev: Option<&Canvas>, canvas: &Canvas) -> io::Result<()>;

    /// Emits passthrough output above the rendered canvas, preserving the UI
    /// beneath it.
    ///
    /// The whole batch queued since the last call is passed at once so the
    /// backend can sequence stdout/stderr writes and any cursor bookkeeping in
    /// one pass.
    fn print_above(&mut self, messages: &[Passthrough<'_>]) -> io::Result<()>;

    /// Returns a stream of input events, enabling input (and, for terminal
    /// backends, raw mode) if it isn't already.
    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>>;
}
