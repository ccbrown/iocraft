use crate::{Hook, Hooks};
use futures::future::BoxFuture;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// `UseFuture` is a hook that allows you to spawn an async task which is bound to the lifetime of
/// the component.
pub trait UseFuture {
    /// Spawns a future which is bound to the lifetime of the component. When the component is
    /// dropped, the future will also be dropped.
    ///
    /// The given future will only be spawned once. After that, calling this function has no
    /// effect.
    fn use_future<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static;
}

impl UseFuture for Hooks<'_> {
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
