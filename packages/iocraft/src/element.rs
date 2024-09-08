use crate::{
    component::{Component, ComponentHelper, ComponentHelperExt},
    render, terminal_render_loop, Canvas,
};
use crossterm::{terminal, tty::IsTty};
use std::{
    any::Any,
    fmt::{self, Display, Formatter},
    future::Future,
    io::{self, stderr, stdout, Write},
    os::fd::AsRawFd,
};

/// Used by the `element!` macro to extend a collection with elements.
#[doc(hidden)]
pub trait ExtendWithElements<T>: Sized {
    fn extend<E: Extend<T>>(self, dest: &mut E);
}

impl<T, U> ExtendWithElements<T> for Element<U>
where
    U: ElementType + 'static,
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

#[derive(Clone)]
pub struct Element<T: ElementType> {
    pub key: ElementKey,
    pub props: T::Props,
}

impl<T> Display for Element<T>
where
    T: Component + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.render(None).fmt(f)
    }
}

pub trait ElementType {
    type Props;
}

pub struct AnyElement {
    key: ElementKey,
    props: Box<dyn Any>,
    helper: Box<dyn ComponentHelperExt>,
}

impl Display for AnyElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.render(None).fmt(f)
    }
}

impl<T> From<Element<T>> for AnyElement
where
    T: Component + 'static,
{
    fn from(e: Element<T>) -> Self {
        Self {
            key: e.key,
            props: Box::new(e.props),
            helper: ComponentHelper::<T>::boxed(),
        }
    }
}

impl<T> From<&Element<T>> for AnyElement
where
    T: Component + 'static,
    <T as Component>::Props: Clone,
{
    fn from(e: &Element<T>) -> Self {
        Self {
            key: e.key.clone(),
            props: Box::new(e.props.clone()),
            helper: ComponentHelper::<T>::boxed(),
        }
    }
}

mod private {
    use super::*;

    pub trait Sealed {}
    impl Sealed for AnyElement {}
    impl Sealed for &AnyElement {}
    impl<T> Sealed for Element<T> where T: Component + 'static {}
    impl<T> Sealed for &Element<T> where T: Component + 'static {}
}

pub trait ElementExt: private::Sealed + Sized {
    fn key(&self) -> &ElementKey;
    fn props(&self) -> &dyn Any;

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt>;

    fn render(&self, max_width: Option<usize>) -> Canvas;

    fn print(self) {
        self.write_to_raw_fd(stdout()).unwrap();
    }

    fn eprint(self) {
        self.write_to_raw_fd(stderr()).unwrap();
    }

    fn write<W: Write>(self, w: W) -> io::Result<()> {
        let canvas = self.render(None);
        canvas.write(w)
    }

    fn write_to_raw_fd<F: Write + AsRawFd>(self, fd: F) -> io::Result<()> {
        if fd.is_tty() {
            let (width, _) = terminal::size().expect("we should be able to get the terminal size");
            let canvas = self.render(Some(width as _));
            canvas.write_ansi(fd)
        } else {
            self.write(fd)
        }
    }

    fn render_loop(&self) -> impl Future<Output = io::Result<()>>;
}

impl ElementExt for AnyElement {
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn props(&self) -> &dyn Any {
        &self.props
    }

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        self.helper.copy()
    }

    fn render(&self, max_width: Option<usize>) -> Canvas {
        render(self, max_width)
    }

    async fn render_loop(&self) -> io::Result<()> {
        terminal_render_loop(self).await
    }
}

impl ElementExt for &AnyElement {
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn props(&self) -> &dyn Any {
        self.props.as_ref()
    }

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        self.helper.copy()
    }

    fn render(&self, max_width: Option<usize>) -> Canvas {
        render(*self, max_width)
    }

    async fn render_loop(&self) -> io::Result<()> {
        terminal_render_loop(*self).await
    }
}

impl<T> ElementExt for Element<T>
where
    T: Component + 'static,
{
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn props(&self) -> &dyn Any {
        &self.props
    }

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        ComponentHelper::<T>::boxed()
    }

    fn render(&self, max_width: Option<usize>) -> Canvas {
        render(self, max_width)
    }

    async fn render_loop(&self) -> io::Result<()> {
        terminal_render_loop(self).await
    }
}

impl<T> ElementExt for &Element<T>
where
    T: Component + 'static,
{
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn props(&self) -> &dyn Any {
        &self.props
    }

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        ComponentHelper::<T>::boxed()
    }

    fn render(&self, max_width: Option<usize>) -> Canvas {
        render(*self, max_width)
    }

    async fn render_loop(&self) -> io::Result<()> {
        terminal_render_loop(*self).await
    }
}
