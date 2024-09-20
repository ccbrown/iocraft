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
    did_spawn_once: bool,
    futures: Vec<BoxFuture<'static, ()>>,
}

impl Hook for UseAsync {
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.futures
            .retain_mut(|f| !matches!(f.as_mut().poll(cx), Poll::Ready(())));
        Poll::Pending
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
        if !self.did_spawn_once {
            self.futures.push(Box::pin(f()));
            self.did_spawn_once = true;
        }
    }

    /// Spawns a future which is bound to the lifetime of the component. When the component is
    /// unmounted, the future will be dropped.
    ///
    /// The future will be spawned every time this function is called. If you need to run a future
    /// in the background, use [`spawn_once`](UseAsync::spawn_once) instead.
    pub fn spawn<F, T>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.futures.push(Box::pin(f));
    }
}
