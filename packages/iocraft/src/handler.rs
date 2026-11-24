use core::{mem, ops::Deref};
use std::{ops::DerefMut, sync::Arc};

/// `HandlerMut` is a type representing an optional event handler, commonly used for component properties.
///
/// Use a handler to validate input or respond to a component's actions.
/// A default-initialized handler returns the result type's default value.
///
/// ```
/// # use iocraft::prelude::*;
/// let mut validate: HandlerMut<'_, &str, Option<String>> = HandlerMut::from(|value: &str| {
///     value.trim().is_empty().then(|| "A name is required".to_owned())
/// });
/// assert_eq!(validate(""), Some("A name is required".to_owned()));
/// assert_eq!(validate("Ada"), None);
/// ```
pub struct HandlerMut<'a, T, R = ()>(bool, Box<dyn FnMut(T) -> R + Send + Sync + 'a>);

impl<T, R> HandlerMut<'_, T, R> {
    /// Returns `true` if the handler was default-initialized.
    pub fn is_default(&self) -> bool {
        !self.0
    }

    /// Takes the handler, leaving a default-initialized handler in its place.
    pub fn take(&mut self) -> Self
    where
        R: Default,
    {
        mem::take(self)
    }
}

impl<T, R: Default> Default for HandlerMut<'_, T, R> {
    fn default() -> Self {
        Self(false, Box::new(|_| R::default()))
    }
}

impl<'a, T, R, F> From<F> for HandlerMut<'a, T, R>
where
    F: FnMut(T) -> R + Send + Sync + 'a,
{
    fn from(f: F) -> Self {
        Self(true, Box::new(f))
    }
}

impl<'a, T: 'a, R> Deref for HandlerMut<'a, T, R> {
    type Target = dyn FnMut(T) -> R + Send + Sync + 'a;

    fn deref(&self) -> &Self::Target {
        self.1.as_ref()
    }
}

impl<'a, T: 'a, R> DerefMut for HandlerMut<'a, T, R> {
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
pub struct Handler<T, R = ()>(bool, Arc<dyn Fn(T) -> R + Send + Sync + 'static>);

impl<T, R> Clone for Handler<T, R> {
    fn clone(&self) -> Self {
        Self(self.0, Arc::clone(&self.1))
    }
}

impl<T, R> Handler<T, R> {
    /// Returns `true` if the handler was default-initialized.
    pub fn is_default(&self) -> bool {
        !self.0
    }
}

impl<T, R> Deref for Handler<T, R> {
    type Target = dyn Fn(T) -> R + Send + Sync + 'static;

    fn deref(&self) -> &Self::Target {
        self.1.as_ref()
    }
}

impl<T, R: Default> Default for Handler<T, R> {
    fn default() -> Self {
        Self(false, Arc::new(|_| R::default()))
    }
}

impl<T, R, F> From<F> for Handler<T, R>
where
    F: Fn(T) -> R + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Self(true, Arc::new(f))
    }
}

impl<T: Clone + Send + Sync + 'static, R: 'static> Handler<T, R> {
    /// Creates a new `Handler` that uses a constant value for it's input.
    pub fn bind(&self, value: T) -> Handler<(), R> {
        let handler = self.clone();
        Handler::from(move |_| handler(value.clone()))
    }
}

impl<T: 'static, R: 'static> From<Handler<T, R>> for HandlerMut<'static, T, R> {
    fn from(handler: Handler<T, R>) -> Self {
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
    fn test_take_preserves_returning_handler_state() {
        let mut next = 0;
        let mut handler = HandlerMut::from(move |()| {
            next += 1;
            Some(next)
        });

        assert_eq!(handler(()), Some(1));
        let mut taken = handler.take();
        assert!(handler.is_default());
        assert!(!taken.is_default());
        assert_eq!(handler(()), None);
        assert_eq!(taken(()), Some(2));
    }

    #[test]
    fn test_returning_handler_clones_share_state_without_result_bounds() {
        use std::{
            rc::Rc,
            sync::atomic::{AtomicUsize, Ordering},
        };

        struct ResultValue(Rc<usize>);

        let next = AtomicUsize::new(0);
        let handler =
            Handler::from(move |()| ResultValue(Rc::new(next.fetch_add(1, Ordering::Relaxed))));

        assert_eq!(*handler(()).0, 0);
        let cloned = handler.clone();
        assert_eq!(*cloned(()).0, 1);
        let bound = cloned.bind(());
        assert_eq!(*bound(()).0, 2);
        let mut converted: HandlerMut<'static, (), ResultValue> = handler.into();
        assert_eq!(*converted(()).0, 3);
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
