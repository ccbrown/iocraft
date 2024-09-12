use crate::{ComponentUpdater, Hook};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

enum Message {
    Stdout(String),
    Stderr(String),
}

#[derive(Default)]
struct UseStdioState {
    queue: Vec<Message>,
    waker: Option<Waker>,
}

impl UseStdioState {
    fn exec(&mut self, updater: &mut ComponentUpdater) {
        if self.queue.is_empty() {
            return;
        }
        updater.clear_terminal_output();
        for msg in self.queue.drain(..) {
            match msg {
                Message::Stdout(msg) => println!("{}", msg),
                Message::Stderr(msg) => eprintln!("{}", msg),
            }
        }
    }
}

#[derive(Clone)]
pub struct UseStdoutHandle {
    state: Arc<Mutex<UseStdioState>>,
}

impl UseStdoutHandle {
    pub fn println<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.queue.push(Message::Stdout(msg.to_string()));
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

#[derive(Clone)]
pub struct UseStderrHandle {
    state: Arc<Mutex<UseStdioState>>,
}

impl UseStderrHandle {
    pub fn println<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.queue.push(Message::Stderr(msg.to_string()));
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

#[derive(Default)]
pub struct UseStdio {
    state: Arc<Mutex<UseStdioState>>,
}

impl Hook for UseStdio {
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let mut state = self.state.lock().unwrap();
        if state.queue.is_empty() {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }

    fn post_component_update(&mut self, updater: &mut ComponentUpdater) {
        let mut state = self.state.lock().unwrap();
        state.exec(updater);
    }
}

impl UseStdio {
    pub fn use_stdout(&mut self) -> UseStdoutHandle {
        UseStdoutHandle {
            state: self.state.clone(),
        }
    }

    pub fn use_stderr(&mut self) -> UseStderrHandle {
        UseStderrHandle {
            state: self.state.clone(),
        }
    }
}
