use crate::{
    component::{AnyComponentProps, Component, ComponentProps},
    render, terminal_render_loop, Canvas,
};
use crossterm::terminal;
use std::{
    fmt::{self, Display, Formatter},
    future::Future,
    io::{self, stdout},
};

/// Used by the `element!` macro to extend a collection with elements.
#[doc(hidden)]
pub trait ExtendWithElements<T>: Sized {
    fn extend<E: Extend<T>>(self, dest: &mut E);
}

impl<T, U> ExtendWithElements<T> for Element<U>
where
    U: Component + 'static,
    <U as Component>::Props: Clone + Send,
    T: From<Element<U>>,
{
    fn extend<E: Extend<T>>(self, dest: &mut E) {
        dest.extend([self.into()]);
    }
}

impl<T, U, I> ExtendWithElements<T> for I
where
    I: IntoIterator<Item = U>,
    U: Into<T>,
{
    fn extend<E: Extend<T>>(self, dest: &mut E) {
        dest.extend(self.into_iter().map(|e| e.into()));
    }
}

/// Used by the `element!` macro to extend a collection with elements.
#[doc(hidden)]
pub fn extend_with_elements<T, U, E>(dest: &mut T, elements: U)
where
    T: Extend<E>,
    U: ExtendWithElements<E>,
{
    elements.extend(dest);
}

#[derive(Clone, Hash, PartialEq, Eq, Debug, derive_more::Display)]
pub struct ElementKey(uuid::Uuid);

impl ElementKey {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

pub struct Element<T: ElementType> {
    pub key: ElementKey,
    pub props: T::Props,
}

impl<T> Display for Element<T>
where
    T: Component + 'static,
    <T as Component>::Props: Clone + Send,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.render(None).fmt(f)
    }
}

pub trait ElementType {
    type Props;
}

#[derive(Clone)]
pub struct AnyElement {
    key: ElementKey,
    props: Box<dyn AnyComponentProps>,
}

impl AnyElement {
    pub(crate) fn into_key_and_props(self) -> (ElementKey, Box<dyn AnyComponentProps>) {
        (self.key, self.props)
    }
}

impl Display for AnyElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.render(None).fmt(f)
    }
}

impl<T> From<Element<T>> for AnyElement
where
    T: Component + 'static,
    <T as Component>::Props: Clone + Send,
{
    fn from(e: Element<T>) -> Self {
        Self {
            key: e.key,
            props: Box::new(ComponentProps::<T>(e.props)),
        }
    }
}

impl<T> From<&Element<T>> for AnyElement
where
    T: Component + 'static,
    <T as Component>::Props: Clone + Send,
{
    fn from(e: &Element<T>) -> Self {
        Self {
            key: e.key.clone(),
            props: Box::new(ComponentProps::<T>(e.props.clone())),
        }
    }
}

impl From<&AnyElement> for AnyElement {
    fn from(e: &AnyElement) -> Self {
        e.clone()
    }
}

pub trait ElementExt: Sized {
    fn render(&self, max_width: Option<usize>) -> Canvas;

    fn print(&self) {
        let (width, _) = terminal::size().expect("we should be able to get the terminal size");
        let canvas = self.render(Some(width as _));
        canvas
            .write_ansi(stdout())
            .expect("we should be able to write to stdout");
    }

    fn write<W: io::Write>(&self, w: W) -> io::Result<()> {
        let canvas = self.render(None);
        canvas.write(w)
    }

    fn render_loop(&self) -> impl Future<Output = ()>;
}

impl<T> ElementExt for T
where
    AnyElement: for<'a> From<&'a T>,
{
    fn render(&self, max_width: Option<usize>) -> Canvas {
        render(self, max_width)
    }

    async fn render_loop(&self) {
        terminal_render_loop(self).await;
    }
}
