use std::{
    mem,
    ops::{Deref, DerefMut},
};

/// `Handler` is a type representing an optional event handler, commonly used for component properties.
///
/// Any function that takes a single argument and returns `()` can be converted into a `Handler`,
/// and it can be invoked using function call syntax.
pub struct Handler<'a, T>(bool, Box<dyn FnMut(T) + Send + Sync + 'a>);

impl<'a, T> Handler<'a, T> {
    /// Returns `true` if the handler was default-initialized.
    pub fn is_default(&self) -> bool {
        !self.0
    }

    /// Takes the handler, leaving a default-initialized handler in its place.
    pub fn take(&mut self) -> Self {
        mem::take(self)
    }
}

impl<'a, T> Default for Handler<'a, T> {
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
        &self.1
    }
}

impl<'a, T: 'a> DerefMut for Handler<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
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
}
