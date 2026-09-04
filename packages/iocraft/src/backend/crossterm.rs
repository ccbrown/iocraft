//! The built-in crossterm terminal backend and compatibility conversions.

use super::{Passthrough, TerminalBackend};
use crate::{
    canvas::Canvas,
    color::Color,
    element::Output,
    event::{
        KeyCode, KeyEventKind, KeyModifiers, MediaKeyCode, ModifierKeyCode, MouseButton,
        MouseEventKind,
    },
    terminal::{FullscreenMouseEvent, KeyEvent, TerminalEvent},
};
use ::crossterm::event as ct;
use ::crossterm::{
    cursor,
    event::{self, Event, EventStream},
    terminal, ExecutableCommand, QueueableCommand,
};
use futures::{
    stream::{self, BoxStream},
    StreamExt,
};
use std::io::{self, stdin, IsTerminal, Write};

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

impl From<MediaKeyCode> for ct::MediaKeyCode {
    fn from(c: MediaKeyCode) -> Self {
        match c {
            MediaKeyCode::Play => Self::Play,
            MediaKeyCode::Pause => Self::Pause,
            MediaKeyCode::PlayPause => Self::PlayPause,
            MediaKeyCode::Reverse => Self::Reverse,
            MediaKeyCode::Stop => Self::Stop,
            MediaKeyCode::FastForward => Self::FastForward,
            MediaKeyCode::Rewind => Self::Rewind,
            MediaKeyCode::TrackNext => Self::TrackNext,
            MediaKeyCode::TrackPrevious => Self::TrackPrevious,
            MediaKeyCode::Record => Self::Record,
            MediaKeyCode::LowerVolume => Self::LowerVolume,
            MediaKeyCode::RaiseVolume => Self::RaiseVolume,
            MediaKeyCode::MuteVolume => Self::MuteVolume,
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

impl From<ModifierKeyCode> for ct::ModifierKeyCode {
    fn from(c: ModifierKeyCode) -> Self {
        match c {
            ModifierKeyCode::LeftShift => Self::LeftShift,
            ModifierKeyCode::LeftControl => Self::LeftControl,
            ModifierKeyCode::LeftAlt => Self::LeftAlt,
            ModifierKeyCode::LeftSuper => Self::LeftSuper,
            ModifierKeyCode::LeftHyper => Self::LeftHyper,
            ModifierKeyCode::LeftMeta => Self::LeftMeta,
            ModifierKeyCode::RightShift => Self::RightShift,
            ModifierKeyCode::RightControl => Self::RightControl,
            ModifierKeyCode::RightAlt => Self::RightAlt,
            ModifierKeyCode::RightSuper => Self::RightSuper,
            ModifierKeyCode::RightHyper => Self::RightHyper,
            ModifierKeyCode::RightMeta => Self::RightMeta,
            ModifierKeyCode::IsoLevel3Shift => Self::IsoLevel3Shift,
            ModifierKeyCode::IsoLevel5Shift => Self::IsoLevel5Shift,
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

impl From<KeyCode> for ct::KeyCode {
    fn from(c: KeyCode) -> Self {
        match c {
            KeyCode::Backspace => Self::Backspace,
            KeyCode::Enter => Self::Enter,
            KeyCode::Left => Self::Left,
            KeyCode::Right => Self::Right,
            KeyCode::Up => Self::Up,
            KeyCode::Down => Self::Down,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,
            KeyCode::Tab => Self::Tab,
            KeyCode::BackTab => Self::BackTab,
            KeyCode::Delete => Self::Delete,
            KeyCode::Insert => Self::Insert,
            KeyCode::F(n) => Self::F(n),
            KeyCode::Char(c) => Self::Char(c),
            KeyCode::Null => Self::Null,
            KeyCode::Esc => Self::Esc,
            KeyCode::CapsLock => Self::CapsLock,
            KeyCode::ScrollLock => Self::ScrollLock,
            KeyCode::NumLock => Self::NumLock,
            KeyCode::PrintScreen => Self::PrintScreen,
            KeyCode::Pause => Self::Pause,
            KeyCode::Menu => Self::Menu,
            KeyCode::KeypadBegin => Self::KeypadBegin,
            KeyCode::Media(m) => Self::Media(m.into()),
            KeyCode::Modifier(m) => Self::Modifier(m.into()),
        }
    }
}

impl From<ct::KeyModifiers> for KeyModifiers {
    fn from(m: ct::KeyModifiers) -> Self {
        // Bit layout is identical, so bits round-trip directly.
        Self::from_bits_retain(m.bits())
    }
}

impl From<KeyModifiers> for ct::KeyModifiers {
    fn from(m: KeyModifiers) -> Self {
        // Bit layout is identical, so bits round-trip directly.
        Self::from_bits_retain(m.bits())
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

impl From<KeyEventKind> for ct::KeyEventKind {
    fn from(k: KeyEventKind) -> Self {
        match k {
            KeyEventKind::Press => Self::Press,
            KeyEventKind::Repeat => Self::Repeat,
            KeyEventKind::Release => Self::Release,
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

impl From<MouseButton> for ct::MouseButton {
    fn from(b: MouseButton) -> Self {
        match b {
            MouseButton::Left => Self::Left,
            MouseButton::Right => Self::Right,
            MouseButton::Middle => Self::Middle,
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

impl From<MouseEventKind> for ct::MouseEventKind {
    fn from(k: MouseEventKind) -> Self {
        match k {
            MouseEventKind::Down(b) => Self::Down(b.into()),
            MouseEventKind::Up(b) => Self::Up(b.into()),
            MouseEventKind::Drag(b) => Self::Drag(b.into()),
            MouseEventKind::Moved => Self::Moved,
            MouseEventKind::ScrollDown => Self::ScrollDown,
            MouseEventKind::ScrollUp => Self::ScrollUp,
            MouseEventKind::ScrollLeft => Self::ScrollLeft,
            MouseEventKind::ScrollRight => Self::ScrollRight,
        }
    }
}

impl From<Color> for ::crossterm::style::Color {
    fn from(c: Color) -> Self {
        use ::crossterm::style::Color as Ct;
        match c {
            Color::Reset => Ct::Reset,
            Color::Black => Ct::Black,
            Color::DarkGrey => Ct::DarkGrey,
            Color::Red => Ct::Red,
            Color::DarkRed => Ct::DarkRed,
            Color::Green => Ct::Green,
            Color::DarkGreen => Ct::DarkGreen,
            Color::Yellow => Ct::Yellow,
            Color::DarkYellow => Ct::DarkYellow,
            Color::Blue => Ct::Blue,
            Color::DarkBlue => Ct::DarkBlue,
            Color::Magenta => Ct::Magenta,
            Color::DarkMagenta => Ct::DarkMagenta,
            Color::Cyan => Ct::Cyan,
            Color::DarkCyan => Ct::DarkCyan,
            Color::White => Ct::White,
            Color::Grey => Ct::Grey,
            Color::Rgb { r, g, b } => Ct::Rgb { r, g, b },
            Color::AnsiValue(v) => Ct::AnsiValue(v),
        }
    }
}

impl From<::crossterm::style::Color> for Color {
    fn from(c: ::crossterm::style::Color) -> Self {
        use ::crossterm::style::Color as Ct;
        match c {
            Ct::Reset => Color::Reset,
            Ct::Black => Color::Black,
            Ct::DarkGrey => Color::DarkGrey,
            Ct::Red => Color::Red,
            Ct::DarkRed => Color::DarkRed,
            Ct::Green => Color::Green,
            Ct::DarkGreen => Color::DarkGreen,
            Ct::Yellow => Color::Yellow,
            Ct::DarkYellow => Color::DarkYellow,
            Ct::Blue => Color::Blue,
            Ct::DarkBlue => Color::DarkBlue,
            Ct::Magenta => Color::Magenta,
            Ct::DarkMagenta => Color::DarkMagenta,
            Ct::Cyan => Color::Cyan,
            Ct::DarkCyan => Color::DarkCyan,
            Ct::White => Color::White,
            Ct::Grey => Color::Grey,
            Ct::Rgb { r, g, b } => Color::Rgb { r, g, b },
            Ct::AnsiValue(v) => Color::AnsiValue(v),
        }
    }
}

fn clear_canvas_inline(
    dest: &mut (impl Write + ?Sized),
    prev_canvas_height: u16,
) -> io::Result<()> {
    let lines_to_rewind = prev_canvas_height - 1;
    if lines_to_rewind == 0 {
        dest.queue(cursor::MoveToColumn(0))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        Ok(())
    } else {
        dest.queue(cursor::MoveToPreviousLine(lines_to_rewind as _))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        Ok(())
    }
}

/// A [`TerminalBackend`] that renders ANSI escape sequences to stdout/stderr
/// via crossterm. This is iocraft's default backend.
pub(crate) struct CrosstermBackend<'a> {
    input_is_terminal: bool,
    /// The render destination (whichever of stdout/stderr `output` selects).
    dest: Box<dyn Write + Send + 'a>,
    /// The other standard stream.
    alt: Box<dyn Write + Send + 'a>,
    /// Which standard stream `dest` corresponds to.
    output: Output,
    fullscreen: bool,
    mouse_capture: bool,
    raw_mode_enabled: bool,
    supports_keyboard_enhancement: bool,
    enabled_keyboard_enhancement: bool,
    enabled_bracketed_paste: bool,
    prev_canvas_top_row: u16,
    prev_canvas_height: u16,
    prev_size_on_write: Option<(u16, u16)>,
    size: Option<(u16, u16)>,
    /// Column recorded after a no-newline passthrough write, so the next
    /// `print_above` can re-anchor the cursor. See [`Self::print_above`].
    ///
    /// This is deliberately backend-wide rather than per `use_output` hook:
    /// there is only one physical cursor, so output from different hooks
    /// interleaves the same way plain `print!`/`println!` calls would.
    passthrough_appended_newline: Option<u16>,
}

impl CrosstermBackend<'_> {
    /// Returns the writer for the given standard stream. `dest` is the render
    /// target; the other stream is `alt`.
    fn stream_writer(&mut self, stream: Output) -> &mut (dyn Write + Send) {
        if stream == self.output {
            &mut *self.dest
        } else {
            &mut *self.alt
        }
    }

    /// The fallible body of [`TerminalBackend::print_above`]. Per-message
    /// writes are best effort: a failed message doesn't prevent later messages
    /// (possibly bound for the other stream) from being written. Returns the
    /// first error encountered.
    fn print_above_impl(&mut self, messages: &[Passthrough<'_>]) -> io::Result<()> {
        let needs_carriage_returns = self.raw_mode_enabled;
        let newline: &[u8] = if needs_carriage_returns {
            b"\r\n"
        } else {
            b"\n"
        };

        // If we appended a newline after the last no-newline message, move back
        // up to that column so this batch continues where it left off.
        if let Some(col) = self.passthrough_appended_newline {
            self.dest
                .queue(cursor::MoveUp(1))
                .and_then(|w| w.queue(cursor::MoveRight(col)))?;
        }
        // Flush the render stream so its escape sequences are emitted before any
        // cross-stream writes (e.g. stdout messages when rendering to stderr).
        self.dest.flush()?;

        let mut needs_extra_newline = self.passthrough_appended_newline.is_some();
        let mut first_err = None;

        for msg in messages {
            let w = self.stream_writer(msg.stream);
            let mut result =
                write_passthrough_content(&mut *w, &msg.content, needs_carriage_returns);
            if result.is_ok() && msg.newline {
                result = w.write_all(newline);
            }
            match result {
                Ok(()) => {
                    if msg.newline {
                        needs_extra_newline = false;
                    } else if !msg.content.is_empty() {
                        // `Passthrough` content without the newline flag never
                        // ends with a newline (use_output normalizes it), so a
                        // trailing newline must be appended and re-anchored.
                        needs_extra_newline = true;
                    }
                }
                Err(err) => {
                    if first_err.is_none() {
                        first_err = Some(err);
                    }
                }
            }
        }
        if let Some(err) = first_err {
            return Err(err);
        }

        if needs_extra_newline {
            // Flush both streams so the terminal has processed everything before
            // we query the cursor position, otherwise the recorded column would
            // reflect stale state.
            self.dest.flush()?;
            self.alt.flush()?;
            if let Ok(pos) = cursor::position() {
                self.passthrough_appended_newline = Some(pos.0);
                self.dest.write_all(newline)?;
            } else {
                self.passthrough_appended_newline = None;
            }
        } else {
            self.passthrough_appended_newline = None;
        }
        Ok(())
    }
}

/// Writes passthrough content, translating embedded `\n` to `\r\n` when the
/// terminal is in raw mode, where a bare `\n` doesn't return the cursor to
/// column 0 and multi-line content would stair-step.
fn write_passthrough_content(
    w: &mut (dyn Write + Send),
    content: &str,
    needs_carriage_returns: bool,
) -> io::Result<()> {
    if !needs_carriage_returns || !content.contains('\n') {
        return w.write_all(content.as_bytes());
    }
    let bytes = content.as_bytes();
    let mut start = 0;
    for (i, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            let end = if i > start && bytes[i - 1] == b'\r' {
                i - 1
            } else {
                i
            };
            w.write_all(&bytes[start..end])?;
            w.write_all(b"\r\n")?;
            start = i + 1;
        }
    }
    w.write_all(&bytes[start..])
}

impl TerminalBackend for CrosstermBackend<'_> {
    fn query_size() -> io::Result<(u16, u16)> {
        terminal::size()
    }

    fn write_once<W: Write>(canvas: &Canvas, writer: W) -> io::Result<()> {
        canvas.write_ansi(writer)
    }

    fn refresh_size(&mut self) {
        self.size = Self::query_size().ok()
    }

    fn size(&self) -> Option<(u16, u16)> {
        self.size
    }

    fn set_fullscreen(&mut self, enabled: bool) -> io::Result<()> {
        if self.fullscreen != enabled {
            if enabled {
                self.dest.queue(terminal::EnterAlternateScreen)?;
            } else {
                self.dest.queue(terminal::LeaveAlternateScreen)?;
            }
            self.fullscreen = enabled;
        }
        Ok(())
    }

    fn set_mouse_capture(&mut self, enabled: bool) -> io::Result<()> {
        if self.mouse_capture != enabled {
            self.mouse_capture = enabled;
            if self.raw_mode_enabled {
                if enabled {
                    self.dest.execute(event::EnableMouseCapture)?;
                } else {
                    self.dest.execute(event::DisableMouseCapture)?;
                }
            }
        }
        Ok(())
    }

    fn clear_canvas(&mut self) -> io::Result<()> {
        if self.prev_canvas_height == 0 {
            return Ok(());
        }

        if self.fullscreen {
            self.dest
                .queue(cursor::MoveTo(0, self.prev_canvas_top_row))?
                .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
            return Ok(());
        }

        if let Some(size) = self.size {
            if self.prev_canvas_height >= size.1 {
                // We have to clear the entire terminal to avoid leaving artifacts.
                // See: https://github.com/ccbrown/iocraft/issues/118
                self.dest
                    .queue(terminal::Clear(terminal::ClearType::All))?
                    .queue(terminal::Clear(terminal::ClearType::Purge))?
                    .queue(cursor::MoveTo(0, 0))?;
                return Ok(());
            }
        }

        clear_canvas_inline(&mut *self.dest, self.prev_canvas_height)
    }

    fn write_canvas(&mut self, prev: Option<&Canvas>, canvas: &Canvas) -> io::Result<()> {
        let Some(prev) = prev else {
            // No previous canvas: full write.
            if self.fullscreen {
                self.prev_canvas_top_row = 0;
                self.dest.queue(cursor::MoveTo(0, 0))?;
            }
            self.prev_canvas_height = canvas.height() as _;
            canvas.write_ansi_without_final_newline(&mut *self.dest)?;
            return Ok(());
        };

        if self.fullscreen {
            if self.prev_size_on_write != self.size {
                // If the terminal is changing size, clear it to make sure we don't leave
                // artifacts. This is especially important when the terminal is shrinking, since
                // characters might flow outside of the visible terminal, where they can't be
                // cleared with `\033[K` and oddly may re-enter the terminal as visible characters
                // are cleared.
                self.clear_canvas()?;
                self.prev_canvas_height = canvas.height() as _;
                self.prev_size_on_write = self.size;
                canvas.write_ansi_without_final_newline(&mut *self.dest)?;
                return Ok(());
            }

            // Fullscreen: absolute positioning.
            let top_row = self.prev_canvas_top_row;
            let max_height = prev.height().max(canvas.height());
            for y in 0..max_height {
                if prev.row_eq(canvas, y) {
                    continue;
                }
                self.dest.queue(cursor::MoveTo(0, top_row + y as u16))?;
                if y < canvas.height() {
                    canvas.write_ansi_row_without_newline(y, &mut *self.dest)?;
                } else {
                    self.dest
                        .queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
                }
            }
            if canvas.height() > 0 {
                self.dest
                    .queue(cursor::MoveTo(0, top_row + canvas.height() as u16 - 1))?;
            }
            self.prev_canvas_height = canvas.height() as _;
            return Ok(());
        } else {
            self.prev_size_on_write = self.size;
        }

        // Inline: row diff with relative cursor movement.
        let prev_height = prev.height();
        let new_height = canvas.height();
        let max_height = prev_height.max(new_height);
        let mut current_y = prev_height.saturating_sub(1);

        for y in 0..max_height {
            if prev.row_eq(canvas, y) {
                continue;
            }
            // If a changed row has scrolled off the top of the visible area,
            // we can't reach it with cursor movement — fall back to full rewrite.
            if let Some((_cols, term_h)) = self.size {
                let visible_start = prev_height.saturating_sub(term_h as usize);
                if y < visible_start {
                    self.clear_canvas()?;
                    self.prev_canvas_height = canvas.height() as _;
                    canvas.write_ansi_without_final_newline(&mut *self.dest)?;
                    return Ok(());
                }
            }
            match y.cmp(&current_y) {
                std::cmp::Ordering::Less => {
                    self.dest
                        .queue(cursor::MoveToPreviousLine((current_y - y) as u16))?;
                }
                std::cmp::Ordering::Greater => {
                    // Lines within the previous canvas already exist in the
                    // terminal and can be reached with MoveToNextLine (CSI E).
                    // Lines beyond prev_height don't exist yet — we must emit
                    // \r\n to create them, since CSI E won't extend the
                    // scrollback when the cursor is at the bottom of the screen.
                    let last_existing_line = prev_height.saturating_sub(1).max(current_y);
                    if y <= last_existing_line {
                        self.dest
                            .queue(cursor::MoveToNextLine((y - current_y) as u16))?;
                    } else {
                        let move_to_last = last_existing_line.saturating_sub(current_y);
                        if move_to_last > 0 {
                            self.dest
                                .queue(cursor::MoveToNextLine(move_to_last as u16))?;
                        }
                        let new_lines = y - last_existing_line;
                        for _ in 0..new_lines {
                            self.dest.write_all(b"\r\n")?;
                        }
                    }
                }
                std::cmp::Ordering::Equal => {
                    self.dest.queue(cursor::MoveToColumn(0))?;
                }
            }
            current_y = y;

            if y < new_height {
                canvas.write_ansi_row_without_newline(y, &mut *self.dest)?;
            } else {
                self.dest
                    .queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
            }
        }

        // Reposition cursor to last row of new canvas.
        let target_y = new_height.saturating_sub(1);
        match target_y.cmp(&current_y) {
            std::cmp::Ordering::Greater => {
                self.dest
                    .queue(cursor::MoveToNextLine((target_y - current_y) as u16))?;
            }
            std::cmp::Ordering::Less => {
                self.dest
                    .queue(cursor::MoveToPreviousLine((current_y - target_y) as u16))?;
            }
            std::cmp::Ordering::Equal => {}
        }

        self.prev_canvas_height = new_height as _;
        Ok(())
    }

    fn begin_frame(&mut self) -> io::Result<()> {
        self.dest.execute(terminal::BeginSynchronizedUpdate)?;
        Ok(())
    }

    fn end_frame(&mut self) -> io::Result<()> {
        self.dest.execute(terminal::EndSynchronizedUpdate)?;
        Ok(())
    }

    fn print_above(&mut self, messages: &[Passthrough<'_>]) -> io::Result<()> {
        if messages.is_empty() {
            return Ok(());
        }
        let result = self.print_above_impl(messages);
        if result.is_err() {
            // After a failed or partial write the cursor position is unknown,
            // so never re-anchor onto it. The worst case is a stray blank line
            // rather than overwriting previously rendered output.
            self.passthrough_appended_newline = None;
        }
        result
    }

    fn event_stream(&mut self) -> io::Result<BoxStream<'static, io::Result<TerminalEvent>>> {
        if !self.input_is_terminal {
            return Ok(stream::pending().boxed());
        }

        self.set_raw_mode_enabled(true)?;

        Ok(EventStream::new()
            .filter_map(|event| async move {
                match event {
                    Ok(Event::Key(event)) => Some(Ok(TerminalEvent::Key(KeyEvent {
                        code: event.code.into(),
                        modifiers: event.modifiers.into(),
                        kind: event.kind.into(),
                    }))),
                    Ok(Event::Mouse(event)) => {
                        Some(Ok(TerminalEvent::FullscreenMouse(FullscreenMouseEvent {
                            modifiers: event.modifiers.into(),
                            column: event.column,
                            row: event.row,
                            kind: event.kind.into(),
                        })))
                    }
                    Ok(Event::Resize(width, height)) => {
                        Some(Ok(TerminalEvent::Resize(width, height)))
                    }
                    Ok(Event::Paste(data)) => Some(Ok(TerminalEvent::Paste(data))),
                    // Ignore crossterm events that iocraft does not expose.
                    Ok(_) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .boxed())
    }
}

/// Queries the size using the built-in backend without creating a render
/// session. One-shot element rendering uses this so even its terminal access
/// crosses the same backend boundary as render loops.
impl<'a> CrosstermBackend<'a> {
    fn new(
        stdout: Box<dyn Write + Send + 'a>,
        stderr: Box<dyn Write + Send + 'a>,
        output: Output,
    ) -> io::Result<Self> {
        // dest is the render destination, alt is the other stream
        let (dest, alt) = match output {
            Output::Stdout => (stdout, stderr),
            Output::Stderr => (stderr, stdout),
        };
        let input_is_terminal = stdin().is_terminal();
        // The probe blocks on a query response, and some terminals (e.g. WezTerm)
        // don't answer queries while a synchronized update is open — probing lazily
        // from within a render would stall until the query times out.
        let supports_keyboard_enhancement =
            input_is_terminal && terminal::supports_keyboard_enhancement().unwrap_or(false);
        let mut term = Self {
            dest,
            alt,
            output,
            input_is_terminal,
            fullscreen: false,
            mouse_capture: false,
            raw_mode_enabled: false,
            supports_keyboard_enhancement,
            enabled_keyboard_enhancement: false,
            enabled_bracketed_paste: false,
            prev_canvas_top_row: 0,
            prev_canvas_height: 0,
            size: None,
            prev_size_on_write: None,
            passthrough_appended_newline: None,
        };
        term.dest.queue(cursor::Hide)?;
        Ok(term)
    }

    fn set_raw_mode_enabled(&mut self, raw_mode_enabled: bool) -> io::Result<()> {
        if raw_mode_enabled != self.raw_mode_enabled {
            if raw_mode_enabled {
                if self.supports_keyboard_enhancement {
                    self.dest.execute(event::PushKeyboardEnhancementFlags(
                        event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
                    ))?;
                    self.enabled_keyboard_enhancement = true;
                }
                if !self.enabled_bracketed_paste {
                    self.dest.execute(event::EnableBracketedPaste)?;
                    self.enabled_bracketed_paste = true;
                }
                if self.mouse_capture {
                    self.dest.execute(event::EnableMouseCapture)?;
                }
                terminal::enable_raw_mode()?;
            } else {
                terminal::disable_raw_mode()?;
                if self.enabled_bracketed_paste {
                    self.dest.execute(event::DisableBracketedPaste)?;
                    self.enabled_bracketed_paste = false;
                }
                if self.mouse_capture {
                    self.dest.execute(event::DisableMouseCapture)?;
                }
                if self.enabled_keyboard_enhancement {
                    self.dest.execute(event::PopKeyboardEnhancementFlags)?;
                }
            }
            self.raw_mode_enabled = raw_mode_enabled;
        }
        Ok(())
    }
}

impl Drop for CrosstermBackend<'_> {
    fn drop(&mut self) {
        let _ = self.set_raw_mode_enabled(false);
        if self.fullscreen {
            let _ = self.dest.queue(terminal::LeaveAlternateScreen);
        } else if self.prev_canvas_height > 0 {
            let _ = self.dest.write_all(b"\r\n");
        }
        let _ = self.dest.execute(cursor::Show);
    }
}

pub(crate) fn new_terminal_backend<'a>(
    stdout: Box<dyn Write + Send + 'a>,
    stderr: Box<dyn Write + Send + 'a>,
    output: Output,
) -> io::Result<Box<dyn TerminalBackend + 'a>> {
    Ok(Box::new(CrosstermBackend::new(stdout, stderr, output)?))
}

pub(crate) fn default_terminal_size() -> io::Result<(u16, u16)> {
    CrosstermBackend::query_size()
}

pub(crate) fn default_terminal_write<W: Write>(canvas: &Canvas, writer: W) -> io::Result<()> {
    CrosstermBackend::write_once(canvas, writer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::SgrColor;
    use crate::prelude::*;
    use ::crossterm::QueueableCommand;
    use macro_rules_attribute::apply;
    use smol_macros::test;
    use std::sync::{Arc, Mutex};

    #[component]
    fn UseOutputComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let (stdout, stderr) = hooks.use_output();
        stdout.println("Hello, world!");
        stderr.println("Hello, error!");
        stdout.print("Testing ");
        stdout.print("print ");
        stdout.println("method!");
        stderr.print("Error: ");
        stderr.println("test");
        stderr.print("Warning: ");
        stderr.println("print test");
        system.exit();
        element!(View)
    }

    #[apply(test!)]
    async fn test_use_output() {
        element!(UseOutputComponent).render_loop().await.unwrap();
    }

    #[test]
    fn crossterm_color_roundtrip_and_output_parity() {
        let colors = [
            Color::Reset,
            Color::Black,
            Color::DarkGrey,
            Color::Red,
            Color::DarkRed,
            Color::Green,
            Color::DarkGreen,
            Color::Yellow,
            Color::DarkYellow,
            Color::Blue,
            Color::DarkBlue,
            Color::Magenta,
            Color::DarkMagenta,
            Color::Cyan,
            Color::DarkCyan,
            Color::White,
            Color::Grey,
            Color::Rgb {
                r: 10,
                g: 20,
                b: 30,
            },
            Color::AnsiValue(123),
        ];
        for color in colors {
            let crossterm_color: ::crossterm::style::Color = color.into();
            assert_eq!(
                Color::from(crossterm_color),
                color,
                "roundtrip failed for {color:?}"
            );
            assert_eq!(
                SgrColor::Foreground(color).to_string(),
                ::crossterm::style::Colored::ForegroundColor(crossterm_color).to_string(),
                "foreground SGR mismatch for {color:?}"
            );
            assert_eq!(
                SgrColor::Background(color).to_string(),
                ::crossterm::style::Colored::BackgroundColor(crossterm_color).to_string(),
                "background SGR mismatch for {color:?}"
            );
        }
    }

    #[test]
    fn event_display_matches_crossterm() {
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
        let modifiers = ct::KeyModifiers::SHIFT | ct::KeyModifiers::CONTROL | ct::KeyModifiers::ALT;
        assert_eq!(
            KeyModifiers::from(modifiers).to_string(),
            modifiers.to_string()
        );
    }

    #[test]
    fn key_modifier_bits_match_crossterm() {
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

    #[test]
    fn crossterm_event_conversions_roundtrip() {
        let media_codes = [
            ct::MediaKeyCode::Play,
            ct::MediaKeyCode::Pause,
            ct::MediaKeyCode::PlayPause,
            ct::MediaKeyCode::Reverse,
            ct::MediaKeyCode::Stop,
            ct::MediaKeyCode::FastForward,
            ct::MediaKeyCode::Rewind,
            ct::MediaKeyCode::TrackNext,
            ct::MediaKeyCode::TrackPrevious,
            ct::MediaKeyCode::Record,
            ct::MediaKeyCode::LowerVolume,
            ct::MediaKeyCode::RaiseVolume,
            ct::MediaKeyCode::MuteVolume,
        ];
        for value in media_codes {
            let ours: MediaKeyCode = value.into();
            let roundtrip: ct::MediaKeyCode = ours.into();
            assert_eq!(roundtrip, value);
        }

        let modifier_codes = [
            ct::ModifierKeyCode::LeftShift,
            ct::ModifierKeyCode::LeftControl,
            ct::ModifierKeyCode::LeftAlt,
            ct::ModifierKeyCode::LeftSuper,
            ct::ModifierKeyCode::LeftHyper,
            ct::ModifierKeyCode::LeftMeta,
            ct::ModifierKeyCode::RightShift,
            ct::ModifierKeyCode::RightControl,
            ct::ModifierKeyCode::RightAlt,
            ct::ModifierKeyCode::RightSuper,
            ct::ModifierKeyCode::RightHyper,
            ct::ModifierKeyCode::RightMeta,
            ct::ModifierKeyCode::IsoLevel3Shift,
            ct::ModifierKeyCode::IsoLevel5Shift,
        ];
        for value in modifier_codes {
            let ours: ModifierKeyCode = value.into();
            let roundtrip: ct::ModifierKeyCode = ours.into();
            assert_eq!(roundtrip, value);
        }

        let key_codes = [
            ct::KeyCode::Backspace,
            ct::KeyCode::Enter,
            ct::KeyCode::Left,
            ct::KeyCode::Right,
            ct::KeyCode::Up,
            ct::KeyCode::Down,
            ct::KeyCode::Home,
            ct::KeyCode::End,
            ct::KeyCode::PageUp,
            ct::KeyCode::PageDown,
            ct::KeyCode::Tab,
            ct::KeyCode::BackTab,
            ct::KeyCode::Delete,
            ct::KeyCode::Insert,
            ct::KeyCode::F(12),
            ct::KeyCode::Char('q'),
            ct::KeyCode::Null,
            ct::KeyCode::Esc,
            ct::KeyCode::CapsLock,
            ct::KeyCode::ScrollLock,
            ct::KeyCode::NumLock,
            ct::KeyCode::PrintScreen,
            ct::KeyCode::Pause,
            ct::KeyCode::Menu,
            ct::KeyCode::KeypadBegin,
            ct::KeyCode::Media(ct::MediaKeyCode::PlayPause),
            ct::KeyCode::Modifier(ct::ModifierKeyCode::LeftSuper),
        ];
        for value in key_codes {
            let ours: KeyCode = value.into();
            let roundtrip: ct::KeyCode = ours.into();
            assert_eq!(roundtrip, value);
        }

        let modifiers = ct::KeyModifiers::SHIFT
            | ct::KeyModifiers::CONTROL
            | ct::KeyModifiers::from_bits_retain(0b1000_0000);
        let ours: KeyModifiers = modifiers.into();
        let roundtrip: ct::KeyModifiers = ours.into();
        assert_eq!(roundtrip, modifiers);

        for value in [
            ct::KeyEventKind::Press,
            ct::KeyEventKind::Repeat,
            ct::KeyEventKind::Release,
        ] {
            let ours: KeyEventKind = value.into();
            let roundtrip: ct::KeyEventKind = ours.into();
            assert_eq!(roundtrip, value);
        }

        for value in [
            ct::MouseButton::Left,
            ct::MouseButton::Right,
            ct::MouseButton::Middle,
        ] {
            let ours: MouseButton = value.into();
            let roundtrip: ct::MouseButton = ours.into();
            assert_eq!(roundtrip, value);
        }

        let mouse_event_kinds = [
            ct::MouseEventKind::Down(ct::MouseButton::Left),
            ct::MouseEventKind::Up(ct::MouseButton::Right),
            ct::MouseEventKind::Drag(ct::MouseButton::Middle),
            ct::MouseEventKind::Moved,
            ct::MouseEventKind::ScrollDown,
            ct::MouseEventKind::ScrollUp,
            ct::MouseEventKind::ScrollLeft,
            ct::MouseEventKind::ScrollRight,
        ];
        for value in mouse_event_kinds {
            let ours: MouseEventKind = value.into();
            let roundtrip: ct::MouseEventKind = ours.into();
            assert_eq!(roundtrip, value);
        }
    }

    #[derive(Clone, Default)]
    struct TestWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn new_test_writer() -> (TestWriter, Arc<Mutex<Vec<u8>>>) {
        let writer = TestWriter::default();
        let buf = writer.buf.clone();
        (writer, buf)
    }

    #[test]
    fn test_std_terminal() {
        // There's unfortunately not much here we can really test, but we'll do our best.
        // TODO: Is there a library we can use to emulate terminal input/output?
        let mut terminal = Terminal::new(
            Box::new(std::io::stdout()),
            Box::new(std::io::stderr()),
            Output::Stdout,
        )
        .unwrap();
        assert!(!terminal.received_ctrl_c());
        let canvas = Canvas::new(10, 1);
        terminal.write_canvas(None, &canvas).unwrap();
    }

    #[test]
    fn test_print_above_write_error_best_effort() {
        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let (dest, dest_buf) = new_test_writer();
        let mut term = new_inline_term(dest, 0);
        term.alt = Box::new(FailingWriter);
        term.passthrough_appended_newline = Some(3);

        let messages = [
            Passthrough {
                stream: Output::Stderr, // the alt stream, which fails
                content: "lost".into(),
                newline: true,
            },
            Passthrough {
                stream: Output::Stdout, // the render stream, which works
                content: "kept".into(),
                newline: true,
            },
        ];
        let result = term.print_above(&messages);
        assert!(result.is_err());

        // Later messages must still be written after an earlier write fails.
        let written = String::from_utf8(dest_buf.lock().unwrap().clone()).unwrap();
        assert!(written.contains("kept"), "got: {written:?}");

        // After an error the cursor position is unknown, so the re-anchor
        // state must reset rather than go stale.
        assert_eq!(term.passthrough_appended_newline, None);
    }

    #[test]
    fn test_print_above_translates_newlines_in_raw_mode() {
        let (dest, dest_buf) = new_test_writer();
        let mut term = new_inline_term(dest, 0);
        term.raw_mode_enabled = true;

        let messages = [Passthrough {
            stream: Output::Stdout,
            content: "line1\r\nline2\nline3\rline4".into(),
            newline: true,
        }];
        term.print_above(&messages).unwrap();

        // Embedded newlines must become \r\n in raw mode, or multi-line
        // content stair-steps.
        let written = String::from_utf8(dest_buf.lock().unwrap().clone()).unwrap();
        assert_eq!(written, "line1\r\nline2\r\nline3\rline4\r\n");
        assert_eq!(term.passthrough_appended_newline, None);
    }

    fn render_canvas_to_vt(canvas: &Canvas, cols: usize, rows: usize) -> avt::Vt {
        render_canvases_to_vt(&[canvas], cols, rows)
    }

    fn render_canvases_to_vt(canvases: &[&Canvas], cols: usize, rows: usize) -> avt::Vt {
        let mut buf = Vec::new();
        for (i, canvas) in canvases.iter().enumerate() {
            if i > 0 {
                super::clear_canvas_inline(&mut buf, canvases[i - 1].height() as _).unwrap();
            }
            canvas.write_ansi_without_final_newline(&mut buf).unwrap();
        }
        let mut vt = avt::Vt::new(cols, rows);
        vt.feed_str(&String::from_utf8(buf).unwrap());
        vt
    }

    #[test]
    fn test_inline_rewrite_single_line_cursor() {
        let mut canvas = Canvas::new(10, 1);
        canvas
            .subview_mut(0, 0, 0, 0, 10, 1)
            .set_text(0, 0, "hello", CanvasTextStyle::default());

        let vt = render_canvas_to_vt(&canvas, 10, 5);

        assert_eq!(vt.line(0).text(), "hello     ");
        assert_eq!(vt.cursor().row, 0, "cursor should stay on the first row");

        // clear and rerender with new content
        let mut canvas2 = Canvas::new(10, 1);
        canvas2
            .subview_mut(0, 0, 0, 0, 10, 1)
            .set_text(0, 0, "world", CanvasTextStyle::default());

        let vt = render_canvases_to_vt(&[&canvas, &canvas2], 10, 5);

        assert_eq!(vt.line(0).text(), "world     ");
        assert_eq!(vt.cursor().row, 0);
    }

    #[test]
    fn test_inline_rewrite_multi_line_cursor() {
        let mut canvas = Canvas::new(10, 3);
        canvas
            .subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 0, "line1", CanvasTextStyle::default());
        canvas
            .subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 2, "line3", CanvasTextStyle::default());

        let vt = render_canvas_to_vt(&canvas, 10, 5);

        assert_eq!(vt.line(0).text(), "line1     ");
        assert_eq!(vt.line(1).text(), "          ");
        assert_eq!(vt.line(2).text(), "line3     ");
        assert_eq!(
            vt.cursor().row,
            2,
            "cursor should be on the last content row"
        );

        // clear and rerender with fewer lines
        let mut canvas2 = Canvas::new(10, 2);
        canvas2
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "new1", CanvasTextStyle::default());
        canvas2
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "new2", CanvasTextStyle::default());

        let vt = render_canvases_to_vt(&[&canvas, &canvas2], 10, 5);

        assert_eq!(vt.line(0).text(), "new1      ");
        assert_eq!(vt.line(1).text(), "new2      ");
        assert_eq!(
            vt.line(2).text(),
            "          ",
            "old line 3 should be cleared"
        );
        assert_eq!(vt.cursor().row, 1);
    }

    #[test]
    fn test_inline_rewrite_no_extra_blank_line() {
        let mut canvas = Canvas::new(10, 2);
        canvas
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "first", CanvasTextStyle::default());
        canvas
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "second", CanvasTextStyle::default());

        let vt = render_canvas_to_vt(&canvas, 10, 5);

        assert_eq!(vt.line(0).text(), "first     ");
        assert_eq!(vt.line(1).text(), "second    ");
        assert_eq!(vt.cursor().row, 1, "cursor stays on last content row");

        // clear and rerender
        let mut canvas2 = Canvas::new(10, 2);
        canvas2
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "third", CanvasTextStyle::default());
        canvas2
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "fourth", CanvasTextStyle::default());

        let vt = render_canvases_to_vt(&[&canvas, &canvas2], 10, 5);

        assert_eq!(vt.line(0).text(), "third     ");
        assert_eq!(vt.line(1).text(), "fourth    ");
        assert_eq!(vt.cursor().row, 1);
    }

    #[test]
    fn test_fullscreen_diff_preserves_origin() {
        let mut prev = Canvas::new(10, 2);
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "first", CanvasTextStyle::default());
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "second", CanvasTextStyle::default());

        let mut next = Canvas::new(10, 2);
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "first", CanvasTextStyle::default());
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "changed", CanvasTextStyle::default());

        let (dest, diff_buf) = new_test_writer();
        let mut term = new_fullscreen_term(dest, 1, prev.height() as _);
        term.write_canvas(Some(&prev), &next).unwrap();

        let mut setup = Vec::new();
        write!(setup, "log\r\n").unwrap();
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&diff_buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, 5);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        assert_eq!(vt.line(0).text(), "log       ");
        assert_eq!(vt.line(1).text(), "first     ");
        assert_eq!(vt.line(2).text(), "changed   ");
        assert_eq!(
            vt.cursor().row,
            2,
            "cursor should stay on the canvas bottom"
        );
    }

    #[test]
    fn test_fullscreen_clear_preserves_output_above() {
        let mut canvas = Canvas::new(10, 2);
        canvas
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "first", CanvasTextStyle::default());
        canvas
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "second", CanvasTextStyle::default());

        let (dest, clear_buf) = new_test_writer();
        let mut term = new_fullscreen_term(dest, 1, canvas.height() as _);
        term.clear_canvas().unwrap();

        let mut setup = Vec::new();
        write!(setup, "log\r\n").unwrap();
        canvas.write_ansi_without_final_newline(&mut setup).unwrap();
        write!(setup, "\r\ntail").unwrap();
        setup.queue(cursor::MoveTo(0, 0)).unwrap();
        setup.extend_from_slice(&clear_buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, 5);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        assert_eq!(vt.line(0).text(), "log       ");
        assert_eq!(vt.line(1).text(), "          ");
        assert_eq!(vt.line(2).text(), "          ");
        assert_eq!(vt.line(3).text(), "          ");
    }

    fn new_fullscreen_term(
        dest: TestWriter,
        prev_canvas_top_row: u16,
        prev_canvas_height: u16,
    ) -> CrosstermBackend<'static> {
        CrosstermBackend {
            input_is_terminal: false,
            dest: Box::new(dest),
            alt: Box::new(io::sink()),
            output: Output::Stdout,
            fullscreen: true,
            mouse_capture: false,
            raw_mode_enabled: false,
            supports_keyboard_enhancement: false,
            enabled_keyboard_enhancement: false,
            enabled_bracketed_paste: false,
            prev_canvas_top_row,
            prev_canvas_height,
            size: None,
            prev_size_on_write: None,
            passthrough_appended_newline: None,
        }
    }

    fn new_inline_term(dest: TestWriter, prev_canvas_height: u16) -> CrosstermBackend<'static> {
        new_inline_term_with_size(dest, prev_canvas_height, (10, 10))
    }

    fn new_inline_term_with_size(
        dest: TestWriter,
        prev_canvas_height: u16,
        term_size: (u16, u16),
    ) -> CrosstermBackend<'static> {
        CrosstermBackend {
            input_is_terminal: false,
            dest: Box::new(dest),
            alt: Box::new(io::sink()),
            output: Output::Stdout,
            fullscreen: false,
            mouse_capture: false,
            raw_mode_enabled: false,
            supports_keyboard_enhancement: false,
            enabled_keyboard_enhancement: false,
            enabled_bracketed_paste: false,
            prev_canvas_top_row: 0,
            prev_canvas_height,
            size: Some(term_size),
            prev_size_on_write: None,
            passthrough_appended_newline: None,
        }
    }

    /// Run an inline diff (prev → next) and return the raw diff bytes plus
    /// an `avt::Vt` showing the final visible state.
    fn inline_diff_vt(prev: &Canvas, next: &Canvas, term_size: (u16, u16)) -> (Vec<u8>, avt::Vt) {
        let (dest, diff_buf) = new_test_writer();
        let mut term = new_inline_term_with_size(dest, prev.height() as _, term_size);
        term.write_canvas(Some(prev), next).unwrap();

        let diff = diff_buf.lock().unwrap().clone();
        let mut setup = Vec::new();
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&diff);

        let mut vt = avt::Vt::new(term_size.0 as _, term_size.1 as _);
        vt.feed_str(&String::from_utf8(setup).unwrap());
        (diff, vt)
    }

    #[test]
    fn test_inline_diff_unchanged_row_skipped() {
        let mut prev = Canvas::new(10, 2);
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "first", CanvasTextStyle::default());
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "second", CanvasTextStyle::default());

        let mut next = Canvas::new(10, 2);
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "first", CanvasTextStyle::default());
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "changed", CanvasTextStyle::default());

        let (dest, diff_buf) = new_test_writer();
        let mut term = new_inline_term(dest, prev.height() as _);
        term.write_canvas(Some(&prev), &next).unwrap();

        // Build vt: render prev, then apply diff output.
        let mut setup = Vec::new();
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&diff_buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, 5);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        assert_eq!(vt.line(0).text(), "first     ");
        assert_eq!(vt.line(1).text(), "changed   ");
        assert_eq!(vt.cursor().row, 1);
    }

    #[test]
    fn test_inline_diff_shrinking() {
        let mut prev = Canvas::new(10, 3);
        prev.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 0, "aaa", CanvasTextStyle::default());
        prev.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 1, "bbb", CanvasTextStyle::default());
        prev.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 2, "ccc", CanvasTextStyle::default());

        let mut next = Canvas::new(10, 2);
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "aaa", CanvasTextStyle::default());
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "ddd", CanvasTextStyle::default());

        let (dest, diff_buf) = new_test_writer();
        let mut term = new_inline_term(dest, prev.height() as _);
        term.write_canvas(Some(&prev), &next).unwrap();

        let mut setup = Vec::new();
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&diff_buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, 5);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        assert_eq!(vt.line(0).text(), "aaa       ");
        assert_eq!(vt.line(1).text(), "ddd       ");
        assert_eq!(
            vt.line(2).text(),
            "          ",
            "old row 2 should be cleared"
        );
        assert_eq!(vt.cursor().row, 1, "cursor on last row of new canvas");
    }

    #[test]
    fn test_inline_diff_growing() {
        let mut prev = Canvas::new(10, 2);
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "aaa", CanvasTextStyle::default());
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "bbb", CanvasTextStyle::default());

        let mut next = Canvas::new(10, 3);
        next.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 0, "aaa", CanvasTextStyle::default());
        next.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 1, "bbb", CanvasTextStyle::default());
        next.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 2, "ccc", CanvasTextStyle::default());

        let (dest, diff_buf) = new_test_writer();
        let mut term = new_inline_term(dest, prev.height() as _);
        term.write_canvas(Some(&prev), &next).unwrap();

        let mut setup = Vec::new();
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&diff_buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, 5);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        assert_eq!(vt.line(0).text(), "aaa       ");
        assert_eq!(vt.line(1).text(), "bbb       ");
        assert_eq!(vt.line(2).text(), "ccc       ");
        assert_eq!(vt.cursor().row, 2, "cursor on last row of new canvas");
    }

    #[test]
    fn test_inline_diff_non_adjacent_rows_forward() {
        // Two non-adjacent rows change within the existing canvas. The diff
        // visits row 1 first (moving the cursor up from row 4), then row 3
        // (moving forward but still within the old canvas). This exercises the
        // Greater branch when y < prev_height.
        let style = CanvasTextStyle::default();

        let mut prev = Canvas::new(10, 5);
        for i in 0..5 {
            prev.subview_mut(0, 0, 0, 0, 10, 5)
                .set_text(0, i, &format!("row{i}"), style);
        }

        let mut next = Canvas::new(10, 5);
        for i in 0..5 {
            next.subview_mut(0, 0, 0, 0, 10, 5)
                .set_text(0, i, &format!("row{i}"), style);
        }
        // Use same-length replacements to avoid masking the bug with
        // trailing-cell issues in write_ansi_row_without_newline.
        next.subview_mut(0, 0, 0, 0, 10, 5)
            .set_text(0, 1, "AAA1", style);
        next.subview_mut(0, 0, 0, 0, 10, 5)
            .set_text(0, 3, "BBB3", style);

        let (_diff, vt) = inline_diff_vt(&prev, &next, (10, 10));

        assert_eq!(vt.line(0).text(), "row0      ");
        assert_eq!(vt.line(1).text(), "AAA1      ");
        assert_eq!(vt.line(2).text(), "row2      ");
        assert_eq!(vt.line(3).text(), "BBB3      ");
        assert_eq!(vt.line(4).text(), "row4      ");
    }

    #[test]
    fn test_inline_diff_growing_at_bottom_of_screen() {
        // Simulate the canvas being at the bottom of the terminal so that
        // growing from 1 row to 2 requires scrolling. MoveToNextLine (CSI E)
        // won't create new lines at the screen bottom — only \r\n will.
        let style = CanvasTextStyle::default();

        let mut prev = Canvas::new(10, 1);
        prev.subview_mut(0, 0, 0, 0, 10, 1)
            .set_text(0, 0, "hello", style);

        let mut next = Canvas::new(10, 2);
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "hello", style);
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "world", style);

        let (dest, diff_buf) = new_test_writer();
        let mut term = new_inline_term(dest, prev.height() as _);
        term.write_canvas(Some(&prev), &next).unwrap();

        // Fill the VT so the canvas starts on the last row, then apply the diff.
        let mut setup = Vec::new();
        let vt_rows = 5;
        for i in 0..vt_rows - 1 {
            write!(setup, "line{i}\r\n").unwrap();
        }
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&diff_buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, vt_rows);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        // The VT should have scrolled: line0 is gone, canvas occupies last 2 rows.
        assert_eq!(vt.line(vt_rows - 2).text(), "hello     ");
        assert_eq!(vt.line(vt_rows - 1).text(), "world     ");
        assert_eq!(
            vt.cursor().row,
            vt_rows - 1,
            "cursor on last row of new canvas"
        );
    }

    #[test]
    fn test_inline_diff_identical_canvas_is_noop() {
        let mut canvas = Canvas::new(10, 2);
        canvas
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "hello", CanvasTextStyle::default());
        canvas
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "world", CanvasTextStyle::default());

        let (dest, diff_buf) = new_test_writer();
        let mut term = new_inline_term(dest, canvas.height() as _);
        term.write_canvas(Some(&canvas), &canvas).unwrap();

        assert!(
            diff_buf.lock().unwrap().is_empty(),
            "identical canvas should produce no output"
        );
    }

    #[test]
    fn test_fullscreen_diff_identical_canvas_is_noop() {
        let mut canvas = Canvas::new(10, 2);
        canvas
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "hello", CanvasTextStyle::default());
        canvas
            .subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "world", CanvasTextStyle::default());

        let (dest, diff_buf) = new_test_writer();
        let mut term = new_fullscreen_term(dest, 0, canvas.height() as _);
        term.write_canvas(Some(&canvas), &canvas).unwrap();

        // Fullscreen always queues a final MoveTo for cursor repositioning,
        // but no row content should be written. Verify by checking the output
        // contains no row data (the only bytes are the trailing MoveTo).
        let buf = diff_buf.lock().unwrap();
        let s = String::from_utf8(buf.clone()).unwrap();
        assert!(
            !s.contains("hello") && !s.contains("world"),
            "identical canvas should not rewrite any row content"
        );
    }

    #[test]
    fn test_inline_diff_styled_text_preserved() {
        let bold_style = CanvasTextStyle {
            weight: Weight::Bold,
            color: Some(Color::Red),
            ..Default::default()
        };

        let mut prev = Canvas::new(10, 2);
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "hello", bold_style);
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "old", CanvasTextStyle::default());

        let mut next = Canvas::new(10, 2);
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "hello", bold_style);
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "new", bold_style);

        let (dest, diff_buf) = new_test_writer();
        let mut term = new_inline_term(dest, prev.height() as _);
        term.write_canvas(Some(&prev), &next).unwrap();

        let mut setup = Vec::new();
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&diff_buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, 5);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        // Row 0 unchanged: bold red "hello"
        let row0 = vt.line(0);
        assert_eq!(row0.text(), "hello     ");
        assert!(row0.cells()[0].pen().is_bold());
        assert!(row0.cells()[0].pen().foreground().is_some());

        // Row 1 updated: bold red "new"
        let row1 = vt.line(1);
        assert_eq!(row1.text(), "new       ");
        assert!(row1.cells()[0].pen().is_bold());
        assert!(row1.cells()[0].pen().foreground().is_some());
    }

    #[test]
    fn test_fullscreen_diff_styled_text_preserved() {
        let underline_style = CanvasTextStyle {
            underline: true,
            color: Some(Color::Green),
            ..Default::default()
        };

        let mut prev = Canvas::new(10, 2);
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "keep", underline_style);
        prev.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "old", CanvasTextStyle::default());

        let mut next = Canvas::new(10, 2);
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "keep", underline_style);
        next.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "new", underline_style);

        let (dest, diff_buf) = new_test_writer();
        let mut term = new_fullscreen_term(dest, 0, prev.height() as _);
        term.write_canvas(Some(&prev), &next).unwrap();

        let mut setup = Vec::new();
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&diff_buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, 5);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        // Row 0 unchanged
        let row0 = vt.line(0);
        assert_eq!(row0.text(), "keep      ");
        assert!(row0.cells()[0].pen().is_underline());

        // Row 1 updated with underline green
        let row1 = vt.line(1);
        assert_eq!(row1.text(), "new       ");
        assert!(row1.cells()[0].pen().is_underline());
        assert!(row1.cells()[0].pen().foreground().is_some());
    }

    #[test]
    fn test_inline_diff_at_terminal_height_boundary() {
        // Canvas height == terminal height uses the normal diff path when only
        // visible rows changed (no off-screen changes trigger a fallback).
        let mut prev = Canvas::new(10, 5);
        prev.subview_mut(0, 0, 0, 0, 10, 5)
            .set_text(0, 0, "aaa", CanvasTextStyle::default());
        prev.subview_mut(0, 0, 0, 0, 10, 5)
            .set_text(0, 4, "bbb", CanvasTextStyle::default());

        let mut next = Canvas::new(10, 5);
        next.subview_mut(0, 0, 0, 0, 10, 5)
            .set_text(0, 0, "aaa", CanvasTextStyle::default());
        next.subview_mut(0, 0, 0, 0, 10, 5)
            .set_text(0, 4, "ccc", CanvasTextStyle::default());

        let (_diff, vt) = inline_diff_vt(&prev, &next, (10, 5));

        assert_eq!(vt.line(0).text(), "aaa       ");
        assert_eq!(vt.line(4).text(), "ccc       ");
    }

    #[test]
    fn test_inline_diff_tall_canvas_visible_change() {
        // Canvas (8 rows) taller than terminal (5 rows). Only the last row
        // changes, which is in the visible area — the normal diff path should
        // handle it without a full clear+rewrite.
        let style = CanvasTextStyle::default();

        let mut prev = Canvas::new(10, 8);
        for i in 0..8 {
            prev.subview_mut(0, 0, 0, 0, 10, 8)
                .set_text(0, i, &format!("row{i}"), style);
        }

        let mut next = Canvas::new(10, 8);
        for i in 0..7 {
            next.subview_mut(0, 0, 0, 0, 10, 8)
                .set_text(0, i, &format!("row{i}"), style);
        }
        next.subview_mut(0, 0, 0, 0, 10, 8)
            .set_text(0, 7, "CHANGED", style);

        let (diff, vt) = inline_diff_vt(&prev, &next, (10, 5));

        // Should NOT contain a full clear (ClearAll = ESC[2J)
        let diff_str = String::from_utf8_lossy(&diff);
        assert!(
            !diff_str.contains("\x1b[2J"),
            "expected row-level diff, not full clear; got: {diff_str:?}"
        );

        // The bottom 5 rows of the 8-row canvas are visible in the terminal.
        assert_eq!(vt.line(0).text(), "row3      ");
        assert_eq!(vt.line(4).text(), "CHANGED   ");
    }

    #[test]
    fn test_inline_diff_tall_canvas_offscreen_change() {
        // Canvas (8 rows) taller than terminal (5 rows). A row above the
        // visible area changes — this must trigger the full-rewrite fallback
        // since we can't cursor to an off-screen row.
        let style = CanvasTextStyle::default();

        let mut prev = Canvas::new(10, 8);
        for i in 0..8 {
            prev.subview_mut(0, 0, 0, 0, 10, 8)
                .set_text(0, i, &format!("row{i}"), style);
        }

        let mut next = Canvas::new(10, 8);
        for i in 0..8 {
            next.subview_mut(0, 0, 0, 0, 10, 8)
                .set_text(0, i, &format!("row{i}"), style);
        }
        // Change row 1, which is above the visible area (visible_start = 8-5 = 3).
        next.subview_mut(0, 0, 0, 0, 10, 8)
            .set_text(0, 1, "OFFSCR", style);

        let (diff, vt) = inline_diff_vt(&prev, &next, (10, 5));

        // Should contain a full clear (ClearAll = ESC[2J, because
        // prev_canvas_height >= term_height triggers the heavy clear path).
        let diff_str = String::from_utf8_lossy(&diff);
        assert!(
            diff_str.contains("\x1b[2J"),
            "expected full clear fallback; got: {diff_str:?}"
        );

        // After full rewrite, the bottom 5 rows of the new canvas are visible.
        assert_eq!(vt.line(0).text(), "row3      ");
        assert_eq!(vt.line(4).text(), "row7      ");
    }

    #[test]
    fn test_inline_diff_sequential_updates() {
        let style = CanvasTextStyle::default();

        let mut c1 = Canvas::new(10, 2);
        c1.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "aaa", style);
        c1.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "bbb", style);

        let mut c2 = Canvas::new(10, 2);
        c2.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "aaa", style);
        c2.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "ccc", style);

        let mut c3 = Canvas::new(10, 3);
        c3.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 0, "xxx", style);
        c3.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 1, "ccc", style);
        c3.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 2, "ddd", style);

        let (dest, buf) = new_test_writer();
        let mut term = new_inline_term(dest, c1.height() as _);

        // First diff: c1 -> c2
        term.write_canvas(Some(&c1), &c2).unwrap();
        // Second diff: c2 -> c3
        term.write_canvas(Some(&c2), &c3).unwrap();

        let mut setup = Vec::new();
        c1.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, 6);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        assert_eq!(vt.line(0).text(), "xxx       ");
        assert_eq!(vt.line(1).text(), "ccc       ");
        assert_eq!(vt.line(2).text(), "ddd       ");
        assert_eq!(vt.cursor().row, 2, "cursor on last row of final canvas");
    }

    #[test]
    fn test_fullscreen_diff_sequential_updates() {
        let style = CanvasTextStyle::default();

        let mut c1 = Canvas::new(10, 2);
        c1.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "aaa", style);
        c1.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "bbb", style);

        let mut c2 = Canvas::new(10, 2);
        c2.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 0, "aaa", style);
        c2.subview_mut(0, 0, 0, 0, 10, 2)
            .set_text(0, 1, "ccc", style);

        let mut c3 = Canvas::new(10, 3);
        c3.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 0, "xxx", style);
        c3.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 1, "ccc", style);
        c3.subview_mut(0, 0, 0, 0, 10, 3)
            .set_text(0, 2, "ddd", style);

        let (dest, buf) = new_test_writer();
        let mut term = new_fullscreen_term(dest, 0, c1.height() as _);

        term.write_canvas(Some(&c1), &c2).unwrap();
        term.write_canvas(Some(&c2), &c3).unwrap();

        let mut setup = Vec::new();
        c1.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&buf.lock().unwrap());

        let mut vt = avt::Vt::new(10, 6);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        assert_eq!(vt.line(0).text(), "xxx       ");
        assert_eq!(vt.line(1).text(), "ccc       ");
        assert_eq!(vt.line(2).text(), "ddd       ");
        assert_eq!(vt.cursor().row, 2, "cursor on last row of final canvas");
    }

    #[test]
    fn test_borrowed_writers() {
        let mut stdout_buf: Vec<u8> = Vec::new();
        let mut stderr_buf: Vec<u8> = Vec::new();

        {
            let mut terminal = Terminal::new(
                Box::new(&mut stdout_buf),
                Box::new(&mut stderr_buf),
                Output::Stdout,
            )
            .unwrap();
            let canvas = Canvas::new(10, 1);
            terminal.write_canvas(None, &canvas).unwrap();
        }

        assert!(!stdout_buf.is_empty());
    }

    /// Helper: build a pair of 10×5 canvases (4 content rows + 1 footer) that
    /// differ only in a single cell's background color on `changed_row`,
    /// simulating a mouse-highlight overlay.
    fn make_fullscreen_diff_canvases(changed_row: usize) -> (Canvas, Canvas) {
        let style = CanvasTextStyle::default();
        let width = 10;
        let height = 5;

        let build = |highlight: bool| {
            let mut c = Canvas::new(width, height);
            let mut sv = c.subview_mut(0, 0, 0, 0, width, height);
            for y in 0..4u32 {
                sv.set_text(0, y as isize, &format!("row{y}"), style);
            }
            sv.set_text(0, 4, "FOOTER", style);
            sv.set_background_color(0, 4, width, 1, Color::Green);
            if highlight {
                sv.set_background_color(0, changed_row as isize, 1, 1, Color::Yellow);
            }
            c
        };

        (build(false), build(true))
    }

    /// Verify that with `prev_canvas_top_row = 0` the fullscreen row-level
    /// diff writes each changed row to its correct terminal position.
    ///
    /// Uses a layout with numbered content rows and a distinct footer, where
    /// a single cell changes between frames (as a mouse-highlight overlay
    /// would cause).
    #[test]
    fn test_fullscreen_diff_zero_top_row_renders_correctly() {
        let (prev, next) = make_fullscreen_diff_canvases(2);
        let width = prev.width();
        let height = prev.height();

        let (dest, buf) = new_test_writer();
        let mut term = new_fullscreen_term(dest, 0, height as _);
        term.write_canvas(Some(&prev), &next).unwrap();

        // Replay: write prev canvas as the baseline already on screen,
        // then apply the diff on top.
        let mut setup = Vec::new();
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&buf.lock().unwrap());

        let mut vt = avt::Vt::new(width, height + 2);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        assert_eq!(vt.line(0).text(), "row0      ");
        assert_eq!(vt.line(1).text(), "row1      ");
        assert_eq!(vt.line(2).text(), "row2      ");
        assert_eq!(vt.line(3).text(), "row3      ");
        assert_eq!(
            vt.line(4).text(),
            "FOOTER    ",
            "every row must appear at its correct terminal position"
        );
    }

    /// Counterpart: with a non-zero `prev_canvas_top_row`, every changed row Y
    /// is written to terminal line `top_row + Y` instead of line Y.  Unchanged
    /// rows are skipped by `row_eq`, so the corruption is never self-correcting.
    ///
    /// This demonstrates why `prev_canvas_top_row` must be anchored at 0 in
    /// fullscreen mode — any stale cursor position causes the entire diff to
    /// be offset.
    #[test]
    fn test_fullscreen_diff_nonzero_top_row_offsets_changed_rows() {
        let (prev, next) = make_fullscreen_diff_canvases(1);
        let width = prev.width();
        let height = prev.height();

        // With top_row = 2 (simulating a stale cursor value), the diff for
        // changed row 1 writes to terminal position 2+1 = 3 instead of 1.
        let (dest, buf) = new_test_writer();
        let mut term = new_fullscreen_term(dest, 2, height as _);
        term.write_canvas(Some(&prev), &next).unwrap();

        let mut setup = Vec::new();
        prev.write_ansi_without_final_newline(&mut setup).unwrap();
        setup.extend_from_slice(&buf.lock().unwrap());

        let mut vt = avt::Vt::new(width, height + 4);
        vt.feed_str(&String::from_utf8(setup).unwrap());

        // Row 1's diff landed at terminal line 3 (offset 2+1) instead of 1,
        // overwriting row 3's original content.
        assert_eq!(
            vt.line(3).text(),
            "row1      ",
            "row 1's diff landed at terminal line 3 (offset 2+1) instead of line 1"
        );
    }

    /// Regression test: exercises the full initial-write → diff pipeline.
    ///
    /// `write_canvas(None, …)` must set `prev_canvas_top_row` to 0.  This path
    /// runs both on the very first frame and whenever `clear_terminal_output()`
    /// triggers a full rewrite on a subsequent frame — in either case the
    /// cursor may not be at (0, 0), so the code must reset it explicitly.
    ///
    /// Without the fix, the old code called `cursor::position()` which returns
    /// a stale value inside `BeginSynchronizedUpdate` on real terminals, and
    /// fails outright in non-TTY test environments (timeout → panic).
    #[test]
    fn test_fullscreen_initial_write_sets_zero_top_row() {
        let (initial, next) = make_fullscreen_diff_canvases(2);
        let width = initial.width();
        let height = initial.height();

        let (dest, buf) = new_test_writer();
        let mut term = new_fullscreen_term(dest, 99, 0); // start with intentionally wrong value

        // The initial write must set prev_canvas_top_row = 0 (the fix).
        // Without the fix, this panics due to cursor::position() timeout.
        term.write_canvas(None, &initial).unwrap();
        assert_eq!(
            term.prev_canvas_top_row, 0,
            "initial fullscreen write must anchor prev_canvas_top_row at 0"
        );

        // Subsequent diff should render correctly with top_row = 0.
        term.write_canvas(Some(&initial), &next).unwrap();

        let mut vt = avt::Vt::new(width, height + 2);
        vt.feed_str(&String::from_utf8(buf.lock().unwrap().clone()).unwrap());

        assert_eq!(vt.line(0).text(), "row0      ");
        assert_eq!(vt.line(1).text(), "row1      ");
        assert_eq!(vt.line(2).text(), "row2      ");
        assert_eq!(vt.line(3).text(), "row3      ");
        assert_eq!(vt.line(4).text(), "FOOTER    ");
    }
}
