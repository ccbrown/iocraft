use crate::{
    component::{AnyComponentProps, ComponentProps},
    render::Tree,
};
use std::future::Future;

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
    T: ElementType + 'static,
    <T as ElementType>::Props: ComponentProps + Clone,
{
    fn from(e: Element<T>) -> Self {
        Self {
            key: e.key,
            props: Box::new(e.props),
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
