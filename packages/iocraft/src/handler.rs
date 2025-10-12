use core::{mem, ops::Deref};
use std::{ops::DerefMut, sync::Arc};

/// `Handler` is a type representing an optional event handler, commonly used for component properties.
///
/// Any function that takes a single argument and returns `()` can be converted into a `Handler`,
/// and it can be invoked using function call syntax.
pub struct Handler<'a, T>(bool, Box<dyn FnMut(T) + Send + Sync + 'a>);

impl<T> Handler<'_, T> {
    /// Returns `true` if the handler was default-initialized.
    pub fn is_default(&self) -> bool {
        !self.0
    }

    /// Takes the handler, leaving a default-initialized handler in its place.
    pub fn take(&mut self) -> Self {
        mem::take(self)
    }
}

impl<T> Default for Handler<'_, T> {
    fn default() -> Self {
        Self(false, Box::new(|_| {}))
    }
}

impl<'a, T, F> From<F> for Handler<'a, T>
where
    F: FnMut(T) + Send + Sync + 'a,
{
    fn from(f: F) -> Self {
        Self(true, Box::new(f))
    }
}

impl<'a, T: 'a> Deref for Handler<'a, T> {
    type Target = dyn FnMut(T) + Send + Sync + 'a;

    fn deref(&self) -> &Self::Target {
        self.1.as_ref()
    }
}

impl<'a, T: 'a> DerefMut for Handler<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1.as_mut()
    }
}

/// Immutable variant of [`Handler`]: it lacks function to mutate captured values, but can be cloned.
#[derive(Clone)]
pub struct RefHandler<T>(bool, Arc<dyn Fn(T) + Send + Sync + 'static>);

impl<T> RefHandler<T> {
    /// Returns `true` if the handler was default-initialized.
    pub fn is_default(&self) -> bool {
        !self.0
    }
}

impl<T> Deref for RefHandler<T> {
    type Target = dyn Fn(T) + Send + Sync + 'static;

    fn deref(&self) -> &Self::Target {
        self.1.as_ref()
    }
}

impl<T> Default for RefHandler<T> {
    fn default() -> Self {
        Self(false, Arc::new(|_| {}))
    }
}

impl<T, F> From<F> for RefHandler<T>
where
    F: Fn(T) + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Self(true, Arc::new(f))
    }
}

impl<T: Clone + Send + Sync + 'static> RefHandler<T> {
    /// Creates a new async handler that uses a constant value for it's input.
    pub fn bind(&self, value: T) -> RefHandler<()> {
        let handler = self.clone();
        RefHandler::from(move |_| handler(value.clone()))
    }
}

impl<T: Clone + Send + Sync + 'static> From<RefHandler<T>> for Handler<'static, T> {
    fn from(handler: RefHandler<T>) -> Self {
        Self::from(move |value| handler.1.clone()(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler() {
        let mut handler = Handler::<i32>::default();
        handler(0);
        handler(0);
        assert!(handler.is_default());

        let mut handler = Handler::from(|value| {
            assert_eq!(value, 42);
        });
        handler(42);
        handler(42);
        assert!(!handler.is_default());
    }

    #[test]
    fn test_async_handler() {
        let handler = RefHandler::<i32>::default();
        handler(0);
        handler(0);
        assert!(handler.is_default());

        let handler = RefHandler::from(|value| {
            assert_eq!(value, 42);
        });
        handler(42);
        let binded = handler.bind(42);
        binded(());
        assert!(!handler.is_default());
    }
}
