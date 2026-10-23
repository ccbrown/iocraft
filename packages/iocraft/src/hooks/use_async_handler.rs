use crate::{Handler, Hook, Hooks};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use futures::future::BoxFuture;
use std::sync::{Arc, Mutex};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseAsyncHandler` is a hook that allows you to create a [`Handler`] which executes an
/// asynchronous task that is bound to the lifetime of the component.
///
/// If the component is dropped, all executing tasks will also be dropped, and invoking the
/// handler after that point has no effect.
pub trait UseAsyncHandler: private::Sealed {
    /// Returns a [`Handler`] which when invoked will execute the given function and drive the
    /// resulting future to completion.
    fn use_async_handler<T, Fun, Fut>(&mut self, f: Fun) -> Handler<T>
    where
        Fun: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static;
}

impl UseAsyncHandler for Hooks<'_, '_> {
    fn use_async_handler<T, Fun, Fut>(&mut self, f: Fun) -> Handler<T>
    where
        Fun: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_impl_state = Arc::downgrade(&self.use_hook(UseAsyncHandlerImpl::default).state);
        Handler::<T>::from(move |value| {
            let Some(handler_impl_state) = handler_impl_state.upgrade() else {
                return;
            };
            let future = Box::pin(f(value));
            let mut state = handler_impl_state
                .lock()
                .expect("we should be able to lock the mutex");
            state.futures.push(future);
            if let Some(waker) = &state.waker {
                waker.wake_by_ref();
            }
        })
    }
}

#[derive(Default)]
struct UseAsyncHandlerState {
    futures: Vec<BoxFuture<'static, ()>>,
    waker: Option<Waker>,
}

#[derive(Default)]
struct UseAsyncHandlerImpl {
    state: Arc<Mutex<UseAsyncHandlerState>>,
    futures: Vec<BoxFuture<'static, ()>>,
}

impl Hook for UseAsyncHandlerImpl {
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        {
            let mut state = this
                .state
                .lock()
                .expect("we should be able to lock the mutex");
            state.waker = Some(cx.waker().clone());
            if !state.futures.is_empty() {
                if this.futures.is_empty() {
                    std::mem::swap(&mut this.futures, &mut state.futures);
                } else {
                    this.futures.append(&mut state.futures);
                }
            }
        }
        this.futures
            .retain_mut(|future| !future.as_mut().poll(cx).is_ready());

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use futures::stream::StreamExt;
    use macro_rules_attribute::apply;
    use smol_macros::test;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex, OnceLock,
    };
    use std::time::Duration;

    #[derive(Props)]
    struct ReentrantHandlerProps {
        factory: bool,
        #[iocraft(required)]
        slot: Arc<OnceLock<Handler<usize>>>,
        #[iocraft(required)]
        completed: Arc<AtomicUsize>,
        #[iocraft(required)]
        release: smol::channel::Sender<()>,
        #[iocraft(required)]
        released: smol::channel::Receiver<()>,
    }

    #[component]
    fn ReentrantHandler(
        mut hooks: Hooks,
        props: &ReentrantHandlerProps,
    ) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let done = hooks.use_state(|| false);
        let weak_slot = Arc::downgrade(&props.slot);
        let completed = props.completed.clone();
        let release = props.release.clone();
        let released = props.released.clone();
        let factory = props.factory;
        let handler = hooks.use_async_handler(move |value: usize| {
            let weak_slot = weak_slot.clone();
            let completed = completed.clone();
            let release = release.clone();
            let released = released.clone();
            let mut done = done;
            if factory && value > 0 {
                weak_slot.upgrade().unwrap().get().unwrap()(value - 1);
            }
            async move {
                if value == 0 {
                    release.send(()).await.unwrap();
                    release.send(()).await.unwrap();
                } else {
                    if !factory {
                        weak_slot.upgrade().unwrap().get().unwrap()(value - 1);
                    }
                    released.recv().await.unwrap();
                }
                if completed.fetch_add(1, Ordering::SeqCst) + 1 == 3 {
                    done.set(true);
                }
            }
        });
        props.slot.get_or_init(|| handler);
        let slot = props.slot.clone();
        hooks.use_effect(move || slot.get().unwrap()(2), ());
        if done.get() {
            system.exit();
        }
        element!(Text(content: format!("completed={}", props.completed.load(Ordering::SeqCst))))
    }

    #[test]
    fn test_reentrant_async_handlers_complete_pending_parents() {
        for factory in [false, true] {
            let (finished, result) = mpsc::channel();
            let worker = std::thread::spawn(move || {
                let completed = Arc::new(AtomicUsize::new(0));
                let slot = Arc::new(OnceLock::new());
                let (release, released) = smol::channel::unbounded::<()>();
                smol::block_on(
                    element!(ReentrantHandler(
                        factory: factory,
                        slot: slot,
                        completed: completed.clone(),
                        release: release,
                        released: released,
                    ))
                    .mock_terminal_render_loop(Default::default())
                    .for_each(|_| async {}),
                );
                finished.send(completed.load(Ordering::SeqCst)).unwrap();
            });
            assert_eq!(
                result.recv_timeout(Duration::from_secs(10)).unwrap(),
                3,
                "factory re-entry: {factory}",
            );
            worker.join().unwrap();
        }
    }

    struct ActiveTask(Arc<AtomicUsize>);

    impl Drop for ActiveTask {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    #[derive(Default, Props)]
    struct CancellationProps {
        handler: Arc<Mutex<Option<Handler<()>>>>,
        active_tasks: Arc<AtomicUsize>,
    }

    #[component]
    fn Cancellation(mut hooks: Hooks, props: &CancellationProps) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let started = hooks.use_state(|| false);
        let active_tasks = props.active_tasks.clone();
        let handler = hooks.use_async_handler(move |()| {
            active_tasks.fetch_add(1, Ordering::SeqCst);
            let guard = ActiveTask(active_tasks.clone());
            let mut started = started;
            std::future::poll_fn(move |_| {
                let _guard = &guard;
                started.set(true);
                std::task::Poll::Pending
            })
        });
        *props.handler.lock().unwrap() = Some(handler);
        let external_handler = props.handler.clone();
        hooks.use_effect(
            move || external_handler.lock().unwrap().as_mut().unwrap()(()),
            (),
        );
        if started.get() {
            assert_eq!(props.active_tasks.load(Ordering::SeqCst), 1);
            system.exit();
        }
        element!(View)
    }

    #[apply(test!)]
    async fn test_retained_handler_does_not_outlive_component_tasks() {
        let handler = Arc::new(Mutex::new(None));
        let active_tasks = Arc::new(AtomicUsize::new(0));
        element!(Cancellation(handler: handler.clone(), active_tasks: active_tasks.clone()))
            .mock_terminal_render_loop(Default::default())
            .for_each(|_| async {})
            .await;

        assert_eq!(active_tasks.load(Ordering::SeqCst), 0);
        handler.lock().unwrap().as_mut().unwrap()(());
        assert_eq!(active_tasks.load(Ordering::SeqCst), 0);
    }

    #[derive(Props)]
    struct CallbackCompletionProps {
        #[iocraft(required)]
        handler: Arc<Mutex<Option<Handler<()>>>>,
    }

    #[component]
    fn CallbackCompletion(
        mut hooks: Hooks,
        props: &CallbackCompletionProps,
    ) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut completed = hooks.use_state(|| false);
        let handler = hooks.use_async_handler(move |()| async move {
            completed.set(true);
        });
        *props.handler.lock().unwrap() = Some(handler);
        if completed.get() {
            system.exit();
        }
        element!(Text(content: if completed.get() { "completed" } else { "waiting" }))
    }

    #[apply(test!)]
    async fn test_async_handler_wakes_an_already_waiting_component() {
        let handler = Arc::new(Mutex::new(None));
        let mut app = element!(CallbackCompletion(handler: handler.clone()));
        let mut output = app.mock_terminal_render_loop(Default::default());
        assert_eq!(output.next().await.unwrap().to_string().trim(), "waiting");

        let mut next = std::pin::pin!(output.next());
        assert!(futures::poll!(next.as_mut()).is_pending());
        handler.lock().unwrap().as_ref().unwrap()(());
        assert_eq!(next.await.unwrap().to_string().trim(), "completed");
    }
}
