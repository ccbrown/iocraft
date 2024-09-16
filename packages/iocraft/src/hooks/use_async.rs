use crate::Hook;
use futures::future::BoxFuture;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// `UseAsync` is a hook that allows you to spawn async tasks bound to the lifetime of the
/// component. When the component is unmounted, the tasks will be dropped.
#[derive(Default)]
pub struct UseAsync {
    once_fut: Option<BoxFuture<'static, ()>>,
}

impl Hook for UseAsync {
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        match self.once_fut.as_mut() {
            Some(f) => f.as_mut().poll(cx),
            None => Poll::Pending,
        }
    }
}

impl UseAsync {
    /// Spawns a future which is bound to the lifetime of the component. When the component is
    /// unmounted, the future will be dropped.
    ///
    /// The given function will only be invoked once. After that, calling this function has no
    /// effect.
    pub fn spawn_once<F, T>(&mut self, f: F)
    where
        F: FnOnce() -> T,
        T: Future<Output = ()> + Send + 'static,
    {
        if self.once_fut.is_none() {
            self.once_fut = Some(Box::pin(f()));
        }
    }
}
