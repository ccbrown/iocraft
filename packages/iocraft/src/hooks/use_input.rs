use crate::{ComponentUpdater, Hook, TerminalEvent, TerminalEvents};
use futures::stream::Stream;
use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

/// `UseInput` is a hook that allows you to listen for user input such as key strokes.
#[derive(Default)]
pub struct UseInput {
    events: Option<TerminalEvents>,
    f: Option<Box<dyn FnMut(TerminalEvent) + Send + 'static>>,
}

impl Hook for UseInput {
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        while let Some(Poll::Ready(Some(event))) = self
            .events
            .as_mut()
            .map(|events| pin!(events).poll_next(cx))
        {
            if let Some(f) = self.f.as_mut() {
                f(event);
            }
        }
        Poll::Pending
    }

    fn pre_component_update(&mut self, updater: &mut ComponentUpdater) {
        if self.events.is_none() {
            self.events = updater.terminal_events();
        }
    }
}

impl UseInput {
    /// Sets the callback to be invoked whenever a terminal event occurs.
    pub fn use_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static,
    {
        self.f = Some(Box::new(f));
    }
}
