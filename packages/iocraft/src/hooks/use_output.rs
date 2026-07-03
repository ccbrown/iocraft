use crate::{backend::Passthrough, element::Output, ComponentUpdater, Hook, Hooks};
use core::{
    pin::Pin,
    task::{Context, Poll, Waker},
};
use std::sync::{Arc, Mutex};

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

#[derive(Default)]
struct UseOutputState {
    queue: Vec<Passthrough<'static>>,
    waker: Option<Waker>,
}

impl UseOutputState {
    fn push(&mut self, stream: Output, mut content: String, mut newline: bool) {
        if !newline && content.ends_with('\n') {
            // Move the trailing newline into the flag so the backend chooses
            // the line terminator (see `Passthrough`).
            content.pop();
            if content.ends_with('\r') {
                content.pop();
            }
            newline = true;
        }
        self.queue.push(Passthrough {
            stream,
            content: content.into(),
            newline,
        });
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    fn exec(&mut self, updater: &mut ComponentUpdater) {
        if self.queue.is_empty() {
            return;
        }

        // Check if we have a terminal - if not, messages stay queued
        if updater.terminal_mut().is_none() {
            return;
        }

        updater.clear_terminal_output();

        // The backend owns placement, newline choice (`\r\n` vs `\n`), and any
        // cursor re-anchoring.
        let terminal = updater.terminal_mut().unwrap();
        let _ = terminal.print_above(&self.queue);
        self.queue.clear();
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
        state.push(Output::Stdout, msg.to_string(), true);
    }

    /// Queues a message to be written asynchronously to stdout without a newline, above the
    /// rendered component output.
    pub fn print<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.push(Output::Stdout, msg.to_string(), false);
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
        state.push(Output::Stderr, msg.to_string(), true);
    }

    /// Queues a message to be written asynchronously to stderr without a newline, above the
    /// rendered component output.
    pub fn print<S: ToString>(&self, msg: S) {
        let mut state = self.state.lock().unwrap();
        state.push(Output::Stderr, msg.to_string(), false);
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

    #[test]
    fn test_print_trailing_newline_normalization() {
        let mut state = UseOutputState::default();
        state.push(Output::Stdout, "done\n".to_string(), false);
        state.push(Output::Stdout, "crlf\r\n".to_string(), false);
        state.push(Output::Stdout, "blank\n\n".to_string(), true);

        // A trailing newline in no-newline content moves into the flag.
        assert_eq!(state.queue[0].content, "done");
        assert!(state.queue[0].newline);
        assert_eq!(state.queue[1].content, "crlf");
        assert!(state.queue[1].newline);
        // println content is left untouched; its newlines are intentional.
        assert_eq!(state.queue[2].content, "blank\n\n");
        assert!(state.queue[2].newline);
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
