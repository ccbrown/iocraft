use crate::{
    component::{AnyComponentProps, Component, ComponentProps},
    render::Tree,
};
use std::future::Future;

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

pub trait ElementExt {
    fn print(self);
    fn render(self) -> impl Future<Output = ()>;
}

impl<T: Into<AnyElement>> ElementExt for T {
    fn print(self) {
        let mut tree = Tree::new(self.into());
        tree.render();
        println!("");
    }

    async fn render(self) {
        let mut tree = Tree::new(self.into());
        tree.render_loop().await;
    }
}
