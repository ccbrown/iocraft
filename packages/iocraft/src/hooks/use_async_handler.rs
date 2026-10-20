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
/// If the component is dropped, all executing tasks will also be dropped.
/// Keeping a handler after its component has been dropped does not keep its tasks alive.
/// Invoking the handler after that point has no effect.
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
            let mut state = handler_impl_state
                .lock()
                .expect("we should be able to lock the mutex");
            state.futures.push(Box::pin(f(value)));
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
}

impl Hook for UseAsyncHandlerImpl {
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self
            .state
            .lock()
            .expect("we should be able to lock the mutex");

        state
            .futures
            .retain_mut(|f| !f.as_mut().poll(cx).is_ready());
        state.waker = Some(cx.waker().clone());

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
        Arc, Mutex,
    };

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
