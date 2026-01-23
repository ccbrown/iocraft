use crate::{context::SystemContext, element::Output, ComponentUpdater, Hook, Hooks};
use core::{
    pin::Pin,
    task::{Context, Poll, Waker},
};
use crossterm::{cursor, queue};
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseOutput` is a hook that allows you to write to stdout and stderr from a component. The
/// output will be appended to stdout or stderr, above the rendered component output.
///
/// Both `print` and `println` methods are available for writing output with or without newlines.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # use std::time::Duration;
/// #[component]
/// fn Example(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let (stdout, stderr) = hooks.use_output();
///
///     hooks.use_future(async move {
///         stdout.println("Hello from iocraft to stdout!");
///         stderr.println("  And hello to stderr too!");
///
///         stdout.print("Working...");
///         for _ in 0..5 {
///             smol::Timer::after(Duration::from_secs(1)).await;
///             stdout.print(".");
///         }
///         stdout.println("\nDone!");
///     });
///
///     element! {
///         View(border_style: BorderStyle::Round, border_color: Color::Green) {
///             Text(content: "Hello, use_output!")
///         }
///     }
/// }
/// ```
pub trait UseOutput: private::Sealed {
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
    StdoutNoNewline(String),
    Stderr(String),
    StderrNoNewline(String),
}

#[derive(Default)]
struct UseOutputState {
    queue: Vec<Message>,
    waker: Option<Waker>,
    appended_newline: Option<u16>,
}

impl UseOutputState {
    fn exec(&mut self, updater: &mut ComponentUpdater) {
        if self.queue.is_empty() {
            return;
        }

        // Check if we have a terminal - if not, messages stay queued
        if !updater.is_terminal_render_loop() {
            return;
        }

        updater.clear_terminal_output();
        let needs_carriage_returns = updater.is_terminal_raw_mode_enabled();

        let system = updater.get_context::<SystemContext>().unwrap();
        let stdout = system.stdout();
        let stderr = system.stderr();
        let render_to = system.render_to();

        let render_handle = match render_to {
            Output::Stdout => &stdout,
            Output::Stderr => &stderr,
        };

        if let Some(col) = self.appended_newline {
            let _ = queue!(
                render_handle.lock().unwrap(),
                cursor::MoveUp(1),
                cursor::MoveRight(col)
            );
        }
        let mut needs_extra_newline = self.appended_newline.is_some();

        for msg in self.queue.drain(..) {
            // Cursor manipulation only works when message output matches the render target
            let msg_matches_render = matches!(
                (&msg, render_to),
                (
                    Message::Stdout(_) | Message::StdoutNoNewline(_),
                    Output::Stdout
                ) | (
                    Message::Stderr(_) | Message::StderrNoNewline(_),
                    Output::Stderr
                )
            );

            match msg {
                Message::Stdout(msg) => {
                    let formatted = if needs_carriage_returns {
                        format!("{}\r\n", msg)
                    } else {
                        format!("{}\n", msg)
                    };
                    let _ = stdout.lock().unwrap().write_all(formatted.as_bytes());
                    if msg_matches_render {
                        needs_extra_newline = false;
                    }
                }
                Message::StdoutNoNewline(msg) => {
                    let _ = stdout.lock().unwrap().write_all(msg.as_bytes());
                    if msg_matches_render && !msg.is_empty() {
                        needs_extra_newline = !msg.ends_with('\n');
                    }
                }
                Message::Stderr(msg) => {
                    let formatted = if needs_carriage_returns {
                        format!("{}\r\n", msg)
                    } else {
                        format!("{}\n", msg)
                    };
                    let _ = stderr.lock().unwrap().write_all(formatted.as_bytes());
                    if msg_matches_render {
                        needs_extra_newline = false;
                    }
                }
                Message::StderrNoNewline(msg) => {
                    let _ = stderr.lock().unwrap().write_all(msg.as_bytes());
                    if msg_matches_render && !msg.is_empty() {
                        needs_extra_newline = !msg.ends_with('\n');
                    }
                }
            }
        }

        if needs_extra_newline {
            if let Ok(pos) = cursor::position() {
                self.appended_newline = Some(pos.0);
                let newline = if needs_carriage_returns { "\r\n" } else { "\n" };
                let _ = render_handle.lock().unwrap().write_all(newline.as_bytes());
            } else {
                self.appended_newline = None;
            }
        } else {
            self.appended_newline = None;
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

    /// Queues a message to be written asynchronously to stdout without a newline, above the
    /// rendered component output.
    pub fn print<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.queue.push(Message::StdoutNoNewline(msg.to_string()));
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

    /// Queues a message to be written asynchronously to stderr without a newline, above the
    /// rendered component output.
    pub fn print<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.queue.push(Message::StderrNoNewline(msg.to_string()));
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
    use crate::prelude::*;
    use futures::task::noop_waker;
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[test]
    fn test_use_output_polling() {
        let mut use_output = UseOutputImpl::default();
        assert_eq!(
            Pin::new(&mut use_output)
                .poll_change(&mut core::task::Context::from_waker(&noop_waker())),
            Poll::Pending
        );

        let stdout = use_output.use_stdout();
        stdout.println("Hello, world!");
        assert_eq!(
            Pin::new(&mut use_output)
                .poll_change(&mut core::task::Context::from_waker(&noop_waker())),
            Poll::Ready(())
        );

        let stderr = use_output.use_stderr();
        stderr.println("Hello, error!");
        assert_eq!(
            Pin::new(&mut use_output)
                .poll_change(&mut core::task::Context::from_waker(&noop_waker())),
            Poll::Ready(())
        );

        // Test print methods
        stdout.print("Hello, ");
        stdout.print("world!");
        stderr.print("Error: ");
        stderr.print("test");
        stderr.print("Warning: ");
        stderr.print("print test");
        assert_eq!(
            Pin::new(&mut use_output)
                .poll_change(&mut core::task::Context::from_waker(&noop_waker())),
            Poll::Ready(())
        );
    }

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let (stdout, stderr) = hooks.use_output();
        stdout.println("Hello, world!");
        stderr.println("Hello, error!");
        stdout.print("Testing ");
        stdout.print("print ");
        stdout.println("method!");
        stderr.print("Error: ");
        stderr.println("test");
        stderr.print("Warning: ");
        stderr.println("print test");
        system.exit();
        element!(View)
    }

    #[apply(test!)]
    async fn test_use_output() {
        element!(MyComponent).render_loop().await.unwrap();
    }
}
