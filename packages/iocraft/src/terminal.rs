use crossterm::{
    cursor,
    event::{self, Event, EventStream},
    execute, queue, terminal,
};
use futures::{
    future::pending,
    stream::{Stream, StreamExt},
};
use std::{
    io::{self, stdout},
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    task::{Context, Poll, Waker},
};

// Re-exports for basic types.
pub use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers, MouseEventKind};

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

/// An event fired when a mouse action occurs.
#[derive(Clone, Debug)]
pub struct MouseEvent {
    /// Indicates the kind of mouse event that occurred.
    pub kind: MouseEventKind,

    /// The modifiers that were active when the key was pressed.
    pub modifiers: KeyModifiers,

    /// The row of the terminal where the event occurred.
    pub row: u16,

    /// The column of the terminal where the event occurred.
    pub column: u16,

    /// The row of the terminal where the event occurred, relative to the canvas.
    pub canvas_row: i16,

    /// The column of the terminal where the event occurred, relative to the canvas.
    pub canvas_column: i16,
}

/// An event fired by the terminal.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum TerminalEvent {
    /// A key event, fired when a key is pressed.
    Key(KeyEvent),
    /// A mouse event, fired when a mouse action occurs.
    Mouse(MouseEvent),
}

struct TerminalEventsInner {
    pending: Vec<TerminalEvent>,
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
        if let Some(event) = inner.pending.pop() {
            Poll::Ready(Some(event))
        } else {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub(crate) struct Terminal {
    raw_mode_enabled: bool,
    event_stream: Option<EventStream>,
    subscribers: Vec<Weak<Mutex<TerminalEventsInner>>>,
    received_ctrl_c: bool,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        queue!(stdout(), cursor::Hide)?;
        Ok(Self {
            raw_mode_enabled: false,
            event_stream: None,
            subscribers: Vec::new(),
            received_ctrl_c: false,
        })
    }

    pub fn is_raw_mode_enabled(&self) -> bool {
        self.raw_mode_enabled
    }

    pub fn received_ctrl_c(&self) -> bool {
        self.received_ctrl_c
    }

    pub async fn wait(&mut self, origin: (u16, u16)) {
        match &mut self.event_stream {
            Some(event_stream) => {
                while let Some(event) = event_stream.next().await {
                    let event = event.ok().and_then(|event| match event {
                        Event::Key(event) => {
                            if event.code == KeyCode::Char('c')
                                && event.kind == KeyEventKind::Press
                                && event.modifiers == KeyModifiers::CONTROL
                            {
                                self.received_ctrl_c = true;
                            }
                            Some(TerminalEvent::Key(KeyEvent {
                                code: event.code,
                                modifiers: event.modifiers,
                                kind: event.kind,
                            }))
                        }
                        Event::Mouse(event) => Some(TerminalEvent::Mouse(MouseEvent {
                            kind: event.kind,
                            modifiers: event.modifiers,
                            row: event.row,
                            column: event.column,
                            canvas_row: event.row as i16 - origin.0 as i16,
                            canvas_column: event.column as i16 - origin.1 as i16,
                        })),
                        _ => None,
                    });
                    if self.received_ctrl_c {
                        return;
                    }
                    if let Some(event) = event {
                        self.subscribers.retain(|subscriber| {
                            if let Some(subscriber) = subscriber.upgrade() {
                                let mut subscriber = subscriber.lock().unwrap();
                                subscriber.pending.push(event.clone());
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
            }
            None => pending().await,
        }
    }

    pub fn events(&mut self) -> io::Result<TerminalEvents> {
        if !self.raw_mode_enabled {
            execute!(
                stdout(),
                event::EnableMouseCapture,
                event::PushKeyboardEnhancementFlags(
                    event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                )
            )?;
            self.set_raw_mode_enabled(true)?;
            self.event_stream = Some(EventStream::new());
        }
        let inner = Arc::new(Mutex::new(TerminalEventsInner {
            pending: Vec::new(),
            waker: None,
        }));
        self.subscribers.push(Arc::downgrade(&inner));
        Ok(TerminalEvents { inner })
    }

    fn set_raw_mode_enabled(&mut self, raw_mode_enabled: bool) -> io::Result<()> {
        if raw_mode_enabled != self.raw_mode_enabled {
            if raw_mode_enabled {
                terminal::enable_raw_mode()?;
            } else {
                execute!(
                    stdout(),
                    event::DisableMouseCapture,
                    event::PopKeyboardEnhancementFlags
                )?;
                terminal::disable_raw_mode()?;
            }
            self.raw_mode_enabled = raw_mode_enabled;
        }
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.set_raw_mode_enabled(false);
        let _ = execute!(stdout(), cursor::Show);
    }
}
