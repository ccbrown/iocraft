use crossterm::{
    cursor,
    event::{Event, EventStream},
    queue, terminal,
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

pub use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

#[derive(Clone, Debug)]
pub struct TerminalKeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
    pub state: KeyEventState,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum TerminalEvent {
    Key(TerminalKeyEvent),
}

struct TerminalEventsInner {
    pending: Vec<TerminalEvent>,
    waker: Option<Waker>,
}

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

    pub async fn wait(&mut self) {
        match &mut self.event_stream {
            Some(event_stream) => {
                while let Some(event) = event_stream.next().await {
                    let event = event.ok().and_then(|event| match event {
                        Event::Key(event) => {
                            if event.code == KeyCode::Char('c')
                                && event.modifiers == KeyModifiers::CONTROL
                            {
                                self.received_ctrl_c = true;
                            }
                            Some(TerminalEvent::Key(TerminalKeyEvent {
                                code: event.code,
                                modifiers: event.modifiers,
                                kind: event.kind,
                                state: event.state,
                            }))
                        }
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
                terminal::disable_raw_mode()?;
            }
            self.raw_mode_enabled = raw_mode_enabled;
        }
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = queue!(stdout(), cursor::Show);
        let _ = self.set_raw_mode_enabled(false);
    }
}