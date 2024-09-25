use std::ops::{Deref, DerefMut};

/// `Handler` is a type representing an optional event handler, commonly used for component properties.
///
/// Any function that takes a single argument and returns `()` can be converted into a `Handler`,
/// and it can be invoked using function call syntax.
pub struct Handler<'a, T>(Box<dyn FnMut(T) + Send + 'a>);

impl<'a, T> Default for Handler<'a, T> {
    fn default() -> Self {
        Self::from(|_| {})
    }
}

impl<'a, T, F> From<F> for Handler<'a, T>
where
    F: FnMut(T) + Send + 'a,
{
    fn from(f: F) -> Self {
        Self(Box::new(f))
    }
}

impl<'a, T: 'a> Deref for Handler<'a, T> {
    type Target = dyn FnMut(T) + Send + 'a;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: 'a> DerefMut for Handler<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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

        let mut handler = Handler::from(|value| {
            assert_eq!(value, 42);
        });
        handler(42);
        handler(42);
    }
}
