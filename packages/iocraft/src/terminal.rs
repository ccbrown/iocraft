use crate::backend::{new_terminal_backend, Passthrough, TerminalBackend};
use crate::canvas::Canvas;
use crate::element::Output;
use futures::{
    channel::mpsc,
    future::pending,
    stream::{self, BoxStream, Stream, StreamExt},
};
use std::{
    collections::VecDeque,
    io::{self, Write},
    mem,
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    task::{Context, Poll, Waker},
};

// Re-exports for basic input event types (owned by iocraft; see `crate::event`).
pub use crate::event::{
    KeyCode, KeyEventKind, KeyModifiers, MediaKeyCode, ModifierKeyCode, MouseButton, MouseEventKind,
};

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
    /// A bracketed paste, fired when the terminal delivers pasted text in one
    /// chunk (requires bracketed paste mode to be enabled).
    Paste(String),
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

impl TerminalBackend for MockTerminal {
    fn clear_canvas(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_canvas(&mut self, _prev: Option<&Canvas>, canvas: &Canvas) -> io::Result<()> {
        let _ = self.output.unbounded_send(canvas.clone());
        Ok(())
    }

    fn print_above(&mut self, _messages: &[Passthrough<'_>]) -> io::Result<()> {
        // The mock terminal discards passthrough output.
        Ok(())
    }

    fn event_stream(&mut self) -> io::Result<BoxStream<'static, io::Result<TerminalEvent>>> {
        let mut events = stream::pending().boxed();
        mem::swap(&mut events, &mut self.config.events);
        Ok(events.map(Ok).chain(stream::pending()).boxed())
    }
}

pub(crate) struct Terminal<'a> {
    inner: Box<dyn TerminalBackend + 'a>,
    event_stream: Option<BoxStream<'static, io::Result<TerminalEvent>>>,
    subscribers: Vec<Weak<Mutex<TerminalEventsInner>>>,
    received_ctrl_c: bool,
    ignore_ctrl_c: bool,
}

impl<'a> Terminal<'a> {
    /// Builds a terminal from an arbitrary [`TerminalBackend`].
    pub fn with_backend(backend: Box<dyn TerminalBackend + 'a>) -> Self {
        Self {
            inner: backend,
            event_stream: None,
            subscribers: Vec::new(),
            received_ctrl_c: false,
            ignore_ctrl_c: false,
        }
    }

    /// Builds a terminal using the built-in backend, rendering to `output`
    /// (with the other stream available for passthrough writes).
    pub fn new(
        stdout: Box<dyn Write + Send + 'a>,
        stderr: Box<dyn Write + Send + 'a>,
        output: Output,
    ) -> io::Result<Self> {
        Ok(Self::with_backend(new_terminal_backend(
            stdout, stderr, output,
        )?))
    }

    pub fn enable_mouse_capture(&mut self) -> io::Result<()> {
        self.inner.set_mouse_capture(true)
    }

    pub fn disable_mouse_capture(&mut self) -> io::Result<()> {
        self.inner.set_mouse_capture(false)
    }

    pub fn set_fullscreen(&mut self, enabled: bool) -> io::Result<()> {
        self.inner.set_fullscreen(enabled)
    }

    pub fn ignore_ctrl_c(&mut self) {
        self.ignore_ctrl_c = true;
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

    /// Emits passthrough output above the rendered canvas (used by `use_output`).
    pub fn print_above(&mut self, messages: &[Passthrough<'_>]) -> io::Result<()> {
        self.inner.print_above(messages)
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

    pub async fn wait(&mut self) -> io::Result<()> {
        match &mut self.event_stream {
            Some(event_stream) => {
                while let Some(event) = event_stream.next().await {
                    let event = event?;
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
                            return Ok(());
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
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "terminal event stream ended",
                ))
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
        (Self::with_backend(Box::new(term)), output_stream)
    }

    #[cfg(test)]
    pub(crate) fn mock_with_event_error(error: io::Error) -> Self {
        let (mut terminal, _output) = Self::mock(MockTerminalConfig::default());
        terminal.event_stream = Some(stream::once(async move { Err(error) }).boxed());
        terminal
    }
}

/// Synchronized update terminal guard.
/// Begins a frame on creation, ends it when dropped (even on error or panic).
pub(crate) struct SynchronizedUpdate<'a, 'b> {
    inner: &'a mut Terminal<'b>,
}

impl<'a, 'b> SynchronizedUpdate<'a, 'b> {
    pub fn begin(terminal: &'a mut Terminal<'b>) -> io::Result<Self> {
        terminal.inner.begin_frame()?;
        Ok(Self { inner: terminal })
    }
}

impl Drop for SynchronizedUpdate<'_, '_> {
    fn drop(&mut self) {
        let _ = self.inner.inner.end_frame();
    }
}
