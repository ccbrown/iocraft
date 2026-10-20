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
/// Formatting, wake callbacks, and backend writes run without holding the output queue lock.
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
    fn push(&mut self, stream: Output, mut content: String, mut newline: bool) -> Option<Waker> {
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
        self.waker.take()
    }
}

fn queue_output<S: ToString>(state: &Mutex<UseOutputState>, stream: Output, msg: S, newline: bool) {
    let content = msg.to_string();
    let waker = {
        let mut state = state.lock().unwrap();
        state.push(stream, content, newline)
    };
    if let Some(waker) = waker {
        waker.wake();
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
        queue_output(&self.state, Output::Stdout, msg, true);
    }

    /// Queues a message to be written asynchronously to stdout without a newline, above the
    /// rendered component output.
    pub fn print<S: ToString>(&self, msg: S) {
        queue_output(&self.state, Output::Stdout, msg, false);
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
        queue_output(&self.state, Output::Stderr, msg, true);
    }

    /// Queues a message to be written asynchronously to stderr without a newline, above the
    /// rendered component output.
    pub fn print<S: ToString>(&self, msg: S) {
        queue_output(&self.state, Output::Stderr, msg, false);
    }
}

#[derive(Default)]
struct UseOutputImpl {
    state: Arc<Mutex<UseOutputState>>,
    pending: Vec<Passthrough<'static>>,
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
        {
            let mut state = self.state.lock().unwrap();
            if state.queue.is_empty() || updater.terminal_mut().is_none() {
                return;
            }
            std::mem::swap(&mut self.pending, &mut state.queue);
        }
        updater.clear_terminal_output();
        let _ = updater.terminal_mut().unwrap().print_above(&self.pending);
        self.pending.clear();
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
        let _ = state.push(Output::Stdout, "done\n".to_string(), false);
        let _ = state.push(Output::Stdout, "crlf\r\n".to_string(), false);
        let _ = state.push(Output::Stdout, "blank\n\n".to_string(), true);

        // A trailing newline in no-newline content moves into the flag.
        assert_eq!(state.queue[0].content, "done");
        assert!(state.queue[0].newline);
        assert_eq!(state.queue[1].content, "crlf");
        assert!(state.queue[1].newline);
        // println content is left untouched; its newlines are intentional.
        assert_eq!(state.queue[2].content, "blank\n\n");
        assert!(state.queue[2].newline);
    }

    #[test]
    fn test_formatting_and_waking_can_enqueue_output() {
        struct NestedMessage(StderrHandle);
        impl std::fmt::Display for NestedMessage {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.println("inner");
                f.write_str("outer")
            }
        }
        struct WriteOnWake(StdoutHandle);
        impl std::task::Wake for WriteOnWake {
            fn wake(self: Arc<Self>) {
                self.0.println("wake");
            }
        }

        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut output = UseOutputImpl::default();
            let stdout = output.use_stdout();
            let stderr = output.use_stderr();
            output.state.lock().unwrap().waker =
                Some(Waker::from(Arc::new(WriteOnWake(stdout.clone()))));
            stdout.println(NestedMessage(stderr));
            let messages: Vec<_> = output
                .state
                .lock()
                .unwrap()
                .queue
                .iter()
                .map(|message| {
                    (
                        matches!(message.stream, Output::Stderr),
                        message.content.to_string(),
                    )
                })
                .collect();
            sender.send(messages).unwrap();
        });
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .expect("reentrant output must not deadlock"),
            vec![
                (true, "inner".to_string()),
                (false, "wake".to_string()),
                (false, "outer".to_string()),
            ]
        );
        worker.join().unwrap();
    }

    #[test]
    fn test_backend_output_is_flushed_on_the_next_update() {
        use crate::prelude::*;
        use futures::{stream, stream::BoxStream, StreamExt};
        use std::io;

        #[derive(Default)]
        struct Trace {
            stdout: Option<StdoutHandle>,
            batches: Vec<Vec<String>>,
            ready_after_flush: Vec<bool>,
        }

        struct RecordingBackend(Arc<Mutex<Trace>>);

        impl TerminalBackend for RecordingBackend {
            fn clear_canvas(&mut self) -> io::Result<()> {
                Ok(())
            }

            fn write_canvas(&mut self, _: Option<&Canvas>, _: &Canvas) -> io::Result<()> {
                Ok(())
            }

            fn print_above(&mut self, messages: &[Passthrough<'_>]) -> io::Result<()> {
                let nested = {
                    let mut trace = self.0.lock().unwrap();
                    trace.batches.push(
                        messages
                            .iter()
                            .map(|message| message.content.to_string())
                            .collect(),
                    );
                    (trace.batches.len() == 1).then(|| trace.stdout.as_ref().unwrap().clone())
                };
                if let Some(stdout) = nested {
                    stdout.println("nested");
                }
                Ok(())
            }

            fn event_stream(
                &mut self,
            ) -> io::Result<BoxStream<'static, io::Result<TerminalEvent>>> {
                Ok(stream::pending().boxed())
            }
        }

        struct FlushProbe {
            output: UseOutputImpl,
            trace: Arc<Mutex<Trace>>,
            updates: usize,
        }

        impl Hook for FlushProbe {
            fn poll_change(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<()> {
                Pin::new(&mut self.get_mut().output).poll_change(cx)
            }

            fn post_component_update(&mut self, updater: &mut ComponentUpdater) {
                self.output.post_component_update(updater);
                let ready = Pin::new(&mut self.output)
                    .poll_change(&mut core::task::Context::from_waker(&noop_waker()))
                    .is_ready();
                self.trace.lock().unwrap().ready_after_flush.push(ready);
            }
        }

        #[derive(Props)]
        struct FlushProps {
            trace: Arc<Mutex<Trace>>,
        }

        #[component]
        fn FlushingComponent(
            mut hooks: Hooks,
            props: &FlushProps,
        ) -> impl Into<AnyElement<'static>> {
            let trace = props.trace.clone();
            let probe = hooks.use_hook(move || {
                let mut output = UseOutputImpl::default();
                let stdout = output.use_stdout();
                trace.lock().unwrap().stdout = Some(stdout.clone());
                stdout.println("outer");
                FlushProbe {
                    output,
                    trace,
                    updates: 0,
                }
            });
            probe.updates += 1;
            if probe.updates == 2 {
                hooks.use_context_mut::<SystemContext>().exit();
            }
            element!(Text(content: "frame"))
        }

        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let trace = Arc::new(Mutex::new(Trace::default()));
            let backend = RecordingBackend(trace.clone());
            let mut element = element!(FlushingComponent(trace: trace.clone()));
            smol::block_on(element.render_loop().backend(backend)).unwrap();
            let mut trace = trace.lock().unwrap();
            sender
                .send((
                    std::mem::take(&mut trace.batches),
                    std::mem::take(&mut trace.ready_after_flush),
                ))
                .unwrap();
        });
        let (batches, ready) = receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("backend output must not deadlock");
        assert_eq!(batches, [vec!["outer"], vec!["nested"]]);
        assert_eq!(ready, [true, false]);
        worker.join().unwrap();
    }
}
