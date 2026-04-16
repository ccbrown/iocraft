use crate::{canvas::Canvas, element::Output};
use crossterm::{
    cursor,
    event::{self, Event, EventStream},
    terminal, ExecutableCommand, QueueableCommand,
};
use futures::{
    channel::mpsc,
    future::pending,
    stream::{self, BoxStream, Stream, StreamExt},
};
use std::{
    collections::VecDeque,
    io::{self, stdin, IsTerminal, Write},
    mem,
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    task::{Context, Poll, Waker},
};

// Re-exports for basic types.
pub use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers, MouseEventKind};

/// An event fired when a key is pressed.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct KeyEvent {
    /// A code indicating the key that was pressed.
    pub code: KeyCode,

    /// The modifiers that were active when the key was pressed.
    pub modifiers: KeyModifiers,

    /// Whether the key was pressed or released.
    pub kind: KeyEventKind,
}

impl KeyEvent {
    /// Creates a new `KeyEvent`.
    pub fn new(kind: KeyEventKind, code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::empty(),
            kind,
        }
    }
}

/// An event fired when the mouse is moved, clicked, scrolled, etc. in fullscreen mode.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct FullscreenMouseEvent {
    /// The modifiers that were active when the event occurred.
    pub modifiers: KeyModifiers,

    /// The column that the event occurred on.
    pub column: u16,

    /// The row that the event occurred on.
    pub row: u16,

    /// The kind of mouse event.
    pub kind: MouseEventKind,
}

impl FullscreenMouseEvent {
    /// Creates a new `FullscreenMouseEvent`.
    pub fn new(kind: MouseEventKind, column: u16, row: u16) -> Self {
        Self {
            modifiers: KeyModifiers::empty(),
            column,
            row,
            kind,
        }
    }
}

/// An event fired by the terminal.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum TerminalEvent {
    /// A key event, fired when a key is pressed.
    Key(KeyEvent),
    /// A mouse event, fired when the mouse is moved, clicked, scrolled, etc. in fullscreen mode.
    FullscreenMouse(FullscreenMouseEvent),
    /// A resize event, fired when the terminal is resized.
    Resize(u16, u16),
}

struct TerminalEventsInner {
    pending: VecDeque<TerminalEvent>,
    waker: Option<Waker>,
}

/// A stream of terminal events.
pub struct TerminalEvents {
    inner: Arc<Mutex<TerminalEventsInner>>,
}

impl Stream for TerminalEvents {
    type Item = TerminalEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(event) = inner.pending.pop_front() {
            Poll::Ready(Some(event))
        } else {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

trait TerminalImpl: Write + Send {
    fn refresh_size(&mut self) {}
    fn size(&self) -> Option<(u16, u16)> {
        None
    }
    fn set_mouse_capture(&mut self, _enabled: bool) -> io::Result<()> {
        Ok(())
    }

    fn is_raw_mode_enabled(&self) -> bool;
    fn clear_canvas(&mut self) -> io::Result<()>;
    fn write_canvas(&mut self, prev: Option<&Canvas>, canvas: &Canvas) -> io::Result<()>;
    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>>;
    fn dest(&mut self) -> &mut dyn Write;
    fn alt(&mut self) -> &mut dyn Write;
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

struct StdTerminal<'a> {
    input_is_terminal: bool,
    dest: Box<dyn Write + Send + 'a>,
    alt: Box<dyn Write + Send + 'a>,
    fullscreen: bool,
    mouse_capture: bool,
    raw_mode_enabled: bool,
    enabled_keyboard_enhancement: bool,
    prev_canvas_top_row: u16,
    prev_canvas_height: u16,
    size: Option<(u16, u16)>,
}

impl Write for StdTerminal<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.dest.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.dest.flush()
    }
}

impl TerminalImpl for StdTerminal<'_> {
    fn refresh_size(&mut self) {
        self.size = terminal::size().ok()
    }

    fn size(&self) -> Option<(u16, u16)> {
        self.size
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

    fn is_raw_mode_enabled(&self) -> bool {
        self.raw_mode_enabled
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
                self.dest.flush()?;
                self.alt.flush()?;
                // In fullscreen (alternate screen) the cursor is guaranteed to
                // be at (0, 0) after EnterAlternateScreen.  Calling
                // cursor::position() inside BeginSynchronizedUpdate can return
                // a stale value from the main screen on some terminals.
                self.prev_canvas_top_row = 0;
                self.dest.queue(cursor::MoveTo(0, 0))?;
            }
            self.prev_canvas_height = canvas.height() as _;
            canvas.write_ansi_without_final_newline(&mut *self.dest)?;
            return Ok(());
        };

        if self.fullscreen {
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

    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>> {
        if !self.input_is_terminal {
            return Ok(stream::pending().boxed());
        }

        self.set_raw_mode_enabled(true)?;

        Ok(EventStream::new()
            .filter_map(|event| async move {
                match event {
                    Ok(Event::Key(event)) => Some(TerminalEvent::Key(KeyEvent {
                        code: event.code,
                        modifiers: event.modifiers,
                        kind: event.kind,
                    })),
                    Ok(Event::Mouse(event)) => {
                        Some(TerminalEvent::FullscreenMouse(FullscreenMouseEvent {
                            modifiers: event.modifiers,
                            column: event.column,
                            row: event.row,
                            kind: event.kind,
                        }))
                    }
                    Ok(Event::Resize(width, height)) => Some(TerminalEvent::Resize(width, height)),
                    _ => None,
                }
            })
            .boxed())
    }

    fn dest(&mut self) -> &mut dyn Write {
        &mut *self.dest
    }

    fn alt(&mut self) -> &mut dyn Write {
        &mut *self.alt
    }
}

impl<'a> StdTerminal<'a> {
    fn new(
        dest: Box<dyn Write + Send + 'a>,
        alt: Box<dyn Write + Send + 'a>,
        fullscreen: bool,
        mouse_capture: bool,
    ) -> io::Result<Self> {
        let mut term = Self {
            dest,
            alt,
            input_is_terminal: stdin().is_terminal(),
            fullscreen,
            mouse_capture,
            raw_mode_enabled: false,
            enabled_keyboard_enhancement: false,
            prev_canvas_top_row: 0,
            prev_canvas_height: 0,
            size: None,
        };
        term.dest.queue(cursor::Hide)?;
        if fullscreen {
            term.dest.queue(terminal::EnterAlternateScreen)?;
        }
        Ok(term)
    }

    fn set_raw_mode_enabled(&mut self, raw_mode_enabled: bool) -> io::Result<()> {
        if raw_mode_enabled != self.raw_mode_enabled {
            if raw_mode_enabled {
                if terminal::supports_keyboard_enhancement().unwrap_or(false) {
                    self.dest.execute(event::PushKeyboardEnhancementFlags(
                        event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
                    ))?;
                    self.enabled_keyboard_enhancement = true;
                }
                if self.mouse_capture {
                    self.dest.execute(event::EnableMouseCapture)?;
                }
                terminal::enable_raw_mode()?;
            } else {
                terminal::disable_raw_mode()?;
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

impl Drop for StdTerminal<'_> {
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

pub(crate) struct MockTerminalOutputStream {
    inner: mpsc::UnboundedReceiver<Canvas>,
}

impl Stream for MockTerminalOutputStream {
    type Item = Canvas;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

/// Used to provide the configuration for a mock terminal which can be used for testing.
///
/// This can be passed to [`ElementExt::mock_terminal_render_loop`](crate::ElementExt::mock_terminal_render_loop) for testing your dynamic components.
#[non_exhaustive]
pub struct MockTerminalConfig {
    /// The events to be emitted by the mock terminal.
    pub events: BoxStream<'static, TerminalEvent>,
}

impl MockTerminalConfig {
    /// Creates a new `MockTerminalConfig` with the given event stream.
    pub fn with_events<T: Stream<Item = TerminalEvent> + Send + 'static>(events: T) -> Self {
        Self {
            events: events.boxed(),
        }
    }
}

impl Default for MockTerminalConfig {
    fn default() -> Self {
        Self {
            events: stream::pending().boxed(),
        }
    }
}

struct MockTerminal {
    config: MockTerminalConfig,
    output: mpsc::UnboundedSender<Canvas>,
    dummy_dest: io::Sink,
    dummy_alt: io::Sink,
}

impl MockTerminal {
    fn new(config: MockTerminalConfig) -> (Self, MockTerminalOutputStream) {
        let (output_tx, output_rx) = mpsc::unbounded();
        let output = MockTerminalOutputStream { inner: output_rx };
        (
            Self {
                config,
                output: output_tx,
                dummy_dest: io::sink(),
                dummy_alt: io::sink(),
            },
            output,
        )
    }
}

impl Write for MockTerminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TerminalImpl for MockTerminal {
    fn is_raw_mode_enabled(&self) -> bool {
        false
    }

    fn clear_canvas(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_canvas(&mut self, _prev: Option<&Canvas>, canvas: &Canvas) -> io::Result<()> {
        let _ = self.output.unbounded_send(canvas.clone());
        Ok(())
    }

    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>> {
        let mut events = stream::pending().boxed();
        mem::swap(&mut events, &mut self.config.events);
        Ok(events.chain(stream::pending()).boxed())
    }

    fn dest(&mut self) -> &mut dyn Write {
        &mut self.dummy_dest
    }

    fn alt(&mut self) -> &mut dyn Write {
        &mut self.dummy_alt
    }
}

pub(crate) struct Terminal<'a> {
    inner: Box<dyn TerminalImpl + 'a>,
    output: Output,
    event_stream: Option<BoxStream<'static, TerminalEvent>>,
    subscribers: Vec<Weak<Mutex<TerminalEventsInner>>>,
    received_ctrl_c: bool,
    ignore_ctrl_c: bool,
}

impl<'a> Terminal<'a> {
    pub fn new(
        stdout: Box<dyn Write + Send + 'a>,
        stderr: Box<dyn Write + Send + 'a>,
        output: Output,
        fullscreen: bool,
        mouse_capture: bool,
    ) -> io::Result<Self> {
        // dest is the render destination, alt is the other stream
        let (dest, alt) = match output {
            Output::Stdout => (stdout, stderr),
            Output::Stderr => (stderr, stdout),
        };
        Ok(Self {
            inner: Box::new(StdTerminal::new(dest, alt, fullscreen, mouse_capture)?),
            output,
            event_stream: None,
            subscribers: Vec::new(),
            received_ctrl_c: false,
            ignore_ctrl_c: false,
        })
    }

    pub fn enable_mouse_capture(&mut self) -> io::Result<()> {
        self.inner.set_mouse_capture(true)
    }

    pub fn disable_mouse_capture(&mut self) -> io::Result<()> {
        self.inner.set_mouse_capture(false)
    }

    pub fn ignore_ctrl_c(&mut self) {
        self.ignore_ctrl_c = true;
    }

    pub fn is_raw_mode_enabled(&self) -> bool {
        self.inner.is_raw_mode_enabled()
    }

    pub fn refresh_size(&mut self) {
        self.inner.refresh_size()
    }

    pub fn size(&self) -> Option<(u16, u16)> {
        self.inner.size()
    }

    pub fn clear_canvas(&mut self) -> io::Result<()> {
        self.inner.clear_canvas()
    }

    pub fn write_canvas(&mut self, prev: Option<&Canvas>, canvas: &Canvas) -> io::Result<()> {
        self.inner.write_canvas(prev, canvas)
    }

    pub fn received_ctrl_c(&self) -> bool {
        self.received_ctrl_c
    }

    /// Returns a mutable reference to the stdout handle.
    pub fn stdout(&mut self) -> &mut dyn Write {
        match self.output {
            Output::Stdout => self.inner.dest(),
            Output::Stderr => self.inner.alt(),
        }
    }

    /// Returns a mutable reference to the stderr handle.
    pub fn stderr(&mut self) -> &mut dyn Write {
        match self.output {
            Output::Stdout => self.inner.alt(),
            Output::Stderr => self.inner.dest(),
        }
    }

    /// Returns a mutable reference to the render output handle (stdout or stderr based on output setting).
    pub fn render_output(&mut self) -> &mut dyn Write {
        self.inner.dest()
    }

    /// Wraps a series of terminal updates in a synchronized update block, making sure to end the
    /// synchronized update even if there is an error or panic.
    pub fn synchronized_update<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Self) -> io::Result<()>,
    {
        let t = SynchronizedUpdate::begin(self)?;
        f(t.inner)
    }

    pub async fn wait(&mut self) {
        match &mut self.event_stream {
            Some(event_stream) => {
                while let Some(event) = event_stream.next().await {
                    if !self.ignore_ctrl_c {
                        if let TerminalEvent::Key(KeyEvent {
                            code: KeyCode::Char('c'),
                            kind: KeyEventKind::Press,
                            modifiers: KeyModifiers::CONTROL,
                        }) = event
                        {
                            self.received_ctrl_c = true;
                        }
                        if self.received_ctrl_c {
                            return;
                        }
                    }
                    self.subscribers.retain(|subscriber| {
                        if let Some(subscriber) = subscriber.upgrade() {
                            let mut subscriber = subscriber.lock().unwrap();
                            subscriber.pending.push_back(event.clone());
                            if let Some(waker) = subscriber.waker.take() {
                                waker.wake();
                            }
                            true
                        } else {
                            false
                        }
                    });
                }
            }
            None => pending().await,
        }
    }

    pub fn events(&mut self) -> io::Result<TerminalEvents> {
        if self.event_stream.is_none() {
            self.event_stream = Some(self.inner.event_stream()?);
        }
        let inner = Arc::new(Mutex::new(TerminalEventsInner {
            pending: VecDeque::new(),
            waker: None,
        }));
        self.subscribers.push(Arc::downgrade(&inner));
        Ok(TerminalEvents { inner })
    }
}

impl Terminal<'static> {
    pub fn mock(config: MockTerminalConfig) -> (Self, MockTerminalOutputStream) {
        let (term, output_stream) = MockTerminal::new(config);
        (
            Self {
                inner: Box::new(term),
                output: Output::Stdout,
                event_stream: None,
                subscribers: Vec::new(),
                received_ctrl_c: false,
                ignore_ctrl_c: false,
            },
            output_stream,
        )
    }
}

impl Write for Terminal<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Synchronized update terminal guard.
/// Enters synchronized update on creation, exits when dropped.
pub(crate) struct SynchronizedUpdate<'a, 'b> {
    inner: &'a mut Terminal<'b>,
}

impl<'a, 'b> SynchronizedUpdate<'a, 'b> {
    pub fn begin(terminal: &'a mut Terminal<'b>) -> io::Result<Self> {
        terminal.execute(terminal::BeginSynchronizedUpdate)?;
        Ok(Self { inner: terminal })
    }
}

impl Drop for SynchronizedUpdate<'_, '_> {
    fn drop(&mut self) {
        let _ = self.inner.execute(terminal::EndSynchronizedUpdate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crossterm::QueueableCommand;
    use std::sync::{Arc, Mutex};

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
            false,
            true,
        )
        .unwrap();
        assert!(!terminal.is_raw_mode_enabled());
        assert!(!terminal.received_ctrl_c());
        assert!(!terminal.is_raw_mode_enabled());
        let canvas = Canvas::new(10, 1);
        terminal.write_canvas(None, &canvas).unwrap();
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
        setup.extend_from_slice(&*diff_buf.lock().unwrap());

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
        setup.extend_from_slice(&*clear_buf.lock().unwrap());

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
    ) -> StdTerminal<'static> {
        StdTerminal {
            input_is_terminal: false,
            dest: Box::new(dest),
            alt: Box::new(io::sink()),
            fullscreen: true,
            mouse_capture: false,
            raw_mode_enabled: false,
            enabled_keyboard_enhancement: false,
            prev_canvas_top_row,
            prev_canvas_height,
            size: None,
        }
    }

    fn new_inline_term(dest: TestWriter, prev_canvas_height: u16) -> StdTerminal<'static> {
        new_inline_term_with_size(dest, prev_canvas_height, (10, 10))
    }

    fn new_inline_term_with_size(
        dest: TestWriter,
        prev_canvas_height: u16,
        term_size: (u16, u16),
    ) -> StdTerminal<'static> {
        StdTerminal {
            input_is_terminal: false,
            dest: Box::new(dest),
            alt: Box::new(io::sink()),
            fullscreen: false,
            mouse_capture: false,
            raw_mode_enabled: false,
            enabled_keyboard_enhancement: false,
            prev_canvas_top_row: 0,
            prev_canvas_height,
            size: Some(term_size),
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
        setup.extend_from_slice(&*diff_buf.lock().unwrap());

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
        setup.extend_from_slice(&*diff_buf.lock().unwrap());

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
        setup.extend_from_slice(&*diff_buf.lock().unwrap());

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
        setup.extend_from_slice(&*diff_buf.lock().unwrap());

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
        setup.extend_from_slice(&*diff_buf.lock().unwrap());

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
        setup.extend_from_slice(&*diff_buf.lock().unwrap());

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
        setup.extend_from_slice(&*buf.lock().unwrap());

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
        setup.extend_from_slice(&*buf.lock().unwrap());

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
                false,
                true,
            )
            .unwrap();
            let canvas = Canvas::new(10, 1);
            terminal.write_canvas(None, &canvas).unwrap();
        }

        assert!(!stdout_buf.is_empty());
    }
}
