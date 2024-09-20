use crate::{ComponentUpdater, Hook, Hooks, TerminalEvent, TerminalEvents};
use futures::stream::Stream;
use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

/// `UseTerminalEvents` is a hook that allows you to listen for user input such as key strokes.
pub trait UseTerminalEvents {
    /// Defines a callback to be invoked whenever a terminal event occurs.
    fn use_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static;
}

impl UseTerminalEvents for Hooks<'_> {
    fn use_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static,
    {
        self.use_hook(move || UseTerminalEventsImpl {
            events: None,
            f: Box::new(f),
        });
    }
}

struct UseTerminalEventsImpl {
    events: Option<TerminalEvents>,
    f: Box<dyn FnMut(TerminalEvent) + Send + 'static>,
}

impl Hook for UseTerminalEventsImpl {
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        while let Some(Poll::Ready(Some(event))) = self
            .events
            .as_mut()
            .map(|events| pin!(events).poll_next(cx))
        {
            (self.f)(event);
        }
        Poll::Pending
    }

    fn post_component_update(&mut self, updater: &mut ComponentUpdater) {
        if self.events.is_none() {
            self.events = updater.terminal_events();
        }
    }
}
