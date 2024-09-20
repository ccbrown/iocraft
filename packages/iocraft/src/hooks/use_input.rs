use crate::{ComponentUpdater, Hook, Hooks, TerminalEvent, TerminalEvents};
use futures::stream::Stream;
use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

/// `UseInput` is a hook that allows you to listen for user input such as key strokes.
pub trait UseInput {
    /// Defines a callback to be invoked whenever a terminal event occurs.
    fn use_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static;
}

impl UseInput for Hooks<'_> {
    fn use_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static,
    {
        self.use_hook(move || UseInputImpl {
            events: None,
            f: Box::new(f),
        });
    }
}

struct UseInputImpl {
    events: Option<TerminalEvents>,
    f: Box<dyn FnMut(TerminalEvent) + Send + 'static>,
}

impl Hook for UseInputImpl {
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
