use crate::{ComponentUpdater, Hook, Hooks};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

/// `UseOutput` is a hook that allows you to write to stdout and stderr from a component. The
/// output will be appended to stdout or stderr, above the rendered component output.
///
/// # Example
///
/// ```no_run
#[doc = include_str!("../../examples/use_output.rs")]
/// ```
pub trait UseOutput {
    /// Gets handles which can be used to write to stdout and stderr.
    fn use_output(&mut self) -> (StdoutHandle, StderrHandle);
}

impl UseOutput for Hooks<'_, '_> {
    fn use_output(&mut self) -> (StdoutHandle, StderrHandle) {
        let output = self.use_hook(UseOutputImpl::default);
        (output.use_stdout(), output.use_stderr())
    }
}

enum Message {
    Stdout(String),
    Stderr(String),
}

#[derive(Default)]
struct UseOutputState {
    queue: Vec<Message>,
    waker: Option<Waker>,
}

impl UseOutputState {
    fn exec(&mut self, updater: &mut ComponentUpdater) {
        if self.queue.is_empty() {
            return;
        }
        updater.clear_terminal_output();
        let needs_carriage_returns = updater.is_terminal_raw_mode_enabled();
        for msg in self.queue.drain(..) {
            match msg {
                Message::Stdout(msg) => {
                    if needs_carriage_returns {
                        print!("{}\r\n", msg)
                    } else {
                        println!("{}", msg)
                    }
                }
                Message::Stderr(msg) => {
                    if needs_carriage_returns {
                        eprint!("{}\r\n", msg)
                    } else {
                        eprintln!("{}", msg)
                    }
                }
            }
        }
    }
}

/// A handle to write to stdout, obtained from [`UseOutput::use_output`].
#[derive(Clone)]
pub struct StdoutHandle {
    state: Arc<Mutex<UseOutputState>>,
}

impl StdoutHandle {
    /// Queues a message to be written asynchronously to stdout, above the rendered component
    /// output.
    pub fn println<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.queue.push(Message::Stdout(msg.to_string()));
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

/// A handle to write to stderr, obtained from [`UseOutput::use_output`].
#[derive(Clone)]
pub struct StderrHandle {
    state: Arc<Mutex<UseOutputState>>,
}

impl StderrHandle {
    /// Queues a message to be written asynchronously to stderr, above the rendered component
    /// output.
    pub fn println<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.queue.push(Message::Stderr(msg.to_string()));
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

#[derive(Default)]
struct UseOutputImpl {
    state: Arc<Mutex<UseOutputState>>,
}

impl Hook for UseOutputImpl {
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

impl UseOutputImpl {
    pub fn use_stdout(&mut self) -> StdoutHandle {
        StdoutHandle {
            state: self.state.clone(),
        }
    }

    pub fn use_stderr(&mut self) -> StderrHandle {
        StderrHandle {
            state: self.state.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::noop_waker;

    #[test]
    fn test_use_output_polling() {
        let mut use_output = UseOutputImpl::default();
        assert_eq!(
            Pin::new(&mut use_output).poll_change(&mut Context::from_waker(&noop_waker())),
            Poll::Pending
        );

        let stdout = use_output.use_stdout();
        stdout.println("Hello, world!");
        assert_eq!(
            Pin::new(&mut use_output).poll_change(&mut Context::from_waker(&noop_waker())),
            Poll::Ready(())
        );

        let stderr = use_output.use_stderr();
        stderr.println("Hello, error!");
        assert_eq!(
            Pin::new(&mut use_output).poll_change(&mut Context::from_waker(&noop_waker())),
            Poll::Ready(())
        );
    }
}
