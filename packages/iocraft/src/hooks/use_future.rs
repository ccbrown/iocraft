use crate::{Hook, Hooks};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use futures::future::BoxFuture;

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseFuture` is a hook that allows you to spawn an async task which is bound to the lifetime of
/// the component.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # use std::time::Duration;
/// #[component]
/// fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let mut count = hooks.use_state(|| 0);
///
///     hooks.use_future(async move {
///         loop {
///             smol::Timer::after(Duration::from_millis(100)).await;
///             count += 1;
///         }
///     });
///
///     element! {
///         Text(color: Color::Blue, content: format!("counter: {}", count))
///     }
/// }
/// ```
pub trait UseFuture: private::Sealed {
    /// Spawns a future which is bound to the lifetime of the component. When the component is
    /// dropped, the future will also be dropped.
    ///
    /// The given future will only be spawned once. After that, calling this function has no
    /// effect.
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static;
}

impl UseFuture for Hooks<'_, '_> {
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.use_hook(move || UseFutureImpl::new(f));
    }
}

struct UseFutureImpl {
    f: Option<BoxFuture<'static, ()>>,
}

impl Hook for UseFutureImpl {
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(f) = self.f.as_mut() {
            if let Poll::Ready(()) = f.as_mut().poll(cx) {
                self.f = None;
            }
        }
        Poll::Pending
    }
}

impl UseFutureImpl {
    pub fn new<F>(f: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Self {
            f: Some(Box::pin(f)),
        }
    }
}
