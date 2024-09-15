use crate::Hook;
use futures::future::BoxFuture;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// `UseFuture` is a hook that allows you to spawn a future which is bound to the lifetime of the
/// component. When the component is unmounted, the future will be dropped.
#[derive(Default)]
pub struct UseFuture {
    f: Option<BoxFuture<'static, ()>>,
}

impl Hook for UseFuture {
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        match self.f.as_mut() {
            Some(f) => f.as_mut().poll(cx),
            None => Poll::Pending,
        }
    }
}

impl UseFuture {
    /// Spawns a future which is bound to the lifetime of the component. When the component is
    /// unmounted, the future will be dropped.
    ///
    /// The future will be spawned only once. After that, calling this function has no effect.
    pub fn use_future<F, T>(&mut self, f: F)
    where
        F: FnOnce() -> T,
        T: Future<Output = ()> + Send + 'static,
    {
        if self.f.is_none() {
            self.f = Some(Box::pin(f()));
        }
    }
}
