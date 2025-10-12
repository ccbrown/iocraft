use crate::{Hook, Hooks, RefHandler};
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
pub trait UseAsyncHandler: private::Sealed {
    /// Returns a [`Handler`] which when invoked will execute the given function and drive the
    /// resulting future to completion.
    fn use_async_handler<T, Fun, Fut>(&mut self, f: Fun) -> RefHandler<T>
    where
        Fun: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static;
}

impl UseAsyncHandler for Hooks<'_, '_> {
    fn use_async_handler<T, Fun, Fut>(&mut self, f: Fun) -> RefHandler<T>
    where
        Fun: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler_impl_state = self.use_hook(UseAsyncHandlerImpl::default).state.clone();
        RefHandler::<T>::from(move |value| {
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

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut should_exit = hooks.use_state(|| false);
        let exit = hooks.use_async_handler(move |_| async move {
            should_exit.set(true);
        });

        if should_exit.get() {
            system.exit();
        } else {
            exit(());
        }

        element!(View)
    }

    #[apply(test!)]
    async fn test_use_async_handler() {
        let canvases: Vec<_> = element!(MyComponent)
            .mock_terminal_render_loop(Default::default())
            .collect()
            .await;
        assert_eq!(canvases.len(), 1);
    }
}
