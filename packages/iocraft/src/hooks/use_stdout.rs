use crate::Hook;
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

#[derive(Default)]
struct UseStdoutState {
    queue: Vec<String>,
    waker: Option<Waker>,
}

#[derive(Clone)]
pub struct UseStdoutHandle {
    state: Arc<Mutex<UseStdoutState>>,
}

impl UseStdoutHandle {
    pub fn println<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.queue.push(msg.to_string());
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

#[derive(Default)]
pub struct UseStdout {
    state: Arc<Mutex<UseStdoutState>>,
}

impl Hook for UseStdout {
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.state.lock().unwrap();
        if state.queue.is_empty() {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

impl UseStdout {
    pub fn use_stdout(&mut self) -> UseStdoutHandle {
        let mut state = self.state.lock().unwrap();
        for msg in state.queue.drain(..) {
            println!("{}", msg);
        }
        UseStdoutHandle {
            state: self.state.clone(),
        }
    }
}
