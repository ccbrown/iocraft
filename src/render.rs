use std::{
    fmt::{self, Display, Formatter},
    future::Future,
    ops::Deref,
};

#[derive(Clone, Copy)]
pub struct Signal;

pub struct Value<T> {
    signal: Signal,
    value: T,
}

impl<T: Default> Value<T> {
    pub fn new_with_default(signal: Signal) -> Self {
        Self {
            signal,
            value: T::default(),
        }
    }
}

impl<T> Value<T> {
    pub fn set(&mut self, v: T) {
        self.value = v;
    }
}

impl<T> Deref for Value<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Display for Value<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

pub trait Component {
    type State;

    fn new() -> Self;
    fn render(&self);
    fn wait(&mut self) -> impl Future<Output = ()>;
}

pub async fn render<C: Component>() {
    let mut component = C::new();
    loop {
        component.render();
        component.wait().await;
    }
}
