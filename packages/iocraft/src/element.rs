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

/// Used to identify an element within the scope of its parent. This is used to minimize the number
/// of times components are destroyed and recreated from render-to-render.
#[derive(Clone, Hash, PartialEq, Eq, Debug, derive_more::Display)]
pub struct ElementKey(uuid::Uuid);

impl Default for ElementKey {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementKey {
    /// Constructs a new, random element key.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// An element is a description of an uninstantiated components, including its key and properties.
#[derive(Clone)]
pub struct Element<'a, T: ElementType + 'a> {
    /// The key of the element.
    pub key: ElementKey,
    /// The properties of the element.
    pub props: T::Props<'a>,
}

/// A trait implemented by all element types to define the properties that can be passed to them.
///
/// This trait is automatically implemented for all types that implement [`Component`].
pub trait ElementType {
    /// The type of the properties that can be passed to the element.
    type Props<'a>
    where
        Self: 'a;
}

/// A type-erased element that can be created from any [`Element`].
pub struct AnyElement<'a> {
    key: ElementKey,
    props: AnyProps<'a>,
    helper: Box<dyn ComponentHelperExt>,
}

impl<'a, T> Element<'a, T>
where
    T: Component + 'a,
{
    /// Converts the element into an [`AnyElement`].
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

impl<'a, 'b: 'a, T> From<&'a mut Element<'b, T>> for AnyElement<'a>
where
    T: Component,
{
    fn from(e: &'a mut Element<'b, T>) -> Self {
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

/// A trait implemented by all element types, providing methods for common operations on them.
pub trait ElementExt: private::Sealed + Sized {
    /// Returns the key of the element.
    fn key(&self) -> &ElementKey;

    #[doc(hidden)]
    fn props_mut(&mut self) -> AnyProps;

    #[doc(hidden)]
    fn helper(&self) -> Box<dyn ComponentHelperExt>;

    /// Renders the element into a canvas.
    fn render(&mut self, max_width: Option<usize>) -> Canvas;

    /// Renders the element into a string.
    ///
    /// Note that unlike [`std::fmt::Display`] and [`std::string::ToString`], this method requires
    /// the element to be mutable, as it's possible for the properties of the element to change
    /// during rendering.
    fn to_string(&mut self) -> String {
        self.render(None).to_string()
    }

    /// Renders the element and prints it to stdout.
    fn print(&mut self) {
        self.write_to_raw_fd(stdout()).unwrap();
    }

    /// Renders the element and prints it to stderr.
    fn eprint(&mut self) {
        self.write_to_raw_fd(stderr()).unwrap();
    }

    /// Renders the element and writes it to the given writer.
    fn write<W: Write>(&mut self, w: W) -> io::Result<()> {
        let canvas = self.render(None);
        canvas.write(w)
    }

    /// Renders the element and writes it to the given raw file descriptor. If the file descriptor
    /// is a TTY, the canvas will be rendered based on its size, with ANSI escape codes.
    fn write_to_raw_fd<F: Write + AsRawFd>(&mut self, fd: F) -> io::Result<()> {
        if fd.is_tty() {
            let (width, _) = terminal::size().expect("we should be able to get the terminal size");
            let canvas = self.render(Some(width as _));
            canvas.write_ansi(fd)
        } else {
            self.write(fd)
        }
    }

    /// Renders the element in a loop, allowing it to be dynamic and interactive.
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
        terminal_render_loop(self, stdout()).await
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
        terminal_render_loop(&mut **self, stdout()).await
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
        terminal_render_loop(self, stdout()).await
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
        terminal_render_loop(&mut **self, stdout()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn test_element() {
        let mut box_element = element!(Box);
        box_element.key();
        box_element.print();
        box_element.eprint();
        (&mut box_element).key();
        (&mut box_element).print();
        (&mut box_element).eprint();

        let mut any_element: AnyElement<'static> = box_element.into_any();
        any_element.key();
        any_element.print();
        any_element.eprint();
        (&mut any_element).key();
        (&mut any_element).print();
        (&mut any_element).eprint();

        let mut box_element = element!(Box);
        let mut any_element_ref: AnyElement = (&mut box_element).into();
        any_element_ref.print();
        any_element_ref.eprint();
    }
}
