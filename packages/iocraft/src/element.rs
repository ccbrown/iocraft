use crate::{
    component::{Component, ComponentHelper, ComponentHelperExt},
    props::AnyProps,
    render, terminal_render_loop, Canvas,
};
use crossterm::{terminal, tty::IsTty};
use std::{
    future::Future,
    io::{self, stderr, stdout, Write},
    os::fd::AsRawFd,
};

/// Used by the `element!` macro to extend a collection with elements.
#[doc(hidden)]
pub trait ExtendWithElements<T>: Sized {
    fn extend<E: Extend<T>>(self, dest: &mut E);
}

impl<'a, T, U> ExtendWithElements<T> for Element<'a, U>
where
    U: ElementType + 'a,
    T: From<Element<'a, U>>,
{
    fn extend<E: Extend<T>>(self, dest: &mut E) {
        dest.extend([self.into()]);
    }
}

impl<'a> ExtendWithElements<AnyElement<'a>> for AnyElement<'a> {
    fn extend<E: Extend<AnyElement<'a>>>(self, dest: &mut E) {
        dest.extend([self]);
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
pub struct Element<'a, T: ElementType + 'a> {
    pub key: ElementKey,
    pub props: T::Props<'a>,
}

pub trait ElementType {
    type Props<'a>
    where
        Self: 'a;
}

pub struct AnyElement<'a> {
    key: ElementKey,
    props: AnyProps<'a>,
    helper: Box<dyn ComponentHelperExt>,
}

impl<'a, T> Element<'a, T>
where
    T: Component + 'a,
{
    pub fn into_any(self) -> AnyElement<'a> {
        self.into()
    }
}

impl<'a, T> From<Element<'a, T>> for AnyElement<'a>
where
    T: Component + 'a,
{
    fn from(e: Element<'a, T>) -> Self {
        Self {
            key: e.key,
            props: AnyProps::owned(e.props),
            helper: ComponentHelper::<T>::boxed(),
        }
    }
}

impl<'a, T> From<&'a mut Element<'a, T>> for AnyElement<'a>
where
    T: Component,
{
    fn from(e: &'a mut Element<'a, T>) -> Self {
        Self {
            key: e.key.clone(),
            props: AnyProps::borrowed(&mut e.props),
            helper: ComponentHelper::<T>::boxed(),
        }
    }
}

mod private {
    use super::*;

    pub trait Sealed {}
    impl<'a> Sealed for AnyElement<'a> {}
    impl<'a> Sealed for &mut AnyElement<'a> {}
    impl<'a, T> Sealed for Element<'a, T> where T: Component {}
    impl<'a, T> Sealed for &mut Element<'a, T> where T: Component {}
}

pub trait ElementExt: private::Sealed + Sized {
    fn key(&self) -> &ElementKey;
    fn props_mut(&mut self) -> AnyProps;

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt>;

    fn render(&mut self, max_width: Option<usize>) -> Canvas;

    fn into_string(mut self) -> String {
        self.render(None).to_string()
    }

    fn print(self) {
        self.write_to_raw_fd(stdout()).unwrap();
    }

    fn eprint(self) {
        self.write_to_raw_fd(stderr()).unwrap();
    }

    fn write<W: Write>(mut self, w: W) -> io::Result<()> {
        let canvas = self.render(None);
        canvas.write(w)
    }

    fn write_to_raw_fd<F: Write + AsRawFd>(mut self, fd: F) -> io::Result<()> {
        if fd.is_tty() {
            let (width, _) = terminal::size().expect("we should be able to get the terminal size");
            let canvas = self.render(Some(width as _));
            canvas.write_ansi(fd)
        } else {
            self.write(fd)
        }
    }

    fn render_loop(&mut self) -> impl Future<Output = io::Result<()>>;
}

impl<'a> ElementExt for AnyElement<'a> {
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn props_mut(&mut self) -> AnyProps {
        self.props.borrow()
    }

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        self.helper.copy()
    }

    fn render(&mut self, max_width: Option<usize>) -> Canvas {
        render(self, max_width)
    }

    async fn render_loop(&mut self) -> io::Result<()> {
        terminal_render_loop(self).await
    }
}

impl<'a> ElementExt for &mut AnyElement<'a> {
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn props_mut(&mut self) -> AnyProps {
        self.props.borrow()
    }

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        self.helper.copy()
    }

    fn render(&mut self, max_width: Option<usize>) -> Canvas {
        render(&mut **self, max_width)
    }

    async fn render_loop(&mut self) -> io::Result<()> {
        terminal_render_loop(&mut **self).await
    }
}

impl<'a, T> ElementExt for Element<'a, T>
where
    T: Component + 'static,
{
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn props_mut(&mut self) -> AnyProps {
        AnyProps::borrowed(&mut self.props)
    }

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        ComponentHelper::<T>::boxed()
    }

    fn render(&mut self, max_width: Option<usize>) -> Canvas {
        render(self, max_width)
    }

    async fn render_loop(&mut self) -> io::Result<()> {
        terminal_render_loop(self).await
    }
}

impl<'a, T> ElementExt for &mut Element<'a, T>
where
    T: Component + 'static,
{
    fn key(&self) -> &ElementKey {
        &self.key
    }

    fn props_mut(&mut self) -> AnyProps {
        AnyProps::borrowed(&mut self.props)
    }

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt> {
        ComponentHelper::<T>::boxed()
    }

    fn render(&mut self, max_width: Option<usize>) -> Canvas {
        render(&mut **self, max_width)
    }

    async fn render_loop(&mut self) -> io::Result<()> {
        terminal_render_loop(&mut **self).await
    }
}
