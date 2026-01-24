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

    fn is_raw_mode_enabled(&self) -> bool;
    fn clear_canvas(&mut self) -> io::Result<()>;
    fn write_canvas(&mut self, canvas: &Canvas) -> io::Result<()>;
    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>>;
    fn stdout(&mut self) -> &mut dyn Write;
    fn stderr(&mut self) -> &mut dyn Write;
}

struct StdTerminal {
    input_is_terminal: bool,
    stdout: Box<dyn Write + Send>,
    stderr: Box<dyn Write + Send>,
    fullscreen: bool,
    raw_mode_enabled: bool,
    enabled_keyboard_enhancement: bool,
    prev_canvas_height: u16,
    size: Option<(u16, u16)>,
}

impl Write for StdTerminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}

impl TerminalImpl for StdTerminal {
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
                    self.stdout
                        .queue(terminal::Clear(terminal::ClearType::Purge))?
                        .queue(cursor::MoveTo(0, 0))?;
                    return Ok(());
                }
            }
        }

        let lines_to_rewind = self.prev_canvas_height - if self.fullscreen { 1 } else { 0 };
        self.stdout
            .queue(cursor::MoveToPreviousLine(lines_to_rewind as _))?
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        Ok(())
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

    fn stdout(&mut self) -> &mut dyn Write {
        &mut *self.stdout
    }

    fn stderr(&mut self) -> &mut dyn Write {
        &mut *self.stderr
    }
}

impl StdTerminal {
    fn new(
        stdout: Box<dyn Write + Send>,
        stderr: Box<dyn Write + Send>,
        fullscreen: bool,
    ) -> io::Result<Self> {
        let mut term = Self {
            stdout,
            stderr,
            input_is_terminal: stdin().is_terminal(),
            fullscreen,
            raw_mode_enabled: false,
            enabled_keyboard_enhancement: false,
            prev_canvas_height: 0,
            size: None,
        };
        term.stdout.queue(cursor::Hide)?;
        if fullscreen {
            term.stdout.queue(terminal::EnterAlternateScreen)?;
        }
        Ok(term)
    }

    fn set_raw_mode_enabled(&mut self, raw_mode_enabled: bool) -> io::Result<()> {
        if raw_mode_enabled != self.raw_mode_enabled {
            if raw_mode_enabled {
                if terminal::supports_keyboard_enhancement().unwrap_or(false) {
                    self.stdout.execute(event::PushKeyboardEnhancementFlags(
                        event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
                    ))?;
                    self.enabled_keyboard_enhancement = true;
                }
                if self.fullscreen {
                    self.stdout.execute(event::EnableMouseCapture)?;
                }
                terminal::enable_raw_mode()?;
            } else {
                terminal::disable_raw_mode()?;
                if self.fullscreen {
                    self.stdout.execute(event::DisableMouseCapture)?;
                }
                if self.enabled_keyboard_enhancement {
                    self.stdout.execute(event::PopKeyboardEnhancementFlags)?;
                }
            }
            self.raw_mode_enabled = raw_mode_enabled;
        }
        Ok(())
    }
}

impl Drop for StdTerminal {
    fn drop(&mut self) {
        let _ = self.set_raw_mode_enabled(false);
        if self.fullscreen {
            let _ = self.stdout.queue(terminal::LeaveAlternateScreen);
        }
        let _ = self.stdout.execute(cursor::Show);
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
    dummy_stdout: io::Sink,
    dummy_stderr: io::Sink,
}

impl MockTerminal {
    fn new(config: MockTerminalConfig) -> (Self, MockTerminalOutputStream) {
        let (output_tx, output_rx) = mpsc::unbounded();
        let output = MockTerminalOutputStream { inner: output_rx };
        (
            Self {
                config,
                output: output_tx,
                dummy_stdout: io::sink(),
                dummy_stderr: io::sink(),
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

    fn stdout(&mut self) -> &mut dyn Write {
        &mut self.dummy_stdout
    }

    fn stderr(&mut self) -> &mut dyn Write {
        &mut self.dummy_stderr
    }
}

pub(crate) struct Terminal {
    inner: Box<dyn TerminalImpl>,
    output: Output,
    event_stream: Option<BoxStream<'static, TerminalEvent>>,
    subscribers: Vec<Weak<Mutex<TerminalEventsInner>>>,
    received_ctrl_c: bool,
    ignore_ctrl_c: bool,
}

impl Terminal {
    pub fn new(
        stdout: Box<dyn Write + Send>,
        stderr: Box<dyn Write + Send>,
        output: Output,
        fullscreen: bool,
    ) -> io::Result<Self> {
        // Flip handles so StdTerminal.stdout is always the render destination
        let (stdout, stderr) = match output {
            Output::Stdout => (stdout, stderr),
            Output::Stderr => (stderr, stdout),
        };
        Ok(Self {
            inner: Box::new(StdTerminal::new(stdout, stderr, fullscreen)?),
            output,
            event_stream: None,
            subscribers: Vec::new(),
            received_ctrl_c: false,
            ignore_ctrl_c: false,
        })
    }

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

    /// Returns which output handle is being used for TUI rendering.
    pub fn output(&self) -> Output {
        self.output
    }

    /// Returns a mutable reference to the stdout handle.
    pub fn stdout(&mut self) -> &mut dyn Write {
        // Flip back: inner.stdout is render dest, inner.stderr is alternate
        match self.output {
            Output::Stdout => self.inner.stdout(),
            Output::Stderr => self.inner.stderr(),
        }
    }

    /// Returns a mutable reference to the stderr handle.
    pub fn stderr(&mut self) -> &mut dyn Write {
        // Flip back: inner.stdout is render dest, inner.stderr is alternate
        match self.output {
            Output::Stdout => self.inner.stderr(),
            Output::Stderr => self.inner.stdout(),
        }
    }

    /// Returns a mutable reference to the render output handle (stdout or stderr based on output setting).
    pub fn render_output(&mut self) -> &mut dyn Write {
        self.inner.stdout()
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
        terminal.execute(terminal::BeginSynchronizedUpdate)?;
        Ok(Self { inner: terminal })
    }
}

impl Drop for SynchronizedUpdate<'_> {
    fn drop(&mut self) {
        let _ = self.inner.execute(terminal::EndSynchronizedUpdate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_std_terminal() {
        // There's unfortunately not much here we can really test, but we'll do our best.
        // TODO: Is there a library we can use to emulate terminal input/output?
        let mut terminal = Terminal::new(
            Box::new(std::io::stdout()),
            Box::new(std::io::stderr()),
            Output::Stdout,
            false,
        )
        .unwrap();
        assert!(!terminal.is_raw_mode_enabled());
        assert!(!terminal.received_ctrl_c());
        assert!(!terminal.is_raw_mode_enabled());
        let canvas = Canvas::new(10, 1);
        terminal.write_canvas(&canvas).unwrap();
    }
}
