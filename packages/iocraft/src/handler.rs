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

/// Immutable event handler, which lacks ability to mutate captured variables, but can be cloned.
///
/// For component properties consider using [`HandlerMut`] over this, as [`Handler`] is more restrictive.
/// Just as [`Fn`] can be used where an [`FnMut`] is expected, [`Handler`] can be used where a [`HandlerMut`]
/// is expected via [`From`] or [`Handler::bind`].
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # fn foo(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
/// let mut counter = hooks.use_state(|| 0_i32);
///
/// let counter_handler: Handler<_> = hooks.use_async_handler(move |n| async move {
///     counter += n;
/// });
///
/// element! {
///     Fragment {
///         Button(handler: counter_handler.bind(1), has_focus: true) {
///             Text(content: "[ +1 ]")
///         }
///         Button(handler: counter_handler.bind(-1)) {
///             Text(content: "[ -1 ]")
///         }
///     }
/// }
/// # }
/// ```
pub struct Handler<T>(bool, Arc<dyn Fn(T) + Send + Sync + 'static>);

impl<T> Clone for Handler<T> {
    fn clone(&self) -> Self {
        Self(self.0, Arc::clone(&self.1))
    }
}

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

impl<T: 'static> From<Handler<T>> for HandlerMut<'static, T> {
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

    #[test]
    fn test_non_clone_non_send_payload_can_clone_and_convert() {
        use std::{rc::Rc, sync::Mutex};

        struct Payload(Rc<usize>);

        let calls = Arc::new(Mutex::new(Vec::new()));
        let callback_calls = calls.clone();
        let handler = Handler::from(move |payload: Payload| {
            callback_calls.lock().unwrap().push(*payload.0);
        });
        let mut mutable: HandlerMut<'static, Payload> = handler.clone().into();

        handler(Payload(Rc::new(7)));
        mutable(Payload(Rc::new(11)));

        assert_eq!(*calls.lock().unwrap(), [7, 11]);
    }
}
