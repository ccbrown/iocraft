use crossterm::{
    cursor,
    event::{self, Event, EventStream},
    execute, queue, terminal,
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
    fn width(&self) -> io::Result<u16>;
    fn is_fullscreen(&self) -> bool;
    fn is_raw_mode_enabled(&self) -> bool;
    fn rewind_lines(&mut self, lines: u16) -> io::Result<()>;
    fn event_stream(&mut self) -> io::Result<BoxStream<'static, TerminalEvent>>;
}

struct StdTerminal {
    dest: io::Stdout,
    fullscreen: bool,
    raw_mode_enabled: bool,
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
    fn width(&self) -> io::Result<u16> {
        terminal::size().map(|(w, _)| w)
    }

    fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    fn is_raw_mode_enabled(&self) -> bool {
        self.raw_mode_enabled
    }

    fn rewind_lines(&mut self, lines: u16) -> io::Result<()> {
        queue!(
            self.dest,
            cursor::MoveToPreviousLine(lines as _),
            terminal::Clear(terminal::ClearType::FromCursorDown)
        )
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

pub(crate) struct Terminal {
    inner: Box<dyn TerminalImpl>,
    event_stream: Option<BoxStream<'static, TerminalEvent>>,
    subscribers: Vec<Weak<Mutex<TerminalEventsInner>>>,
    received_ctrl_c: bool,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        Self::new_with_impl(StdTerminal::new(false)?)
    }

    pub fn fullscreen() -> io::Result<Self> {
        Self::new_with_impl(StdTerminal::new(true)?)
    }

    fn new_with_impl<T: TerminalImpl + 'static>(inner: T) -> io::Result<Self> {
        Ok(Self {
            inner: Box::new(inner),
            event_stream: None,
            subscribers: Vec::new(),
            received_ctrl_c: false,
        })
    }

    pub fn is_fullscreen(&self) -> bool {
        self.inner.is_fullscreen()
    }

    pub fn is_raw_mode_enabled(&self) -> bool {
        self.inner.is_raw_mode_enabled()
    }

    pub fn width(&self) -> io::Result<u16> {
        self.inner.width()
    }

    pub fn rewind_lines(&mut self, lines: u16) -> io::Result<()> {
        if lines > 0 {
            self.inner.rewind_lines(lines)
        } else {
            Ok(())
        }
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

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_std_terminal() {
        // There's unfortunately not much here we can really test, but we'll do our best.
        // TODO: Is there a library we can use to emulate terminal input/output?
        let terminal = Terminal::new().unwrap();
        assert!(!terminal.is_raw_mode_enabled());
        assert!(!terminal.received_ctrl_c());
        assert!(!terminal.is_raw_mode_enabled());
    }
}
