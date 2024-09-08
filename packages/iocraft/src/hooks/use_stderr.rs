use crate::Hook;
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

#[derive(Default)]
struct UseStderrState {
    queue: Vec<String>,
    waker: Option<Waker>,
}

pub struct UseStderrHandle {
    state: Arc<Mutex<UseStderrState>>,
}

impl UseStderrHandle {
    pub fn println<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.queue.push(msg.to_string());
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

#[derive(Default)]
pub struct UseStderr {
    state: Arc<Mutex<UseStderrState>>,
}

impl Hook for UseStderr {
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

impl UseStderr {
    pub fn use_stderr(&mut self) -> UseStderrHandle {
        let mut state = self.state.lock().unwrap();
        for msg in state.queue.drain(..) {
            eprintln!("{}", msg);
        }
        UseStderrHandle {
            state: self.state.clone(),
        }
    }
}
