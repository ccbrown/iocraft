use crate::canvas::Canvas;
use crossterm::{
    cursor,
    event::{self, Event, EventStream},
    execute, queue, terminal,
    tty::IsTty,
};
use futures::{
    future::pending,
    stream::{BoxStream, Stream, StreamExt},
};
use std::{
    collections::VecDeque,
    io::{self, stdout, Write},
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    task::{Context, Poll, Waker},
};

// Re-exports for basic types.
pub use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

/// An event fired when a key is pressed.
#[derive(Clone, Debug)]
pub struct KeyEvent {
    /// A code indicating the key that was pressed.
    pub code: KeyCode,

    /// The modifiers that were active when the key was pressed.
    pub modifiers: KeyModifiers,

    /// Whether the key was pressed or released.
    pub kind: KeyEventKind,
}

/// An event fired by the terminal.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum TerminalEvent {
    /// A key event, fired when a key is pressed.
    Key(KeyEvent),
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

trait TerminalImpl: Write {
    fn width(&self) -> Option<u16>;
    fn is_raw_mode_enabled(&self) -> bool;
    fn clear_canvas(&mut self) -> io::Result<()>;
    fn write_canvas(&mut self, canvas: &Canvas) -> io::Result<()>;
    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>>;
}

struct StdTerminal {
    dest: io::Stdout,
    fullscreen: bool,
    raw_mode_enabled: bool,
    prev_canvas_height: u16,
}

impl Write for StdTerminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.dest.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.dest.flush()
    }
}

impl TerminalImpl for StdTerminal {
    fn width(&self) -> Option<u16> {
        terminal::size().ok().map(|(w, _)| w)
    }

    fn is_raw_mode_enabled(&self) -> bool {
        self.raw_mode_enabled
    }

    fn clear_canvas(&mut self) -> io::Result<()> {
        if self.prev_canvas_height == 0 {
            return Ok(());
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
        self.set_raw_mode_enabled(true)?;

        Ok(EventStream::new()
            .filter_map(|event| async move {
                match event {
                    Ok(Event::Key(event)) => Some(TerminalEvent::Key(KeyEvent {
                        code: event.code,
                        modifiers: event.modifiers,
                        kind: event.kind,
                    })),
                    Ok(Event::Resize(width, height)) => Some(TerminalEvent::Resize(width, height)),
                    _ => None,
                }
            })
            .boxed())
    }
}

impl StdTerminal {
    fn new(fullscreen: bool) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut dest = stdout();
        queue!(dest, cursor::Hide)?;
        if fullscreen {
            queue!(dest, terminal::EnterAlternateScreen)?;
        }
        Ok(Self {
            dest,
            fullscreen,
            raw_mode_enabled: false,
            prev_canvas_height: 0,
        })
    }

    fn set_raw_mode_enabled(&mut self, raw_mode_enabled: bool) -> io::Result<()> {
        if raw_mode_enabled != self.raw_mode_enabled {
            if raw_mode_enabled {
                execute!(
                    self.dest,
                    event::PushKeyboardEnhancementFlags(
                        event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                    )
                )?;
                if self.fullscreen {
                    execute!(self.dest, event::EnableMouseCapture)?;
                }
                terminal::enable_raw_mode()?;
            } else {
                terminal::disable_raw_mode()?;
                if self.fullscreen {
                    execute!(self.dest, event::DisableMouseCapture)?;
                }
                execute!(self.dest, event::PopKeyboardEnhancementFlags)?;
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
            let _ = queue!(self.dest, terminal::LeaveAlternateScreen);
        }
        let _ = execute!(self.dest, cursor::Show);
    }
}

#[cfg(test)]
pub struct MockTerminalOutput {
    state: Arc<Mutex<MockTerminalState>>,
}

#[cfg(test)]
impl MockTerminalOutput {
    pub fn canvases(&self) -> Vec<Canvas> {
        self.state.lock().unwrap().canvases.clone()
    }
}

#[cfg(test)]
#[derive(Default)]
struct MockTerminalState {
    canvases: Vec<Canvas>,
}

#[cfg(test)]
struct MockTerminal {
    state: Arc<Mutex<MockTerminalState>>,
}

#[cfg(test)]
impl MockTerminal {
    fn new() -> (Self, MockTerminalOutput) {
        let output = MockTerminalOutput {
            state: Default::default(),
        };
        (
            Self {
                state: output.state.clone(),
            },
            output,
        )
    }
}

#[cfg(test)]
impl Write for MockTerminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
impl TerminalImpl for MockTerminal {
    fn width(&self) -> Option<u16> {
        None
    }

    fn is_raw_mode_enabled(&self) -> bool {
        false
    }

    fn clear_canvas(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_canvas(&mut self, canvas: &Canvas) -> io::Result<()> {
        self.state.lock().unwrap().canvases.push(canvas.clone());
        Ok(())
    }

    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>> {
        Ok(futures::stream::iter(vec![
            TerminalEvent::Key(KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            }),
            TerminalEvent::Key(KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Release,
            }),
            TerminalEvent::Key(KeyEvent {
                code: KeyCode::Char('o'),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            }),
            TerminalEvent::Key(KeyEvent {
                code: KeyCode::Char('o'),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Repeat,
            }),
            TerminalEvent::Key(KeyEvent {
                code: KeyCode::Char('o'),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Release,
            }),
        ])
        .chain(futures::stream::pending())
        .boxed())
    }
}

pub(crate) struct Terminal {
    inner: Box<dyn TerminalImpl>,
    event_stream: Option<BoxStream<'static, TerminalEvent>>,
    subscribers: Vec<Weak<Mutex<TerminalEventsInner>>>,
    received_ctrl_c: bool,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        Ok(Self::new_with_impl(StdTerminal::new(false)?))
    }

    pub fn fullscreen() -> io::Result<Self> {
        Ok(Self::new_with_impl(StdTerminal::new(true)?))
    }

    #[cfg(test)]
    pub fn mock() -> (Self, MockTerminalOutput) {
        let (term, output) = MockTerminal::new();
        (Self::new_with_impl(term), output)
    }

    fn new_with_impl<T: TerminalImpl + 'static>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
            event_stream: None,
            subscribers: Vec::new(),
            received_ctrl_c: false,
        }
    }

    pub fn is_raw_mode_enabled(&self) -> bool {
        self.inner.is_raw_mode_enabled()
    }

    pub fn width(&self) -> Option<u16> {
        self.inner.width()
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

    pub async fn wait(&mut self) {
        match &mut self.event_stream {
            Some(event_stream) => {
                while let Some(event) = event_stream.next().await {
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

/// Returns whether the standard output is a TTY terminal.
pub fn stdout_is_tty() -> bool {
    stdout().is_tty()
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

    #[test]
    fn test_stdout_is_tty() {
        let _ = stdout_is_tty();
    }
}
