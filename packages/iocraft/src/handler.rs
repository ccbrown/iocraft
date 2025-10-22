use core::{mem, ops::Deref};
use std::{ops::DerefMut, sync::Arc};

/// `HandlerMut` is a type representing an optional event handler, commonly used for component properties.
///
/// Any function that takes a single argument and returns `()` can be converted into a `HandlerMut`,
/// and it can be invoked using function call syntax.
pub struct HandlerMut<'a, T>(bool, Box<dyn FnMut(T) + Send + Sync + 'a>);

impl<T> HandlerMut<'_, T> {
    /// Returns `true` if the handler was default-initialized.
    pub fn is_default(&self) -> bool {
        !self.0
    }

    /// Takes the handler, leaving a default-initialized handler in its place.
    pub fn take(&mut self) -> Self {
        mem::take(self)
    }
}

impl<T> Default for HandlerMut<'_, T> {
    fn default() -> Self {
        Self(false, Box::new(|_| {}))
    }
}

impl<'a, T, F> From<F> for HandlerMut<'a, T>
where
    F: FnMut(T) + Send + Sync + 'a,
{
    fn from(f: F) -> Self {
        Self(true, Box::new(f))
    }
}

impl<'a, T: 'a> Deref for HandlerMut<'a, T> {
    type Target = dyn FnMut(T) + Send + Sync + 'a;

    fn deref(&self) -> &Self::Target {
        self.1.as_ref()
    }
}

impl<'a, T: 'a> DerefMut for HandlerMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1.as_mut()
    }
}

/// Immutable variant of [`HandlerMut`]: it lacks ability to mutate captured variables, but can be cloned.
#[derive(Clone)]
pub struct Handler<T>(bool, Arc<dyn Fn(T) + Send + Sync + 'static>);

impl<T> Handler<T> {
    /// Returns `true` if the handler was default-initialized.
    pub fn is_default(&self) -> bool {
        !self.0
    }
}

impl<T> Deref for Handler<T> {
    type Target = dyn Fn(T) + Send + Sync + 'static;

    fn deref(&self) -> &Self::Target {
        self.1.as_ref()
    }
}

impl<T> Default for Handler<T> {
    fn default() -> Self {
        Self(false, Arc::new(|_| {}))
    }
}

impl<T, F> From<F> for Handler<T>
where
    F: Fn(T) + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Self(true, Arc::new(f))
    }
}

impl<T: Clone + Send + Sync + 'static> Handler<T> {
    /// Creates a new `Handler` that uses a constant value for it's input.
    pub fn bind(&self, value: T) -> Handler<()> {
        let handler = self.clone();
        Handler::from(move |_| handler(value.clone()))
    }
}

impl<T: Clone + Send + Sync + 'static> From<Handler<T>> for HandlerMut<'static, T> {
    fn from(handler: Handler<T>) -> Self {
        Self::from(move |value| handler.1(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler() {
        let mut handler = HandlerMut::<i32>::default();
        handler(0);
        handler(0);
        assert!(handler.is_default());

        let mut handler = HandlerMut::from(|value| {
            assert_eq!(value, 42);
        });
        handler(42);
        handler(42);
        assert!(!handler.is_default());
    }

    #[test]
    fn test_async_handler() {
        let handler = Handler::<i32>::default();
        handler(0);
        handler(0);
        assert!(handler.is_default());

        let handler = Handler::from(|value| {
            assert_eq!(value, 42);
        });
        handler(42);
        let binded = handler.bind(42);
        binded(());
        assert!(!handler.is_default());
    }
}
