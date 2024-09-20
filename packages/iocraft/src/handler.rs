/// `Handler` is a type representing an optional event handler, commonly used for component properties.
#[derive(Default)]
pub enum Handler<'a, T> {
    /// No handler is set.
    #[default]
    None,
    /// A function handler.
    Function(Box<dyn FnMut(T) + Send + 'a>),
}

impl<'a, T, F> From<F> for Handler<'a, T>
where
    F: FnMut(T) + Send + 'a,
{
    fn from(f: F) -> Self {
        Self::Function(Box::new(f))
    }
}

impl<'a, T> Handler<'a, T> {
    /// Returns `true` if the handler is not set.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Takes the handler, leaving `None` in its place.
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    /// Invokes the handler with the given value.
    pub fn invoke(&mut self, value: T) {
        match self {
            Self::Function(f) => f(value),
            Self::None => {}
        }
    }
}
