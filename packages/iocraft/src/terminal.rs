use crate::{canvas::Canvas, element::Output};
use crossterm::{
    cursor,
    event::{self, Event, EventStream},
    execute, queue, terminal,
};
use futures::{
    channel::mpsc,
    future::pending,
    stream::{self, BoxStream, Stream, StreamExt},
};
use std::{
    collections::VecDeque,
    io::{self, stderr, stdin, stdout, IsTerminal, LineWriter, Write},
    mem,
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    task::{Context, Poll, Waker},
};

/// Configuration for output handles used by the render loop.
pub struct TerminalConfig {
    /// The stdout handle for hook output.
    pub stdout: Arc<Mutex<Box<dyn Write + Send>>>,
    /// The stderr handle for hook output.
    pub stderr: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Which handle to render the TUI to.
    pub render_to: Output,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            stdout: Arc::new(Mutex::new(Box::new(stdout()))),
            stderr: Arc::new(Mutex::new(Box::new(LineWriter::new(stderr())))),
            render_to: Output::default(),
        }
    }
}

/// A writer that delegates to a shared handle.
struct SharedWriter {
    inner: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.lock().unwrap().flush()
    }
}

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

    fn is_raw_mode_enabled(&self) -> bool;
    fn clear_canvas(&mut self) -> io::Result<()>;
    fn write_canvas(&mut self, canvas: &Canvas) -> io::Result<()>;
    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>>;
}

struct StdTerminal<W: Write + Send> {
    input_is_terminal: bool,
    dest: W,
    fullscreen: bool,
    raw_mode_enabled: bool,
    enabled_keyboard_enhancement: bool,
    prev_canvas_height: u16,
    size: Option<(u16, u16)>,
}

impl<W: Write + Send> Write for StdTerminal<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.dest.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.dest.flush()
    }
}

impl<W: Write + Send> TerminalImpl for StdTerminal<W> {
    fn refresh_size(&mut self) {
        self.size = terminal::size().ok()
    }

    fn size(&self) -> Option<(u16, u16)> {
        self.size
    }

    fn is_raw_mode_enabled(&self) -> bool {
        self.raw_mode_enabled
    }

    fn clear_canvas(&mut self) -> io::Result<()> {
        if self.prev_canvas_height == 0 {
            return Ok(());
        }

        if !self.fullscreen {
            if let Some(size) = self.size {
                if self.prev_canvas_height >= size.1 {
                    // We have to clear the entire terminal to avoid leaving artifacts.
                    // See: https://github.com/ccbrown/iocraft/issues/118
                    return queue!(
                        self.dest,
                        terminal::Clear(terminal::ClearType::Purge),
                        cursor::MoveTo(0, 0),
                    );
                }
            }
        }

        let lines_to_rewind = self.prev_canvas_height - if self.fullscreen { 1 } else { 0 };
        queue!(
            self.dest,
            cursor::MoveToPreviousLine(lines_to_rewind as _),
            terminal::Clear(terminal::ClearType::FromCursorDown)
        )
    }

    fn write_canvas(&mut self, canvas: &Canvas) -> io::Result<()> {
        self.prev_canvas_height = canvas.height() as _;
        if self.fullscreen {
            canvas.write_ansi_without_final_newline(self)?;
        } else {
            canvas.write_ansi(self)?;
        }
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
}

impl<W: Write + Send> StdTerminal<W> {
    fn new(mut dest: W, fullscreen: bool) -> io::Result<Self> {
        queue!(dest, cursor::Hide)?;
        if fullscreen {
            queue!(dest, terminal::EnterAlternateScreen)?;
        }
        Ok(Self {
            dest,
            input_is_terminal: stdin().is_terminal(),
            fullscreen,
            raw_mode_enabled: false,
            enabled_keyboard_enhancement: false,
            prev_canvas_height: 0,
            size: None,
        })
    }

    fn set_raw_mode_enabled(&mut self, raw_mode_enabled: bool) -> io::Result<()> {
        if raw_mode_enabled != self.raw_mode_enabled {
            if raw_mode_enabled {
                if terminal::supports_keyboard_enhancement().unwrap_or(false) {
                    execute!(
                        self.dest,
                        event::PushKeyboardEnhancementFlags(
                            event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                        )
                    )?;
                    self.enabled_keyboard_enhancement = true;
                }
                if self.fullscreen {
                    execute!(self.dest, event::EnableMouseCapture)?;
                }
                terminal::enable_raw_mode()?;
            } else {
                terminal::disable_raw_mode()?;
                if self.fullscreen {
                    execute!(self.dest, event::DisableMouseCapture)?;
                }
                if self.enabled_keyboard_enhancement {
                    execute!(self.dest, event::PopKeyboardEnhancementFlags)?;
                }
            }
            self.raw_mode_enabled = raw_mode_enabled;
        }
        Ok(())
    }
}

impl<W: Write + Send> Drop for StdTerminal<W> {
    fn drop(&mut self) {
        let _ = self.set_raw_mode_enabled(false);
        if self.fullscreen {
            let _ = queue!(self.dest, terminal::LeaveAlternateScreen);
        }
        let _ = execute!(self.dest, cursor::Show);
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
}

impl MockTerminal {
    fn new(config: MockTerminalConfig) -> (Self, MockTerminalOutputStream) {
        let (output_tx, output_rx) = mpsc::unbounded();
        let output = MockTerminalOutputStream { inner: output_rx };
        (
            Self {
                config,
                output: output_tx,
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

    fn write_canvas(&mut self, canvas: &Canvas) -> io::Result<()> {
        let _ = self.output.unbounded_send(canvas.clone());
        Ok(())
    }

    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>> {
        let mut events = stream::pending().boxed();
        mem::swap(&mut events, &mut self.config.events);
        Ok(events.chain(stream::pending()).boxed())
    }
}

pub(crate) struct Terminal {
    inner: Box<dyn TerminalImpl>,
    event_stream: Option<BoxStream<'static, TerminalEvent>>,
    subscribers: Vec<Weak<Mutex<TerminalEventsInner>>>,
    received_ctrl_c: bool,
    ignore_ctrl_c: bool,
    terminal_config: TerminalConfig,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        Self::with_terminal_config(TerminalConfig::default(), false)
    }

    pub fn fullscreen() -> io::Result<Self> {
        Self::with_terminal_config(TerminalConfig::default(), true)
    }

    pub fn with_terminal_config(
        terminal_config: TerminalConfig,
        fullscreen: bool,
    ) -> io::Result<Self> {
        let writer = SharedWriter {
            inner: match terminal_config.render_to {
                Output::Stdout => terminal_config.stdout.clone(),
                Output::Stderr => terminal_config.stderr.clone(),
            },
        };
        Ok(Self::new_with_impl(
            StdTerminal::new(writer, fullscreen)?,
            terminal_config,
        ))
    }

    pub fn mock(config: MockTerminalConfig) -> (Self, MockTerminalOutputStream) {
        let (term, output) = MockTerminal::new(config);
        (Self::new_with_impl(term, TerminalConfig::default()), output)
    }

    fn new_with_impl<T: TerminalImpl + 'static>(inner: T, terminal_config: TerminalConfig) -> Self {
        Self {
            inner: Box::new(inner),
            event_stream: None,
            subscribers: Vec::new(),
            received_ctrl_c: false,
            ignore_ctrl_c: false,
            terminal_config,
        }
    }

    pub fn terminal_config(&self) -> &TerminalConfig {
        &self.terminal_config
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

    pub fn write_canvas(&mut self, canvas: &Canvas) -> io::Result<()> {
        self.inner.write_canvas(canvas)
    }

    pub fn received_ctrl_c(&self) -> bool {
        self.received_ctrl_c
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

impl Write for Terminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Synchronized update terminal guard.
/// Enters synchronized update on creation, exits when dropped.
pub(crate) struct SynchronizedUpdate<'a> {
    inner: &'a mut Terminal,
}

impl<'a> SynchronizedUpdate<'a> {
    pub fn begin(terminal: &'a mut Terminal) -> io::Result<Self> {
        execute!(terminal, terminal::BeginSynchronizedUpdate)?;
        Ok(Self { inner: terminal })
    }
}

impl Drop for SynchronizedUpdate<'_> {
    fn drop(&mut self) {
        let _ = execute!(self.inner, terminal::EndSynchronizedUpdate);
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_std_terminal() {
        // There's unfortunately not much here we can really test, but we'll do our best.
        // TODO: Is there a library we can use to emulate terminal input/output?
        let mut terminal = Terminal::new().unwrap();
        assert!(!terminal.is_raw_mode_enabled());
        assert!(!terminal.received_ctrl_c());
        assert!(!terminal.is_raw_mode_enabled());
        let canvas = Canvas::new(10, 1);
        terminal.write_canvas(&canvas).unwrap();
    }
}
